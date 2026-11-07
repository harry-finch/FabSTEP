use fabstep_core::{
    analyze_mesh, recognize_complete_torus, Mesh, Options, ReconstructionMode, RoundedFaceRef,
};
use std::f64::consts::{PI, TAU};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}

// Independent parametric sampler; neither implicit fitting nor the production
// UV proof is used to generate these sources. Irregular grids deliberately
// have no shared planar latitude loops or imposed axial source landmarks.
fn torus(major: f64, minor: f64, nu: usize, nv: usize, irregular: bool, winding: usize) -> Mesh {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for i in 0..nu {
        for j in 0..nv {
            let base_u = TAU * i as f64 * winding as f64 / nu as f64;
            let base_v = TAU * j as f64 / nv as f64;
            let u = base_u
                + if irregular {
                    0.07 * base_v.sin() + 0.03 * (3. * base_u).sin()
                } else {
                    0.
                };
            let v = base_v
                + if irregular {
                    0.06 * (2. * base_u).sin() + 0.02 * (3. * base_v).sin()
                } else {
                    0.
                };
            // A double-winding negative has two nearby but distinct sheets.
            // All vertices still fit one torus, so coverage must reject it.
            let offset = if winding == 2 { 3e-6 } else { 0. };
            let rho = major + minor * v.cos() + offset * (base_u / 2.).cos();
            vertices.push([
                rho * u.cos(),
                rho * u.sin(),
                minor * v.sin() + offset * (base_u / 2.).sin(),
            ]);
        }
    }
    for i in 0..nu {
        for j in 0..nv {
            let a = i * nv + j;
            let b = ((i + 1) % nu) * nv + j;
            let c = ((i + 1) % nu) * nv + (j + 1) % nv;
            let d = i * nv + (j + 1) % nv;
            triangles.push([a, b, c]);
            triangles.push([a, c, d]);
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

fn transform(
    mesh: &mut Mesh,
    center: [f64; 3],
    factor: f64,
    mirror: bool,
    quantize: bool,
) -> [f64; 3] {
    let axis = [1. / 14_f64.sqrt(), 2. / 14_f64.sqrt(), 3. / 14_f64.sqrt()];
    let c = 0.63_f64.cos();
    let s = 0.63_f64.sin();
    let rotate = |p: [f64; 3]| {
        let d: f64 = (0..3).map(|i| axis[i] * p[i]).sum();
        let cross = [
            axis[1] * p[2] - axis[2] * p[1],
            axis[2] * p[0] - axis[0] * p[2],
            axis[0] * p[1] - axis[1] * p[0],
        ];
        std::array::from_fn::<_, 3, _>(|i| c * p[i] + s * cross[i] + (1. - c) * d * axis[i])
    };
    for p in &mut mesh.vertices {
        let mut q = rotate(*p);
        if mirror {
            q[0] = -q[0];
        }
        *p = std::array::from_fn(|i| {
            let v = center[i] + factor * q[i];
            if quantize {
                (v as f32) as f64
            } else {
                v
            }
        });
    }
    let mut expected = rotate([0., 0., 1.]);
    if mirror {
        expected[0] = -expected[0];
    }
    expected
}

#[test]
fn complete_tori_fit_arbitrary_axes_samples_scales_and_f32_without_source_rings() {
    for (name, irregular, scale, mirror, f32) in [
        ("regular", false, 1., false, false),
        ("irregular", true, 1., false, false),
        ("mirror", true, 1., true, false),
        ("float32", true, 1., false, true),
        ("small", true, 0.01, false, false),
    ] {
        let o = options();
        let mut mesh = torus(8., 2., 64, 32, irregular, 1);
        let center = [13., -7., 11.];
        let expected_axis = transform(&mut mesh, center, scale, mirror, f32);
        let a = analyze_mesh(&mesh, &o).unwrap();
        let r = recognize_complete_torus(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.tori, 1);
        assert_eq!(r.report.toroidal_fillets, 0);
        assert_eq!(r.report.spheres, 0);
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(r
            .triangle_faces
            .iter()
            .all(|f| matches!(f, RoundedFaceRef::Torus { index: 0 })));
        assert!(
            r.model.revolved.mixed.rings.is_empty()
                && r.model.revolved.mixed.planes.is_empty()
                && r.model.spheres.is_empty()
        );
        let t = &r.model.tori[0];
        assert!(t.rings.is_none() && !t.inner);
        assert_eq!(t.meridian_rad, [0., TAU]);
        let axis = t.axis.unwrap();
        let allowance = if f32 { 4e-6 } else { 1e-8 };
        assert!((t.major_radius_mm - 8. * scale).abs() < allowance);
        assert!((t.minor_radius_mm - 2. * scale).abs() < allowance);
        for (actual, expected) in t.center.iter().zip(center) {
            assert!((actual - expected).abs() < allowance);
        }
        let alignment: f64 = axis.iter().zip(expected_axis).map(|(a, b)| a * b).sum();
        assert!(alignment.abs() > 1. - 1e-10);
        let nominal = 2. * PI * PI * 8. * 4. * scale.powi(3);
        assert!((r.report.expected_volume_mm3 - nominal).abs() < nominal * 1e-5);
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        eprintln!(
            "{name}: bound {} residual {}",
            r.report.max_deviation_bound_mm, r.report.max_radial_residual_mm
        );
    }
}

#[test]
fn elliptical_tubes_dents_folds_and_multiple_winding_cannot_pass_the_vertex_fit_alone() {
    let o = options();
    for name in [
        "elliptical_tube",
        "elliptical_ring",
        "dent",
        "fold",
        "double_winding",
        "spindle",
    ] {
        let mut mesh = torus(
            if name == "spindle" { 1. } else { 8. },
            2.,
            if name == "double_winding" { 128 } else { 64 },
            32,
            false,
            if name == "double_winding" { 2 } else { 1 },
        );
        match name {
            "elliptical_tube" => {
                for p in &mut mesh.vertices {
                    p[2] *= 1.04;
                }
            }
            "elliptical_ring" => {
                for p in &mut mesh.vertices {
                    p[0] *= 1.03;
                }
            }
            "dent" => mesh.vertices[11][2] += 0.02,
            "fold" => mesh.vertices.swap(11, 12),
            _ => {}
        }
        if name == "double_winding" || name == "fold" {
            // These are genuine counterexamples to vertex-only fitting:
            // every source vertex lies within fit of the nominal torus.
            for point in &mesh.vertices {
                let distance = ((point[0].hypot(point[1]) - 8.).hypot(point[2]) - 2.).abs();
                assert!(distance <= o.cylinder_fit_tolerance_mm);
            }
        }
        let a = analyze_mesh(&mesh, &o).unwrap_or_else(|e| panic!("{name} source: {e}"));
        let error = recognize_complete_torus(&a, &o).unwrap_err();
        if name == "double_winding" {
            assert!(error.message.contains("couverture UV"), "{error}");
        }
        if name == "fold" {
            assert!(error.message.contains("projection UV"), "{error}");
        }
        assert_eq!(
            error.code, "recognition.surface_unproven",
            "{name}: {error}"
        );
    }
}

#[test]
fn full_torus_is_explicit_and_topology_sampling_and_distance_budget_are_independent() {
    let o = options();
    let mesh = torus(8., 2., 64, 32, false, 1);
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert_eq!(
        recognize_complete_torus(&a, &Options::default())
            .unwrap_err()
            .code,
        "recognition.disabled"
    );
    let tight = Options {
        max_deviation_mm: 0.001,
        cylinder_fit_tolerance_mm: 0.0001,
        ..o.clone()
    };
    assert_eq!(
        recognize_complete_torus(&a, &tight).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    let sparse = analyze_mesh(&torus(8., 2., 8, 4, false, 1), &o).unwrap();
    assert_eq!(
        recognize_complete_torus(&sparse, &o).unwrap_err().code,
        "recognition.surface_unproven"
    );
    let mut open = mesh.clone();
    open.triangles.pop();
    assert_eq!(analyze_mesh(&open, &o).unwrap_err().code, "mesh.open");
    let sphere =
        fabstep_core::analyze_stl(include_bytes!("../../../fixtures/m5c_icosphere.stl"), &o)
            .unwrap();
    assert_eq!(
        recognize_complete_torus(&sphere, &o).unwrap_err().code,
        "recognition.unsupported"
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
fn thin_f32_tube_uses_physical_refinement_without_relaxing_the_fit() {
    let o = options();
    let mut mesh = torus(100., 1., 128, 32, false, 1);
    for p in &mut mesh.vertices {
        for x in p {
            *x = (*x as f32) as f64;
        }
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    let result = recognize_complete_torus(&a, &o).unwrap();
    let t = &result.model.tori[0];
    assert!((t.major_radius_mm - 100.).abs() < 1e-5);
    assert!((t.minor_radius_mm - 1.).abs() < 1e-5);
    assert!(result.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
    assert!(result.report.max_deviation_bound_mm <= o.max_deviation_mm);
}

#[test]
fn complete_torus_both_distance_directions_obey_the_reported_bound() {
    let o = options();
    let a = analyze_mesh(&torus(8., 2., 64, 32, true, 1), &o).unwrap();
    let result = recognize_complete_torus(&a, &o).unwrap();
    let bound = result.report.max_deviation_bound_mm;
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
            let distance = ((p[0].hypot(p[1]) - 8.).hypot(p[2]) - 2.).abs();
            assert!(distance <= bound + 1e-12);
        }
    }
    for i in 0..251 {
        let u = TAU * (i as f64 * 0.618033988749895).fract();
        let v = TAU * (i as f64 * 0.4142135623730951).fract();
        let p = [
            (8. + 2. * v.cos()) * u.cos(),
            (8. + 2. * v.cos()) * u.sin(),
            2. * v.sin(),
        ];
        let distance = triangles
            .iter()
            .map(|&t| distance_to_triangle(p, t))
            .fold(f64::INFINITY, f64::min);
        assert!(distance <= bound + 1e-12, "{distance} > {bound}");
    }
}

#[test]
fn geometric_refinement_removes_algebraic_f32_volume_bias_without_relaxing_fit() {
    // Independent Python-generated binary STL: both algebraic fits already
    // met the default tolerance, yet biased the nominal volume by >1e-4 mm³.
    // These checks specifically require geometric refinement below that fit
    // threshold; a strict residual/coverage proof remains mandatory afterward.
    for bytes in [
        include_bytes!("../../../fixtures/m5d_ring_torus.stl").as_slice(),
        include_bytes!("../../../fixtures/m5d_torus_transformed.stl").as_slice(),
    ] {
        let o = options();
        let a = fabstep_core::analyze_stl(bytes, &o).unwrap();
        let r = recognize_complete_torus(&a, &o).unwrap();
        let nominal = 2. * PI * PI * 6. * 4.;
        assert!(
            (r.report.expected_volume_mm3 - nominal).abs() < nominal * 1e-8,
            "{} versus {nominal}",
            r.report.expected_volume_mm3
        );
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    }
}
