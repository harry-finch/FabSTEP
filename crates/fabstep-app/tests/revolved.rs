use fabstep_app::{convert_document, convert_stl, InputFormat};
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use std::{fmt::Write, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    }
}

#[test]
fn independent_conical_corpus_reimports_nominal_surfaces_and_audits_source() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/m4_manifest.json")).unwrap();
    for (name, expected) in manifest["cases"].as_object().unwrap() {
        if expected.get("expected_refusal").is_some() || expected.get("bodies").is_some() {
            continue;
        }
        let opts = Options {
            reconstruction: if expected["mode"] == "planar" {
                ReconstructionMode::Planar
            } else {
                ReconstructionMode::Primitives
            },
            ..Options::default()
        };
        let (bytes, format) = if expected["format"] == "3mf" {
            (
                fs::read(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../fixtures")
                        .join(format!("{name}.3mf")),
                )
                .unwrap(),
                InputFormat::ThreeMf,
            )
        } else {
            (fixture(name), InputFormat::Stl)
        };
        let result =
            convert_document(&bytes, format, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(result.report.bodies.len(), 1, "{name}");
        let body = &result.report.bodies[0].conversion;
        assert_eq!(body.cad.solids, 1, "{name}");
        assert_eq!(
            body.cad.faces as u64,
            expected["faces"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            body.cad.planar_faces as u64,
            expected["planes"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            body.cad.cylindrical_faces,
            expected["cylinders"].as_array().unwrap().len(),
            "{name}"
        );
        assert_eq!(
            body.cad.conical_faces,
            expected["cones"].as_array().unwrap().len(),
            "{name}"
        );
        let volume = expected["volume_mm3"].as_f64().unwrap();
        assert!(
            (body.cad.volume_mm3 - volume).abs() < volume * 1e-8,
            "{name}: {} != {volume}",
            body.cad.volume_mm3
        );
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (body.cad.bounds_mm[end][axis]
                        - expected["bounds_mm"][end][axis].as_f64().unwrap())
                    .abs()
                        < 1e-6,
                    "{name}: bounds"
                );
            }
        }
        if opts.reconstruction == ReconstructionMode::Planar {
            assert!(body.revolved_recognition.is_none());
            continue;
        }
        let recognized = body
            .revolved_recognition
            .as_ref()
            .expect("conical recognition");
        assert_eq!(
            recognized.triangle_faces.len(),
            body.analysis.mesh.triangles.len()
        );
        assert_eq!(recognized.model.mixed.vertices, body.analysis.mesh.vertices);
        assert_eq!(recognized.report.cones, body.cad.conical_faces);
        assert!(recognized.report.max_deviation_bound_mm <= opts.max_deviation_mm);
        let fidelity = body.fidelity.as_ref().expect("bidirectional audit");
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(fidelity.source_to_result_mm < opts.max_deviation_mm);
        assert!(fidelity.result_to_source_mm < opts.max_deviation_mm);
        assert_eq!(
            std::str::from_utf8(&result.step_bytes)
                .unwrap()
                .matches("CONICAL_SURFACE(")
                .count(),
            body.cad.conical_faces
        );
    }
}

#[test]
fn conical_opt_in_preserves_legacy_modes_and_cylindrical_paths() {
    let source = fixture("m4_frustum");
    assert!(convert_stl(
        &source,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        }
    )
    .is_err());
    let planar = convert_stl(&source, &Options::default()).unwrap();
    assert_eq!(planar.report.cad.conical_faces, 0);
    assert!(planar.report.cad.faces > 3);
    for name in ["m2_cylinder", "m2b_boss_blind", "m2c_open_arc_mixed"] {
        let source = fixture(name);
        let old = convert_stl(
            &source,
            &Options {
                reconstruction: ReconstructionMode::Cylinders,
                ..Options::default()
            },
        )
        .unwrap();
        let new = convert_stl(&source, &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(new.report.cad.conical_faces, 0);
        assert_eq!(new.report.cad.faces, old.report.cad.faces);
        assert_eq!(
            new.report.cad.cylindrical_faces,
            old.report.cad.cylindrical_faces
        );
        assert!((new.report.cad.volume_mm3 - old.report.cad.volume_mm3).abs() < 1e-7);
        assert_eq!(
            new.report.settings.reconstruction,
            ReconstructionMode::Primitives
        );
    }
}

#[test]
fn conical_failures_and_budgets_never_become_faceted_successes() {
    for name in [
        "m4_sparse_cone",
        "m4_shifted_tip",
        "m4_warped_frustum",
        "m4_piecewise_taper",
        "m4_curved_meridian",
    ] {
        let error = convert_stl(&fixture(name), &options())
            .err()
            .expect("unproven cone refused");
        assert!(error.code.starts_with("recognition."), "{name}: {error}");
    }
    let opts = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    let error = convert_stl(&fixture("m4_frustum"), &opts)
        .err()
        .expect("chord budget refused");
    assert_eq!(error.code, "recognition.deviation_exceeded");
}

#[test]
fn three_mf_instances_keep_placements_and_refuse_contact() {
    let result = convert_document(
        include_bytes!("../../../fixtures/m4_frustum_instances.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.conical_faces, 2);
    assert_eq!(result.report.cad.faces, 6);
    for (body, expected) in result.report.bodies.iter().zip([
        [[-7., -7., 0.], [7., 7., 8.]],
        [[23., 3., 2.], [37., 17., 10.]],
    ]) {
        for (actual, expected) in body
            .conversion
            .cad
            .bounds_mm
            .iter()
            .flatten()
            .zip(expected.iter().flatten())
        {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(body.conversion.cad.conical_faces, 1);
        assert!(body.conversion.fidelity.is_some());
    }
    let error = convert_document(
        include_bytes!("../../../fixtures/m4_frustum_touching.3mf"),
        InputFormat::ThreeMf,
        &options(),
    )
    .err()
    .expect("contact refused");
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn multi_body_step_preserves_cones_and_settings() {
    let mesh = read_stl_mesh(&fixture("m4_frustum"), InputUnit::Mm).unwrap();
    let mut source = String::from("solid paired_cones\n");
    for offset in [0., 40.] {
        for tri in &mesh.triangles {
            source.push_str("facet normal 0 0 0\nouter loop\n");
            for &i in tri {
                let p = mesh.vertices[i];
                writeln!(
                    source,
                    "vertex {:.15} {:.15} {:.15}",
                    p[0] + offset,
                    p[1],
                    p[2]
                )
                .unwrap();
            }
            source.push_str("endloop\nendfacet\n");
        }
    }
    source.push_str("endsolid paired_cones\n");
    let opts = Options {
        cylinder_fit_tolerance_mm: 2e-5,
        ..options()
    };
    let result = convert_document(source.as_bytes(), InputFormat::Stl, &opts).unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.conical_faces, 2);
    assert_eq!(result.report.cad.cylindrical_faces, 0);
    assert_eq!(result.report.cad.faces, 6);
    assert_eq!(
        result.report.settings.reconstruction,
        ReconstructionMode::Primitives
    );
    assert_eq!(result.report.settings.cylinder_fit_tolerance_mm, 2e-5);
    assert!(result
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "precision.custom_tolerances"));
    assert!(result
        .report
        .bodies
        .iter()
        .all(|b| b.conversion.cad.conical_faces == 1 && b.conversion.fidelity.is_some()));
}
