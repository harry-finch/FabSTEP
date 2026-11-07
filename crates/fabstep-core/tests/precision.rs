use fabstep_core::{analyze_mesh, analyze_stl, recognize_mixed, Options, ReconstructionMode};

const ROUNDED: &[u8] = include_bytes!("../../../fixtures/precision_rounded.stl");

fn explicit_options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        cylinder_fit_tolerance_mm: 1e-4,
        ..Options::default()
    }
}

#[test]
fn six_digit_rounding_requires_explicit_fit_and_keeps_source_geometry() {
    let strict = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let source = analyze_stl(ROUNDED, &strict).unwrap();
    assert_eq!(
        recognize_mixed(&source, &strict).unwrap_err().code,
        "recognition.unsupported"
    );
    let explicit = explicit_options();
    let accepted = recognize_mixed(&source, &explicit).unwrap();
    assert_eq!(accepted.model.vertices, source.mesh.vertices);
    assert_eq!(accepted.triangle_faces.len(), source.mesh.triangles.len());
    assert_eq!(accepted.model.cylinders.len(), 2);
    assert_eq!(accepted.model.partial_cylinders.len(), 1);
    assert_eq!(accepted.model.planes.len(), 7);
    let report = accepted.report;
    assert!(report.max_radial_residual_mm > strict.cylinder_fit_tolerance_mm);
    assert!(report.max_radial_residual_mm <= explicit.cylinder_fit_tolerance_mm);
    assert!(report.max_chord_error_mm > 0.01);
    assert!(
        report.max_deviation_bound_mm >= report.max_radial_residual_mm + report.max_chord_error_mm
    );
    assert!(report.max_deviation_bound_mm <= strict.max_deviation_mm);
}

#[test]
fn explicit_fit_does_not_discard_an_outlier_or_accept_partial_coverage() {
    let options = explicit_options();
    let mut mesh = analyze_stl(ROUNDED, &options).unwrap().mesh;
    let point = mesh
        .vertices
        .iter_mut()
        .find(|p| **p == [50., 45., 23.])
        .expect("synthetic boss has the documented top-rim vertex");
    point[0] += 0.002;
    let source = analyze_mesh(&mesh, &options).unwrap();
    assert_eq!(
        recognize_mixed(&source, &options).unwrap_err().code,
        "recognition.unsupported"
    );
}

#[test]
fn explicit_fit_never_increases_the_deviation_budget() {
    let options = Options {
        max_deviation_mm: 0.001,
        ..explicit_options()
    };
    let source = analyze_stl(ROUNDED, &options).unwrap();
    assert_eq!(
        recognize_mixed(&source, &options).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}
