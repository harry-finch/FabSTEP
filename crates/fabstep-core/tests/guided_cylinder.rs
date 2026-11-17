use fabstep_core::{
    analyze_stl, recognize_guided_cylinder, recognize_mixed, Analysis, MixedFaceRef, MixedLoop,
    MixedRecognition, Options, ReconstructionMode,
};

const MIXED: &[u8] = include_bytes!("../../../fixtures/m2b_boss_blind.stl");

fn automatic(bytes: &[u8]) -> (Analysis, MixedRecognition) {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let analysis = analyze_stl(bytes, &options).unwrap();
    let result = recognize_mixed(&analysis, &options).unwrap();
    (analysis, result)
}

fn wall(result: &MixedRecognition, cylinder: usize) -> Vec<usize> {
    result
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(t, owner)| {
            matches!(owner, MixedFaceRef::Cylinder { index } if *index == cylinder).then_some(t)
        })
        .collect()
}

#[test]
fn selected_boss_or_hole_changes_only_the_requested_cylinder() {
    for bytes in [
        MIXED,
        include_bytes!("../../../fixtures/m2b_boss_blind_rotated.stl").as_slice(),
    ] {
        let (analysis, automatic) = automatic(bytes);
        assert_eq!(automatic.model.cylinders.len(), 2);
        let mut volumes = Vec::new();
        for cylinder in 0..2 {
            let selected = wall(&automatic, cylinder);
            let result =
                recognize_guided_cylinder(&analysis, &Options::default(), &selected).unwrap();
            assert_eq!(result.model.cylinders.len(), 1);
            assert!(result.model.partial_cylinders.is_empty());
            assert_eq!(result.model.rings.len(), 2);
            assert_eq!(
                result.model.cylinders[0].inner,
                automatic.model.cylinders[cylinder].inner
            );
            assert_eq!(result.triangle_faces.len(), analysis.mesh.triangles.len());
            for (t, owner) in result.triangle_faces.iter().enumerate() {
                if selected.contains(&t) {
                    assert!(matches!(owner, MixedFaceRef::Cylinder { index: 0 }));
                } else {
                    assert!(matches!(owner, MixedFaceRef::Plane { .. }));
                }
            }
            volumes.push(result.report.expected_volume_mm3);
        }
        assert!((volumes[0] - volumes[1]).abs() > 0.01);
    }
}

#[test]
fn a_single_seed_proves_the_entire_wall_and_is_deterministic() {
    let (analysis, automatic) =
        automatic(include_bytes!("../../../fixtures/m2b_blind_subdivided.stl"));
    let selected = wall(&automatic, 0);
    assert!(selected.len() > 2);
    let options = Options::default();
    let first = recognize_guided_cylinder(&analysis, &options, &selected[..1]).unwrap();
    let last =
        recognize_guided_cylinder(&analysis, &options, &selected[selected.len() - 1..]).unwrap();
    assert_eq!(wall(&first, 0), selected);
    assert_eq!(wall(&first, 0), wall(&last, 0));
    assert_eq!(
        first.report.expected_volume_mm3,
        last.report.expected_volume_mm3
    );
    let mut reversed = selected.clone();
    reversed.reverse();
    let full = recognize_guided_cylinder(&analysis, &options, &reversed).unwrap();
    assert_eq!(wall(&first, 0), wall(&full, 0));
}

#[test]
fn invalid_ids_planes_and_several_walls_are_rejected() {
    let (analysis, automatic) = automatic(MIXED);
    let options = Options::default();
    let first = wall(&automatic, 0)[0];
    let second = wall(&automatic, 1)[0];
    let plane = automatic
        .triangle_faces
        .iter()
        .position(|owner| matches!(owner, MixedFaceRef::Plane { .. }))
        .unwrap();
    for selection in [
        vec![],
        vec![first, first],
        vec![analysis.mesh.triangles.len()],
    ] {
        assert_eq!(
            recognize_guided_cylinder(&analysis, &options, &selection)
                .unwrap_err()
                .code,
            "guidance.invalid_selection"
        );
    }
    for selection in [vec![plane], vec![first, plane], vec![first, second]] {
        assert_eq!(
            recognize_guided_cylinder(&analysis, &options, &selection)
                .unwrap_err()
                .code,
            "guidance.cylinder_unproven"
        );
    }
}

#[test]
fn false_or_incomplete_cylinders_are_not_forced() {
    for source in [
        include_bytes!("../../../fixtures/m2b_conical_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_nonflat_floor.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_bulged_boss.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_low_sample_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc.stl").as_slice(),
    ] {
        let options = Options::default();
        let analysis = analyze_stl(source, &options).unwrap();
        assert_eq!(
            recognize_guided_cylinder(&analysis, &options, &[0])
                .unwrap_err()
                .code,
            "guidance.cylinder_unproven"
        );
    }
}

#[test]
fn selected_seed_does_not_bypass_fit_or_deviation_limits() {
    let (analysis, automatic) = automatic(MIXED);
    let seed = wall(&automatic, 0)[0];
    let options = Options {
        max_deviation_mm: 0.001,
        ..Options::default()
    };
    assert_eq!(
        recognize_guided_cylinder(&analysis, &options, &[seed])
            .unwrap_err()
            .code,
        "recognition.deviation_exceeded"
    );
    let options = Options {
        cylinder_fit_tolerance_mm: f64::NAN,
        ..Options::default()
    };
    assert_eq!(
        recognize_guided_cylinder(&analysis, &options, &[seed])
            .unwrap_err()
            .code,
        "options.invalid_tolerance"
    );
}

#[test]
fn retained_facets_can_regroup_but_their_residual_is_bounded() {
    let (mut original, automatic) = automatic(include_bytes!("../../../fixtures/m2b_blind.stl"));
    let seed = wall(&automatic, 0)[0];
    let corner = original
        .mesh
        .vertices
        .iter_mut()
        .find(|p| **p == [15., -10., 10.])
        .unwrap();
    corner[2] += 5e-7;
    let mut stl = String::from("solid perturbed\n");
    for triangle in &original.mesh.triangles {
        stl.push_str("facet normal 0 0 0\nouter loop\n");
        for &vertex in triangle {
            let [x, y, z] = original.mesh.vertices[vertex];
            stl.push_str(&format!("vertex {x} {y} {z}\n"));
        }
        stl.push_str("endloop\nendfacet\n");
    }
    stl.push_str("endsolid perturbed\n");
    let options = Options {
        plane_tolerance_mm: 1e-8,
        ..Options::default()
    };
    let analysis = analyze_stl(stl.as_bytes(), &options).unwrap();
    let result = recognize_guided_cylinder(&analysis, &options, &[seed]).unwrap();
    let mut maximum: f64 = 0.;
    for (triangle, owner) in result.triangle_faces.iter().enumerate() {
        let MixedFaceRef::Plane { index } = owner else {
            continue;
        };
        let plane = &result.model.planes[*index];
        let origin = match &plane.loops[0] {
            MixedLoop::Polygon { vertices } => result.model.vertices[vertices[0]],
            MixedLoop::Circle { ring } => result.model.rings[*ring].center,
            MixedLoop::Segments { .. } => panic!("No open arcs in guided recognition"),
        };
        let length = plane.normal.into_iter().map(|v| v * v).sum::<f64>().sqrt();
        for &vertex in &analysis.mesh.triangles[triangle] {
            let position = analysis.mesh.vertices[vertex];
            let distance = (0..3)
                .map(|axis| (position[axis] - origin[axis]) * plane.normal[axis] / length)
                .sum::<f64>()
                .abs();
            maximum = maximum.max(distance);
        }
    }
    assert!(maximum > options.plane_tolerance_mm);
    assert!(result.report.max_wall_residual_mm >= maximum);
    assert!(result.report.max_deviation_bound_mm >= maximum);
}

#[test]
fn several_indications_share_one_model_and_refuse_duplicate_walls_and_limit() {
    use fabstep_core::recognize_guided_cylinders;
    let (analysis, automatic) = automatic(MIXED);
    let first = wall(&automatic, 0);
    let second = wall(&automatic, 1);
    let seeds = vec![vec![first[0]], vec![second[0]]];
    let result = recognize_guided_cylinders(&analysis, &Options::default(), &seeds).unwrap();
    assert_eq!(result.model.cylinders.len(), 2);
    assert_eq!(result.model.planes.len(), 8);
    assert_eq!(wall(&result, 0), wall(&automatic, 0));
    assert_eq!(wall(&result, 1), wall(&automatic, 1));
    for duplicate in [vec![first[0]], vec![first[1]]] {
        assert_eq!(
            recognize_guided_cylinders(
                &analysis,
                &Options::default(),
                &[vec![first[0]], duplicate]
            )
            .unwrap_err()
            .code,
            "guidance.overlapping_indications"
        );
    }
    assert_eq!(
        recognize_guided_cylinders(&analysis, &Options::default(), &vec![vec![first[0]]; 17])
            .unwrap_err()
            .code,
        "guidance.limit"
    );
}
