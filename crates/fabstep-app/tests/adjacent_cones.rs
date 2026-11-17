use fabstep_app::{convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/adjacent_cones")
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
fn automatic_adjacent_cones_has_exact_native_face_provenance() {
    let converted = convert_document_with_provenance(
        &fixture("increasing.stl"),
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    let faces = converted.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    assert_eq!(faces.result.faces.len(), 5);
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
fn adjacent_cones_native_reimport_preserves_surfaces_volume_bounds_and_fidelity() {
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
