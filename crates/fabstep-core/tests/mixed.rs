use fabstep_core::{
    analyze_stl, recognize_mixed, MixedFaceRef, MixedLoop, Options, ReconstructionMode,
};
use std::f64::consts::PI;

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}
fn run(bytes: &[u8]) -> (fabstep_core::Analysis, fabstep_core::MixedRecognition) {
    let o = options();
    let a = analyze_stl(bytes, &o).unwrap();
    let result = recognize_mixed(&a, &o).unwrap();
    (a, result)
}

#[test]
fn independent_blind_holes_keep_the_real_floor_and_all_source_triangles() {
    for source in [
        include_bytes!("../../../fixtures/m2b_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_blind_subdivided.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_blind_rotated.stl").as_slice(),
    ] {
        let (a, r) = run(source);
        assert_eq!(r.model.cylinders.len(), 1);
        assert!(r.model.cylinders[0].inner);
        assert_eq!(r.model.planes.len(), 7);
        assert_eq!(r.model.rings.len(), 2);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        let mut count = vec![0; r.model.planes.len() + r.model.cylinders.len()];
        for face in &r.triangle_faces {
            match face {
                MixedFaceRef::Plane { index } => count[*index] += 1,
                MixedFaceRef::Cylinder { index } => count[r.model.planes.len() + index] += 1,
                MixedFaceRef::PartialCylinder { .. } => {
                    panic!("This fixture contains complete cylinders only")
                }
            }
        }
        assert!(count.iter().all(|&c| c > 0));
        assert!(r
            .model
            .planes
            .iter()
            .any(|p| p.loops.len() == 1 && matches!(p.loops[0], MixedLoop::Circle { .. })));
        assert!((r.report.expected_volume_mm3 - (6000. - 54. * PI)).abs() < 1e-6);
        assert!(r.report.max_deviation_bound_mm < 0.014);
        let cyl = &r.model.cylinders[0];
        let p = r.model.rings[cyl.rings[0]].center;
        let q = r.model.rings[cyl.rings[1]].center;
        assert!(((p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]) - 6.).abs() < 1e-9);
    }
}

#[test]
fn independent_boss_and_mixed_features_preserve_outer_orientation() {
    let (_, boss) = run(include_bytes!("../../../fixtures/m2b_boss.stl"));
    assert_eq!(boss.model.cylinders.len(), 1);
    assert!(!boss.model.cylinders[0].inner);
    assert_eq!(boss.model.planes.len(), 7);
    assert!((boss.report.expected_volume_mm3 - (6000. + 45. * PI)).abs() < 1e-7);
    for source in [
        include_bytes!("../../../fixtures/m2b_boss_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_boss_blind_rotated.stl").as_slice(),
    ] {
        let (_, r) = run(source);
        assert_eq!(r.model.cylinders.len(), 2);
        assert_eq!(r.model.planes.len(), 8);
        assert_eq!(r.model.cylinders.iter().filter(|c| c.inner).count(), 1);
        assert!((r.report.expected_volume_mm3 - (6000. - 9. * PI)).abs() < 1e-6);
    }
    let (_, two) = run(include_bytes!("../../../fixtures/m2b_two_holes.stl"));
    assert_eq!(two.model.cylinders.len(), 2);
    assert!(two.model.cylinders.iter().all(|c| c.inner));
    assert_eq!(two.model.planes.len(), 7);
    assert!((two.report.expected_volume_mm3 - (6000. - 144. * PI)).abs() < 1e-7);
}

#[test]
fn partial_curved_features_are_not_silently_flattened() {
    let o = options();
    for source in [
        include_bytes!("../../../fixtures/m2b_conical_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_nonflat_floor.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_bulged_boss.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_low_sample_blind.stl").as_slice(),
    ] {
        let a = analyze_stl(source, &o).unwrap();
        assert_eq!(
            recognize_mixed(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
    }
}

#[test]
fn ring_incidence_and_budget_are_explicit() {
    let (a, r) = run(include_bytes!("../../../fixtures/m2b_boss_blind.stl"));
    let mut counts = vec![(0, 0); r.model.rings.len()];
    for plane in &r.model.planes {
        for lp in &plane.loops {
            if let MixedLoop::Circle { ring } = lp {
                counts[*ring].0 += 1;
            }
        }
    }
    for cylinder in &r.model.cylinders {
        for ring in cylinder.rings {
            counts[ring].1 += 1;
        }
    }
    assert!(counts.iter().all(|counts| *counts == (1, 1)));
    let o = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        recognize_mixed(&a, &Options::default()).unwrap_err().code,
        "recognition.disabled"
    );
}

fn serialize(mesh: &fabstep_core::Mesh) -> Vec<u8> {
    let mut text = String::from("solid edited\n");
    for triangle in &mesh.triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for &v in triangle {
            let p = mesh.vertices[v];
            text.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid edited\n");
    text.into_bytes()
}

#[test]
fn final_common_cylinder_must_respect_fit_tolerance_independently_of_plane_tolerance() {
    let o = Options {
        plane_tolerance_mm: 1e-4,
        cylinder_fit_tolerance_mm: 1e-6,
        ..options()
    };
    let mut a = analyze_stl(include_bytes!("../../../fixtures/m2b_blind.stl"), &o).unwrap();
    for p in &mut a.mesh.vertices {
        if (p[2] - 10.).abs() < 1e-9 && (p[0].hypot(p[1]) - 3.).abs() < 1e-8 {
            p[0] += 15e-6;
        }
    }
    let a = analyze_stl(&serialize(&a.mesh), &o).unwrap();
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.unsupported"
    );
}

type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn segment_distance(p: V, a: V, b: V) -> f64 {
    let ab = sub(b, a);
    let t = (dot(sub(p, a), ab) / dot(ab, ab)).clamp(0., 1.);
    norm(std::array::from_fn(|i| p[i] - a[i] - t * ab[i]))
}
fn triangle_distance(p: V, [a, b, c]: [V; 3]) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let n = cross(ab, ac);
    let length = norm(n);
    let d00 = dot(ab, ab);
    let d11 = dot(ac, ac);
    let d01 = dot(ab, ac);
    let denominator = d00 * d11 - d01 * d01;
    if length > 0. && denominator > 0. {
        let v = (d11 * dot(ap, ab) - d01 * dot(ap, ac)) / denominator;
        let w = (d00 * dot(ap, ac) - d01 * dot(ap, ab)) / denominator;
        if v >= 0. && w >= 0. && v + w <= 1. {
            return dot(ap, n).abs() / length;
        }
    }
    segment_distance(p, a, b)
        .min(segment_distance(p, b, c))
        .min(segment_distance(p, c, a))
}

#[test]
fn independent_bidirectional_samples_stay_within_local_cylinder_bound() {
    let (a, r) = run(include_bytes!(
        "../../../fixtures/m2b_boss_blind_rotated.stl"
    ));
    let bound = r.report.max_deviation_bound_mm + 1e-10;
    for (index, cylinder) in r.model.cylinders.iter().enumerate() {
        let first = &r.model.rings[cylinder.rings[0]];
        let second = &r.model.rings[cylinder.rings[1]];
        let axis = first.axis;
        let height = dot(sub(second.center, first.center), axis);
        let helper = if axis[0].abs() < 0.8 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let x = cross(helper, axis);
        let x = x.map(|v| v / norm(x));
        let y = cross(axis, x);
        let triangles: Vec<[V; 3]> = r
            .triangle_faces
            .iter()
            .enumerate()
            .filter_map(|(t, face)| {
                matches!(face,MixedFaceRef::Cylinder{index:i} if *i==index)
                    .then(|| a.mesh.triangles[t].map(|v| a.mesh.vertices[v]))
            })
            .collect();
        // Forward: source vertices and triangle centroids to finite cylinder.
        for triangle in &triangles {
            let center =
                std::array::from_fn(|i| (triangle[0][i] + triangle[1][i] + triangle[2][i]) / 3.);
            for p in triangle.iter().copied().chain([center]) {
                let offset = sub(p, first.center);
                let z = dot(offset, axis);
                let radial = norm(std::array::from_fn(|i| offset[i] - axis[i] * z));
                let axial = (-z).max(z - height).max(0.);
                assert!((radial - first.radius_mm).abs().hypot(axial) <= bound);
            }
        }
        // Reverse: samples on the analytic cylinder to the independent source
        // triangle-distance routine. This is sampled validation, not a proof of
        // global Hausdorff distance or of the quality of untested inputs.
        for height_index in 0..=4 {
            for angle_index in 0..128 {
                let theta = 2. * PI * angle_index as f64 / 128.;
                let z = height * height_index as f64 / 4.;
                let p = std::array::from_fn(|i| {
                    first.center[i]
                        + axis[i] * z
                        + first.radius_mm * (x[i] * theta.cos() + y[i] * theta.sin())
                });
                let distance = triangles
                    .iter()
                    .map(|t| triangle_distance(p, *t))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= bound,
                    "reverse sample distance {distance} exceeds {bound}"
                );
            }
        }
    }
}

#[test]
fn retained_plane_residual_is_included_in_the_reported_bound() {
    let o = options();
    let mut a = analyze_stl(include_bytes!("../../../fixtures/m2b_blind.stl"), &o).unwrap();
    let corner = a
        .mesh
        .vertices
        .iter_mut()
        .find(|p| **p == [15., -10., 10.])
        .unwrap();
    corner[2] += 5e-7;
    let a = analyze_stl(&serialize(&a.mesh), &o).unwrap();
    let r = recognize_mixed(&a, &o).unwrap();
    let mut maximum: f64 = 0.;
    for (triangle, face) in r.triangle_faces.iter().enumerate() {
        if let MixedFaceRef::Plane { index } = face {
            let plane = &r.model.planes[*index];
            let origin = match &plane.loops[0] {
                MixedLoop::Polygon { vertices } => r.model.vertices[vertices[0]],
                MixedLoop::Circle { ring } => r.model.rings[*ring].center,
                MixedLoop::Segments { .. } => panic!("This fixture contains no partial arcs"),
            };
            let normal = plane.normal.map(|v| v / norm(plane.normal));
            for &v in &a.mesh.triangles[triangle] {
                maximum = maximum.max(dot(sub(a.mesh.vertices[v], origin), normal).abs());
            }
        }
    }
    assert!(maximum > 1e-8);
    assert!(r.report.max_wall_residual_mm >= maximum);
}
