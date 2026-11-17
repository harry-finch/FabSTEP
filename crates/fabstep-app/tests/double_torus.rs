use fabstep_app::{convert_document, convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/double_torus")
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
fn native_reimport_preserves_both_tori_the_shared_land_and_nominal_volume() {
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        for (name, expected) in manifest()["cases"].as_object().unwrap() {
            if expected.get("bodies").is_some() {
                continue;
            }
            let o = options(mode);
            let converted = convert_stl(&fixture(&format!("{name}.stl")), &o)
                .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            let r = &converted.report;
            assert_eq!(r.cad.solids, 1);
            assert_eq!(r.cad.faces, 5);
            assert_eq!(r.cad.planar_faces, 2);
            assert_eq!(r.cad.cylindrical_faces, 1);
            assert_eq!(r.cad.toroidal_faces, 2);
            assert_eq!(r.cad.conical_faces + r.cad.spherical_faces, 0);
            let volume = expected["volume_mm3"].as_f64().unwrap();
            assert!(
                (r.cad.volume_mm3 - volume).abs() < volume * 2e-6,
                "{name}: {} != {volume}",
                r.cad.volume_mm3
            );
            for end in 0..2 {
                for axis in 0..3 {
                    assert!(
                        (r.cad.bounds_mm[end][axis]
                            - expected["bounds_mm"][end][axis].as_f64().unwrap())
                        .abs()
                            < 3e-6,
                        "{name}: {:?}",
                        r.cad.bounds_mm
                    );
                }
            }
            let rr = r.rounded_recognition.as_ref().unwrap();
            assert_eq!(rr.model.revolved.mixed.vertices, r.analysis.mesh.vertices);
            assert_eq!(rr.triangle_faces.len(), r.analysis.mesh.triangles.len());
            assert_eq!(rr.report.toroidal_fillets, 2);
            assert!(rr.report.max_deviation_bound_mm <= o.max_deviation_mm);
            let fidelity = r.fidelity.as_ref().unwrap();
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(fidelity.source_to_result_mm <= o.max_deviation_mm);
            assert!(fidelity.result_to_source_mm <= o.max_deviation_mm);
            let step = std::str::from_utf8(&converted.step_bytes).unwrap();
            assert_eq!(step.matches("TOROIDAL_SURFACE(").count(), 2);
            assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1);
        }
    }
}

#[test]
fn automatic_double_fillet_exposes_exact_exhaustive_native_face_provenance() {
    let converted = convert_document_with_provenance(
        &fixture("nominal.stl"),
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    let faces = converted.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    assert_eq!(faces.result.faces.len(), 5);
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
}

#[test]
fn mirrored_instances_preserve_two_complete_solids_and_contacts_block_export() {
    let o = options(ReconstructionMode::Surfaces);
    let converted = convert_document(&fixture("instances.3mf"), InputFormat::ThreeMf, &o).unwrap();
    let r = &converted.report;
    assert_eq!(r.cad.solids, 2);
    assert_eq!(r.bodies.len(), 2);
    assert_eq!(r.cad.faces, 10);
    assert_eq!(r.cad.toroidal_faces, 4);
    let m = manifest();
    let expected = &m["cases"]["instances"];
    assert!((r.cad.volume_mm3 - expected["volume_mm3"].as_f64().unwrap()).abs() < 1e-6);
    for (i, body) in r.bodies.iter().enumerate() {
        assert_eq!(body.conversion.cad.toroidal_faces, 2);
        assert!(body.conversion.fidelity.is_some());
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (body.conversion.cad.bounds_mm[end][axis]
                        - expected["body_bounds_mm"][i][end][axis].as_f64().unwrap())
                    .abs()
                        < 2e-6
                );
            }
        }
    }
    assert_eq!(
        convert_document(&fixture("touching.3mf"), InputFormat::ThreeMf, &o)
            .err()
            .unwrap()
            .code,
        m["touching_error"].as_str().unwrap()
    );
}

#[test]
fn unresolved_double_fillets_and_insufficient_deviation_never_export_partial_success() {
    let o = options(ReconstructionMode::Surfaces);
    for (name, expected) in manifest()["negative_cases"].as_object().unwrap() {
        let error = convert_stl(&fixture(&format!("{name}.stl")), &o)
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            expected["app_surfaces_error"].as_str().unwrap(),
            "{name}: {error}"
        );
    }
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    assert_eq!(
        convert_stl(&fixture("nominal.stl"), &strict)
            .err()
            .unwrap()
            .code,
        "recognition.deviation_exceeded"
    );
    let faceted = convert_stl(
        &fixture("warped_arc.stl"),
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    assert_eq!(faceted.report.cad.solids, 1);
    assert_eq!(faceted.report.cad.toroidal_faces, 0);
    for mode in [
        ReconstructionMode::Cylinders,
        ReconstructionMode::Primitives,
    ] {
        assert!(convert_stl(&fixture("nominal.stl"), &options(mode)).is_err());
    }
}
