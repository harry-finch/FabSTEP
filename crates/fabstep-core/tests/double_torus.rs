use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_rounded, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{f64::consts::PI, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/double_torus")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}
fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}
fn nominal_volume(lower: f64) -> f64 {
    let quarter = |q: f64| {
        let major = 8. - q;
        PI * (major * major * q + PI * major * q * q / 2. + 2. * q.powi(3) / 3.)
    };
    PI * 8_f64.powi(2) * 5. + quarter(lower) + quarter(2.)
}

#[test]
fn two_source_quarter_arcs_share_one_resolved_cylindrical_land() {
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        for name in [
            "nominal",
            "equal_radii",
            "rotated",
            "irregular",
            "seam_noise",
            "dense",
            "binary",
        ] {
            let o = options(mode);
            let a = analyze_stl(&fixture(name), &o).unwrap();
            let r = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            assert_eq!(r.report.tori, 2, "{name}");
            assert_eq!(r.report.toroidal_fillets, 2);
            assert_eq!(r.report.cylinders, 1);
            assert_eq!(r.report.cones + r.report.spheres, 0);
            assert_eq!(r.model.revolved.mixed.planes.len(), 2);
            assert_eq!(r.model.revolved.mixed.rings.len(), 4);
            let lower = if name == "equal_radii" { 2. } else { 1.5 };
            let nominal = nominal_volume(lower);
            let tolerance = if name == "binary" { 2e-6 } else { 1e-8 };
            assert!(
                (r.report.expected_volume_mm3 - nominal).abs() <= nominal * tolerance,
                "{name}: {} != {nominal}",
                r.report.expected_volume_mm3
            );
            assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
            if name == "seam_noise" {
                assert!(r.report.max_radial_residual_mm >= 0.0000029);
            }
            let mut radii: Vec<_> = r.model.tori.iter().map(|t| t.minor_radius_mm).collect();
            radii.sort_by(f64::total_cmp);
            assert!((radii[0] - lower).abs() < 2e-6, "{name}: {radii:?}");
            assert!((radii[1] - 2.).abs() < 2e-6);
            let cylinder = &r.model.revolved.mixed.cylinders[0];
            let common = &r.model.revolved.mixed.rings;
            for torus in &r.model.tori {
                assert!(torus.axis.is_none());
                assert!((torus.meridian_rad[1] - torus.meridian_rad[0] - PI / 2.).abs() < 1e-12);
                let rings = torus.rings.unwrap();
                let shared: Vec<_> = rings
                    .into_iter()
                    .filter(|id| cylinder.rings.contains(id))
                    .collect();
                assert_eq!(shared.len(), 1);
                assert!((common[shared[0]].radius_mm - 8.).abs() < 2e-6);
                assert!(
                    (torus.major_radius_mm + torus.minor_radius_mm - common[shared[0]].radius_mm)
                        .abs()
                        < 1e-10
                );
            }
            for index in 0..2 {
                assert!(r.triangle_faces.contains(&RoundedFaceRef::Torus { index }));
            }
        }
    }
}

#[test]
fn ambiguous_or_non_circular_profiles_are_not_promoted_to_double_fillets() {
    let o = options(ReconstructionMode::Surfaces);
    for name in [
        "no_witness",
        "one_witness",
        "short_land",
        "no_land",
        "warped_arc",
        "tapered_land",
        "sparse_meridian",
        "sparse_azimuth",
        "concave",
    ] {
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let e = recognize_rounded(&a, &o).unwrap_err();
        assert_eq!(
            e.code,
            if name == "sparse_azimuth" {
                "recognition.unsupported"
            } else {
                "recognition.surface_unproven"
            },
            "{name}: {e}"
        );
    }
    for name in ["no_witness", "warped_arc", "tapered_land"] {
        let o = options(ReconstructionMode::Auto);
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let r = recognize_automatic(&a, &o).unwrap();
        assert_eq!(r.report.tori, 0, "{name}");
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    }
}

#[test]
fn double_fillet_interiors_and_distance_budget_remain_independent_of_fit() {
    let o = options(ReconstructionMode::Surfaces);
    let a = analyze_stl(&fixture("nominal"), &o).unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    for (triangle, owner) in a.mesh.triangles.iter().zip(&r.triangle_faces) {
        let RoundedFaceRef::Torus { index } = owner else {
            continue;
        };
        let torus = &r.model.tori[*index];
        let axis = r.model.revolved.mixed.rings[torus.rings.unwrap()[0]].axis;
        let points = triangle.map(|id| a.mesh.vertices[id]);
        for weights in [[0.2, 0.3, 0.5], [1. / 3.; 3], [0.5, 0.5, 0.]] {
            let d: [f64; 3] = std::array::from_fn(|i| {
                (0..3).map(|j| weights[j] * points[j][i]).sum::<f64>() - torus.center[i]
            });
            let z: f64 = (0..3).map(|i| d[i] * axis[i]).sum();
            let radial: [f64; 3] = std::array::from_fn(|i| d[i] - z * axis[i]);
            let rho = radial[0].hypot(radial[1]).hypot(radial[2]);
            let distance = ((rho - torus.major_radius_mm).hypot(z) - torus.minor_radius_mm).abs();
            assert!(distance <= r.report.max_deviation_bound_mm + 1e-12);
        }
    }
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    assert_eq!(
        recognize_rounded(&a, &strict).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn historical_single_torus_keeps_its_endpoint_only_cylinder_contract() {
    let o = options(ReconstructionMode::Surfaces);
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m5_toroidal_fillet.stl"),
        &o,
    )
    .unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    assert_eq!(r.report.tori, 1);
    assert_eq!(r.report.cylinders, 1);
    assert!((r.report.expected_volume_mm3 - 1567.7566636691636).abs() < 1e-8);
}
