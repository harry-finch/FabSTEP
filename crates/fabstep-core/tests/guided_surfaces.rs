use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_guided_cylinders, recognize_guided_surfaces,
    recognize_mixed, recognize_revolved, Analysis, GuidanceFamily, MixedFaceRef, Options,
    ReconstructionMode, RevolvedFaceRef, RevolvedRecognition,
};

fn automatic(bytes: &[u8]) -> (Analysis, RevolvedRecognition) {
    let options = Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    };
    let a = analyze_stl(bytes, &options).unwrap();
    let result = recognize_revolved(&a, &options).unwrap();
    (a, result)
}
fn triangles(result: &RevolvedRecognition, owner: RevolvedFaceRef) -> Vec<usize> {
    result
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| (*face == owner).then_some(i))
        .collect()
}

/// Taper only the existing boss's upper circular boundary. Both planar supports
/// and the distinct blind cylindrical hole remain real source surfaces.
fn tapered_boss_and_blind_hole(taper_hole: bool) -> (Analysis, RevolvedRecognition) {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        &options,
    )
    .unwrap();
    let original = recognize_mixed(&a, &options).unwrap();
    let mut mesh = a.mesh;
    for cylinder in &original.model.cylinders {
        if cylinder.inner && !taper_hole {
            continue;
        }
        let ring = cylinder
            .rings
            .iter()
            .map(|&i| &original.model.rings[i])
            .max_by(|a, b| a.center[2].total_cmp(&b.center[2]))
            .unwrap();
        let low_z = cylinder
            .rings
            .iter()
            .map(|&i| original.model.rings[i].center[2])
            .fold(f64::INFINITY, f64::min);
        for p in &mut mesh.vertices {
            if p[2] >= low_z - 1e-8
                && p[2] <= ring.center[2] + 1e-8
                && ((p[0] - ring.center[0]).hypot(p[1] - ring.center[1]) - ring.radius_mm).abs()
                    < 1e-8
            {
                let ratio = 1. - 0.3 * (p[2] - low_z) / (ring.center[2] - low_z);
                p[0] = ring.center[0] + ratio * (p[0] - ring.center[0]);
                p[1] = ring.center[1] + ratio * (p[1] - ring.center[1]);
            }
        }
    }
    let options = Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    };
    let a = analyze_mesh(&mesh, &options).unwrap();
    let result = recognize_revolved(&a, &options).unwrap();
    (a, result)
}

#[test]
fn seed_reconstructs_only_one_complete_cone_with_exhaustive_deterministic_provenance() {
    for bytes in [
        include_bytes!("../../../fixtures/m4_frustum.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_frustum_rotated.stl").as_slice(),
    ] {
        let (a, automatic) = automatic(bytes);
        let wall = triangles(&automatic, RevolvedFaceRef::Cone { index: 0 });
        let first = recognize_guided_surfaces(
            &a,
            &Options::default(),
            &[(GuidanceFamily::Cone, vec![wall[0]])],
        )
        .unwrap();
        let last = recognize_guided_surfaces(
            &a,
            &Options::default(),
            &[(GuidanceFamily::Cone, vec![*wall.last().unwrap()])],
        )
        .unwrap();
        assert_eq!(first.model.cones.len(), 1);
        assert!(first.model.mixed.cylinders.is_empty());
        assert_eq!(first.triangle_faces.len(), a.mesh.triangles.len());
        assert_eq!(triangles(&first, RevolvedFaceRef::Cone { index: 0 }), wall);
        assert_eq!(first.triangle_faces, last.triangle_faces);
        assert_eq!(
            first.report.expected_volume_mm3,
            last.report.expected_volume_mm3
        );
        assert!(
            (first.report.expected_volume_mm3 - automatic.report.expected_volume_mm3).abs() < 1e-8
        );
        assert!(first.report.max_deviation_bound_mm <= Options::default().max_deviation_mm);
        for (i, owner) in first.triangle_faces.iter().enumerate() {
            assert_eq!(
                matches!(owner, RevolvedFaceRef::Cone { .. }),
                wall.contains(&i)
            );
        }
        assert!(first
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "guidance.complete_cone_verified"));
    }
}

#[test]
fn distinct_cone_and_cylinder_indications_share_one_model_and_leave_other_walls_faceted() {
    let (a, all) = tapered_boss_and_blind_hole(false);
    assert_eq!(all.model.cones.len(), 1);
    assert_eq!(all.model.mixed.cylinders.len(), 1);
    let cone = triangles(&all, RevolvedFaceRef::Cone { index: 0 });
    let cylinder = triangles(&all, RevolvedFaceRef::Cylinder { index: 0 });
    let only_cone = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[(GuidanceFamily::Cone, vec![cone[0]])],
    )
    .unwrap();
    assert!(only_cone.model.mixed.cylinders.is_empty());
    assert!(cylinder
        .iter()
        .all(|&t| matches!(only_cone.triangle_faces[t], RevolvedFaceRef::Plane { .. })));
    let both = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[
            (GuidanceFamily::Cone, vec![cone[0]]),
            (GuidanceFamily::Cylinder, vec![cylinder[0]]),
        ],
    )
    .unwrap();
    assert_eq!(triangles(&both, RevolvedFaceRef::Cone { index: 0 }), cone);
    assert_eq!(
        triangles(&both, RevolvedFaceRef::Cylinder { index: 0 }),
        cylinder
    );
    assert!((both.report.expected_volume_mm3 - all.report.expected_volume_mm3).abs() < 1e-8);
}

#[test]
fn independent_cones_can_be_selected_one_at_a_time_or_together() {
    let (a, all) = tapered_boss_and_blind_hole(true);
    assert_eq!(all.model.cones.len(), 2);
    let first = triangles(&all, RevolvedFaceRef::Cone { index: 0 });
    let second = triangles(&all, RevolvedFaceRef::Cone { index: 1 });
    let one = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[(GuidanceFamily::Cone, vec![first[0]])],
    )
    .unwrap();
    assert_eq!(one.model.cones.len(), 1);
    assert!(second
        .iter()
        .all(|&t| matches!(one.triangle_faces[t], RevolvedFaceRef::Plane { .. })));
    let both = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[
            (GuidanceFamily::Cone, vec![first[0]]),
            (GuidanceFamily::Cone, vec![second[0]]),
        ],
    )
    .unwrap();
    assert_eq!(both.model.cones.len(), 2);
}

#[test]
fn an_unselected_non_affine_wall_does_not_block_a_proved_cone() {
    let (a, all) = tapered_boss_and_blind_hole(true);
    let boss_index = all
        .model
        .cones
        .iter()
        .position(|c| all.model.mixed.rings[c.rings[0]].center[0] < 0.)
        .unwrap();
    let boss = triangles(&all, RevolvedFaceRef::Cone { index: boss_index });
    let mut mesh = a.mesh;
    for p in &mut mesh.vertices {
        if p[0] > 4. && p[2] > 4. + 1e-6 && p[2] < 10. - 1e-6 {
            let dx = p[0] - 7.5;
            let radius = dx.hypot(p[1]);
            let ratio = (radius + 0.03) / radius;
            p[0] = 7.5 + dx * ratio;
            p[1] *= ratio;
        }
    }
    let a = analyze_mesh(&mesh, &Options::default()).unwrap();
    let automatic_options = Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    };
    assert_eq!(
        recognize_revolved(&a, &automatic_options).unwrap_err().code,
        "recognition.conic_unproven"
    );
    let guided = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[(GuidanceFamily::Cone, vec![boss[0]])],
    )
    .unwrap();
    assert_eq!(guided.model.cones.len(), 1);
    assert_eq!(triangles(&guided, RevolvedFaceRef::Cone { index: 0 }), boss);
}

#[test]
fn malformed_selections_wrong_families_and_duplicate_walls_are_refused() {
    let (a, result) = automatic(include_bytes!("../../../fixtures/m4_frustum.stl"));
    let wall = triangles(&result, RevolvedFaceRef::Cone { index: 0 });
    let plane = result
        .triangle_faces
        .iter()
        .position(|face| matches!(face, RevolvedFaceRef::Plane { .. }))
        .unwrap();
    for selection in [vec![], vec![wall[0], wall[0]], vec![a.mesh.triangles.len()]] {
        assert_eq!(
            recognize_guided_surfaces(
                &a,
                &Options::default(),
                &[(GuidanceFamily::Cone, selection)]
            )
            .unwrap_err()
            .code,
            "guidance.invalid_selection"
        );
    }
    for selection in [vec![plane], vec![plane, wall[0]]] {
        assert_eq!(
            recognize_guided_surfaces(
                &a,
                &Options::default(),
                &[(GuidanceFamily::Cone, selection)]
            )
            .unwrap_err()
            .code,
            "guidance.cone_unproven"
        );
    }
    assert_eq!(
        recognize_guided_surfaces(
            &a,
            &Options::default(),
            &[(GuidanceFamily::Cylinder, vec![wall[0]])]
        )
        .unwrap_err()
        .code,
        "guidance.cylinder_unproven"
    );
    assert_eq!(
        recognize_guided_surfaces(
            &a,
            &Options::default(),
            &[
                (GuidanceFamily::Cone, vec![wall[0]]),
                (GuidanceFamily::Cone, vec![wall[1]])
            ]
        )
        .unwrap_err()
        .code,
        "guidance.overlapping_indications"
    );
    assert_eq!(
        recognize_guided_surfaces(
            &a,
            &Options::default(),
            &vec![(GuidanceFamily::Cone, vec![wall[0]]); 17]
        )
        .unwrap_err()
        .code,
        "guidance.limit"
    );
    assert_eq!(
        recognize_guided_surfaces(&a, &Options::default(), &[])
            .unwrap_err()
            .code,
        "guidance.limit"
    );
    let cylinder_a = analyze_stl(
        include_bytes!("../../../fixtures/m2_plate_hole.stl"),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        recognize_guided_surfaces(
            &cylinder_a,
            &Options::default(),
            &[(GuidanceFamily::Cone, vec![0])]
        )
        .unwrap_err()
        .code,
        "guidance.cone_unproven"
    );
}

#[test]
fn cone_tips_direct_curved_boundaries_and_non_affine_meridians_are_not_forced() {
    for bytes in [
        include_bytes!("../../../fixtures/m4_cone_tip.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_countersink.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_chamfered_boss.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_curved_meridian.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_piecewise_taper.stl").as_slice(),
    ] {
        let a = analyze_stl(bytes, &Options::default()).unwrap();
        assert_eq!(
            recognize_guided_surfaces(&a, &Options::default(), &[(GuidanceFamily::Cone, vec![0])])
                .unwrap_err()
                .code,
            "guidance.cone_unproven"
        );
    }
}

#[test]
fn explicit_guidance_does_not_bypass_fit_deviation_or_tolerance_checks() {
    let (a, result) = automatic(include_bytes!("../../../fixtures/m4_frustum.stl"));
    let wall = triangles(&result, RevolvedFaceRef::Cone { index: 0 });
    let hints = [(GuidanceFamily::Cone, vec![wall[0]])];
    let tight = Options {
        max_deviation_mm: 0.0001,
        ..Options::default()
    };
    assert_eq!(
        recognize_guided_surfaces(&a, &tight, &hints)
            .unwrap_err()
            .code,
        "recognition.deviation_exceeded"
    );
    for tolerance in [0., f64::NAN, f64::INFINITY] {
        let bad = Options {
            cylinder_fit_tolerance_mm: tolerance,
            ..Options::default()
        };
        assert_eq!(
            recognize_guided_surfaces(&a, &bad, &hints)
                .unwrap_err()
                .code,
            "options.invalid_tolerance"
        );
    }
    let mut distorted = a.mesh;
    let vertex = distorted.triangles[wall[0]][0];
    distorted.vertices[vertex][0] += 0.03;
    let a = analyze_mesh(&distorted, &Options::default()).unwrap();
    assert_eq!(
        recognize_guided_surfaces(&a, &Options::default(), &hints)
            .unwrap_err()
            .code,
        "guidance.cone_unproven"
    );
}

#[test]
fn cylinder_only_generic_api_preserves_existing_reports_and_partition() {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        &options,
    )
    .unwrap();
    let all = recognize_mixed(&a, &options).unwrap();
    let seed = all
        .triangle_faces
        .iter()
        .position(|face| matches!(face, MixedFaceRef::Cylinder { .. }))
        .unwrap();
    let legacy = recognize_guided_cylinders(&a, &Options::default(), &[vec![seed]]).unwrap();
    let generic = recognize_guided_surfaces(
        &a,
        &Options::default(),
        &[(GuidanceFamily::Cylinder, vec![seed])],
    )
    .unwrap();
    assert_eq!(
        format!("{:?}", legacy.model),
        format!("{:?}", generic.model.mixed)
    );
    assert_eq!(
        format!("{:?}", legacy.report),
        format!("{:?}", generic.report)
    );
    assert_eq!(
        format!("{:?}", legacy.triangle_faces),
        format!("{:?}", generic.triangle_faces)
    );
}
