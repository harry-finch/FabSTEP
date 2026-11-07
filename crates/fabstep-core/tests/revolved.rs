use fabstep_core::{
    analyze_stl, recognize_mixed, recognize_revolved, Options, ReconstructionMode, RevolvedFaceRef,
};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    }
}

#[test]
fn frusta_apices_chamfers_and_countersinks_have_complete_source_provenance() {
    let pi = std::f64::consts::PI;
    for (name, bytes, planes, cylinders, volume) in [
        (
            "frustum",
            include_bytes!("../../../fixtures/m4_frustum.stl").as_slice(),
            2,
            0,
            pi * 8. * 79. / 3.,
        ),
        (
            "rotated",
            include_bytes!("../../../fixtures/m4_frustum_rotated.stl").as_slice(),
            2,
            0,
            pi * 8. * 79. / 3.,
        ),
        (
            "tip",
            include_bytes!("../../../fixtures/m4_cone_tip.stl").as_slice(),
            1,
            0,
            pi * 36. * 9. / 3.,
        ),
        (
            "boss",
            include_bytes!("../../../fixtures/m4_chamfered_boss.stl").as_slice(),
            7,
            1,
            6000. + pi * (125. + 98. / 3.),
        ),
        (
            "countersink",
            include_bytes!("../../../fixtures/m4_countersink.stl").as_slice(),
            6,
            1,
            6000. - pi * 126.,
        ),
        (
            "drill",
            include_bytes!("../../../fixtures/m4_drill_tip.stl").as_slice(),
            6,
            1,
            6000. - pi * 51.,
        ),
    ] {
        let opts = options();
        let source = analyze_stl(bytes, &opts).unwrap();
        let result = recognize_revolved(&source, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(result.model.mixed.vertices, source.mesh.vertices, "{name}");
        assert_eq!(result.model.mixed.planes.len(), planes, "{name}");
        assert_eq!(result.model.mixed.cylinders.len(), cylinders, "{name}");
        assert_eq!(result.model.cones.len(), 1, "{name}");
        assert_eq!(result.report.cones, 1, "{name}");
        assert_eq!(result.report.cylinders, cylinders, "{name}");
        assert_eq!(result.triangle_faces.len(), source.mesh.triangles.len());
        for owner in &result.triangle_faces {
            match owner {
                RevolvedFaceRef::Plane { index } => assert!(*index < planes),
                RevolvedFaceRef::Cylinder { index } => assert!(*index < cylinders),
                RevolvedFaceRef::Cone { index } => assert!(*index < 1),
            }
        }
        assert!(
            (result.report.expected_volume_mm3 - volume).abs() < volume * 1e-10,
            "{name}: {} != {volume}",
            result.report.expected_volume_mm3
        );
        assert!(result.report.max_deviation_bound_mm <= opts.max_deviation_mm);
        assert!(result.report.max_radial_residual_mm <= opts.cylinder_fit_tolerance_mm);
        assert!(result.report.max_chord_error_mm > 0.001);
        let mut ring_uses = vec![0_usize; result.model.mixed.rings.len()];
        for cone in &result.model.cones {
            for &ring in &cone.rings {
                ring_uses[ring] += 1;
            }
        }
        for cylinder in &result.model.mixed.cylinders {
            for &ring in &cylinder.rings {
                ring_uses[ring] += 1;
            }
        }
        for plane in &result.model.mixed.planes {
            for lp in &plane.loops {
                if let fabstep_core::MixedLoop::Circle { ring } = lp {
                    ring_uses[*ring] += 1;
                }
            }
        }
        for (ring, uses) in result.model.mixed.rings.iter().zip(ring_uses) {
            if ring.radius_mm == 0. {
                assert_eq!(uses, 1);
                assert!(
                    source.mesh.vertices.iter().any(|p| {
                        (p[0] - ring.center[0])
                            .hypot(p[1] - ring.center[1])
                            .hypot(p[2] - ring.center[2])
                            <= opts.cylinder_fit_tolerance_mm
                    }),
                    "every apex must have an actual source vertex"
                );
            } else {
                assert_eq!(uses, 2);
            }
        }
    }
}

#[test]
fn curved_meridians_are_not_replaced_by_a_sequence_of_cones() {
    for bytes in [
        include_bytes!("../../../fixtures/m4_piecewise_taper.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_curved_meridian.stl").as_slice(),
    ] {
        let opts = options();
        let source = analyze_stl(bytes, &opts).unwrap();
        let error = recognize_revolved(&source, &opts).unwrap_err();
        assert_eq!(error.code, "recognition.conic_unproven");
        assert!(error.message.contains("rayon non affine"));
    }
}

#[test]
fn tiny_radius_changes_cannot_hide_separators_between_conical_slopes() {
    // Independent synthetic source: the two short, genuinely sloping bands
    // change radius by only the fit tolerance. Classifying them as cylinders
    // must not admit three otherwise forbidden conical slopes.
    let profile = [
        (0., 7.),
        (1., 6.6),
        (1.00006, 6.59999),
        (2., 6.3),
        (2.00006, 6.29999),
        (3., 5.9),
    ];
    let segments = 48;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (z, radius) in profile {
        for i in 0..segments {
            let angle = std::f64::consts::TAU * i as f64 / segments as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), z]);
        }
    }
    for level in 0..profile.len() - 1 {
        for i in 0..segments {
            let j = (i + 1) % segments;
            let lower = level * segments;
            let upper = lower + segments;
            triangles.push([lower + i, lower + j, upper + j]);
            triangles.push([lower + i, upper + j, upper + i]);
        }
    }
    for (level, reversed) in [(0, true), (profile.len() - 1, false)] {
        let center = vertices.len();
        vertices.push([0., 0., profile[level].0]);
        for i in 0..segments {
            let mut triangle = [
                center,
                level * segments + i,
                level * segments + (i + 1) % segments,
            ];
            if reversed {
                triangle.swap(1, 2);
            }
            triangles.push(triangle);
        }
    }
    let opts = options();
    let mesh = fabstep_core::Mesh {
        vertices,
        triangles,
    };
    let source = fabstep_core::analyze_mesh(&mesh, &opts).unwrap();
    let error = recognize_revolved(&source, &opts).unwrap_err();
    assert_eq!(error.code, "recognition.conic_unproven");
    assert!(error.message.contains("plusieurs plages coniques"));
}

#[test]
fn malformed_cones_and_sparse_boundaries_are_refused() {
    for (name, bytes) in [
        (
            "sparse",
            include_bytes!("../../../fixtures/m4_sparse_cone.stl").as_slice(),
        ),
        (
            "shifted_tip",
            include_bytes!("../../../fixtures/m4_shifted_tip.stl").as_slice(),
        ),
        (
            "warped",
            include_bytes!("../../../fixtures/m4_warped_frustum.stl").as_slice(),
        ),
    ] {
        let opts = options();
        let source = analyze_stl(bytes, &opts).unwrap();
        let error = recognize_revolved(&source, &opts).unwrap_err();
        assert!(
            matches!(
                error.code.as_str(),
                "recognition.conic_unproven" | "recognition.unsupported"
            ),
            "{name}: {error}"
        );
        if name != "sparse" {
            assert_eq!(error.code, "recognition.conic_unproven", "{name}");
        }
    }
}

#[test]
fn conic_fit_preserves_an_independent_deviation_limit() {
    let opts = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    let source = analyze_stl(include_bytes!("../../../fixtures/m4_frustum.stl"), &opts).unwrap();
    assert_eq!(
        recognize_revolved(&source, &opts).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn conic_mode_is_explicit_and_legacy_cylinder_behavior_stays_strict() {
    let opts = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let source = analyze_stl(include_bytes!("../../../fixtures/m4_frustum.stl"), &opts).unwrap();
    assert_eq!(
        recognize_revolved(&source, &opts).unwrap_err().code,
        "recognition.disabled"
    );
    assert_eq!(
        recognize_mixed(&source, &opts).unwrap_err().code,
        "recognition.unsupported"
    );
    let opts = options();
    let source = analyze_stl(include_bytes!("../../../fixtures/m2_cylinder.stl"), &opts).unwrap();
    assert_eq!(
        recognize_revolved(&source, &opts).unwrap_err().code,
        "recognition.unsupported"
    );
}
