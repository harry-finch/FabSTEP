use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_revolved, recognize_rounded, Options, ReconstructionMode,
};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}

#[test]
fn small_large_rotated_and_concave_caps_have_exact_volume_and_source_provenance() {
    for (name, bytes, volume, inner) in [
        (
            "small",
            include_bytes!("../../../fixtures/m5b_small_cap.stl").as_slice(),
            67.02064327658225,
            false,
        ),
        (
            "large",
            include_bytes!("../../../fixtures/m5b_large_cap.stl").as_slice(),
            837.7580409572781,
            false,
        ),
        (
            "rotated",
            include_bytes!("../../../fixtures/m5b_large_cap_rotated.stl").as_slice(),
            837.7580409572781,
            false,
        ),
        (
            "concave",
            include_bytes!("../../../fixtures/m5b_concave_cap.stl").as_slice(),
            3932.979356723418,
            true,
        ),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.spheres, 1);
        assert_eq!(r.model.spheres[0].inner, inner);
        assert!((r.model.spheres[0].radius_mm - 6.).abs() < 1e-8, "{name}");
        assert!(
            (r.report.expected_volume_mm3 - volume).abs() < volume * 1e-9,
            "{name}"
        );
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        eprintln!(
            "{name}: bound={}, volume={}",
            r.report.max_deviation_bound_mm, r.report.expected_volume_mm3
        );
    }
}

#[test]
fn cap_candidates_require_a_real_pole_curvature_and_dense_boundary() {
    for bytes in [
        include_bytes!("../../../fixtures/m5b_spheroid.stl").as_slice(),
        include_bytes!("../../../fixtures/m5b_shifted_pole.stl").as_slice(),
        include_bytes!("../../../fixtures/m5b_spherical_zone.stl").as_slice(),
        include_bytes!("../../../fixtures/m5b_sparse_cap.stl").as_slice(),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let e = recognize_rounded(&a, &o).unwrap_err();
        assert!(
            matches!(
                e.code.as_str(),
                "recognition.surface_unproven" | "recognition.unsupported"
            ),
            "{e}"
        );
    }
}

#[test]
fn a_shallow_conical_fan_stays_a_cone_instead_of_a_spherical_cap() {
    let o = options();
    let mut mesh = analyze_stl(include_bytes!("../../../fixtures/m4_cone_tip.stl"), &o)
        .unwrap()
        .mesh;
    // A small axial scale makes the same boundary/pole fit a very shallow
    // spherical cap too. Only interior curvature could justify that reading.
    for v in &mut mesh.vertices {
        v[2] *= 0.002;
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.unsupported"
    );
    let primitive = Options {
        reconstruction: ReconstructionMode::Primitives,
        ..o
    };
    let r = recognize_revolved(&a, &primitive).unwrap();
    assert_eq!(r.report.cones, 1);
    assert_eq!(r.report.spheres, 0);
}

#[test]
fn cap_fit_cannot_override_the_independent_deviation_budget() {
    let o = Options {
        cylinder_fit_tolerance_mm: 0.0001,
        max_deviation_mm: 0.001,
        ..options()
    };
    for bytes in [
        include_bytes!("../../../fixtures/m5b_small_cap.stl").as_slice(),
        include_bytes!("../../../fixtures/m5b_large_cap.stl").as_slice(),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_rounded(&a, &o).unwrap_err().code,
            "recognition.deviation_exceeded"
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
fn both_directions_including_small_circle_fringe_respect_the_composed_bound() {
    for (bytes, height) in [
        (
            include_bytes!("../../../fixtures/m5b_small_cap.stl").as_slice(),
            2.,
        ),
        (
            include_bytes!("../../../fixtures/m5b_large_cap.stl").as_slice(),
            10.,
        ),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap();
        let bound = r.report.max_deviation_bound_mm;
        let center = [0., 0., height - 6.];
        let boundary_z = -center[2];
        let rim_radius = 20_f64.sqrt();
        let triangles: Vec<_> = a
            .mesh
            .triangles
            .iter()
            .zip(&r.triangle_faces)
            .filter(|(_, owner)| matches!(owner, fabstep_core::RoundedFaceRef::Sphere { .. }))
            .map(|(triangle, _)| triangle.map(|v| a.mesh.vertices[v]))
            .collect();
        // Source interior points are compared with the trimmed analytic cap,
        // rather than merely their distance to the untrimmed sphere.
        for triangle in &triangles {
            for weights in [[0.5, 0.5, 0.], [0.2, 0.3, 0.5], [1. / 3.; 3]] {
                let point: [f64; 3] =
                    std::array::from_fn(|i| (0..3).map(|j| weights[j] * triangle[j][i]).sum());
                let q = sub(point, center);
                let radius = dot(q, q).sqrt();
                let radial: [f64; 3] = q.map(|v| 6. * v / radius);
                let nearest = if radial[2] >= boundary_z {
                    radial
                } else {
                    let rho = q[0].hypot(q[1]);
                    [rim_radius * q[0] / rho, rim_radius * q[1] / rho, boundary_z]
                };
                let delta = sub(q, nearest);
                assert!(dot(delta, delta).sqrt() <= bound + 1e-12);
            }
        }
        // Independent samples of the exact cap include its boundary and a
        // narrow strip next to it, where geodesic and small-circle trims differ.
        let low = (boundary_z / 6.).asin();
        for fraction in [0., 0.0001, 0.001, 0.01, 0.1, 0.3, 0.5, 0.7, 0.9, 1.] {
            let v = low + fraction * (std::f64::consts::FRAC_PI_2 - low);
            for i in 0..47 {
                let u = (i as f64 + 0.37) * std::f64::consts::TAU / 47.;
                let point = [
                    6. * v.cos() * u.cos(),
                    6. * v.cos() * u.sin(),
                    center[2] + 6. * v.sin(),
                ];
                let distance = triangles
                    .iter()
                    .map(|&t| distance_to_triangle(point, t))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= bound + 1e-12,
                    "height {height}, fraction {fraction}: {distance} > {bound}"
                );
            }
        }
    }
}
