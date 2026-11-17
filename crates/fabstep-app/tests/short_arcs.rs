use fabstep_app::convert_stl;
use fabstep_core::{Options, ReconstructionMode};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/short_arcs")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

#[test]
fn contextual_short_arcs_reimport_and_unresolved_gaps_stay_faceted() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/short_arcs/expected.json")).unwrap();
    for (name, expected) in manifest["cases"].as_object().unwrap() {
        if expected.get("expected_error").is_some() {
            continue;
        }
        for reconstruction in [
            ReconstructionMode::Cylinders,
            ReconstructionMode::Primitives,
            ReconstructionMode::Surfaces,
        ] {
            let options = Options {
                reconstruction,
                cylinder_fit_tolerance_mm: expected["fit_mm"].as_f64().unwrap(),
                ..Options::default()
            };
            let conversion = convert_stl(&fixture(name), &options)
                .unwrap_or_else(|e| panic!("{name}/{reconstruction:?}: {e}"));
            let report = &conversion.report;
            assert_eq!(report.cad.solids, 1);
            let planes = expected["planes"].as_u64().unwrap() as usize;
            let cylinders = expected["cylinders"].as_u64().unwrap() as usize;
            assert_eq!(report.cad.planar_faces, planes, "{name}");
            assert_eq!(report.cad.cylindrical_faces, cylinders, "{name}");
            assert_eq!(report.cad.faces, planes + cylinders, "{name}");
            let numerical = if name.contains("f32") { 0.001 } else { 1e-7 };
            assert!(
                (report.cad.volume_mm3 - expected["volume_mm3"].as_f64().unwrap()).abs()
                    < numerical,
                "{name}: volume {}",
                report.cad.volume_mm3
            );
            for end in 0..2 {
                for axis in 0..3 {
                    let nominal = expected["bounds_mm"][end][axis].as_f64().unwrap();
                    assert!(
                        (report.cad.bounds_mm[end][axis] - nominal).abs() < 2e-5,
                        "{name}: bounds {:?}",
                        report.cad.bounds_mm
                    );
                }
            }
            let mixed = report.mixed_recognition.as_ref().unwrap();
            assert_eq!(mixed.model.partial_cylinders.len(), cylinders);
            assert_eq!(mixed.model.vertices, report.analysis.mesh.vertices);
            assert_eq!(
                mixed.triangle_faces.len(),
                report.analysis.mesh.triangles.len()
            );
            assert!(mixed.report.max_deviation_bound_mm <= options.max_deviation_mm);
            assert!(mixed.report.max_radial_residual_mm <= options.cylinder_fit_tolerance_mm);
            let fidelity = report
                .fidelity
                .as_ref()
                .expect("native bidirectional audit");
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(fidelity.source_to_result_mm < options.max_deviation_mm);
            assert!(fidelity.result_to_source_mm < options.max_deviation_mm);
            assert_eq!(
                std::str::from_utf8(&conversion.step_bytes)
                    .unwrap()
                    .matches("CYLINDRICAL_SURFACE(")
                    .count(),
                cylinders
            );
            assert_eq!(
                report.settings.cylinder_fit_tolerance_mm,
                options.cylinder_fit_tolerance_mm
            );
            assert!(report
                .analysis
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "recognition.circular_intent_assumed"));
        }
    }
}

#[test]
fn short_arc_wall_failures_cannot_restore_the_two_support_cylinders() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    for (name, code) in [
        ("bulge", "recognition.partial_unproven"),
        ("fold", "recognition.partial_unproven"),
        // Surfaces continues the historical unsupported-cylinder dispatch,
        // then the terminal sphere recognizer refuses the planar source caps.
        ("no_anchors", "recognition.surface_unproven"),
    ] {
        let error = convert_stl(&fixture(name), &options)
            .err()
            .expect("must not export");
        assert_eq!(error.code, code, "{name}: {error}");
    }
    let error = convert_stl(
        &fixture("convex"),
        &Options {
            max_deviation_mm: 1e-5,
            ..options
        },
    )
    .err()
    .expect("insufficient deviation budget");
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
