use fabstep_app::{convert_document, convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/same_sign_chains")
            .join(name),
    )
    .unwrap()
}
fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}
fn manifest() -> serde_json::Value {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

#[test]
fn automatic_same_sign_chain_has_exact_native_face_provenance() {
    let converted = convert_document_with_provenance(
        &fixture("two_increasing.stl"),
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    let faces = converted.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    assert_eq!(faces.result.faces.len(), 6);
    assert_eq!(converted.report.cad.conical_faces, 2);
    let mut partition = BTreeSet::new();
    for face in &faces.source_correspondence.faces {
        assert!(!face.source_triangle_ids.is_empty());
        for &triangle in &face.source_triangle_ids {
            assert!(partition.insert(triangle));
        }
    }
    assert_eq!(
        partition.len(),
        converted.report.bodies[0]
            .conversion
            .analysis
            .mesh
            .triangles
            .len()
    );
    assert!(converted.report.bodies[0].conversion.fidelity.is_some());
}

#[test]
fn same_sign_native_reimport_preserves_surfaces_volume_bounds_and_fidelity() {
    for mode in [
        ReconstructionMode::Surfaces,
        ReconstructionMode::Primitives,
        ReconstructionMode::Auto,
    ] {
        for (name, expected) in manifest()["cases"].as_object().unwrap() {
            if expected.get("bodies").is_some() {
                continue;
            }
            let o = options(mode);
            let result = convert_stl(&fixture(&format!("{name}.stl")), &o)
                .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            let r = &result.report;
            assert_eq!(r.cad.solids, 1);
            for (count, key) in [
                (r.cad.faces, "faces"),
                (r.cad.planar_faces, "planes"),
                (r.cad.conical_faces, "cones"),
                (r.cad.cylindrical_faces, "cylinders"),
            ] {
                assert_eq!(
                    count as u64,
                    expected[key].as_u64().unwrap(),
                    "{name}: {key}"
                );
            }
            assert_eq!(r.cad.spherical_faces + r.cad.toroidal_faces, 0);
            let nominal = expected["volume_mm3"].as_f64().unwrap();
            assert!(
                (r.cad.volume_mm3 - nominal).abs() < nominal * 1e-7,
                "{name}: volume {}",
                r.cad.volume_mm3
            );
            for end in 0..2 {
                for axis in 0..3 {
                    assert!(
                        (r.cad.bounds_mm[end][axis]
                            - expected["bounds_mm"][end][axis].as_f64().unwrap())
                        .abs()
                            < 1e-5,
                        "{name}: bounds {:?}",
                        r.cad.bounds_mm
                    );
                }
            }
            let proof = if mode != ReconstructionMode::Primitives {
                let rr = r.rounded_recognition.as_ref().unwrap();
                assert_eq!(rr.model.revolved.mixed.vertices, r.analysis.mesh.vertices);
                assert_eq!(rr.triangle_faces.len(), r.analysis.mesh.triangles.len());
                &rr.report
            } else {
                let rr = r.revolved_recognition.as_ref().unwrap();
                assert_eq!(rr.model.mixed.vertices, r.analysis.mesh.vertices);
                assert_eq!(rr.triangle_faces.len(), r.analysis.mesh.triangles.len());
                &rr.report
            };
            assert!(proof.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!(proof
                .diagnostics
                .iter()
                .any(|d| d.code == "recognition.conic_intent_assumed"
                    && d.count == r.cad.conical_faces));
            let fidelity = r.fidelity.as_ref().unwrap();
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(fidelity.source_to_result_mm < o.max_deviation_mm);
            assert!(fidelity.result_to_source_mm < o.max_deviation_mm);
            let step = std::str::from_utf8(&result.step_bytes).unwrap();
            assert_eq!(
                step.matches("CONICAL_SURFACE(").count(),
                r.cad.conical_faces
            );
            assert_eq!(
                step.matches("CYLINDRICAL_SURFACE(").count(),
                r.cad.cylindrical_faces
            );
        }
    }
}

#[test]
fn same_sign_mirror_instances_keep_all_bodies_and_source_placements() {
    let o = options(ReconstructionMode::Surfaces);
    let result = convert_document(&fixture("instances.3mf"), InputFormat::ThreeMf, &o).unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.conical_faces, 4);
    assert_eq!(result.report.cad.cylindrical_faces, 4);
    let m = manifest();
    let expected = &m["cases"]["instances"];
    assert!((result.report.cad.volume_mm3 - expected["volume_mm3"].as_f64().unwrap()).abs() < 1e-7);
    assert_eq!(result.report.bodies.len(), 2);
    for (i, body) in result.report.bodies.iter().enumerate() {
        let r = &body.conversion;
        assert_eq!(r.cad.solids, 1);
        assert_eq!(r.rounded_recognition.as_ref().unwrap().report.cones, 2);
        assert!(r.fidelity.is_some());
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (r.cad.bounds_mm[end][axis]
                        - expected["body_bounds_mm"][i][end][axis].as_f64().unwrap())
                    .abs()
                        < 1e-6
                );
            }
        }
    }
}

#[test]
fn same_sign_chains_do_not_fall_back_after_proof_or_deviation_failure() {
    for (name, expected) in manifest()["negative_cases"].as_object().unwrap() {
        let bytes = fixture(&format!("{name}.stl"));
        for (mode, key) in [
            (ReconstructionMode::Primitives, "primitives_error"),
            (ReconstructionMode::Surfaces, "surfaces_error"),
        ] {
            assert_eq!(
                convert_stl(&bytes, &options(mode)).err().unwrap().code,
                expected[key].as_str().unwrap(),
                "{name}: {mode:?}"
            );
        }
    }
    let curved = convert_stl(
        &fixture("curved_meridian.stl"),
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    assert_eq!(curved.report.cad.solids, 1);
    assert_eq!(curved.report.cad.conical_faces, 0);
    // C1b2 intentionally promotes this unchanged source: its two affine runs
    // have interior rings at thirds and one uniquely proved shared boundary.
    let adjacent =
        convert_stl(&fixture("no_land.stl"), &options(ReconstructionMode::Auto)).unwrap();
    assert_eq!(adjacent.report.cad.solids, 1);
    assert_eq!(adjacent.report.cad.conical_faces, 2);
    assert_eq!(adjacent.report.cad.faces, 5);
    assert!(adjacent.report.fidelity.is_some());
    let o = options(ReconstructionMode::Surfaces);
    for path in ["m4_alternating_taper.stl", "m4_piecewise_taper.stl"] {
        let bytes = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures")
                .join(path),
        )
        .unwrap();
        assert_eq!(
            convert_stl(&bytes, &o).err().unwrap().code,
            "recognition.surface_unproven"
        );
    }
    let bytes = fixture("two_increasing.stl");
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    assert_eq!(
        convert_stl(&bytes, &strict).err().unwrap().code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        convert_stl(&bytes, &options(ReconstructionMode::Cylinders))
            .err()
            .unwrap()
            .code,
        "recognition.unsupported"
    );
}
