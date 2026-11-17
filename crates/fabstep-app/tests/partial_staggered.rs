use fabstep_app::convert_stl;
use fabstep_core::{MixedFaceRef, Options, ReconstructionMode};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/partial_staggered")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn rotated_half_disk_bounds() -> [[f64; 3]; 2] {
    let (sx, cx) = 0.37_f64.sin_cos();
    let (sy, cy) = (-0.51_f64).sin_cos();
    let rows = [
        [cy, sy * sx, sy * cx],
        [0., cx, -sx],
        [-sy, cy * sx, cy * cx],
    ];
    let mut bounds = [[0.; 3]; 2];
    for (axis, [a, b, c]) in rows.into_iter().enumerate() {
        // Support of the upper half disk: an interior circular extremum is
        // admissible only on its upper half; otherwise the diameter ends win.
        let low = -5. * if b <= 0. { a.hypot(b) } else { a.abs() };
        let high = 5. * if b >= 0. { a.hypot(b) } else { a.abs() };
        bounds[0][axis] = [4., 8., 12.][axis] + low + (7. * c).min(0.);
        bounds[1][axis] = [4., 8., 12.][axis] + high + (7. * c).max(0.);
    }
    bounds
}

#[test]
fn staggered_and_adjacent_cylinders_reimport_with_nominal_geometry_and_fidelity() {
    // Nominal dimensions are independent of recognition: a half disk R5 and
    // the intersection of two disks R3 whose centers are four millimeters apart.
    let cases = [
        (
            "d_staggered",
            3,
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
            [[-5., 0., 0.], [5., 5., 7.]],
        ),
        (
            "d_staggered_delayed",
            3,
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
            [[-5., 0., 0.], [5., 5., 7.]],
        ),
        (
            "d_staggered_rotated",
            3,
            1,
            87.5 * std::f64::consts::PI,
            1e-5,
            rotated_half_disk_bounds(),
        ),
        (
            "d_staggered_f32_translated",
            3,
            1,
            87.5 * std::f64::consts::PI,
            2e-5,
            [[68., -133., 0.], [78., -128., 7.]],
        ),
        (
            "lens_shared_rails",
            2,
            2,
            7. * (18. * (2_f64 / 3.).acos() - 4. * 5_f64.sqrt()),
            1e-5,
            [[-1., -5_f64.sqrt(), 0.], [1., 5_f64.sqrt(), 7.]],
        ),
    ];
    for (name, planes, cylinders, volume, fit, bounds) in cases {
        for mode in [
            ReconstructionMode::Cylinders,
            ReconstructionMode::Primitives,
            ReconstructionMode::Surfaces,
        ] {
            let options = Options {
                reconstruction: mode,
                cylinder_fit_tolerance_mm: fit,
                ..Options::default()
            };
            let result = convert_stl(&fixture(name), &options)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let r = &result.report;
            assert_eq!(r.cad.solids, 1);
            assert_eq!(r.cad.planar_faces, planes);
            assert_eq!(r.cad.cylindrical_faces, cylinders);
            assert_eq!(r.cad.faces, planes + cylinders);
            let error = if name.contains("f32") { 0.001 } else { 1e-7 };
            assert!(
                (r.cad.volume_mm3 - volume).abs() < error,
                "{name}: volume {}",
                r.cad.volume_mm3
            );
            for (actual, expected) in r
                .cad
                .bounds_mm
                .iter()
                .flatten()
                .zip(bounds.iter().flatten())
            {
                assert!(
                    (actual - expected).abs() < 2e-5,
                    "{name}: bounds {actual} != {expected}"
                );
            }
            let mixed = r
                .mixed_recognition
                .as_ref()
                .expect("partial cylinder proof");
            assert_eq!(mixed.model.partial_cylinders.len(), cylinders);
            assert_eq!(mixed.triangle_faces.len(), r.analysis.mesh.triangles.len());
            assert_eq!(mixed.model.vertices, r.analysis.mesh.vertices);
            for index in 0..cylinders {
                assert!(mixed.triangle_faces.iter().any(|owner| matches!(owner, MixedFaceRef::PartialCylinder { index: actual } if *actual == index)));
            }
            let fidelity = r.fidelity.as_ref().expect("native bidirectional audit");
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(fidelity.source_to_result_mm < options.max_deviation_mm);
            assert!(fidelity.result_to_source_mm < options.max_deviation_mm);
            assert_eq!(r.settings.cylinder_fit_tolerance_mm, fit);
            assert_eq!(
                r.analysis
                    .report
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "precision.custom_tolerances"),
                fit != Options::default().cylinder_fit_tolerance_mm
            );
            assert_eq!(
                std::str::from_utf8(&result.step_bytes)
                    .unwrap()
                    .matches("CYLINDRICAL_SURFACE(")
                    .count(),
                cylinders
            );
        }
    }
}

#[test]
fn defective_staggered_walls_and_insufficient_budget_never_export() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    for name in ["d_bulge", "d_dent", "d_excursion", "d_fold", "d_missing"] {
        let error = convert_stl(&fixture(name), &options)
            .err()
            .expect("invalid wall must not export");
        assert!(
            error.code == "recognition.partial_unproven" || error.code.starts_with("mesh."),
            "{name}: {error}"
        );
    }
    let error = convert_stl(
        &fixture("d_staggered"),
        &Options {
            max_deviation_mm: 0.01,
            ..options
        },
    )
    .err()
    .expect("insufficient deviation budget");
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
