use fabstep_app::{
    analyze_document, convert_document, convert_stl, recognize_surfaces, ConversionReport,
    InputFormat,
};
use fabstep_core::{analyze_stl, Options, ReconstructionMode, RoundedFaceRef};
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

fn manifest() -> Value {
    serde_json::from_slice(&fixture("m5c_manifest.json")).unwrap()
}

fn assert_reference(report: &ConversionReport, reference: &Value) {
    assert_eq!(report.cad.solids, 1);
    assert_eq!(
        report.cad.faces as u64,
        reference["faces"].as_u64().unwrap()
    );
    assert_eq!(
        report.cad.planar_faces as u64,
        reference["planes"].as_u64().unwrap()
    );
    assert_eq!(report.cad.spherical_faces, 1);
    assert_eq!(
        report.cad.cylindrical_faces + report.cad.conical_faces + report.cad.toroidal_faces,
        0
    );
    let volume = reference["volume_mm3"].as_f64().unwrap();
    assert!((report.cad.volume_mm3 - volume).abs() < volume * 1e-8);
    for end in 0..2 {
        for axis in 0..3 {
            assert!(
                (report.cad.bounds_mm[end][axis]
                    - reference["bounds_mm"][end][axis].as_f64().unwrap())
                .abs()
                    < 1e-6
            );
        }
    }
    let recognition = report.rounded_recognition.as_ref().unwrap();
    assert_eq!(
        recognition.model.revolved.mixed.vertices,
        report.analysis.mesh.vertices
    );
    assert_eq!(
        recognition.triangle_faces.len(),
        report.analysis.mesh.triangles.len()
    );
    let complete = reference["complete_spheres"] == 1;
    let sphere = &recognition.model.spheres[0];
    assert_eq!(sphere.rings.is_none(), complete);
    assert!(!sphere.inner);
    assert!(
        (sphere.radius_mm - reference["spheres"][0]["radius_mm"].as_f64().unwrap()).abs() < 1e-7
    );
    if complete {
        assert!(recognition.model.revolved.mixed.rings.is_empty());
        assert!(recognition.model.revolved.mixed.planes.is_empty());
        assert!(recognition
            .triangle_faces
            .iter()
            .all(|owner| matches!(owner, RoundedFaceRef::Sphere { index: 0 })));
    }
    let fidelity = report.fidelity.as_ref().unwrap();
    assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
    assert!(fidelity.source_to_result_mm < options().max_deviation_mm);
    assert!(fidelity.result_to_source_mm < options().max_deviation_mm);
}

#[test]
fn complete_spheres_reimport_without_invented_boundaries_and_caps_keep_their_trim() {
    let references = manifest();
    for name in [
        "m5c_icosphere",
        "m5c_icosphere_transformed",
        "m5c_uv_sphere",
        "m5c_truncated_cospherical",
    ] {
        let source = fixture(&format!("{name}.stl"));
        let conversion = convert_stl(&source, &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_reference(&conversion.report, &references["cases"][name]);
        assert_eq!(
            std::str::from_utf8(&conversion.step_bytes)
                .unwrap()
                .matches("SPHERICAL_SURFACE(")
                .count(),
            1
        );
    }
}

#[test]
fn complete_spheres_preserve_mirrors_scales_and_separation_at_both_stages() {
    let references = manifest();
    for name in ["m5c_sphere_instances", "m5c_sphere_scaled"] {
        let result = convert_document(
            &fixture(&format!("{name}.3mf")),
            InputFormat::ThreeMf,
            &options(),
        )
        .unwrap();
        let reference = &references["cases"][name];
        let bodies = reference["bodies"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| vec![reference.clone()]);
        assert_eq!(result.report.bodies.len(), bodies.len());
        assert_eq!(result.report.cad.solids, bodies.len());
        for (body, expected) in result.report.bodies.iter().zip(&bodies) {
            assert_reference(&body.conversion, expected);
        }
    }
    for (name, source_separated) in [
        ("m5c_spheres_contact_source", false),
        ("m5c_spheres_contact_result", true),
    ] {
        let source = fixture(&format!("{name}.3mf"));
        let document = analyze_document(&source, InputFormat::ThreeMf, &options()).unwrap();
        let models = document
            .bodies
            .iter()
            .map(|b| b.analysis.model.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            fabstep_cad::validate_sources_separated(&models).is_ok(),
            source_separated,
            "{name}"
        );
        let error = convert_document(&source, InputFormat::ThreeMf, &options())
            .err()
            .expect("no partial export on contact");
        assert_eq!(error.code, "cad.bodies_not_separated", "{name}: {error}");
    }
}

#[test]
fn incomplete_deformed_or_under_resolved_spheres_are_not_exported() {
    for name in [
        "m5c_spheroid",
        "m5c_sphere_dent",
        "m5c_sparse_sphere",
        "m5c_open_sphere",
    ] {
        let error = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .err()
            .expect("invalid sphere refused");
        assert!(
            error.code.starts_with("recognition.") || error.code.starts_with("mesh."),
            "{name}: {error}"
        );
    }
    let source = fixture("m5c_icosphere.stl");
    let error = convert_stl(
        &source,
        &Options {
            max_deviation_mm: 0.001,
            ..options()
        },
    )
    .err()
    .expect("budget refusal");
    assert_eq!(error.code, "recognition.deviation_exceeded");
    for mode in [
        ReconstructionMode::Cylinders,
        ReconstructionMode::Primitives,
    ] {
        assert!(convert_stl(
            &source,
            &Options {
                reconstruction: mode,
                ..options()
            }
        )
        .is_err());
    }
}

#[test]
fn historical_cylinders_and_blocking_refusals_remain_prioritary() {
    for name in [
        "m2_cylinder.stl",
        "m2c_open_arc.stl",
        "m4_chamfered_cylinder.stl",
    ] {
        let analysis = analyze_stl(&fixture(name), &options()).unwrap();
        let old = recognize_surfaces(
            &analysis,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..options()
            },
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let new =
            recognize_surfaces(&analysis, &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            serde_json::to_value(old.report()).unwrap(),
            serde_json::to_value(new.report()).unwrap(),
            "{name}"
        );
        assert_eq!(new.report().spheres, 0);
    }
    let analysis = analyze_stl(&fixture("m4_alternating_taper.stl"), &options()).unwrap();
    let previous = fabstep_core::recognize_rounded(&analysis, &options()).unwrap_err();
    let error = recognize_surfaces(&analysis, &options())
        .err()
        .expect("existing rounded proof remains blocking");
    assert_eq!(error.code, previous.code);
    assert_eq!(error.message, previous.message);
}
