use fabstep_app::convert_stl;
use fabstep_core::{Options, ReconstructionMode};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/post_v1_combinations")
            .join(format!("{name}.stl")),
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
fn cone_and_notch_native_export_reimports_surfaces_volume_and_fidelity() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/post_v1_combinations/manifest.json"
    ))
    .unwrap();
    for name in [
        "countersink_notch",
        "countersink_notch_rotated",
        "countersink_notch_mirrored",
    ] {
        let result =
            convert_stl(&fixture(name), &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let report = &result.report;
        let expected = &manifest["cases"][name];
        assert_eq!(report.cad.solids, 1);
        assert_eq!(
            report.cad.planar_faces as u64,
            expected["planes"].as_u64().unwrap()
        );
        assert_eq!(report.cad.cylindrical_faces, 2);
        assert_eq!(report.cad.conical_faces, 1);
        assert_eq!(report.cad.spherical_faces, 0);
        assert_eq!(report.cad.toroidal_faces, 0);
        assert_eq!(report.cad.faces, report.cad.planar_faces + 3);
        let nominal = expected["volume_mm3"].as_f64().unwrap();
        assert!(
            (report.cad.volume_mm3 - nominal).abs() < 1e-7,
            "{name}: {}",
            report.cad.volume_mm3
        );
        for end in 0..2 {
            for axis in 0..3 {
                assert!(
                    (report.cad.bounds_mm[end][axis]
                        - expected["bounds_mm"][end][axis].as_f64().unwrap())
                    .abs()
                        < 1e-6,
                    "{name}: bounds"
                );
            }
        }
        let rounded = report.rounded_recognition.as_ref().unwrap();
        assert_eq!(rounded.model.revolved.mixed.partial_cylinders.len(), 1);
        assert_eq!(
            rounded.triangle_faces.len(),
            report.analysis.mesh.triangles.len()
        );
        assert_eq!(
            rounded.model.revolved.mixed.vertices,
            report.analysis.mesh.vertices
        );
        let fidelity = report.fidelity.as_ref().unwrap();
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(fidelity.source_to_result_mm < options().max_deviation_mm);
        assert!(fidelity.result_to_source_mm < options().max_deviation_mm);
        let step = std::str::from_utf8(&result.step_bytes).unwrap();
        assert_eq!(step.matches("CONICAL_SURFACE(").count(), 1);
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 2);
    }
}

#[test]
fn new_combination_does_not_enable_legacy_modes_or_faceted_failures() {
    for (name, code) in [
        ("cone_warp", "recognition.surface_unproven"),
        ("arc_dense_budget", "recognition.too_complex"),
        ("arc_bulge", "recognition.partial_unproven"),
        ("intersecting_features", "recognition.partial_unproven"),
    ] {
        let e = convert_stl(&fixture(name), &options())
            .err()
            .unwrap_or_else(|| panic!("{name}: unproved source must fail"));
        assert_eq!(e.code, code, "{name}: {e}");
    }
    let bytes = fixture("countersink_notch");
    let cylinders = convert_stl(
        &bytes,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(cylinders.code, "recognition.unsupported");
    let primitives = convert_stl(
        &bytes,
        &Options {
            reconstruction: ReconstructionMode::Primitives,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(primitives.report.rounded_recognition.is_none());
    assert_eq!(primitives.report.cad.conical_faces, 1);
    assert_eq!(primitives.report.cad.cylindrical_faces, 1);
    assert_eq!(primitives.report.cad.faces, 33);
    assert!(primitives
        .report
        .revolved_recognition
        .as_ref()
        .unwrap()
        .model
        .mixed
        .partial_cylinders
        .is_empty());
    let e = convert_stl(
        &bytes,
        &Options {
            max_deviation_mm: 0.001,
            ..options()
        },
    )
    .err()
    .unwrap();
    assert_eq!(e.code, "recognition.deviation_exceeded");
}

#[test]
fn cone_without_open_arc_keeps_historical_representation() {
    for name in ["countersink_only", "arc_sparse"] {
        let bytes = fixture(name);
        let old = convert_stl(
            &bytes,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..Options::default()
            },
        )
        .unwrap();
        let new = convert_stl(&bytes, &options()).unwrap();
        assert!(new.report.rounded_recognition.is_none());
        assert!(new.report.revolved_recognition.is_some());
        assert_eq!(new.report.cad.faces, old.report.cad.faces);
        assert_eq!(new.report.cad.volume_mm3, old.report.cad.volume_mm3);
        assert_eq!(new.report.cad.cylindrical_faces, 1);
        assert!(new
            .report
            .revolved_recognition
            .as_ref()
            .unwrap()
            .model
            .mixed
            .partial_cylinders
            .is_empty());
    }
}
