use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_rounded, Options, ReconstructionMode, RoundedFaceRef,
};
use std::{fs, path::Path};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}
fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/post_v1_zones")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

#[test]
fn two_real_rings_bound_convex_concave_rotated_and_irregular_zones_without_poles() {
    for name in [
        "cross_equator",
        "above_equator",
        "below_equator",
        "rotated",
        "irregular",
        "dense",
        "binary",
        "concave",
    ] {
        let o = options();
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.spheres, 1);
        assert_eq!(r.report.cones + r.report.cylinders + r.report.tori, 0);
        let s = &r.model.spheres[0];
        assert_eq!(s.inner, name == "concave");
        assert!((s.radius_mm - 10.).abs() < 1e-5);
        assert_eq!(r.model.revolved.mixed.rings.len(), 2);
        for i in s.rings.unwrap() {
            assert!(r.model.revolved.mixed.rings[i].radius_mm > 0.);
        }
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(r
            .triangle_faces
            .iter()
            .any(|f| matches!(f, RoundedFaceRef::Sphere { .. })));
        assert!(r.report.max_deviation_bound_mm < o.max_deviation_mm);
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        eprintln!(
            "{name}: bound={} volume={}",
            r.report.max_deviation_bound_mm, r.report.expected_volume_mm3
        );
    }
}

#[test]
fn former_cap_zone_refusal_is_now_a_proven_annulus() {
    let o = options();
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m5b_spherical_zone.stl"),
        &o,
    )
    .unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    assert_eq!(r.report.spheres, 1);
    assert_eq!(r.model.revolved.mixed.rings.len(), 2);
    assert!((r.report.expected_volume_mm3 - std::f64::consts::PI * (144. - 16. / 3.)).abs() < 1e-8);
}

#[test]
fn non_spherical_folded_sparse_and_insufficient_budget_zones_are_blocking() {
    let o = options();
    for (name, code) in [
        ("spheroid", "recognition.surface_unproven"),
        ("folded", "recognition.surface_unproven"),
        ("sparse", "recognition.deviation_exceeded"),
    ] {
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let e = recognize_rounded(&a, &o).unwrap_err();
        assert_eq!(e.code, code, "{name}: {e}");
    }
    assert_eq!(
        analyze_stl(&fixture("missing_triangle"), &o)
            .unwrap_err()
            .code,
        "mesh.open"
    );
    let a = analyze_stl(&fixture("cross_equator"), &o).unwrap();
    assert_eq!(
        recognize_rounded(
            &a,
            &Options {
                max_deviation_mm: 0.001,
                ..o
            }
        )
        .unwrap_err()
        .code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn rings_with_an_affine_wall_do_not_invent_spherical_curvature() {
    let o = options();
    let mut mesh = analyze_stl(&fixture("cross_equator"), &o).unwrap().mesh;
    for cylindrical in [false, true] {
        for p in &mut mesh.vertices {
            let rho = p[0].hypot(p[1]);
            if rho > 1. {
                let r = if cylindrical {
                    8.
                } else {
                    84_f64.sqrt() + (8. - 84_f64.sqrt()) * (p[2] + 4.) / 10.
                };
                p[0] *= r / rho;
                p[1] *= r / rho;
            }
        }
        let a = analyze_mesh(&mesh, &o).unwrap();
        assert_eq!(
            recognize_rounded(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn distance_to_triangle(p: [f64; 3], t: [[f64; 3]; 3]) -> f64 {
    let mut distance = f64::INFINITY;
    for i in 0..3 {
        let edge = sub(t[(i + 1) % 3], t[i]);
        let parameter = (dot(sub(p, t[i]), edge) / dot(edge, edge)).clamp(0., 1.);
        let d: [f64; 3] = std::array::from_fn(|j| p[j] - t[i][j] - parameter * edge[j]);
        distance = distance.min(dot(d, d));
    }
    let u = sub(t[1], t[0]);
    let v = sub(t[2], t[0]);
    let w = sub(p, t[0]);
    let determinant = dot(u, u) * dot(v, v) - dot(u, v).powi(2);
    if determinant > 0. {
        let b = (dot(w, u) * dot(v, v) - dot(w, v) * dot(u, v)) / determinant;
        let c = (dot(w, v) * dot(u, u) - dot(w, u) * dot(u, v)) / determinant;
        if b >= 0. && c >= 0. && b + c <= 1. {
            let d: [f64; 3] = std::array::from_fn(|j| w[j] - b * u[j] - c * v[j]);
            distance = distance.min(dot(d, d));
        }
    }
    distance.sqrt()
}

#[test]
fn composed_bound_covers_triangle_interiors_and_both_small_circle_fringes() {
    for (name, low, high) in [
        ("cross_equator", -4., 6.),
        ("above_equator", 2., 8.),
        ("below_equator", -8., -2.),
    ] {
        let o = options();
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap();
        let bound = r.report.max_deviation_bound_mm;
        let triangles: Vec<_> = a
            .mesh
            .triangles
            .iter()
            .zip(&r.triangle_faces)
            .filter(|(_, owner)| matches!(owner, RoundedFaceRef::Sphere { .. }))
            .map(|(t, _)| t.map(|v| a.mesh.vertices[v]))
            .collect();
        for t in &triangles {
            for weights in [[0.5, 0.5, 0.], [0.2, 0.3, 0.5], [1. / 3.; 3]] {
                let p: [f64; 3] =
                    std::array::from_fn(|i| (0..3).map(|j| weights[j] * t[j][i]).sum());
                let len = dot(p, p).sqrt();
                let mut q = p.map(|c| c * 10. / len);
                let limit = q[2].clamp(low, high);
                if q[2] != limit {
                    let rho = p[0].hypot(p[1]);
                    let radius = (100. - limit * limit).sqrt();
                    q = [radius * p[0] / rho, radius * p[1] / rho, limit];
                }
                assert!(dot(sub(p, q), sub(p, q)).sqrt() <= bound + 1e-12, "{name}");
            }
        }
        // Include exact circles and narrow strips at BOTH boundaries.
        for fraction in [
            0., 0.0001, 0.001, 0.01, 0.2, 0.5, 0.8, 0.99, 0.999, 0.9999, 1.,
        ] {
            let v = (low / 10.).asin() + fraction * ((high / 10.).asin() - (low / 10.).asin());
            for i in 0..19 {
                let u = (i as f64 + 0.37) * std::f64::consts::TAU / 19.;
                let p = [
                    10. * v.cos() * u.cos(),
                    10. * v.cos() * u.sin(),
                    10. * v.sin(),
                ];
                let distance = triangles
                    .iter()
                    .map(|&t| distance_to_triangle(p, t))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= bound + 1e-12,
                    "{name}: fraction {fraction}, {distance} > {bound}"
                );
            }
        }
    }
}
