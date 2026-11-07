use fabstep_core::{analyze_mesh, recognize_rounded_box, Mesh, Options, ReconstructionMode};
use std::f64::consts::PI;
type V = [f64; 3];
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}
fn sign(s: bool) -> f64 {
    if s {
        1.
    } else {
        -1.
    }
}
fn others(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn triangle(m: &mut Mesh, p: [V; 3], outward: V) {
    let start = m.vertices.len();
    m.vertices.extend(p);
    let mut t = [start, start + 1, start + 2];
    if dot(cross(sub(p[1], p[0]), sub(p[2], p[0])), outward) < 0. {
        t.swap(1, 2);
    }
    m.triangles.push(t);
}
fn quad(m: &mut Mesh, p: [V; 4], outward: V) {
    triangle(m, [p[0], p[1], p[2]], outward);
    triangle(m, [p[0], p[2], p[3]], outward);
}
fn cs(i: usize, n: usize) -> (f64, f64) {
    if i == 0 {
        (1., 0.)
    } else if i == n {
        (0., 1.)
    } else {
        let u = PI * 0.5 * i as f64 / n as f64;
        (u.cos(), u.sin())
    }
}
// Independent surface sampler: rectangles, quarter-cylinder strips and
// octants constructed directly from design dimensions, with no production
// fitting or projection used to generate points. Separate patch vertices
// deliberately exercise actual source welding at every analytic seam.
fn rounded_box(core: V, radius: f64, n: usize) -> Mesh {
    let mut m = Mesh {
        vertices: vec![],
        triangles: vec![],
    };
    for axis in 0..3 {
        for positive in [false, true] {
            let side = sign(positive);
            let trans = others(axis);
            let p = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]].map(|s| {
                let mut p = [0.; 3];
                p[axis] = side * (core[axis] + radius);
                for k in 0..2 {
                    p[trans[k]] = s[k] * core[trans[k]];
                }
                p
            });
            let mut normal = [0.; 3];
            normal[axis] = side;
            quad(&mut m, p, normal);
        }
    }
    for axis in 0..3 {
        for bits in 0..4 {
            let t = others(axis);
            let signs = [sign(bits & 1 != 0), sign(bits & 2 != 0)];
            let point = |i: usize, z: f64| {
                let (c, s) = cs(i, n);
                let mut p = [0.; 3];
                p[axis] = z;
                p[t[0]] = signs[0] * (core[t[0]] + radius * c);
                p[t[1]] = signs[1] * (core[t[1]] + radius * s);
                p
            };
            for i in 0..n {
                let p = [
                    point(i, -core[axis]),
                    point(i + 1, -core[axis]),
                    point(i + 1, core[axis]),
                    point(i, core[axis]),
                ];
                let mut outward = [0.; 3];
                outward[t[0]] = signs[0];
                outward[t[1]] = signs[1];
                quad(&mut m, p, outward);
            }
        }
    }
    for bits in 0..8 {
        let signs = [
            sign(bits & 1 != 0),
            sign(bits & 2 != 0),
            sign(bits & 4 != 0),
        ];
        let point = |i: usize, j: usize| {
            let (cu, su) = cs(i, n);
            let (cv, sv) = cs(j, n);
            let unit = [cv * cu, cv * su, sv];
            std::array::from_fn(|k| signs[k] * (core[k] + radius * unit[k]))
        };
        for j in 0..n {
            for i in 0..n {
                if j + 1 == n {
                    triangle(&mut m, [point(i, j), point(i + 1, j), point(i, n)], signs);
                } else {
                    quad(
                        &mut m,
                        [
                            point(i, j),
                            point(i + 1, j),
                            point(i + 1, j + 1),
                            point(i, j + 1),
                        ],
                        signs,
                    );
                }
            }
        }
    }
    m
}
fn transform(m: &mut Mesh, factor: f64, mirror: bool, f32: bool) {
    // Exact orthonormal rational rotation, independent of the recognizer.
    for p in &mut m.vertices {
        let [x, y, z] = *p;
        let q = [0.6 * x - 0.8 * z, y, 0.8 * x + 0.6 * z];
        *p = std::array::from_fn(|i| {
            let v = [13., -7., 11.][i] + factor * q[i] * if mirror && i == 0 { -1. } else { 1. };
            if f32 {
                (v as f32) as f64
            } else {
                v
            }
        });
    }
}
#[test]
fn uniform_rounded_boxes_keep_all_twenty_six_domains_and_nominal_volume() {
    for (scale, mirror, f32) in [
        (1., false, false),
        (1., true, false),
        (1., false, true),
        (0.1, false, false),
    ] {
        let mut m = rounded_box([12., 8., 5.], 2., 12);
        transform(&mut m, scale, mirror, f32);
        let o = options();
        let a = analyze_mesh(&m, &o).unwrap();
        let r = recognize_rounded_box(&a, &o)
            .unwrap_or_else(|e| panic!("scale{scale} mirror{mirror} f32{f32}: {e}"));
        assert_eq!(r.model.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert_eq!(r.report.cylinders, 12);
        assert_eq!(r.report.spheres, 8);
        assert!((r.model.radius_mm - 2. * scale).abs() < 3e-6);
        let nominal = 7637.82885235625 * scale.powi(3);
        assert!((r.report.expected_volume_mm3 - nominal).abs() < nominal * 1e-6);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        let mut keys = r
            .triangle_faces
            .iter()
            .map(|f| format!("{f:?}"))
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 26);
    }
}
#[test]
fn local_dents_variable_radii_partial_rounding_and_sparse_octants_are_refused() {
    for name in ["dent", "variable", "partial", "fold", "sparse"] {
        let mut m = rounded_box([12., 8., 5.], 2., if name == "sparse" { 2 } else { 12 });
        for p in &mut m.vertices {
            match name {
                "dent" if p[0] > 12.2 && p[1] > 8.2 && p[2] > 5.2 => p[0] -= 0.025,
                "variable" if p[0] > 12. && p[1] > 8. && p[2] > 5. => {
                    for (coordinate, center) in p.iter_mut().zip([12., 8., 5.]) {
                        *coordinate = center + (*coordinate - center) * 1.015;
                    }
                }
                "partial" if p[0] > 12. && p[1] > 8. => p[0] = 14.,
                // Reverse the radial direction of a small patch while keeping
                // its points on the same corner sphere: fit alone cannot help.
                "fold" if p[0] > 12.2 && p[1] > 8.2 && p[2] > 5.2 => {
                    let x = p[0] - 12.;
                    p[0] = 12. + p[1] - 8.;
                    p[1] = 8. + x;
                }
                _ => {}
            }
        }
        let o = options();
        match analyze_mesh(&m, &o) {
            Ok(a) => assert!(recognize_rounded_box(&a, &o).is_err(), "{name}"),
            Err(e) => assert!(e.code.starts_with("mesh."), "{name}: {e}"),
        }
    }
}
#[test]
fn explicit_mode_closed_source_resolved_radius_and_independent_bound_remain_required() {
    let m = rounded_box([12., 8., 5.], 2., 12);
    let o = options();
    let a = analyze_mesh(&m, &o).unwrap();
    assert_eq!(
        recognize_rounded_box(&a, &Options::default())
            .unwrap_err()
            .code,
        "recognition.disabled"
    );
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o.clone()
    };
    assert_eq!(
        recognize_rounded_box(&a, &strict).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    let mut open = m.clone();
    open.triangles.pop();
    assert_eq!(analyze_mesh(&open, &o).unwrap_err().code, "mesh.open");
    let tiny = rounded_box([12., 8., 5.], 5e-5, 12);
    match analyze_mesh(&tiny, &o) {
        Ok(a) => assert!(recognize_rounded_box(&a, &o).is_err()),
        Err(e) => assert!(e.code.starts_with("mesh.")),
    }
}

#[test]
fn cospherical_corner_fold_is_rejected_by_orientation_not_vertex_fit() {
    let o = options();
    let mut m = analyze_mesh(&rounded_box([12., 8., 5.], 2., 12), &o)
        .unwrap()
        .mesh;
    let ids: Vec<_> = m
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, p)| p[0] > 12.3 && p[1] > 8.3 && p[2] > 5.3)
        .map(|(i, _)| i)
        .take(2)
        .collect();
    assert_eq!(ids.len(), 2);
    m.vertices.swap(ids[0], ids[1]);
    let a = analyze_mesh(&m, &o).unwrap();
    let e = recognize_rounded_box(&a, &o).unwrap_err();
    assert_eq!(e.code, "recognition.surface_unproven");
    assert!(e.message.contains("projection locale"), "{e}");
}

#[test]
fn shared_seam_snapping_counts_the_full_vector_displacement() {
    let o = options();
    let mesh = analyze_mesh(&rounded_box([12., 8., 5.], 2., 12), &o)
        .unwrap()
        .mesh;
    let target = [12. + 2_f64.sqrt(), 8. + 2_f64.sqrt(), 5.];
    let id = mesh
        .vertices
        .iter()
        .position(|p| sub(*p, target).iter().all(|v| v.abs() < 1e-10))
        .unwrap();
    for fraction in [0.4, 0.8] {
        let mut m = mesh.clone();
        let movement = fraction * o.cylinder_fit_tolerance_mm;
        m.vertices[id][0] += movement / 2_f64.sqrt();
        m.vertices[id][1] += movement / 2_f64.sqrt();
        m.vertices[id][2] += movement;
        let a = analyze_mesh(&m, &o).unwrap();
        let result = recognize_rounded_box(&a, &o);
        if fraction < 0.5 {
            let r = result.unwrap();
            assert!(r.report.max_radial_residual_mm >= movement * 2_f64.sqrt() - 1e-12);
        } else {
            let e = result.unwrap_err();
            assert_eq!(e.code, "recognition.surface_unproven");
            assert!(e.message.contains("résidu vectoriel"), "{e}");
        }
    }
    let cube = fabstep_core::analyze_stl(include_bytes!("../../../fixtures/cube.stl"), &o).unwrap();
    let e = recognize_rounded_box(&cube, &o).unwrap_err();
    assert_eq!(e.code, "recognition.surface_unproven");
    assert!(e.message.contains("rayon"));
}

fn triangle_distance(p: V, a: V, b: V, c: V) -> f64 {
    // Independent closest-point barycentric-region algorithm.
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return dot(ap, ap).sqrt();
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return dot(bp, bp).sqrt();
    }
    let vc = d1 * d4 - d3 * d2;
    let interpolate = |q: V, d: V, t: f64| {
        let v = sub(p, std::array::from_fn(|i| q[i] + t * d[i]));
        dot(v, v).sqrt()
    };
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return interpolate(a, ab, d1 / (d1 - d3));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return dot(cp, cp).sqrt();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return interpolate(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        return interpolate(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1. / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    let delta = sub(p, std::array::from_fn(|i| a[i] + v * ab[i] + w * ac[i]));
    dot(delta, delta).sqrt()
}

#[test]
fn rounded_box_triangle_interiors_and_reverse_samples_obey_the_bound() {
    let o = options();
    let a = analyze_mesh(&rounded_box([12., 8., 5.], 2., 12), &o).unwrap();
    let r = recognize_rounded_box(&a, &o).unwrap();
    let bound = r.report.max_deviation_bound_mm;
    let core = [12., 8., 5.];
    for t in &a.mesh.triangles {
        let p = t.map(|v| a.mesh.vertices[v]);
        for weights in [[1. / 3.; 3], [0.2, 0.3, 0.5], [0.05, 0.6, 0.35]] {
            let q: V = std::array::from_fn(|i| (0..3).map(|j| p[j][i] * weights[j]).sum());
            let d: V = std::array::from_fn(|i| (q[i].abs() - core[i]).max(0.));
            assert!((dot(d, d).sqrt() - 2.).abs() <= bound + 1e-10);
        }
    }
    let independent = rounded_box(core, 2., 19);
    for &p in independent.vertices.iter().step_by(53) {
        let distance = a
            .mesh
            .triangles
            .iter()
            .map(|t| {
                triangle_distance(
                    p,
                    a.mesh.vertices[t[0]],
                    a.mesh.vertices[t[1]],
                    a.mesh.vertices[t[2]],
                )
            })
            .fold(f64::INFINITY, f64::min);
        assert!(distance <= bound + 1e-10, "{distance} > {bound}");
    }
}

#[test]
fn absence_of_six_supports_leaves_terminal_sphere_and_torus_recognition_available() {
    let o = options();
    for bytes in [
        include_bytes!("../../../fixtures/m5c_icosphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5c_icosphere_transformed.stl").as_slice(),
        include_bytes!("../../../fixtures/m5c_uv_sphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5d_ring_torus.stl").as_slice(),
        include_bytes!("../../../fixtures/m5d_torus_transformed.stl").as_slice(),
    ] {
        let a = fabstep_core::analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_rounded_box(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
    }
}
