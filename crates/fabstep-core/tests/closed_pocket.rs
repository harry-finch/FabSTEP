use fabstep_core::{analyze_stl, recognize_automatic, Options, ReconstructionMode};

#[test]
fn the_four_double_elliptic_strips_share_a_single_closed_cycle() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (bytes, volume) in [
        (
            include_bytes!("../../../fixtures/closed_pocket/loop-coarse.stl").as_slice(),
            17921.309022,
        ),
        (
            include_bytes!("../../../fixtures/closed_pocket/loop-fine.stl").as_slice(),
            17921.309022,
        ),
        (
            include_bytes!("../../../fixtures/closed_pocket/wide-coarse.stl").as_slice(),
            17446.779975,
        ),
        (
            include_bytes!("../../../fixtures/closed_pocket/narrow-fine.stl").as_slice(),
            20851.071178,
        ),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        let result = recognize_automatic(&a, &o).unwrap();
        let m = &result.model.revolved.mixed;
        assert_eq!(
            (
                m.planes.len(),
                m.elliptic_arcs.len(),
                m.partial_cylinders.len()
            ),
            (11, 4, 4)
        );
        for (i, w) in m.partial_cylinders.iter().enumerate() {
            assert!(w.elliptic_start && w.elliptic_end && w.inner);
            assert_eq!(w.arcs, [i, (i + 1) % 4]);
        }
        assert!((result.report.expected_volume_mm3 - volume).abs() < 0.001);
    }
}

#[test]
fn warped_junction_and_interrupted_floor_refuse_the_cycle() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let raw = include_bytes!("../../../fixtures/closed_pocket/sections.stl");
    let a = analyze_stl(raw, &o).unwrap();
    let mut mesh = a.mesh;
    let v = mesh
        .vertices
        .iter_mut()
        .find(|p| p[0] > 5.1 && p[0] < 7.4 && (p[0] - 5. - (p[1] - 6.)).abs() < 1e-5 && p[2] > 3.1)
        .unwrap();
    v[0] += 0.05;
    for a in [
        fabstep_core::analyze_mesh(&mesh, &o).unwrap(),
        analyze_stl(
            include_bytes!("../../../fixtures/closed_pocket/through-coarse.stl"),
            &o,
        )
        .unwrap(),
        analyze_stl(
            include_bytes!("../../../fixtures/closed_pocket/notched-coarse.stl"),
            &o,
        )
        .unwrap(),
    ] {
        let r = recognize_automatic(&a, &o).unwrap();
        assert!(r.model.revolved.mixed.partial_cylinders.is_empty());
    }
}
