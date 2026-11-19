use fabstep_core::{analyze_stl, recognize_automatic, Options, ReconstructionMode};

#[test]
fn opposite_sides_bound_two_skew_conical_strips() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (bytes, volume) in [
        (
            include_bytes!("../../../fixtures/skew_linear_blends/vertical-coarse.stl").as_slice(),
            3151.607063,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/vertical-fine.stl").as_slice(),
            3151.607063,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/swapped-coarse.stl").as_slice(),
            3152.958225,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/swapped-fine.stl").as_slice(),
            3152.958225,
        ),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_automatic(&a, &o).unwrap();
        assert_eq!(r.report.cones, 2);
        assert_eq!(r.model.revolved.mixed.elliptic_arcs.len(), 4);
        assert_eq!(r.model.revolved.mixed.planes.len(), 6);
        assert!((r.report.expected_volume_mm3 - volume).abs() < 0.0001);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
    }
}

#[test]
fn two_complete_ellipses_on_each_cone_are_required() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/skew_linear_blends/notched-coarse.stl"),
        &o,
    )
    .unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert!(r.model.linear_blend.is_none());
}

#[test]
fn a_reflected_pair_preserves_coverage_and_a_bent_generator_refuses_both_domains() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let bytes = include_bytes!("../../../fixtures/skew_linear_blends/vertical-coarse.stl");
    let a = analyze_stl(bytes, &o).unwrap();
    let mut reflected = a.mesh.clone();
    for p in &mut reflected.vertices {
        let [x, y, z] = *p;
        *p = [-z + 2., x - 5., y + 10.];
    }
    for tri in &mut reflected.triangles {
        tri.swap(0, 1);
    }
    reflected.triangles.reverse();
    let r = recognize_automatic(&fabstep_core::analyze_mesh(&reflected, &o).unwrap(), &o).unwrap();
    assert_eq!(r.report.cones, 2);
    assert!((r.report.expected_volume_mm3 - 3151.607063).abs() < 0.0001);
    let mut bent = a.mesh;
    // Displace one knot only on the cone near X=20. The first cone and
    // the bounding planes retain their input dimensions.
    let knot = bent
        .vertices
        .iter_mut()
        .find(|p| p[0] > 16.1 && p[0] < 19.9 && p[1] > 0.1 && p[1] < 15.9 && p[2] > 0.1)
        .unwrap();
    knot[2] += 0.08;
    let warped = fabstep_core::analyze_mesh(&bent, &o).unwrap();
    assert!(recognize_automatic(&warped, &o).map_or(true, |r| r.model.linear_blend.is_none()));
}
