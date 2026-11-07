use fabstep_app::{analyze_document, convert_document, convert_stl, ConversionReport, InputFormat};
use fabstep_core::{Options, ReconstructionMode, RoundedFaceRef};
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
    assert_eq!(report.cad.faces, 1);
    assert_eq!(report.cad.toroidal_faces, 1);
    let volume = expected["volume_mm3"].as_f64().unwrap();
    assert!(
        (report.cad.volume_mm3 - volume).abs() < volume * 1e-8,
        "actual={} expected={volume}",
        report.cad.volume_mm3
    );
    for end in 0..2 {
        for axis in 0..3 {
            assert!(
                (report.cad.bounds_mm[end][axis]
                    - expected["bounds_mm"][end][axis].as_f64().unwrap())
                .abs()
                    < 1e-6
            );
        }
    }
    let recognized = report.rounded_recognition.as_ref().unwrap();
    assert_eq!(recognized.report.tori, 1);
    assert_eq!(recognized.report.toroidal_fillets, 0);
    let torus = &recognized.model.tori[0];
    assert!(torus.rings.is_none());
    assert!(torus.axis.is_some());
    assert!((torus.major_radius_mm - 6.).abs() < 1e-7);
    assert!((torus.minor_radius_mm - 2.).abs() < 1e-7);
    assert!(recognized.model.revolved.mixed.rings.is_empty());
    assert_eq!(
        recognized.triangle_faces.len(),
        report.analysis.mesh.triangles.len()
    );
    assert!(recognized
        .triangle_faces
        .iter()
        .all(|owner| matches!(owner, RoundedFaceRef::Torus { index: 0 })));
    let audit = report.fidelity.as_ref().unwrap();
    assert!(audit.source_samples > 0 && audit.result_samples > 0);
    assert!(audit.source_to_result_mm < options().max_deviation_mm);
    assert!(audit.result_to_source_mm < options().max_deviation_mm);
}

#[test]
fn complete_tori_reimport_as_one_face_without_invented_boundaries() {
    let manifest: Value = serde_json::from_slice(&fixture("m5d_manifest.json")).unwrap();
    for name in ["m5d_ring_torus", "m5d_torus_transformed"] {
        let result = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        reference(&result.report, &manifest["cases"][name]);
        assert_eq!(
            std::str::from_utf8(&result.step_bytes)
                .unwrap()
                .matches("TOROIDAL_SURFACE(")
                .count(),
            1
        );
    }
    let result = convert_document(
        &fixture("m5d_torus_instances.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.bodies.len(), 2);
    for (body, expected) in result.report.bodies.iter().zip(
        manifest["cases"]["m5d_torus_instances"]["bodies"]
            .as_array()
            .unwrap(),
    ) {
        reference(&body.conversion, expected);
    }
}

#[test]
fn tori_refuse_ambiguous_geometry_and_insufficient_budget() {
    for (name, code) in [
        ("m5d_open_torus", "mesh.open"),
        ("m5d_torus_dent", "recognition.surface_unproven"),
        ("m5d_elliptic_torus", "recognition.surface_unproven"),
        ("m5d_sparse_torus", "recognition.surface_unproven"),
    ] {
        let err = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .err()
            .unwrap();
        assert_eq!(err.code, code, "{name}: {err}");
    }
    let err = convert_stl(
        &fixture("m5d_ring_torus.stl"),
        &Options {
            max_deviation_mm: 0.001,
            ..options()
        },
    )
    .err()
    .unwrap();
    assert_eq!(err.code, "recognition.deviation_exceeded");
}

#[test]
fn tori_preserve_separation_at_source_and_result_without_partial_exports() {
    for (name, separated) in [
        ("m5d_tori_contact_source", false),
        ("m5d_tori_contact_result", true),
    ] {
        let source = fixture(&format!("{name}.3mf"));
        let analysis = analyze_document(&source, InputFormat::ThreeMf, &options()).unwrap();
        let models = analysis
            .bodies
            .iter()
            .map(|b| {
                let mesh = &b.analysis.mesh;
                // Use exact source triangles like the conversion pipeline.
                // A segmented planar face is not the source contact witness.
                let faces = mesh
                    .triangles
                    .iter()
                    .map(|triangle| {
                        let [a, b, c] = triangle.map(|index| mesh.vertices[index]);
                        let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
                        let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
                        let normal = [
                            u[1] * v[2] - u[2] * v[1],
                            u[2] * v[0] - u[0] * v[2],
                            u[0] * v[1] - u[1] * v[0],
                        ];
                        let length = normal[0].hypot(normal[1]).hypot(normal[2]);
                        fabstep_core::PlanarFace {
                            loops: vec![triangle.to_vec()],
                            normal: normal.map(|value| value / length),
                        }
                    })
                    .collect();
                fabstep_core::PlanarModel {
                    vertices: mesh.vertices.clone(),
                    faces,
                }
            })
            .collect::<Vec<_>>();
        let separation = fabstep_cad::validate_sources_separated(&models);
        if separated {
            assert!(separation.is_ok(), "{name}: {separation:?}");
        } else {
            assert_eq!(
                separation.unwrap_err().code,
                "cad.bodies_not_separated",
                "{name}"
            );
        }
        let error = convert_document(&source, InputFormat::ThreeMf, &options())
            .err()
            .unwrap();
        assert_eq!(error.code, "cad.bodies_not_separated", "{name}: {error}");
    }
}
