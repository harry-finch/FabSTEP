use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_guided_surfaces, Analysis, GuidanceFamily, Mesh,
    MixedLoop, Options, RevolvedFaceRef,
};

fn thin_box(bump: f64) -> Analysis {
    let width = 0.001;
    let mesh = Mesh {
        vertices: vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., width, 0.],
            [0., width, 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., width, 1.],
            [0., width, 1.],
            [0.5, width / 2., 1. + bump],
        ],
        triangles: vec![
            [0, 2, 1],
            [0, 3, 2],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
            [4, 5, 8],
            [5, 6, 8],
            [6, 7, 8],
            [7, 4, 8],
        ],
    };
    analyze_mesh(&mesh, &Options::default()).unwrap()
}
fn top(a: &Analysis) -> Vec<usize> {
    a.mesh
        .triangles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.iter().all(|&v| a.mesh.vertices[v][2] >= 1.).then_some(i))
        .collect()
}
#[test]
fn slightly_uneven_patch_merges_without_moving_vertices_or_unselected_faces() {
    let options = Options::default();
    let a = thin_box(5e-8);
    assert!(a.model.faces.len() > 6);
    let selected = top(&a);
    let r = recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, selected.clone())])
        .unwrap();
    assert_eq!(r.model.mixed.planes.len(), 6);
    assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
    assert!(r.model.mixed.cylinders.is_empty());
    assert!(r.model.cones.is_empty());
    assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    let owner = r.triangle_faces[selected[0]];
    for (i, face) in r.triangle_faces.iter().enumerate() {
        assert_eq!(*face == owner, selected.contains(&i));
    }
    assert!((r.report.expected_volume_mm3 - 0.001).abs() < 1e-12);
    assert!(r.report.max_wall_residual_mm >= 4.9e-8);
    assert!(r
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "guidance.plane_residual"));
    assert!(!r
        .report
        .diagnostics
        .iter()
        .any(|d| d.code.contains("circular_intent")));
    let mut reverse = selected;
    reverse.reverse();
    let s = recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, reverse)]).unwrap();
    assert_eq!(r.triangle_faces, s.triangle_faces);
    assert_eq!(r.report.expected_volume_mm3, s.report.expected_volume_mm3);
    for (a, b) in r.model.mixed.planes.iter().zip(&s.model.mixed.planes) {
        assert_eq!(a.normal, b.normal);
    }
}
#[test]
fn hole_is_retained_and_not_capped() {
    let options = Options::default();
    let a = analyze_stl(
        include_bytes!("../../../fixtures/rectangular_hole.stl"),
        &options,
    )
    .unwrap();
    let z = a.report.bounds_mm[1][2];
    let selected = a
        .mesh
        .triangles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            t.iter()
                .all(|&v| (a.mesh.vertices[v][2] - z).abs() < 1e-12)
                .then_some(i)
        })
        .collect::<Vec<_>>();
    let r = recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, selected.clone())])
        .unwrap();
    let RevolvedFaceRef::Plane { index } = r.triangle_faces[selected[0]] else {
        panic!("plane")
    };
    assert_eq!(r.model.mixed.planes[index].loops.len(), 2);
    assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
    assert!((r.report.expected_volume_mm3 - a.report.volume_mm3).abs() < 1e-10);
}

#[test]
fn rejects_nonplanar_disconnected_partial_and_overlapping_indications() {
    let options = Options::default();
    let a = thin_box(1e-4);
    assert_eq!(
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, top(&a))])
            .unwrap_err()
            .code,
        "guidance.plane_unproven"
    );
    let a = thin_box(5e-8);
    let mut selected = top(&a);
    selected.extend([0, 1]);
    assert_eq!(
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, selected)])
            .unwrap_err()
            .code,
        "guidance.plane_unproven"
    );
    assert_eq!(
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, vec![0])])
            .unwrap_err()
            .code,
        "guidance.plane_incomplete"
    );
    let selection = top(&a);
    assert_eq!(
        recognize_guided_surfaces(
            &a,
            &options,
            &[
                (GuidanceFamily::Plane, selection.clone()),
                (GuidanceFamily::Plane, selection)
            ]
        )
        .unwrap_err()
        .code,
        "guidance.overlapping_indications"
    );
}

#[test]
fn native_contour_ceiling_and_budget_are_independent_of_large_plane_tolerance() {
    let a = thin_box(5e-8);
    let options = Options {
        max_deviation_mm: 1e-8,
        ..Options::default()
    };
    assert_eq!(
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, top(&a))])
            .unwrap_err()
            .code,
        "recognition.deviation_exceeded"
    );
    // Twisting a boundary corner cannot be hidden with a generous plane tolerance.
    let mut mesh = a.mesh;
    mesh.vertices[6][2] += 1e-5;
    let options = Options {
        plane_tolerance_mm: 0.01,
        ..Options::default()
    };
    let a = analyze_mesh(&mesh, &options).unwrap();
    assert_eq!(
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Plane, top(&a))])
            .unwrap_err()
            .code,
        "guidance.plane_unproven"
    );
}

#[test]
fn plane_and_cylinder_coexist_but_curved_boundaries_are_explicitly_refused() {
    let options = Options::default();
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        &options,
    )
    .unwrap();
    let automatic = fabstep_core::recognize_mixed(
        &a,
        &Options {
            reconstruction: fabstep_core::ReconstructionMode::Cylinders,
            ..options.clone()
        },
    )
    .unwrap();
    let cylinder = automatic
        .triangle_faces
        .iter()
        .position(|face| matches!(face, fabstep_core::MixedFaceRef::Cylinder { index: 0 }))
        .unwrap();
    let base =
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Cylinder, vec![cylinder])])
            .unwrap();
    let polygon = base
        .model
        .mixed
        .planes
        .iter()
        .position(|p| {
            p.loops
                .iter()
                .all(|lp| matches!(lp, MixedLoop::Polygon { .. }))
        })
        .unwrap();
    let selected = base
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (*f == RevolvedFaceRef::Plane { index: polygon }).then_some(i))
        .collect::<Vec<_>>();
    let requests = [
        (GuidanceFamily::Plane, selected.clone()),
        (GuidanceFamily::Cylinder, vec![cylinder]),
    ];
    let r = recognize_guided_surfaces(&a, &options, &requests).unwrap();
    assert_eq!(r.model.mixed.cylinders.len(), 1);
    assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
    assert!(r.model.cones.is_empty());
    let reversed = [requests[1].clone(), requests[0].clone()];
    let s = recognize_guided_surfaces(&a, &options, &reversed).unwrap();
    assert_eq!(r.triangle_faces, s.triangle_faces);
    let cap = base
        .model
        .mixed
        .planes
        .iter()
        .position(|p| {
            p.loops
                .iter()
                .any(|lp| matches!(lp, MixedLoop::Circle { .. }))
        })
        .unwrap();
    let cap_selection = base
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (*f == RevolvedFaceRef::Plane { index: cap }).then_some(i))
        .collect();
    assert_eq!(
        recognize_guided_surfaces(
            &a,
            &options,
            &[
                (GuidanceFamily::Plane, cap_selection),
                (GuidanceFamily::Cylinder, vec![cylinder])
            ]
        )
        .unwrap_err()
        .code,
        "guidance.plane_unsupported_boundary"
    );
}

#[test]
fn plane_and_cone_coexist_without_reconstructing_an_unselected_cylinder() {
    let options = Options::default();
    let original = analyze_stl(
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        &options,
    )
    .unwrap();
    let automatic = fabstep_core::recognize_mixed(
        &original,
        &Options {
            reconstruction: fabstep_core::ReconstructionMode::Cylinders,
            ..options.clone()
        },
    )
    .unwrap();
    let boss = automatic.model.cylinders.iter().find(|c| !c.inner).unwrap();
    let top = boss
        .rings
        .iter()
        .map(|&i| &automatic.model.rings[i])
        .max_by(|a, b| a.center[2].total_cmp(&b.center[2]))
        .unwrap();
    let low = boss
        .rings
        .iter()
        .map(|&i| automatic.model.rings[i].center[2])
        .fold(f64::INFINITY, f64::min);
    let mut mesh = original.mesh;
    for p in &mut mesh.vertices {
        if p[2] >= low - 1e-8
            && p[2] <= top.center[2] + 1e-8
            && ((p[0] - top.center[0]).hypot(p[1] - top.center[1]) - top.radius_mm).abs() < 1e-8
        {
            let ratio = 1. - 0.3 * (p[2] - low) / (top.center[2] - low);
            p[0] = top.center[0] + ratio * (p[0] - top.center[0]);
            p[1] = top.center[1] + ratio * (p[1] - top.center[1]);
        }
    }
    let a = analyze_mesh(&mesh, &options).unwrap();
    let automatic = fabstep_core::recognize_revolved(
        &a,
        &Options {
            reconstruction: fabstep_core::ReconstructionMode::Primitives,
            ..options.clone()
        },
    )
    .unwrap();
    let cone = automatic
        .triangle_faces
        .iter()
        .position(|face| matches!(face, RevolvedFaceRef::Cone { index: 0 }))
        .unwrap();
    let base =
        recognize_guided_surfaces(&a, &options, &[(GuidanceFamily::Cone, vec![cone])]).unwrap();
    let polygon = base
        .model
        .mixed
        .planes
        .iter()
        .position(|p| {
            p.loops
                .iter()
                .all(|lp| matches!(lp, MixedLoop::Polygon { .. }))
        })
        .unwrap();
    let selected = base
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (*f == RevolvedFaceRef::Plane { index: polygon }).then_some(i))
        .collect();
    let r = recognize_guided_surfaces(
        &a,
        &options,
        &[
            (GuidanceFamily::Plane, selected),
            (GuidanceFamily::Cone, vec![cone]),
        ],
    )
    .unwrap();
    assert_eq!(r.model.cones.len(), 1);
    assert!(r.model.mixed.cylinders.is_empty());
    assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
    assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    assert!((r.report.expected_volume_mm3 - base.report.expected_volume_mm3).abs() < 1e-8);
}
