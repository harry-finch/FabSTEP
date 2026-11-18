use fabstep_app::{analyze_document, convert_document, InputFormat};
use fabstep_core::{Options, ReconstructionMode};

const EXACT: &[u8] = include_bytes!("../../../fixtures/precision_exact.3mf");
const ROUNDED: &[u8] = include_bytes!("../../../fixtures/precision_rounded.3mf");
const OUTLIER: &[u8] = include_bytes!("../../../fixtures/precision_outlier.3mf");

fn options(fit: f64) -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        cylinder_fit_tolerance_mm: fit,
        ..Options::default()
    }
}

#[test]
fn rounded_3mf_requires_explicit_fit_and_reimports_all_analytic_surfaces() {
    let strict = options(1e-5);
    let exact_source = analyze_document(EXACT, InputFormat::ThreeMf, &strict).unwrap();
    let rounded_source = analyze_document(ROUNDED, InputFormat::ThreeMf, &strict).unwrap();
    let exact_mesh = &exact_source.bodies[0].analysis.mesh;
    let rounded_mesh = &rounded_source.bodies[0].analysis.mesh;
    assert_eq!(exact_mesh.triangles, rounded_mesh.triangles);
    let displacement = exact_mesh
        .vertices
        .iter()
        .zip(&rounded_mesh.vertices)
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]))
        .fold(0., f64::max);
    assert!(displacement > 1e-5 && displacement < 1e-4);
    let error = convert_document(ROUNDED, InputFormat::ThreeMf, &strict)
        .err()
        .expect("rounding exceeds the strict fit tolerance");
    assert_eq!(error.code, "recognition.unsupported");

    for (bytes, opts) in [(EXACT, strict), (ROUNDED, options(1e-4))] {
        let result = convert_document(bytes, InputFormat::ThreeMf, &opts).unwrap();
        assert_eq!(result.report.cad.solids, 1);
        assert_eq!(result.report.bodies.len(), 1);
        let body = &result.report.bodies[0].conversion;
        assert_eq!(body.cad.faces, 10);
        assert_eq!(body.cad.planar_faces, 7);
        assert_eq!(body.cad.cylindrical_faces, 3);
        let mixed = body.mixed_recognition.as_ref().unwrap();
        assert_eq!(mixed.triangle_faces.len(), 2114);
        assert_eq!(mixed.model.partial_cylinders.len(), 1);
        assert!(mixed.report.max_radial_residual_mm <= opts.cylinder_fit_tolerance_mm);
        assert!(mixed.report.max_deviation_bound_mm <= opts.max_deviation_mm);
        assert!((body.cad.volume_mm3 - mixed.report.expected_volume_mm3).abs() < 1e-5);
        let fidelity = body.fidelity.as_ref().unwrap();
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(fidelity.source_to_result_mm < opts.max_deviation_mm);
        assert!(fidelity.result_to_source_mm < opts.max_deviation_mm);
        let step = std::str::from_utf8(&result.step_bytes).unwrap();
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 3);
    }
}

#[test]
fn explicit_fit_still_refuses_an_outlier_in_the_3mf_source() {
    let error = convert_document(OUTLIER, InputFormat::ThreeMf, &options(1e-4))
        .err()
        .expect("outlier must not disappear in an accepted analytic reconstruction");
    assert_eq!(error.code, "recognition.unsupported");
}

#[test]
fn explicit_fit_still_enforces_the_independent_deviation_budget() {
    let opts = Options {
        max_deviation_mm: 0.001,
        ..options(1e-4)
    };
    let error = convert_document(ROUNDED, InputFormat::ThreeMf, &opts)
        .err()
        .expect("radial tolerance must not relax the chord/deviation budget");
    assert_eq!(error.code, "recognition.deviation_exceeded");
}

#[test]
fn a_perforated_plate_retains_its_two_valid_top_faces_in_planar_mode() {
    // The right half of the plate rises by 1.6e-6 mm. Its two planes can each
    // be built exactly, while their union exceeds native contour tolerance.
    let source = include_bytes!("../../../fixtures/precision_perforated_crease.stl");
    let options = Options {
        reconstruction: ReconstructionMode::Planar,
        ..Options::default()
    };
    let document = convert_document(source, InputFormat::Stl, &options).unwrap();
    let cad = &document.report.cad;
    assert_eq!((cad.solids, cad.faces, cad.planar_faces), (1, 11, 11));
    // (20*20 - 16*8) mm² at a millimetre thickness, plus the shallow wedge.
    let expected = 272. + (1000. - 256.) * 1.6e-7;
    assert!((cad.volume_mm3 - expected).abs() < 1e-6);
}
