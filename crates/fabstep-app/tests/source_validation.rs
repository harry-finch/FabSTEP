use fabstep_app::{convert_document, convert_stl, InputFormat};
use fabstep_core::{analyze_stl, Options};

fn options() -> Options {
    Options {
        plane_tolerance_mm: 0.01,
        ..Options::default()
    }
}

#[test]
fn planar_grouping_cannot_hide_intersections_in_the_original_triangles() {
    let bytes = include_bytes!("../../../fixtures/v1_intersecting_plate.stl");
    let options = options();
    let analysis = analyze_stl(bytes, &options).unwrap();
    assert_eq!(analysis.mesh.triangles.len(), 22);
    assert_eq!(analysis.model.faces.len(), 6);
    // This deliberately establishes the old false-success precondition: the
    // grouped six-plane model is valid, despite its self-intersecting source.
    let grouped = fabstep_cad::build_step(&analysis.model).unwrap();
    assert_eq!(grouped.report.solids, 1);
    assert!((grouped.report.volume_mm3 - 1000.).abs() < 1e-8);
    let error = convert_stl(bytes, &options)
        .err()
        .expect("invalid source must be refused");
    assert_eq!(error.code, "cad.invalid_solid");
}

#[test]
fn single_body_stl_and_three_mf_documents_also_validate_the_source() {
    for (bytes, format) in [
        (
            include_bytes!("../../../fixtures/v1_intersecting_plate.stl").as_slice(),
            InputFormat::Stl,
        ),
        (
            include_bytes!("../../../fixtures/v1_intersecting_plate.3mf").as_slice(),
            InputFormat::ThreeMf,
        ),
    ] {
        let error = convert_document(bytes, format, &options())
            .err()
            .expect("invalid document must be refused");
        assert_eq!(error.code, "cad.invalid_solid");
    }
}

#[test]
fn a_valid_shallow_planar_relief_remains_convertible_with_explicit_tolerance() {
    let result = convert_stl(
        include_bytes!("../../../fixtures/v1_shallow_plate.stl"),
        &options(),
    )
    .unwrap();
    assert_eq!(result.report.cad.solids, 1);
    assert_eq!(result.report.cad.planar_faces, 6);
    assert!(result
        .report
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "planar.approximate"));
    assert!(!result
        .report
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "validation.native_required"));
}
