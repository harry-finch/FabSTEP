use fabstep_core::{InputUnit, Options};

#[test]
fn whole_pipeline_preserves_planar_solids_and_holes() {
    for (bytes, faces, volume) in [
        (
            include_bytes!("../../../fixtures/cube.stl").as_slice(),
            6,
            1.0,
        ),
        (
            include_bytes!("../../../fixtures/concave_prism.stl").as_slice(),
            8,
            6.0,
        ),
        (
            include_bytes!("../../../fixtures/rectangular_hole.stl").as_slice(),
            10,
            24.0,
        ),
    ] {
        let result = fabstep_app::convert_stl(bytes, &Options::default()).unwrap();
        assert_eq!(result.report.cad.solids, 1);
        assert_eq!(result.report.cad.faces, faces);
        assert!(!result
            .report
            .analysis
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "validation.native_required"));
        assert!((result.report.cad.volume_mm3 - volume).abs() < 1e-7);
        assert!(std::str::from_utf8(&result.step_bytes)
            .unwrap()
            .contains("MANIFOLD_SOLID_BREP"));
    }
}

#[test]
fn source_inches_produce_a_millimetre_step() {
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    let result =
        fabstep_app::convert_stl(include_bytes!("../../../fixtures/cube.stl"), &options).unwrap();
    assert!((result.report.cad.volume_mm3 - 25.4_f64.powi(3)).abs() < 1e-5);
    for dimension in result.report.cad.bounds_mm[1] {
        assert!((dimension - 25.4).abs() < 1e-7);
    }
}
