use fabstep_app::{convert_document, convert_stl, ConversionReport, InputFormat};
use fabstep_core::{Options, ReconstructionMode, RoundedBoxFaceRef};
use serde_json::Value;
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
    .unwrap()
}
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}
fn reference(report: &ConversionReport, expected: &Value) {
    assert_eq!(report.cad.solids, 1);
    assert_eq!(
        (
            report.cad.faces,
            report.cad.planar_faces,
            report.cad.cylindrical_faces,
            report.cad.spherical_faces
        ),
        (26, 6, 12, 8)
    );
    assert_eq!(report.cad.conical_faces + report.cad.toroidal_faces, 0);
    // Nominal-reference tolerance accounts for the STL f32 quantization.
    // convert_stl separately requires CAD volume to match the fitted model at 1e-8 relative.
    let volume = expected["volume_mm3"].as_f64().unwrap();
    assert!(
        (report.cad.volume_mm3 - volume).abs() < 0.001,
        "{} vs {volume}",
        report.cad.volume_mm3
    );
    for end in 0..2 {
        for axis in 0..3 {
            assert!(
                (report.cad.bounds_mm[end][axis]
                    - expected["bounds_mm"][end][axis].as_f64().unwrap())
                .abs()
                    < 1e-5
            );
        }
    }
    let recognized = report.rounded_box_recognition.as_ref().unwrap();
    assert!((recognized.model.radius_mm - 2.).abs() < 1e-5);
    assert_eq!(
        recognized.triangle_faces.len(),
        report.analysis.mesh.triangles.len()
    );
    let mut kinds = [0usize; 3];
    for owner in &recognized.triangle_faces {
        kinds[match owner {
            RoundedBoxFaceRef::Plane { .. } => 0,
            RoundedBoxFaceRef::Cylinder { .. } => 1,
            RoundedBoxFaceRef::Sphere { .. } => 2,
        }] += 1;
    }
    assert!(kinds.iter().all(|&n| n > 0));
    assert!(report.rounded_recognition.is_none());
    let audit = report.fidelity.as_ref().unwrap();
    assert!(audit.source_samples > 0 && audit.result_samples > 0);
    assert!(audit.source_to_result_mm < options().max_deviation_mm);
    assert!(audit.result_to_source_mm < options().max_deviation_mm);
}

#[test]
fn rounded_boxes_reimport_all_26_surfaces_and_preserve_source_provenance() {
    let manifest: Value = serde_json::from_slice(&fixture("m6a_manifest.json")).unwrap();
    for name in ["m6a_rounded_box", "m6a_rounded_box_rotated"] {
        let result = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        reference(&result.report, &manifest["cases"][name]);
        assert_eq!(
            std::str::from_utf8(&result.step_bytes)
                .unwrap()
                .matches("SPHERICAL_SURFACE(")
                .count(),
            8
        );
    }
}

#[test]
fn rounded_boxes_keep_mirrored_3mf_instances_separate() {
    let manifest: Value = serde_json::from_slice(&fixture("m6a_manifest.json")).unwrap();
    let result = convert_document(
        &fixture("m6a_rounded_box_instances.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .unwrap();
    assert_eq!((result.report.cad.solids, result.report.cad.faces), (2, 52));
    assert_eq!(result.report.bodies.len(), 2);
    for (body, expected) in result.report.bodies.iter().zip(
        manifest["cases"]["m6a_rounded_box_instances"]["bodies"]
            .as_array()
            .unwrap(),
    ) {
        reference(&body.conversion, expected);
    }
}

#[test]
fn rounded_box_budget_refusal_never_returns_a_conversion() {
    let error = convert_stl(
        &fixture("m6a_rounded_box.stl"),
        &Options {
            max_deviation_mm: 0.0001,
            ..options()
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
