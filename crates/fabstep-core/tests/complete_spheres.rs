use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_complete_sphere, recognize_rounded, Options,
    ReconstructionMode, RoundedFaceRef,
};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}

#[test]
fn complete_spheres_preserve_sources_without_fictitious_poles_or_boundaries() {
    for (name, bytes, center) in [
        (
            "icosphere",
            include_bytes!("../../../fixtures/m5c_icosphere.stl").as_slice(),
            [0., 0., 0.],
        ),
        (
            "transformed",
            include_bytes!("../../../fixtures/m5c_icosphere_transformed.stl").as_slice(),
            [13., -7., 11.],
        ),
        (
            "uv",
            include_bytes!("../../../fixtures/m5c_uv_sphere.stl").as_slice(),
            [0., 0., 0.],
        ),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_complete_sphere(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.spheres, 1);
        let s = &r.model.spheres[0];
        assert!(s.rings.is_none() && !s.inner);
        assert!((s.radius_mm - 6.).abs() < 1e-9, "{name}");
        for (actual, nominal) in s.center.iter().zip(center) {
            assert!((actual - nominal).abs() < 1e-9, "{name}");
        }
        assert!(r.model.revolved.mixed.rings.is_empty());
        assert!(r.model.revolved.mixed.planes.is_empty());
        assert!(r.model.revolved.cones.is_empty() && r.model.tori.is_empty());
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(r
            .triangle_faces
            .iter()
            .all(|f| matches!(f, RoundedFaceRef::Sphere { index: 0 })));
        assert!((r.report.expected_volume_mm3 - 904.7786842338604).abs() < 1e-7);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        eprintln!(
            "{name}: bound {}, residual {}",
            r.report.max_deviation_bound_mm, r.report.max_radial_residual_mm
        );
    }
}

#[test]
fn flattened_closures_and_non_spherical_or_sparse_sources_are_not_completed() {
    let o = options();
    for bytes in [
        include_bytes!("../../../fixtures/m5c_truncated_cospherical.stl").as_slice(),
        include_bytes!("../../../fixtures/m5c_sparse_sphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5c_spheroid.stl").as_slice(),
        include_bytes!("../../../fixtures/m5c_sphere_dent.stl").as_slice(),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_complete_sphere(&a, &o).unwrap_err().code,
            "recognition.surface_unproven"
        );
    }
    assert_eq!(
        analyze_stl(include_bytes!("../../../fixtures/m5c_open_sphere.stl"), &o)
            .unwrap_err()
            .code,
        "mesh.open"
    );
    let cap = analyze_stl(
        include_bytes!("../../../fixtures/m5c_truncated_cospherical.stl"),
        &o,
    )
    .unwrap();
    assert!(recognize_rounded(&cap, &o).unwrap().model.spheres[0]
        .rings
        .is_some());
}

#[test]
fn full_sphere_fit_is_stable_under_scale_translation_and_repeated_execution() {
    let o = options();
    let source = analyze_stl(include_bytes!("../../../fixtures/m5c_icosphere.stl"), &o).unwrap();
    for (factor, center) in [
        (0.01, [1., 2., 3.]),
        (1.5, [-30., 50., 70.]),
        (1., [1e8, -1e8, 1e8]),
    ] {
        let mut mesh = source.mesh.clone();
        for p in &mut mesh.vertices {
            for i in 0..3 {
                p[i] = factor * p[i] + center[i];
            }
        }
        let a = analyze_mesh(&mesh, &o).unwrap();
        let first = recognize_complete_sphere(&a, &o).unwrap();
        let second = recognize_complete_sphere(&a, &o).unwrap();
        let sphere = &first.model.spheres[0];
        assert_eq!(sphere.center, second.model.spheres[0].center);
        assert_eq!(sphere.radius_mm, second.model.spheres[0].radius_mm);
        assert_eq!(
            first.report.max_deviation_bound_mm,
            second.report.max_deviation_bound_mm
        );
        for (actual, expected) in sphere.center.iter().zip(center) {
            assert!((actual - expected).abs() <= 2e-8);
        }
        assert!((sphere.radius_mm - 6. * factor).abs() <= 2e-8);
        assert!(first.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
    }
    // At this translation the floating-point resolution guard must refuse,
    // even if a normalized fit appears well conditioned.
    let mut huge = source.mesh.clone();
    for p in &mut huge.vertices {
        p[0] += 1e10;
    }
    let a = analyze_mesh(&huge, &o).unwrap();
    assert_eq!(
        recognize_complete_sphere(&a, &o).unwrap_err().code,
        "recognition.surface_unproven"
    );
}

#[test]
fn full_sphere_requires_explicit_mode_and_an_independent_distance_budget() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/m5c_icosphere.stl"), &o).unwrap();
    assert_eq!(
        recognize_complete_sphere(&a, &Options::default())
            .unwrap_err()
            .code,
        "recognition.disabled"
    );
    let tight = Options {
        max_deviation_mm: 0.001,
        cylinder_fit_tolerance_mm: 0.0001,
        ..o
    };
    assert_eq!(
        recognize_complete_sphere(&a, &tight).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
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
fn sphere_interior_and_global_surface_samples_obey_the_composed_bound() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/m5c_icosphere.stl"), &o).unwrap();
    let r = recognize_complete_sphere(&a, &o).unwrap();
    let bound = r.report.max_deviation_bound_mm;
    let triangles: Vec<_> = a
        .mesh
        .triangles
        .iter()
        .map(|t| t.map(|v| a.mesh.vertices[v]))
        .collect();
    for triangle in &triangles {
        for weights in [[0.5, 0.5, 0.], [0.2, 0.3, 0.5], [1. / 3.; 3]] {
            let p: [f64; 3] =
                std::array::from_fn(|i| (0..3).map(|j| weights[j] * triangle[j][i]).sum());
            assert!((dot(p, p).sqrt() - 6.).abs() <= bound + 1e-12);
        }
    }
    // The analytic-side samples use a Fibonacci distribution independent of
    // the generating triangulation, and include neither its edges nor poles.
    let golden_angle = std::f64::consts::PI * (3. - 5_f64.sqrt());
    for i in 0..401 {
        let z = 1. - 2. * (i as f64 + 0.5) / 401.;
        let rho = (1. - z * z).sqrt();
        let angle = i as f64 * golden_angle;
        let p = [6. * rho * angle.cos(), 6. * rho * angle.sin(), 6. * z];
        let distance = triangles
            .iter()
            .map(|&t| distance_to_triangle(p, t))
            .fold(f64::INFINITY, f64::min);
        assert!(distance <= bound + 1e-12, "{distance} > {bound}");
    }
}
