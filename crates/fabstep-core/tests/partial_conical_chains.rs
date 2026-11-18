use fabstep_core::*;
fn source(rotated: bool, warped: bool, nonplanar_support: bool) -> Vec<u8> {
    const N: usize = 48;
    let levels = [(0., 1.), (2., 1.), (3., 2.), (4., 2.)];
    let mut vertices = Vec::new();
    for (z, radius) in levels {
        for i in 0..=N {
            let angle = std::f64::consts::PI * i as f64 / N as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), z]);
        }
    }
    if warped {
        vertices[N + 1 + N / 2][1] += 0.02;
    }
    let mut triangles = Vec::new();
    for layer in 0..3 {
        let lo = layer * (N + 1);
        let hi = lo + N + 1;
        for i in 0..=N {
            let j = (i + 1) % (N + 1);
            triangles.extend([[lo + i, lo + j, hi + j], [lo + i, hi + j, hi + i]]);
        }
    }
    for (layer, reverse) in [(0, true), (3, false)] {
        let (z, radius) = levels[layer];
        let center = vertices.len();
        vertices.push([0., radius * 0.25, z]);
        for i in 0..=N {
            let j = (i + 1) % (N + 1);
            let mut triangle = [center, layer * (N + 1) + i, layer * (N + 1) + j];
            if reverse {
                triangle.swap(1, 2);
            }
            triangles.push(triangle);
        }
    }
    if nonplanar_support {
        // Split the bottom cap's straight diameter edge on both incident faces.
        // This vertex is outside the circular wall and remains inside the
        // segmentation tolerance, but cannot belong to an exact curved plane.
        let midpoint = vertices.len();
        vertices.push([0., 0., 8e-7]);
        let mut split = Vec::new();
        for triangle in triangles {
            let mut done = false;
            for i in 0..3 {
                let a = triangle[i];
                let b = triangle[(i + 1) % 3];
                let c = triangle[(i + 2) % 3];
                if (a == 0 && b == N) || (a == N && b == 0) {
                    split.extend([[a, midpoint, c], [midpoint, b, c]]);
                    done = true;
                    break;
                }
            }
            if !done {
                split.push(triangle);
            }
        }
        triangles = split;
    }
    let mut text = String::from("solid independent_partial_chain\n");
    for triangle in triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for v in triangle {
            let [mut x, mut y, mut z] = vertices[v];
            if rotated {
                // Two non-axis rotations and a translation, without quantizing ASCII.
                let (s, c) = 0.43_f64.sin_cos();
                (y, z) = (c * y - s * z, s * y + c * z);
                let (s, c) = 0.72_f64.sin_cos();
                (x, y) = (c * x - s * y, s * x + c * y);
                x += 17.;
                y -= 23.;
                z += 8.;
            }
            text.push_str(&format!("vertex {x:.17} {y:.17} {z:.17}\n"));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid independent_partial_chain\n");
    text.into_bytes()
}

#[test]
fn semicircular_chain_is_jointly_proven_and_rotation_invariant() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for rotated in [false, true] {
        let a = analyze_stl(&source(rotated, false, false), &options).unwrap();
        let r = recognize_automatic(&a, &options).unwrap();
        assert_eq!(r.model.partial_cones.len(), 1, "rotated={rotated}");
        assert_eq!(r.model.revolved.mixed.partial_cylinders.len(), 2);
        assert!((r.report.expected_volume_mm3 - 25. * std::f64::consts::PI / 6.).abs() < 1e-7);
        let cone = r
            .triangle_faces
            .iter()
            .position(|f| matches!(f, RoundedFaceRef::PartialCone { .. }))
            .unwrap();
        let cylinder = r
            .triangle_faces
            .iter()
            .position(|f| matches!(f, RoundedFaceRef::PartialCylinder { .. }))
            .unwrap();
        let cone_group = guided_rounded_region(&r, GuidanceFamily::Cone, &[cone]).unwrap();
        let cylinder_group =
            guided_rounded_region(&r, GuidanceFamily::Cylinder, &[cylinder]).unwrap();
        assert_eq!(cone_group, cylinder_group);
        for selected in [[cone, cylinder], [cylinder, cone]] {
            for family in [
                GuidanceFamily::Cylinder,
                GuidanceFamily::Cone,
                GuidanceFamily::Auto,
            ] {
                assert_eq!(
                    guided_rounded_region(&r, family, &selected).unwrap(),
                    cone_group
                );
            }
        }
        let plane = r
            .triangle_faces
            .iter()
            .position(|f| matches!(f, RoundedFaceRef::Plane { .. }))
            .unwrap();
        assert!(
            guided_rounded_region(&r, GuidanceFamily::Cylinder, &[cone, cylinder, plane]).is_err()
        );
        assert_eq!(cone_group.len(), 48 * 2 * 3);
        assert!(guided_rounded_region(&r, GuidanceFamily::Cylinder, &[cone]).is_err());
        assert!(guided_rounded_region(&r, GuidanceFamily::Cone, &[cylinder]).is_err());
        let guided =
            recognize_automatic_guided(&a, &options, &[(GuidanceFamily::Cylinder, vec![cylinder])])
                .unwrap();
        assert_eq!(guided.triangle_faces, r.triangle_faces);
    }
}
#[test]
fn a_warped_joint_never_forces_a_partial_cone() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(&source(false, true, false), &options).unwrap();
    let r = recognize_automatic(&a, &options).unwrap();
    assert!(r.model.partial_cones.is_empty());
    assert!(!r
        .triangle_faces
        .iter()
        .any(|f| matches!(f, RoundedFaceRef::PartialCone { .. })));
}

#[test]
fn unrelated_nonplanar_cap_boundary_stays_faceted() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(&source(false, false, true), &options).unwrap();
    let r = recognize_automatic(&a, &options).unwrap();
    assert!(r.model.partial_cones.is_empty());
    assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
}

#[test]
fn family_witness_does_not_merge_independent_chain_identities() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(&source(false, false, false), &options).unwrap();
    let mut r = recognize_automatic(&a, &options).unwrap();
    let original = r.clone();
    let arc_offset = r.model.revolved.mixed.arcs.len();
    let cylinder_offset = r.model.revolved.mixed.partial_cylinders.len();
    let cone_offset = r.model.partial_cones.len();
    r.model
        .revolved
        .mixed
        .arcs
        .extend(original.model.revolved.mixed.arcs.iter().cloned());
    r.model.revolved.mixed.partial_cylinders.extend(
        original
            .model
            .revolved
            .mixed
            .partial_cylinders
            .iter()
            .map(|c| MixedPartialCylinder {
                elliptic_end: false,
                arcs: c.arcs.map(|i| i + arc_offset),
                inner: c.inner,
            }),
    );
    r.model.partial_cones.extend(
        original
            .model
            .partial_cones
            .iter()
            .map(|c| MixedPartialCone {
                arcs: c.arcs.map(|i| i + arc_offset),
                inner: c.inner,
            }),
    );
    r.triangle_faces
        .extend(original.triangle_faces.iter().map(|f| match *f {
            RoundedFaceRef::PartialCylinder { index } => RoundedFaceRef::PartialCylinder {
                index: index + cylinder_offset,
            },
            RoundedFaceRef::PartialCone { index } => RoundedFaceRef::PartialCone {
                index: index + cone_offset,
            },
            other => other,
        }));
    let first = original
        .triangle_faces
        .iter()
        .position(|f| matches!(f, RoundedFaceRef::PartialCylinder { .. }))
        .unwrap();
    let second = original.triangle_faces.len()
        + original
            .triangle_faces
            .iter()
            .position(|f| matches!(f, RoundedFaceRef::PartialCone { .. }))
            .unwrap();
    for family in [
        GuidanceFamily::Auto,
        GuidanceFamily::Cylinder,
        GuidanceFamily::Cone,
    ] {
        assert!(guided_rounded_region(&r, family, &[second, first]).is_err());
    }
}
