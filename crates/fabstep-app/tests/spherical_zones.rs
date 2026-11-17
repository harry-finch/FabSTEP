use fabstep_app::{convert_document, convert_stl, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join("post_v1_zones")
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
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

#[test]
fn zones_reimport_with_nominal_volume_bounds_and_complete_source_provenance() {
    for (name, expected) in manifest()["cases"].as_object().unwrap() {
        if expected.get("expected_refusal").is_some() || expected.get("bodies").is_some() {
            continue;
        }
        let result = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let r = &result.report;
        assert_eq!(r.cad.solids, 1, "{name}");
        for (count, key) in [(r.cad.faces, "faces"), (r.cad.planar_faces, "planes")] {
            assert_eq!(
                count as u64,
                expected[key].as_u64().unwrap(),
                "{name}: {key}"
            );
        }
        assert_eq!(r.cad.spherical_faces, 1, "{name}");
        assert_eq!(r.cad.cylindrical_faces + r.cad.conical_faces, 0);
        assert_eq!(r.cad.toroidal_faces, 0, "{name}");
        let nominal = expected["volume_mm3"].as_f64().unwrap();
        assert!(
            (r.cad.volume_mm3 - nominal).abs() < nominal * 1e-7,
            "{name}: volume"
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
        let proof = r
            .rounded_recognition
            .as_ref()
            .expect("proven spherical zone");
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
fn zone_mirror_instances_preserve_two_source_boundaries_and_placements() {
    let result =
        convert_document(&fixture("instances.3mf"), InputFormat::ThreeMf, &options()).unwrap();
    let m = manifest();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.spherical_faces, 2);
    for (i, body) in result.report.bodies.iter().enumerate() {
        let r = &body.conversion;
        assert_eq!(
            r.rounded_recognition
                .as_ref()
                .unwrap()
                .model
                .revolved
                .mixed
                .rings
                .len(),
            2
        );
        assert!(r.fidelity.is_some());
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (r.cad.bounds_mm[end][axis]
                        - m["cases"]["instances"]["body_bounds_mm"][i][end][axis]
                            .as_f64()
                            .unwrap())
                    .abs()
                        < 1e-6
                );
            }
        }
    }
    assert_eq!(
        convert_document(&fixture("touching.3mf"), InputFormat::ThreeMf, &options())
            .err()
            .unwrap()
            .code,
        "cad.bodies_not_separated"
    );
}

#[test]
fn zones_keep_proof_and_budget_failures_blocking_and_legacy_modes_unchanged() {
    for (name, code) in [
        ("spheroid", "recognition.surface_unproven"),
        ("folded", "recognition.surface_unproven"),
        ("sparse", "recognition.deviation_exceeded"),
        ("missing_triangle", "mesh.open"),
    ] {
        let e = convert_stl(&fixture(&format!("{name}.stl")), &options())
            .err()
            .unwrap();
        assert_eq!(e.code, code, "{name}: {e}");
    }
    let source = fixture("cross_equator.stl");
    assert_eq!(
        convert_stl(
            &source,
            &Options {
                max_deviation_mm: 0.001,
                ..options()
            }
        )
        .err()
        .unwrap()
        .code,
        "recognition.deviation_exceeded"
    );
    for mode in [
        ReconstructionMode::Primitives,
        ReconstructionMode::Cylinders,
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
    let historical = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m5b_spherical_zone.stl"),
    )
    .unwrap();
    let result = convert_stl(&historical, &options()).unwrap();
    assert_eq!(result.report.cad.spherical_faces, 1);
    assert_eq!(result.report.cad.planar_faces, 2);
}
