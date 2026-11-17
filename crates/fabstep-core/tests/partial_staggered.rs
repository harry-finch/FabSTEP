use fabstep_core::{analyze_stl, recognize_mixed, Options, ReconstructionMode};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}

#[test]
fn staggered_angular_rings_have_complete_bounded_cylindrical_walls() {
    for (bytes, cylinders, volume, fit) in [
        (
            include_bytes!("../../../fixtures/partial_staggered/d_staggered_delayed.stl")
                .as_slice(),
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/partial_staggered/d_staggered_rotated.stl")
                .as_slice(),
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/partial_staggered/d_staggered.stl").as_slice(),
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
        ),
        (
            include_bytes!("../../../fixtures/partial_staggered/d_staggered_f32_translated.stl")
                .as_slice(),
            1,
            87.5 * std::f64::consts::PI,
            2e-5,
        ),
        (
            include_bytes!("../../../fixtures/partial_staggered/lens_shared_rails.stl").as_slice(),
            2,
            7. * (18. * (2_f64 / 3.).acos() - 4. * 5_f64.sqrt()),
            1e-5,
        ),
    ] {
        let o = Options {
            cylinder_fit_tolerance_mm: fit,
            ..options()
        };
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_mixed(&a, &o).unwrap();
        assert_eq!(r.model.partial_cylinders.len(), cylinders);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!((r.report.expected_volume_mm3 - volume).abs() < 0.001);
        assert!(r.report.max_chord_error_mm > 0.);
        assert!(r.report.max_radial_residual_mm <= fit);
    }
}

#[test]
fn defective_staggered_walls_are_not_partial_successes() {
    for bytes in [
        include_bytes!("../../../fixtures/partial_staggered/d_bulge.stl").as_slice(),
        include_bytes!("../../../fixtures/partial_staggered/d_fold.stl").as_slice(),
        include_bytes!("../../../fixtures/partial_staggered/d_dent.stl").as_slice(),
        include_bytes!("../../../fixtures/partial_staggered/d_excursion.stl").as_slice(),
        include_bytes!("../../../fixtures/partial_staggered/d_missing.stl").as_slice(),
    ] {
        let o = options();
        let error = analyze_stl(bytes, &o)
            .and_then(|a| recognize_mixed(&a, &o))
            .unwrap_err();
        assert!(
            matches!(
                error.code.as_str(),
                "recognition.partial_unproven"
                    | "mesh.open"
                    | "mesh.non_manifold"
                    | "mesh.inconsistent_orientation"
            ),
            "{}",
            error.code
        );
    }
}

#[test]
fn staggered_wall_chord_bound_uses_interior_ring_angles() {
    let o = Options {
        max_deviation_mm: 0.01,
        ..options()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/partial_staggered/d_staggered.stl"),
        &o,
    )
    .unwrap();
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn concave_staggered_wall_keeps_inner_orientation_and_volume() {
    use fabstep_core::{analyze_mesh, read_stl_mesh, InputUnit};
    let mut mesh = read_stl_mesh(
        include_bytes!("../../../fixtures/m2c_open_arc_subdivided.stl"),
        InputUnit::Mm,
    )
    .unwrap();
    let first = -(-7.5_f64 / 18.).acos();
    let last = (9_f64 / 18.).asin();
    for p in &mut mesh.vertices {
        let x = p[0] - 7.5;
        let y = p[1] - 51.;
        if p[2] > 0. && p[2] < 18. && (x.hypot(y) - 18.).abs() < 1e-9 {
            let angle = y.atan2(x);
            let fraction = (angle - first) / (last - first);
            if fraction > 1e-8 && fraction < 1. - 1e-8 {
                let shifted =
                    angle + 0.2 * (last - first) / 36. * (std::f64::consts::PI * fraction).sin();
                p[0] = 7.5 + 18. * shifted.cos();
                p[1] = 51. + 18. * shifted.sin();
            }
        }
    }
    let o = options();
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_mixed(&a, &o).unwrap();
    assert_eq!(r.model.partial_cylinders.len(), 1);
    assert!(r.model.partial_cylinders[0].inner);
    assert!((r.report.expected_volume_mm3 - 53857.34658341286).abs() < 1e-6);
}
