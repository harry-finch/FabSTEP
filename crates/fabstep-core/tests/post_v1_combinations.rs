use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode, RoundedFaceRef};
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
fn complete_cone_and_disjoint_open_cylinder_have_exhaustive_provenance() {
    for name in [
        "countersink_notch",
        "countersink_notch_rotated",
        "countersink_notch_mirrored",
    ] {
        let bytes = fixture(&format!("post_v1_combinations/{name}.stl"));
        let a = analyze_stl(&bytes, &options()).unwrap();
        let result = recognize_rounded(&a, &options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(result.model.revolved.cones.len(), 1);
        assert_eq!(result.model.revolved.mixed.cylinders.len(), 1);
        assert_eq!(result.model.revolved.mixed.partial_cylinders.len(), 1);
        assert!(result.model.spheres.is_empty() && result.model.tori.is_empty());
        assert_eq!(result.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(result.triangle_faces.len(), a.mesh.triangles.len());
        assert!(result.model.revolved.cones[0].inner);
        assert!(result.model.revolved.mixed.cylinders[0].inner);
        assert!(result.model.revolved.mixed.partial_cylinders[0].inner);
        assert_eq!(result.report.cones, 1);
        assert_eq!(result.report.cylinders, 2);
        let nominal = 44. * 24. * 10. - std::f64::consts::PI * (80. + 54. + 84.);
        assert!((result.report.expected_volume_mm3 - nominal).abs() < 1e-7);
        assert!(result.report.max_deviation_bound_mm <= options().max_deviation_mm);
        assert!(result
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "recognition.conic_intent_assumed" && d.count == 1));
        for owner in &result.triangle_faces {
            match owner {
                RoundedFaceRef::Plane { index } => {
                    assert!(*index < result.model.revolved.mixed.planes.len())
                }
                RoundedFaceRef::Cone { index }
                | RoundedFaceRef::Cylinder { index }
                | RoundedFaceRef::PartialCylinder { index } => assert_eq!(*index, 0),
                _ => panic!("unexpected rounded face"),
            }
        }
        // Repeat the full recognition, including all owner indices and residuals.
        assert_eq!(
            format!("{result:?}"),
            format!("{:?}", recognize_rounded(&a, &options()).unwrap())
        );
    }
}

#[test]
fn no_arc_single_cones_keep_the_historical_fallback_before_uv_proof() {
    for name in [
        "post_v1_combinations/countersink_only.stl",
        "m4_frustum.stl",
        "m4_chamfered_cylinder.stl",
        "m4_cone_tip.stl",
        "m4_countersink.stl",
        "m4_chamfered_boss.stl",
        "m4_drill_tip.stl",
    ] {
        // A deliberately strict budget would reject the new UV proof. Absence
        // of an open wall must still return unsupported to the old dispatcher.
        let o = Options {
            max_deviation_mm: 1e-8,
            ..options()
        };
        let a = analyze_stl(&fixture(name), &o).unwrap();
        assert_eq!(
            recognize_rounded(&a, &o).unwrap_err().code,
            "recognition.unsupported",
            "{name}"
        );
    }
}

#[test]
fn false_arcs_and_cones_are_not_promoted_and_deviation_stays_blocking() {
    for (name, expected) in [
        ("cone_warp", "recognition.surface_unproven"),
        ("arc_bulge", "recognition.partial_unproven"),
        ("arc_sparse", "recognition.unsupported"),
    ] {
        let a = analyze_stl(
            &fixture(&format!("post_v1_combinations/{name}.stl")),
            &options(),
        )
        .unwrap();
        assert_eq!(
            recognize_rounded(&a, &options()).unwrap_err().code,
            expected,
            "{name}"
        );
    }
    let o = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    let a = analyze_stl(&fixture("post_v1_combinations/countersink_notch.stl"), &o).unwrap();
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        recognize_rounded(
            &a,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..o
            }
        )
        .unwrap_err()
        .code,
        "recognition.disabled"
    );
}

#[test]
fn dense_open_arc_respects_the_shared_work_budget() {
    let a = analyze_stl(
        &fixture("post_v1_combinations/arc_dense_budget.stl"),
        &options(),
    )
    .unwrap();
    assert_eq!(
        recognize_rounded(&a, &options()).unwrap_err().code,
        "recognition.too_complex"
    );
}
