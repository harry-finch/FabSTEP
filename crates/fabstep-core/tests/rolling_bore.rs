use fabstep_core::{analyze_stl, rolling_bore::candidate, Options};
#[test]
fn the_caps_and_both_trimmed_cylinders_stay_within_the_budget() {
    let o = Options::default();
    for src in [
        include_bytes!("../../../fixtures/rolling_bore/shaft-coarse.stl").as_slice(),
        include_bytes!("../../../fixtures/rolling_bore/shaft-fine.stl").as_slice(),
        include_bytes!("../../../fixtures/rolling_bore/short-shaft-coarse.stl").as_slice(),
        include_bytes!("../../../fixtures/rolling_bore/short-shaft-fine.stl").as_slice(),
    ] {
        let a = analyze_stl(src, &o).unwrap();
        let model = candidate(&a, &o).expect("two cap disks and two cylindrical domains");
        // The intact rings constrain both transverse coordinates even when a
        // particular triangle order gives a noisier three-point circle.
        assert!(model.origin[0].hypot(model.origin[1]) < 2e-7);
        assert!((model.offset - 1.).abs() < 2e-7);
        assert!((model.bore - 2.8).abs() < 2e-7);
        assert!(model.support_bound_mm < 0.05);
        assert_eq!(model.owners.len(), a.mesh.triangles.len());
    }
    let spindle = analyze_stl(
        include_bytes!("../../../fixtures/rolling_bore/spindle-coarse.stl"),
        &o,
    )
    .unwrap();
    assert!(candidate(&spindle, &o).is_none());
}
