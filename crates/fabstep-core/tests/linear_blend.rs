use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_automatic_guided, recognize_rounded,
    EllipticArc, GuidanceFamily, MixedLoop, MixedSegment, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{collections::BTreeSet, fs, path::Path};
fn source(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/local_blends")
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
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}
fn endpoint(a: &EllipticArc, t: f64) -> [f64; 3] {
    let [x, y, z] = a.axis;
    let [u, v, w] = a.major_direction;
    let minor = [y * w - z * v, z * u - x * w, x * v - y * u];
    std::array::from_fn(|i| {
        a.center[i] + a.major_mm * t.cos() * a.major_direction[i] + a.minor_mm * t.sin() * minor[i]
    })
}

#[test]
fn constant_open_arcs_keep_the_historical_surfaces_fallback() {
    let o = opts(ReconstructionMode::Surfaces);
    for bytes in [
        include_bytes!("../../../fixtures/m2c_open_arc.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_subdivided.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_rotated.stl").as_slice(),
        include_bytes!("../../../fixtures/partial_fillets/one_edge.stl").as_slice(),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        // Only this code permits the application's historical recognition pass.
        assert_eq!(
            recognize_rounded(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
    }
}

#[test]
fn assigned_linear_blend_has_two_shared_ellipses_and_exact_volume() {
    for (name, cone_tris) in [
        ("g_increasing", 240),
        ("g_decreasing", 240),
        ("g_mirror", 240),
        ("g_permuted", 392),
        ("g_oblique_staggered", 512),
    ] {
        for reconstruction in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = opts(reconstruction);
            let a = analyze_stl(&source(name), &o).unwrap();
            let rec = if reconstruction == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{name} {reconstruction:?}: {e}"));
            let g = rec.model.linear_blend.as_ref().unwrap();
            let m = &rec.model.revolved.mixed;
            assert_eq!(rec.report.cones, 1);
            assert!(rec.model.partial_cones.is_empty() && rec.model.revolved.cones.is_empty());
            assert_eq!(m.planes.len(), 6);
            assert_eq!(m.elliptic_arcs.len(), 2);
            assert!((g.semi_angle_rad - (0.2f64 / (1.08f64).sqrt()).asin()).abs() < 1e-7);
            assert!((rec.report.expected_volume_mm3 - 3165.4015953235107).abs() < 2e-5);
            assert!(rec.report.max_deviation_bound_mm <= 0.011);
            let mut owners = [0; 7];
            for f in &rec.triangle_faces {
                match *f {
                    RoundedFaceRef::Plane { index } => owners[index] += 1,
                    RoundedFaceRef::LinearBlend { index: 0 } => owners[6] += 1,
                    _ => panic!("unexpected owner {f:?}"),
                }
            }
            assert!(owners.iter().all(|&n| n > 0));
            assert_eq!(owners[6], cone_tris);
            assert_eq!(owners.iter().sum::<usize>(), a.mesh.triangles.len());
            let mut uses = vec![];
            for plane in &m.planes {
                if let MixedLoop::Segments { segments } = &plane.loops[0] {
                    for s in segments {
                        if let MixedSegment::EllipticArc { arc, reversed } = s {
                            uses.push((*arc, *reversed));
                        }
                    }
                }
            }
            uses.sort();
            assert_eq!(uses, [(0, true), (1, false)]);
            let corner_ids: BTreeSet<_> = m
                .elliptic_arcs
                .iter()
                .flat_map(|arc| arc.vertices)
                .collect();
            assert_eq!(corner_ids.len(), 4);
            for (i, (old, p)) in a.mesh.vertices.iter().zip(&m.vertices).enumerate() {
                if old != p {
                    assert!(corner_ids.contains(&i));
                    assert!(distance(*old, *p) <= rec.report.max_wall_residual_mm);
                }
            }
            let count = m
                .vertices
                .iter()
                .zip(&a.mesh.vertices)
                .filter(|(p, q)| p != q)
                .count();
            let reported = rec
                .report
                .diagnostics
                .iter()
                .find(|d| d.code == "recognition.linear_blend_endpoints_projected")
                .map_or(0, |d| d.count);
            assert_eq!(count, reported);
            for a in &m.elliptic_arcs {
                assert!(a.major_mm > a.minor_mm);
                for (id, t) in a
                    .vertices
                    .into_iter()
                    .zip([a.start_rad, a.start_rad + a.sweep_rad])
                {
                    assert!(distance(m.vertices[id], endpoint(a, t)) < 1e-7);
                }
            }
        }
    }
}

#[test]
fn rejects_unproved_linear_blends_and_preserves_automatic_fallback() {
    let o = opts(ReconstructionMode::Surfaces);
    let auto = opts(ReconstructionMode::Auto);
    for name in ["g_nonlinear", "g_circular_stack", "g_parabolic_cap"] {
        let a = analyze_stl(&source(name), &o).unwrap();
        assert!(recognize_rounded(&a, &o).is_err(), "{name}");
        let rec = recognize_automatic(&a, &auto).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(rec.model.linear_blend.is_none());
    }
    assert_eq!(
        analyze_stl(&source("g_missing_cap"), &o).unwrap_err().code,
        "mesh.open"
    );
    let a = analyze_stl(&source("g_increasing"), &o).unwrap();
    assert_eq!(
        recognize_rounded(
            &a,
            &Options {
                max_deviation_mm: 1e-4,
                ..o.clone()
            }
        )
        .unwrap_err()
        .code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        recognize_automatic_guided(&a, &auto, &[(GuidanceFamily::Cone, vec![0])])
            .unwrap_err()
            .code,
        "guidance.family_unproven"
    );
}

#[test]
fn rejects_folded_chart_with_vertices_still_on_the_envelope() {
    let o = opts(ReconstructionMode::Surfaces);
    let mut mesh = analyze_stl(&source("g_increasing"), &o).unwrap().mesh;
    let &[a, b, _] = mesh
        .triangles
        .iter()
        .find(|ids| {
            ids.iter().all(|&v| {
                let [x, y, z] = mesh.vertices[v];
                x > 0.1 && x < 2. && y > 0.1 && y < 2. && z > 1. && z < 9.
            })
        })
        .unwrap();
    mesh.vertices.swap(a, b);
    let a = analyze_mesh(&mesh, &o).unwrap();
    let err = recognize_rounded(&a, &o).unwrap_err();
    assert_eq!(err.code, "recognition.surface_unproven");
    assert!(err.message.contains("replié"), "{err}");
}

#[test]
fn fitted_endpoints_move_only_on_the_result_and_report_the_displacement() {
    let o = opts(ReconstructionMode::Surfaces);
    let mut mesh = analyze_stl(&source("g_increasing"), &o).unwrap().mesh;
    let i = mesh
        .vertices
        .iter()
        .position(|p| *p == [3., 0., 0.])
        .unwrap();
    mesh.vertices[i][0] += 1.6e-6;
    let a = analyze_mesh(&mesh, &o).unwrap();
    let original = a.mesh.vertices.clone();
    let r = recognize_rounded(&a, &o).unwrap();
    let result = &r.model.revolved.mixed.vertices;
    let displacement = original
        .iter()
        .zip(result)
        .map(|(p, q)| distance(*p, *q))
        .fold(0., f64::max);
    assert!(displacement > 5e-7 && displacement <= r.report.max_wall_residual_mm);
    assert_eq!(original, a.mesh.vertices);
    assert_eq!(
        r.report
            .diagnostics
            .iter()
            .find(|d| d.code == "recognition.linear_blend_endpoints_projected")
            .unwrap()
            .count,
        original.iter().zip(result).filter(|(p, q)| p != q).count()
    );
    mesh.vertices[i][0] += 0.03;
    let altered = analyze_mesh(&mesh, &o).unwrap();
    assert_eq!(
        recognize_rounded(&altered, &o).unwrap_err().code,
        "recognition.surface_unproven"
    );
}

#[test]
fn chart_is_invariant_to_triangle_order_and_general_rigid_placement() {
    let o = opts(ReconstructionMode::Surfaces);
    let mut mesh = analyze_stl(&source("g_oblique_staggered"), &o)
        .unwrap()
        .mesh;
    let count = mesh.vertices.len();
    mesh.vertices.reverse();
    for p in &mut mesh.vertices {
        let [x, y, z] = *p;
        *p = [17. + x, -11. + 0.6 * y + 0.8 * z, 80. - 0.8 * y + 0.6 * z];
    }
    mesh.triangles.reverse();
    for tri in &mut mesh.triangles {
        tri.rotate_left(1);
        *tri = tri.map(|v| count - 1 - v);
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    assert!(r.model.linear_blend.is_some());
    assert_eq!(
        r.triangle_faces
            .iter()
            .filter(|f| matches!(f, RoundedFaceRef::LinearBlend { .. }))
            .count(),
        512
    );
    assert!((r.report.expected_volume_mm3 - 3165.4015953235107).abs() < 1e-7);
}
