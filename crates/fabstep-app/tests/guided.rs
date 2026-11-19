use fabstep_app::{
    convert_document_with_provenance, convert_shared_document_with_provenance, DocumentStep,
    InputFormat,
};
use fabstep_core::{analyze_stl, recognize_mixed, MixedFaceRef, Options, ReconstructionMode};
use std::sync::Arc;

const MIXED: &[u8] = include_bytes!("../../../fixtures/m2b_boss_blind.stl");

fn wall_seed(step: &DocumentStep, source: &[u8], cylinder: usize, body: &str) -> usize {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let analysis = analyze_stl(source, &options).unwrap();
    let mixed = recognize_mixed(&analysis, &options).unwrap();
    let triangle = mixed
        .triangle_faces
        .iter()
        .position(|face| matches!(face, MixedFaceRef::Cylinder { index } if *index == cylinder))
        .unwrap();
    step.result_faces()
        .unwrap()
        .source_correspondence
        .faces
        .iter()
        .find(|face| face.analysis_body_id == body && face.source_triangle_ids.contains(&triangle))
        .unwrap()
        .face_id
}

#[test]
fn native_guidance_reconstructs_only_selected_wall_and_audits_the_entire_body() {
    let mut bytes = MIXED.to_vec();
    let mut options = Options {
        cylinder_fit_tolerance_mm: 0.0001,
        ..Options::default()
    };
    let planar = convert_document_with_provenance(&bytes, InputFormat::Stl, &options).unwrap();
    assert!(planar.report.guidance.is_none());
    assert!(serde_json::to_value(&planar.report)
        .unwrap()
        .get("guidance")
        .is_none());
    let seed = wall_seed(&planar.step, MIXED, 0, "stl/component/0");
    // Subsequent caller changes cannot replace the bound source or settings.
    bytes.fill(0);
    options.max_deviation_mm = 0.00001;
    let guided = planar.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(guided.report.cad.solids, 1);
    assert_eq!(guided.report.cad.cylindrical_faces, 1);
    assert!(guided.report.cad.planar_faces > 8);
    assert_eq!(
        guided.report.settings.reconstruction,
        ReconstructionMode::Planar
    );
    assert_eq!(guided.report.settings.max_deviation_mm, 0.05);
    assert_eq!(guided.report.settings.cylinder_fit_tolerance_mm, 0.0001);
    assert!(guided
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "precision.custom_tolerances"));
    let hint = guided.report.guidance.as_ref().unwrap();
    assert_eq!(hint.selected_face_ids, vec![seed]);
    assert_eq!(hint.analysis_body_id, "stl/component/0");
    assert!(hint.reconstructed_source_triangle_ids.len() > hint.selected_source_triangle_ids.len());
    assert!(hint
        .selected_source_triangle_ids
        .iter()
        .all(|t| hint.reconstructed_source_triangle_ids.contains(t)));
    let conversion = &guided.report.bodies[0].conversion;
    assert!(conversion.fidelity.as_ref().unwrap().source_samples > 0);
    let mixed = conversion.mixed_recognition.as_ref().unwrap();
    assert_eq!(mixed.model.cylinders.len(), 1);
    assert!((guided.report.cad.volume_mm3 - mixed.report.expected_volume_mm3).abs() < 1e-6);
    assert_eq!(
        guided
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    let duplicate = wall_seed(&guided.step, MIXED, 0, "stl/component/0");
    assert_eq!(
        guided
            .step
            .guided_cylinder(&[duplicate])
            .err()
            .unwrap()
            .code,
        "guidance.overlapping_indications"
    );
}

#[test]
fn native_faces_are_checked_and_cannot_force_a_cap_or_multiple_walls() {
    let planar =
        convert_document_with_provenance(MIXED, InputFormat::Stl, &Options::default()).unwrap();
    let a = wall_seed(&planar.step, MIXED, 0, "stl/component/0");
    let b = wall_seed(&planar.step, MIXED, 1, "stl/component/0");
    for ids in [vec![], vec![a, a], vec![usize::MAX]] {
        assert_eq!(
            planar.step.guided_cylinder(&ids).err().unwrap().code,
            "guidance.invalid_selection"
        );
    }
    assert_eq!(
        planar.step.guided_cylinder(&[a, b]).err().unwrap().code,
        "guidance.cylinder_unproven"
    );
    let cube = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        cube.step.guided_cylinder(&[0]).err().unwrap().code,
        "guidance.cylinder_unproven"
    );
    let untracked = DocumentStep::untracked(planar.step.bytes().to_vec());
    assert_eq!(
        untracked.guided_cylinder(&[a]).err().unwrap().code,
        "guidance.provenance_unavailable"
    );
}

#[test]
fn shared_source_binding_requires_the_same_allocation_and_format() {
    let bytes = Arc::new(MIXED.to_vec());
    let planar = convert_shared_document_with_provenance(
        Arc::clone(&bytes),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    assert!(planar.step.is_guidance_source(&bytes, InputFormat::Stl));
    assert!(!planar
        .step
        .is_guidance_source(&Arc::new(MIXED.to_vec()), InputFormat::Stl));
    assert!(!planar.step.is_guidance_source(&bytes, InputFormat::ThreeMf));
}

#[test]
fn guided_document_preserves_separated_bodies_and_rejects_cross_body_selection() {
    let mesh = fabstep_core::read_stl_mesh(MIXED, fabstep_core::InputUnit::Mm).unwrap();
    let mut source = String::from("solid pair\n");
    for offset in [0.0, 100.0] {
        for triangle in &mesh.triangles {
            source.push_str("facet normal 0 0 0\nouter loop\n");
            for &vertex in triangle {
                let [x, y, z] = mesh.vertices[vertex];
                source.push_str(&format!("vertex {} {y} {z}\n", x + offset));
            }
            source.push_str("endloop\nendfacet\n");
        }
    }
    source.push_str("endsolid pair\n");
    let planar =
        convert_document_with_provenance(source.as_bytes(), InputFormat::Stl, &Options::default())
            .unwrap();
    let seed = wall_seed(&planar.step, MIXED, 0, "stl/component/0");
    let guided = planar.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(guided.report.cad.solids, 2);
    assert_eq!(guided.report.cad.cylindrical_faces, 1);
    assert_eq!(guided.report.bodies[1].conversion.cad.cylindrical_faces, 0);
    assert_eq!(
        guided.report.bodies[1].conversion.cad.volume_mm3,
        planar.report.bodies[1].conversion.cad.volume_mm3
    );
    assert_eq!(guided.report.cad.bounds_mm, planar.report.cad.bounds_mm);
    assert!(guided
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "document.separate_bodies"));
    let second = planar
        .step
        .result_faces()
        .unwrap()
        .source_correspondence
        .faces
        .into_iter()
        .find(|face| face.analysis_body_id == "stl/component/1")
        .unwrap()
        .face_id;
    assert_eq!(
        planar
            .step
            .guided_cylinder(&[seed, second])
            .err()
            .unwrap()
            .code,
        "guidance.multiple_bodies"
    );
}

#[test]
fn guided_mirrored_3mf_instance_keeps_placements_and_other_instance_faceted() {
    let bytes = include_bytes!("../../../fixtures/guided_cylinder_instances.3mf");
    let options = Options::default();
    let planar = convert_document_with_provenance(bytes, InputFormat::ThreeMf, &options).unwrap();
    let document = fabstep_app::analyze_document(bytes, InputFormat::ThreeMf, &options).unwrap();
    let topology = fabstep_app::selection_topology(bytes, InputFormat::ThreeMf, &options).unwrap();
    let mirrored = &document.bodies[1];
    assert_eq!(mirrored.provenance.build_item, Some(1));
    let recognized = recognize_mixed(
        &mirrored.analysis,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..options.clone()
        },
    )
    .unwrap();
    let triangle = recognized
        .triangle_faces
        .iter()
        .position(|owner| matches!(owner, MixedFaceRef::Cylinder { index: 0 }))
        .unwrap();
    let component = topology
        .components
        .iter()
        .find(|component| component.analysis_body_id == mirrored.id)
        .unwrap();
    let source_triangle = component.analysis_to_source_triangles[triangle];
    let inspected = planar.step.result_faces().unwrap();
    let face = inspected
        .source_correspondence
        .faces
        .iter()
        .find(|face| {
            face.analysis_body_id == mirrored.id
                && face.source_triangle_ids.contains(&source_triangle)
        })
        .unwrap();
    let guided = planar.step.guided_cylinder(&[face.face_id]).unwrap();
    let hint = guided.report.guidance.as_ref().unwrap();
    assert_eq!(hint.preview_body_id, component.preview_body_id);
    assert_eq!(hint.analysis_body_id, mirrored.id);
    assert!(hint.selected_source_triangle_ids.contains(&source_triangle));
    assert!(hint
        .reconstructed_source_triangle_ids
        .contains(&source_triangle));
    assert_eq!(guided.report.cad.solids, 2);
    assert_eq!(guided.report.cad.cylindrical_faces, 1);
    assert_eq!(guided.report.bodies[0].conversion.cad.cylindrical_faces, 0);
    assert_eq!(
        guided.report.bodies[0].conversion.cad.faces,
        planar.report.bodies[0].conversion.cad.faces
    );
    assert_eq!(
        guided.report.bodies[0].conversion.cad.volume_mm3,
        planar.report.bodies[0].conversion.cad.volume_mm3
    );
    assert_eq!(guided.report.bodies[1].conversion.cad.cylindrical_faces, 1);
    assert_eq!(guided.report.bodies[1].provenance.build_item, Some(1));
    assert!(guided.report.bodies[1].conversion.fidelity.is_some());
    for (actual, expected) in guided
        .report
        .cad
        .bounds_mm
        .iter()
        .flatten()
        .zip(planar.report.cad.bounds_mm.iter().flatten())
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert!(guided.report.bodies[0].conversion.cad.bounds_mm[1][0] < 30.0);
    assert!(guided.report.bodies[1].conversion.cad.bounds_mm[0][0] > 70.0);
    // Continue from a guided multi-root STEP into a different source instance.
    let other = &document.bodies[0];
    let recognized = recognize_mixed(
        &other.analysis,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..options.clone()
        },
    )
    .unwrap();
    let triangle = recognized
        .triangle_faces
        .iter()
        .position(|owner| matches!(owner, MixedFaceRef::Cylinder { index: 0 }))
        .unwrap();
    let component = topology
        .components
        .iter()
        .find(|component| component.analysis_body_id == other.id)
        .unwrap();
    let source_triangle = component.analysis_to_source_triangles[triangle];
    let inspected = guided.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let seed = inspected
        .source_correspondence
        .faces
        .iter()
        .find(|face| {
            face.analysis_body_id == other.id && face.source_triangle_ids.contains(&source_triangle)
        })
        .unwrap()
        .face_id;
    let second = guided.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(second.report.cad.solids, 2);
    assert_eq!(second.report.cad.cylindrical_faces, 2);
    assert!(second
        .report
        .bodies
        .iter()
        .all(|body| body.conversion.cad.cylindrical_faces == 1));
    assert_eq!(
        second
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    let removed = second.step.remove_guidance(1).unwrap();
    assert_eq!(removed.report.bodies[0].conversion.cad.cylindrical_faces, 1);
    assert_eq!(removed.report.bodies[1].conversion.cad.cylindrical_faces, 0);
    assert_eq!(removed.report.guidance_hints[0].hint_id, 2);
}

#[test]
fn successive_native_indications_keep_prior_walls_and_remove_one_by_stable_id() {
    let bytes = Arc::new(MIXED.to_vec());
    let planar = convert_shared_document_with_provenance(
        Arc::clone(&bytes),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    let seed = wall_seed(&planar.step, MIXED, 0, "stl/component/0");
    let first = planar.step.guided_cylinder(&[seed]).unwrap();
    let seed = wall_seed(&first.step, MIXED, 1, "stl/component/0");
    let second = first.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(second.report.cad.cylindrical_faces, 2);
    assert_eq!(second.report.cad.planar_faces, 8);
    assert_eq!(second.report.guidance_hints.len(), 2);
    assert_eq!(
        second
            .report
            .guidance_hints
            .iter()
            .map(|h| h.hint_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(second.step.is_guidance_source(&bytes, InputFormat::Stl));
    let inspected = second.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let all = inspected
        .source_correspondence
        .faces
        .iter()
        .flat_map(|f| &f.source_triangle_ids)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        all.len(),
        second.report.bodies[0]
            .conversion
            .analysis
            .mesh
            .triangles
            .len()
    );
    let source_count = second.report.bodies[0]
        .conversion
        .analysis
        .mesh
        .triangles
        .len();
    assert_eq!(
        inspected
            .source_correspondence
            .faces
            .iter()
            .map(|face| face.source_triangle_ids.len())
            .sum::<usize>(),
        source_count
    );
    let owners = &second.report.bodies[0]
        .conversion
        .mixed_recognition
        .as_ref()
        .unwrap()
        .triangle_faces;
    for face in &inspected.source_correspondence.faces {
        let native = &inspected.result.faces[face.face_id];
        for &triangle in &face.source_triangle_ids {
            match owners[triangle] {
                MixedFaceRef::Plane { .. } => assert!(matches!(
                    native.family,
                    fabstep_cad::ResultFaceFamily::Plane
                )),
                MixedFaceRef::Cylinder { .. } => assert!(matches!(
                    native.family,
                    fabstep_cad::ResultFaceFamily::Cylinder
                )),
                _ => panic!("No partial cylinder in guided model"),
            }
        }
    }
    let removed = second.step.remove_guidance(1).unwrap();
    assert_eq!(removed.report.cad.cylindrical_faces, 1);
    assert_eq!(removed.report.guidance_hints[0].hint_id, 2);
    assert_eq!(removed.report.guidance.as_ref().unwrap().hint_id, 2);
    let seed = wall_seed(&removed.step, MIXED, 0, "stl/component/0");
    let third = removed.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(
        third
            .report
            .guidance_hints
            .iter()
            .map(|h| h.hint_id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    let recomputed = third.step.recompute_guidance().unwrap();
    assert_eq!(recomputed.report.cad.cylindrical_faces, 2);
    assert_eq!(
        third.step.remove_guidance(99).err().unwrap().code,
        "guidance.unknown_indication"
    );
    let empty = removed.step.remove_guidance(2).unwrap();
    assert!(empty.report.guidance_hints.is_empty());
    assert!(empty.report.guidance.is_none());
    assert_eq!(empty.report.cad.cylindrical_faces, 0);
    assert_eq!(empty.report.cad.faces, planar.report.cad.faces);
}

#[test]
fn editing_and_restoring_hints_reprove_the_whole_step_from_the_same_source_region() {
    use fabstep_core::GuidanceFamily;
    let base =
        convert_document_with_provenance(MIXED, InputFormat::Stl, &Options::default()).unwrap();
    let seed = wall_seed(&base.step, MIXED, 0, "stl/component/0");
    let add = base.step.guided_cylinder(&[seed]).unwrap();
    let hint = add.report.guidance.as_ref().unwrap();
    let mut check = |_| Ok(());
    let edit = add
        .step
        .edit_guidance_controlled(hint.hint_id, GuidanceFamily::Auto, &mut check)
        .unwrap();
    let edited = edit.report.guidance.as_ref().unwrap();
    assert_eq!((edited.hint_id, edited.family), (hint.hint_id, "auto"));
    assert_eq!(
        edited.selected_source_triangle_ids,
        hint.selected_source_triangle_ids
    );
    assert_eq!(edited.selected_face_ids, hint.selected_face_ids);
    assert_eq!(edit.report.cad.solids, 1);
    let denied = edit
        .step
        .edit_guidance_controlled(hint.hint_id, GuidanceFamily::Cone, &mut check)
        .err()
        .unwrap();
    assert!(denied.code.starts_with("guidance."), "{denied:?}");
    let mut phases = Vec::new();
    let undo = edit
        .step
        .undo_guidance_controlled(&mut |cp| {
            phases.push(cp.phase);
            Ok(())
        })
        .unwrap();
    for required in [
        fabstep_app::ConversionPhase::Analysis,
        fabstep_app::ConversionPhase::Build,
        fabstep_app::ConversionPhase::AuditComparison,
        fabstep_app::ConversionPhase::Finish,
    ] {
        assert!(
            phases.contains(&required),
            "missing validation on undo: {required:?}"
        );
    }
    assert_eq!(undo.report.guidance.as_ref().unwrap().family, "cylinder");
    assert_eq!(
        undo.report.cad.cylindrical_faces,
        add.report.cad.cylindrical_faces
    );
    assert_eq!(
        undo.step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    let replay = undo.step.redo_guidance_controlled(&mut check).unwrap();
    assert_eq!(replay.report.guidance.as_ref().unwrap().family, "auto");
    let remove = undo.step.remove_guidance(hint.hint_id).unwrap();
    assert!(remove.report.guidance_hints.is_empty());
    assert_eq!(remove.report.cad.faces, base.report.cad.faces);
    assert_eq!(
        remove
            .step
            .redo_guidance_controlled(&mut check)
            .err()
            .unwrap()
            .code,
        "guidance.history_unavailable"
    );
    let restored = remove.step.undo_guidance_controlled(&mut check).unwrap();
    assert_eq!(
        restored.report.guidance.as_ref().unwrap().family,
        "cylinder"
    );
    let empty = restored.step.undo_guidance_controlled(&mut check).unwrap();
    assert!(empty.report.guidance_hints.is_empty());
    let history = empty.report.guidance_history.unwrap();
    assert!(!history.undo && history.redo);
    let seed = wall_seed(&empty.step, MIXED, 0, "stl/component/0");
    let fresh = empty.step.guided_cylinder(&[seed]).unwrap();
    assert!(fresh.report.guidance.as_ref().unwrap().hint_id > hint.hint_id);
    assert!(!fresh.report.guidance_history.unwrap().redo);
    assert_eq!(
        base.step
            .undo_guidance_controlled(&mut check)
            .err()
            .unwrap()
            .code,
        "guidance.history_unavailable"
    );
}
