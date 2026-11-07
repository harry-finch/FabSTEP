use fabstep_app::{convert_document, convert_stl, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
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

fn manifest() -> serde_json::Value {
    serde_json::from_slice(&fixture("m5b_manifest.json")).unwrap()
}

#[test]
fn caps_reimport_with_nominal_volume_bounds_and_complete_source_provenance() {
    for (name, expected) in manifest()["cases"].as_object().unwrap() {
        if expected.get("expected_refusal").is_some() || expected.get("bodies").is_some() {
            continue;
        }
        let result = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let r = &result.report;
        assert_eq!(r.cad.solids, 1, "{name}");
        for (count, key) in [
            (r.cad.faces, "faces"),
            (r.cad.planar_faces, "planes"),
            (r.cad.cylindrical_faces, "cylinders"),
            (r.cad.conical_faces, "cones"),
        ] {
            assert_eq!(
                count as u64,
                expected[key].as_u64().unwrap(),
                "{name}: {key}"
            );
        }
        assert_eq!(r.cad.spherical_faces, 1, "{name}");
        assert_eq!(r.cad.toroidal_faces, 0, "{name}");
        let nominal = expected["volume_mm3"].as_f64().unwrap();
        assert!(
            (r.cad.volume_mm3 - nominal).abs() < nominal * 1e-8,
            "{name}: volume"
        );
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (r.cad.bounds_mm[end][axis]
                        - expected["bounds_mm"][end][axis].as_f64().unwrap())
                    .abs()
                        < 1e-6,
                    "{name}: bounds {:?}",
                    r.cad.bounds_mm
                );
            }
        }
        let proof = r
            .rounded_recognition
            .as_ref()
            .expect("proven spherical cap");
        assert_eq!(
            proof.model.revolved.mixed.vertices,
            r.analysis.mesh.vertices
        );
        assert_eq!(proof.triangle_faces.len(), r.analysis.mesh.triangles.len());
        assert_eq!(proof.report.spheres, 1);
        assert!(proof.report.max_deviation_bound_mm <= options().max_deviation_mm);
        assert_eq!(r.settings.reconstruction, ReconstructionMode::Surfaces);
        assert_eq!(
            r.settings.cylinder_fit_tolerance_mm,
            options().cylinder_fit_tolerance_mm
        );
        let fidelity = r.fidelity.as_ref().expect("bidirectional native audit");
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(
            fidelity.source_to_result_mm < options().max_deviation_mm,
            "{name}"
        );
        assert!(
            fidelity.result_to_source_mm < options().max_deviation_mm,
            "{name}"
        );
        assert_eq!(
            std::str::from_utf8(&result.step_bytes)
                .unwrap()
                .matches("SPHERICAL_SURFACE(")
                .count(),
            1
        );
    }
}

#[test]
fn cap_instances_preserve_mirror_placement_and_refuse_contact() {
    let expected = manifest();
    let reference = &expected["cases"]["m5b_cap_instances"];
    let result = convert_document(
        &fixture("m5b_cap_instances.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .unwrap();
    assert_eq!(result.report.bodies.len(), 2);
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.spherical_faces, 2);
    for (body, bounds) in result
        .report
        .bodies
        .iter()
        .zip(reference["body_bounds_mm"].as_array().unwrap())
    {
        let r = &body.conversion;
        assert_eq!(r.cad.spherical_faces, 1);
        assert!(r.fidelity.is_some());
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (r.cad.bounds_mm[end][axis] - bounds[end][axis].as_f64().unwrap()).abs() < 1e-6
                );
            }
        }
    }
    let error = convert_document(
        &fixture("m5b_cap_touching.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .err()
    .expect("contact refused");
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn unproven_caps_and_insufficient_budgets_cannot_fall_back_to_facets() {
    for name in [
        "m5b_spheroid.stl",
        "m5b_shifted_pole.stl",
        "m5b_spherical_zone.stl",
        "m5b_sparse_cap.stl",
    ] {
        let error = convert_stl(&fixture(name), &options())
            .err()
            .expect("proof must fail");
        assert!(error.code.starts_with("recognition."), "{name}: {error}");
    }
    for name in ["m5b_small_cap.stl", "m5b_large_cap.stl"] {
        let source = fixture(name);
        let error = convert_stl(
            &source,
            &Options {
                max_deviation_mm: 0.001,
                ..options()
            },
        )
        .err()
        .expect("budget too small");
        assert_eq!(
            error.code, "recognition.deviation_exceeded",
            "{name}: {error}"
        );
        let error = convert_stl(
            &source,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..options()
            },
        )
        .err()
        .expect("legacy mode must not infer sphere");
        assert!(error.code.starts_with("recognition."), "{name}: {error}");
    }
}
