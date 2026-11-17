use fabstep_core::{
    analyze_mesh, analyze_stl, read_stl_mesh, recognize_mixed, InputUnit, Options,
    ReconstructionMode,
};

fn options(fit: f64) -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        cylinder_fit_tolerance_mm: fit,
        ..Options::default()
    }
}

#[test]
fn supported_short_arcs_preserve_complete_source_walls_and_analytic_volume() {
    for (bytes, volume, fit) in [
        (
            include_bytes!("../../../fixtures/short_arcs/convex.stl").as_slice(),
            81.71583288834287,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/short_arcs/concave.stl").as_slice(),
            59.58863245622936,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/short_arcs/rotated.stl").as_slice(),
            81.71583288834287,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/short_arcs/f32_translated.stl").as_slice(),
            81.71583288834287,
            2e-5,
        ),
    ] {
        let o = options(fit);
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_mixed(&a, &o).unwrap();
        assert_eq!(r.model.partial_cylinders.len(), 3);
        assert_eq!(r.model.planes.len(), 3);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!((r.report.expected_volume_mm3 - volume).abs() < 0.001);
        assert!(r.report.max_radial_residual_mm <= fit);
        assert!(r.report.max_deviation_bound_mm < o.max_deviation_mm);
        assert_eq!(
            r.model
                .arcs
                .iter()
                .filter(|arc| arc.sweep_rad < 30_f64.to_radians())
                .count(),
            2
        );
        let again = recognize_mixed(&a, &o).unwrap();
        assert_eq!(
            r.report.expected_volume_mm3,
            again.report.expected_volume_mm3
        );
        assert_eq!(
            r.report.max_deviation_bound_mm,
            again.report.max_deviation_bound_mm
        );
    }
}

#[test]
fn unresolved_or_incompatible_gaps_remain_faceted_without_changing_supports() {
    for (bytes, volume) in [
        (
            include_bytes!("../../../fixtures/short_arcs/unresolved.stl").as_slice(),
            70.239732376302,
        ),
        (
            include_bytes!("../../../fixtures/short_arcs/polygonal_gap.stl").as_slice(),
            81.70575706064876,
        ),
    ] {
        let o = options(1e-5);
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_mixed(&a, &o).unwrap();
        assert_eq!(r.model.partial_cylinders.len(), 2);
        assert_eq!(r.model.planes.len(), 6);
        assert!((r.report.expected_volume_mm3 - volume).abs() < 1e-8);
    }
}

#[test]
fn eligible_short_arc_wall_failures_block_the_whole_result() {
    for bytes in [
        include_bytes!("../../../fixtures/short_arcs/bulge.stl").as_slice(),
        include_bytes!("../../../fixtures/short_arcs/fold.stl").as_slice(),
    ] {
        let o = options(1e-5);
        let a = analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_mixed(&a, &o).unwrap_err().code,
            "recognition.partial_unproven"
        );
    }
}

#[test]
fn absent_supports_missing_source_and_insufficient_budget_are_not_bypassed() {
    let o = options(1e-5);
    let a = analyze_stl(
        include_bytes!("../../../fixtures/short_arcs/no_anchors.stl"),
        &o,
    )
    .unwrap();
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.unsupported"
    );
    let mut mesh = read_stl_mesh(
        include_bytes!("../../../fixtures/short_arcs/convex.stl"),
        InputUnit::Mm,
    )
    .unwrap();
    mesh.triangles.remove(20);
    assert_eq!(analyze_mesh(&mesh, &o).unwrap_err().code, "mesh.open");
    let limited = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/short_arcs/convex.stl"),
        &limited,
    )
    .unwrap();
    assert_eq!(
        recognize_mixed(&a, &limited).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn noisy_four_point_gap_is_not_promoted_by_neighboring_circles() {
    // Perturb one interior witness on BOTH source caps, preserving endpoints,
    // the two historical supports, closure, and the exact vertical rails.
    let mut mesh = read_stl_mesh(
        include_bytes!("../../../fixtures/short_arcs/convex.stl"),
        InputUnit::Mm,
    )
    .unwrap();
    let initial = -70_f64.to_radians();
    let join = -10_f64.to_radians();
    let center = [
        -4. * initial.sin() + 3. * join.sin(),
        4. * initial.cos() - 3. * join.cos(),
    ];
    let angle = (-100_f64 + 20. / 3.).to_radians();
    let target = [center[0] + angle.cos(), center[1] + angle.sin()];
    let mut changed = 0;
    for p in &mut mesh.vertices {
        if (p[0] - target[0]).hypot(p[1] - target[1]) < 1e-9 {
            p[0] = center[0] + 1.001 * angle.cos();
            p[1] = center[1] + 1.001 * angle.sin();
            changed += 1;
        }
    }
    assert!(changed >= 2);
    let o = options(1e-5);
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_mixed(&a, &o).unwrap();
    assert_eq!(r.model.partial_cylinders.len(), 2);
    assert_eq!(r.model.planes.len(), 6);
}
