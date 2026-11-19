use fabstep_cad::build_rounded_step;
use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode};

#[test]
fn a_false_undercut_cannot_be_built_or_exported() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let model = recognize_rounded(
        &analyze_stl(
            include_bytes!("../../../fixtures/returning_meridian/flared-coarse.stl"),
            &o,
        )
        .unwrap(),
        &o,
    )
    .unwrap()
    .model;
    for poles in [[2., 6.], [50., -50.], [f64::NAN, 0.]] {
        let mut bad = model.clone();
        bad.smooth_revolution.as_mut().unwrap().axial_controls_mm = Some(poles);
        assert!(build_rounded_step(&bad).is_err());
    }
    let mut bad = model;
    bad.smooth_revolution.as_mut().unwrap().radii_mm[2] += 0.02;
    assert!(build_rounded_step(&bad).is_err());
}
