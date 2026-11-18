use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_automatic_guided, recognize_revolved,
    recognize_rounded, CrossDrill, GuidanceFamily, MixedLoop, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{collections::BTreeSet, fs, path::Path};
fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/cross_drill")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}
fn opts(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

#[test]
fn four_separate_blind_drills_keep_the_historical_revolved_path() {
    let raw = include_bytes!("../../../fixtures/historical_four_blind_drills.stl");
    let surfaces = opts(ReconstructionMode::Surfaces);
    let a = analyze_stl(raw, &surfaces).unwrap();
    // This status allows the application to invoke the historical recognizer.
    assert_eq!(
        recognize_rounded(&a, &surfaces).unwrap_err().code,
        "recognition.unsupported"
    );
    let r = recognize_revolved(&a, &opts(ReconstructionMode::Primitives)).unwrap();
    assert_eq!(r.model.mixed.planes.len(), 6);
    assert_eq!(r.model.mixed.cylinders.len(), 4);
    assert_eq!(r.model.cones.len(), 4);
}
fn to_local(f: &CrossDrill, p: [f64; 3]) -> [f64; 3] {
    f.axes
        .map(|v| (0..3).map(|i| v[i] * (p[i] - f.center[i])).sum())
}

#[test]
fn source_partition_proves_mouths_and_two_disjoint_oriented_branches() {
    for (name, small_count, large_count, vol) in [
        ("f_nominal", 384, 1344, 5302.986330628418),
        ("f_mirror", 384, 1344, 5302.986330628418),
        ("f_rotated", 384, 1344, 5302.986330628418),
        ("f_retessellated", 880, 3040, 5302.986330628418),
        ("f_scaled", 576, 1968, 17897.57886587091),
    ] {
        let raw = fixture(name);
        for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = opts(mode);
            let a = analyze_stl(&raw, &o).unwrap();
            let r = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{mode:?} {name}: {e}"));
            let f = r.model.cross_drill.as_deref().unwrap();
            let m = &r.model.revolved.mixed;
            assert_eq!(m.vertices, a.mesh.vertices);
            assert_eq!(r.report.cylinders, 3);
            assert!(m.cylinders.is_empty());
            assert_eq!(m.planes.len(), 6);
            assert_eq!(m.rings.len(), 4);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            let mut owners = [0; 9];
            for face in &r.triangle_faces {
                match *face {
                    RoundedFaceRef::Plane { index } => owners[index] += 1,
                    RoundedFaceRef::CrossDrill { index } => owners[6 + index] += 1,
                    _ => panic!("unrelated owner {face:?}"),
                }
            }
            assert!(owners.iter().all(|&n| n > 0));
            assert_eq!(&owners[6..], &[small_count, small_count, large_count]);
            assert!((r.report.expected_volume_mm3 - vol).abs() < 1e-4);
            assert!(
                r.report.max_deviation_bound_mm > r.report.max_chord_error_mm
                    && r.report.max_deviation_bound_mm < 0.016
            );
            assert!(r.report.max_wall_residual_mm >= 4e-7);
            let mut uses = BTreeSet::new();
            for p in &m.planes {
                assert!(matches!(p.loops[0], MixedLoop::Polygon { .. }));
                if let Some(MixedLoop::Circle { ring }) = p.loops.get(1) {
                    assert!(uses.insert(*ring));
                }
            }
            assert_eq!(uses, f.rims.into_iter().collect());
            let mut vertices = BTreeSet::new();
            for (i, branch) in f.branches.iter().enumerate() {
                assert!(branch.len() >= 32);
                let mut turn = 0.;
                for (j, &v) in branch.iter().enumerate() {
                    assert!(vertices.insert(v));
                    let p = to_local(f, m.vertices[v]);
                    let [small, big] = f.radii;
                    assert!((p[0].hypot(p[1]) - small).abs() < o.cylinder_fit_tolerance_mm);
                    assert!((p[1].hypot(p[2]) - big).abs() < o.cylinder_fit_tolerance_mm);
                    assert_eq!(p[2] > 0., i == 1);
                    let q = to_local(f, m.vertices[branch[(j + 1) % branch.len()]]);
                    let d = (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
                    assert!(d > 0.);
                    turn += d;
                }
                assert!((turn - std::f64::consts::TAU).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn folded_same_support_and_other_nonqualifying_geometry_never_get_the_domain() {
    let o = opts(ReconstructionMode::Surfaces);
    let auto = opts(ReconstructionMode::Auto);
    for name in ["f_ratio", "f_off_center", "f_fold"] {
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let e = recognize_rounded(&a, &o).unwrap_err();
        assert_eq!(e.code, "recognition.surface_unproven");
        if name == "f_fold" {
            assert!(e.message.contains("repliée"), "{e}");
        }
        assert!(recognize_automatic(&a, &auto)
            .unwrap()
            .model
            .cross_drill
            .is_none());
    }
    assert_eq!(
        analyze_stl(&fixture("f_missing_mouth"), &o)
            .unwrap_err()
            .code,
        "mesh.open"
    );
    let a = analyze_stl(&fixture("f_nominal"), &o).unwrap();
    let tight = Options {
        max_deviation_mm: 1e-4,
        ..o.clone()
    };
    assert_eq!(
        recognize_rounded(&a, &tight).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        recognize_automatic_guided(&a, &auto, &[(GuidanceFamily::Cylinder, vec![0])])
            .unwrap_err()
            .code,
        "guidance.family_unproven"
    );
    let r = recognize_rounded(&a, &o).unwrap();
    let f = r.model.cross_drill.unwrap();
    let v = f.branches[0][3];
    let mut altered = a.mesh;
    for (coord, dir) in altered.vertices[v].iter_mut().zip(f.axes[2]) {
        *coord += 0.002 * dir;
    }
    let a = analyze_mesh(&altered, &o).unwrap();
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.surface_unproven"
    );
}

#[test]
fn reversed_and_double_winding_intersections_fail_on_the_source_branch_itself() {
    let o = opts(ReconstructionMode::Surfaces);
    let a = analyze_stl(&fixture("f_nominal"), &o).unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    let f = r.model.cross_drill.unwrap();
    let [small, big] = f.radii;
    for winding in [-1., 2.] {
        let mut forged = a.mesh.clone();
        for &v in &f.branches[0] {
            let p = to_local(&f, a.mesh.vertices[v]);
            let phase = p[1].atan2(p[0]);
            // The phase perturbation keeps both turns' samples distinct at
            // all antipodal source vertices. All altered points lie exactly
            // on both cylinders; the combinatorial mesh remains unchanged.
            let theta = winding * phase + 0.016 * (phase + 0.314159).sin();
            let y = small * theta.sin();
            let q = [small * theta.cos(), y, -((big - y) * (big + y)).sqrt()];
            forged.vertices[v] = std::array::from_fn(|i| {
                f.center[i] + (0..3).map(|j| q[j] * f.axes[j][i]).sum::<f64>()
            });
        }
        let source = analyze_mesh(&forged, &o).unwrap_or_else(|e| panic!("winding {winding}: {e}"));
        let e = recognize_rounded(&source, &o).unwrap_err();
        assert_eq!(e.code, "recognition.surface_unproven");
        assert!(
            e.message.contains("branche transverse"),
            "winding {winding}: {e}"
        );
    }
}
