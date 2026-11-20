use fabstep_cad::{build_oblique_lip, build_revolved_step_with_provenance, result_faces};
use fabstep_core::{analyze_stl, oblique_lip::ObliqueLip, recognize_automatic, Options};

#[test]
fn exact_surface_transfer_preserves_the_next_writers_narrowing_cone() {
    let options = Options::default();
    let analysis =
        analyze_stl(include_bytes!("../../../fixtures/m4_frustum.stl"), &options).unwrap();
    let model = recognize_automatic(&analysis, &options)
        .unwrap()
        .model
        .revolved;
    let cone = &model.cones[0];
    let [lower, upper] = cone.rings.map(|i| model.mixed.rings[i].radius_mm);
    assert!(lower > upper);
    let transfer = || {
        let step = build_revolved_step_with_provenance(&model, &[1, 2], &[], &[3]).unwrap();
        assert_eq!(step.report.faces, 3);
        assert_eq!(step.report.conical_faces, 1);
        assert_eq!(result_faces(&step.bytes).unwrap().faces.len(), 3);
    };
    transfer();
    let lip = ObliqueLip {
        origin: [0.; 3],
        axis: [0., 0., 1.],
        radial: [1., 0., 0.],
        outer_radius_mm: 12.,
        major_radius_mm: 10.,
        height_mm: 8.,
        cut_offset_mm: -3.74,
        slope: 0.268,
    };
    build_oblique_lip(&lip, None).unwrap();
    transfer();
}
