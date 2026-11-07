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

#[test]
fn independent_rounded_corpus_reimports_nominal_surfaces_and_audits_source() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/m5_manifest.json")).unwrap();
    for (name, expected) in manifest["cases"].as_object().unwrap() {
        if expected.get("expected_refusal").is_some() || expected.get("bodies").is_some() {
            continue;
        }
        let result = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let report = &result.report;
        assert_eq!(report.cad.solids, 1, "{name}");
        for (actual, key) in [
            (report.cad.faces, "faces"),
            (report.cad.planar_faces, "planes"),
            (report.cad.cylindrical_faces, "cylinders"),
            (report.cad.conical_faces, "cones"),
        ] {
            assert_eq!(
                actual as u64,
                expected[key].as_u64().unwrap(),
                "{name}: {key}"
            );
        }
        assert_eq!(
            report.cad.spherical_faces,
            expected["spheres"].as_array().unwrap().len(),
            "{name}"
        );
        assert_eq!(
            report.cad.toroidal_faces,
            expected["tori"].as_array().unwrap().len(),
            "{name}"
        );
        let nominal = expected["volume_mm3"].as_f64().unwrap();
        assert!(
            (report.cad.volume_mm3 - nominal).abs() < nominal * 1e-8,
            "{name}: volume {} vs {nominal}",
            report.cad.volume_mm3
        );
        if let Some(bounds) = expected.get("bounds_mm") {
            for end in 0..2 {
                for axis in 0..3 {
                    assert!(
                        (report.cad.bounds_mm[end][axis] - bounds[end][axis].as_f64().unwrap())
                            .abs()
                            < 1e-6,
                        "{name}: bounds"
                    );
                }
            }
        }
        let recognized = report
            .rounded_recognition
            .as_ref()
            .expect("rounded representation");
        assert_eq!(
            recognized.model.revolved.mixed.vertices, report.analysis.mesh.vertices,
            "source retained"
        );
        assert_eq!(
            recognized.triangle_faces.len(),
            report.analysis.mesh.triangles.len(),
            "exhaustive provenance"
        );
        assert_eq!(recognized.report.spheres, report.cad.spherical_faces);
        assert_eq!(recognized.report.tori, report.cad.toroidal_faces);
        assert_eq!(
            recognized.report.toroidal_fillets as u64,
            expected["toroidal_fillets"].as_u64().unwrap_or(0)
        );
        assert!(recognized.report.max_deviation_bound_mm <= options().max_deviation_mm);
        let fidelity = report
            .fidelity
            .as_ref()
            .expect("native bidirectional audit");
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(
            fidelity.source_to_result_mm < options().max_deviation_mm,
            "{name}"
        );
        assert!(
            fidelity.result_to_source_mm < options().max_deviation_mm,
            "{name}"
        );
        let step = std::str::from_utf8(&result.step_bytes).unwrap();
        assert_eq!(
            step.matches("SPHERICAL_SURFACE(").count(),
            report.cad.spherical_faces
        );
        assert_eq!(
            step.matches("TOROIDAL_SURFACE(").count(),
            report.cad.toroidal_faces
        );
        if name == "m5_rounded_mixed" {
            assert_eq!(recognized.model.revolved.mixed.partial_cylinders.len(), 1);
            assert!(recognized
                .triangle_faces
                .iter()
                .any(|r| matches!(r, fabstep_core::RoundedFaceRef::PartialCylinder { .. })));
        }
    }
}

#[test]
fn rounded_failures_and_budgets_do_not_produce_faceted_successes() {
    for name in [
        "m5_warped_sphere.stl",
        "m5_sparse_sphere.stl",
        "m5_warped_torus.stl",
        "m5_non_tangent_torus.stl",
    ] {
        let error = convert_stl(&fixture(name), &options())
            .err()
            .expect("unproved rounded surface refused");
        assert!(error.code.starts_with("recognition."), "{name}: {error}");
    }
    for name in ["m5_hemisphere.stl", "m5_toroidal_fillet.stl"] {
        let error = convert_stl(
            &fixture(name),
            &Options {
                max_deviation_mm: 0.001,
                ..options()
            },
        )
        .err()
        .expect("insufficient deviation budget refused");
        assert_eq!(
            error.code, "recognition.deviation_exceeded",
            "{name}: {error}"
        );
        assert!(
            convert_stl(
                &fixture(name),
                &Options {
                    reconstruction: ReconstructionMode::Primitives,
                    ..Options::default()
                }
            )
            .is_err(),
            "legacy mode must stay strict"
        );
    }
}

#[test]
fn rounded_instances_keep_native_counts_placements_and_contact_refusal() {
    let result = convert_document(
        &fixture("m5_fillet_instances.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .unwrap();
    assert_eq!(result.report.bodies.len(), 2);
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.faces, 8);
    assert_eq!(result.report.cad.toroidal_faces, 2);
    assert_eq!(result.report.cad.cylindrical_faces, 2);
    assert_eq!(result.report.cad.spherical_faces, 0);
    assert_eq!(
        result.report.settings.reconstruction,
        ReconstructionMode::Surfaces
    );
    for (body, offset) in result.report.bodies.iter().zip([0., 30.]) {
        let report = &body.conversion;
        assert_eq!(report.cad.toroidal_faces, 1);
        assert_eq!(
            report
                .rounded_recognition
                .as_ref()
                .unwrap()
                .report
                .toroidal_fillets,
            1
        );
        assert!(report.fidelity.is_some());
        assert!((report.cad.bounds_mm[0][0] - (-8. + offset)).abs() < 1e-6);
        assert!((report.cad.bounds_mm[1][0] - (8. + offset)).abs() < 1e-6);
    }
    let error = convert_document(
        &fixture("m5_fillet_touching.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .err()
    .expect("touching rounded bodies refused");
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn rounded_mode_preserves_existing_surfaces_and_requested_settings() {
    for name in [
        "m2_cylinder.stl",
        "m2b_boss_blind.stl",
        "m2c_open_arc_mixed.stl",
        "m4_chamfered_cylinder.stl",
        "m4_cone_tip.stl",
    ] {
        let bytes = fixture(name);
        let previous = convert_stl(
            &bytes,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..Options::default()
            },
        )
        .unwrap();
        let current = convert_stl(&bytes, &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            current.report.settings.reconstruction,
            ReconstructionMode::Surfaces
        );
        assert_eq!(
            current.report.cad.faces, previous.report.cad.faces,
            "{name}"
        );
        assert_eq!(
            current.report.cad.cylindrical_faces, previous.report.cad.cylindrical_faces,
            "{name}"
        );
        assert_eq!(
            current.report.cad.conical_faces, previous.report.cad.conical_faces,
            "{name}"
        );
        assert_eq!(current.report.cad.spherical_faces, 0);
        assert_eq!(current.report.cad.toroidal_faces, 0);
        assert!(
            (current.report.cad.volume_mm3 - previous.report.cad.volume_mm3).abs() < 1e-7,
            "{name}"
        );
        assert!(current.report.fidelity.is_some());
    }
}

#[test]
fn rounded_mode_preserves_historical_proof_failures() {
    for name in [
        "m4_sparse_cone.stl",
        "m4_shifted_tip.stl",
        "m4_warped_frustum.stl",
        "m4_piecewise_taper.stl",
        "m4_alternating_taper.stl",
    ] {
        let error = convert_stl(&fixture(name), &options())
            .err()
            .expect("unproven surface must fail");
        assert!(error.code.starts_with("recognition."), "{name}: {error}");
    }
    let error = convert_document(
        &fixture("m4_frustum_touching.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .err()
    .expect("contact must fail");
    assert_eq!(error.code, "cad.bodies_not_separated");
}
