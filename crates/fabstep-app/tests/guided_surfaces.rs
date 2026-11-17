use fabstep_app::{convert_document_with_provenance, DocumentStep, InputFormat};
use fabstep_core::{GuidanceFamily, Options, ReconstructionMode, RevolvedFaceRef};

const FRUSTUM: &[u8] = include_bytes!("../../../fixtures/m4_frustum.stl");

fn cone_seed(step: &DocumentStep, bytes: &[u8], body_index: usize) -> usize {
    cone_seed_format(step, bytes, body_index, InputFormat::Stl)
}

fn cone_seed_format(
    step: &DocumentStep,
    bytes: &[u8],
    body_index: usize,
    format: InputFormat,
) -> usize {
    let options = Options::default();
    let document = fabstep_app::analyze_document(bytes, format, &options).unwrap();
    let topology = fabstep_app::selection_topology(bytes, format, &options).unwrap();
    let body = &document.bodies[body_index];
    let recognition = fabstep_core::recognize_revolved(
        &body.analysis,
        &Options {
            reconstruction: ReconstructionMode::Primitives,
            ..options
        },
    )
    .unwrap();
    let triangle = recognition
        .triangle_faces
        .iter()
        .position(|face| matches!(face, RevolvedFaceRef::Cone { .. }))
        .unwrap();
    let component = topology
        .components
        .iter()
        .find(|c| c.analysis_body_id == body.id)
        .unwrap();
    let triangle = component.analysis_to_source_triangles[triangle];
    step.result_faces()
        .unwrap()
        .source_correspondence
        .faces
        .iter()
        .find(|face| {
            face.analysis_body_id == body.id && face.source_triangle_ids.contains(&triangle)
        })
        .unwrap()
        .face_id
}

#[test]
fn cone_hypothesis_tracks_the_full_wall_and_can_be_removed() {
    for bytes in [
        FRUSTUM,
        include_bytes!("../../../fixtures/m4_frustum_rotated.stl").as_slice(),
    ] {
        let base =
            convert_document_with_provenance(bytes, InputFormat::Stl, &Options::default()).unwrap();
        let seed = cone_seed(&base.step, bytes, 0);
        assert_eq!(
            base.step.guided_cylinder(&[seed]).err().unwrap().code,
            "guidance.cylinder_unproven"
        );
        let guided = base
            .step
            .guided_surface(&[seed], GuidanceFamily::Cone)
            .unwrap();
        assert_eq!(guided.report.cad.conical_faces, 1);
        assert_eq!(guided.report.cad.cylindrical_faces, 0);
        assert_eq!(
            guided.report.settings.reconstruction,
            ReconstructionMode::Planar
        );
        assert!(
            guided.report.bodies[0]
                .conversion
                .fidelity
                .as_ref()
                .unwrap()
                .source_samples
                > 0
        );
        let hint = &guided.report.guidance_hints[0];
        assert_eq!(hint.family, "cone");
        assert!(
            hint.reconstructed_source_triangle_ids.len() > hint.selected_source_triangle_ids.len()
        );
        assert!(hint
            .selected_source_triangle_ids
            .iter()
            .all(|t| hint.reconstructed_source_triangle_ids.contains(t)));
        let faces = guided.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let recognition = guided.report.bodies[0]
            .conversion
            .revolved_recognition
            .as_ref()
            .unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for face in &faces.source_correspondence.faces {
            for &triangle in &face.source_triangle_ids {
                assert!(seen.insert(triangle));
                let expected = match recognition.triangle_faces[triangle] {
                    RevolvedFaceRef::Plane { .. } => fabstep_cad::ResultFaceFamily::Plane,
                    RevolvedFaceRef::Cylinder { .. } => fabstep_cad::ResultFaceFamily::Cylinder,
                    RevolvedFaceRef::Cone { .. } => fabstep_cad::ResultFaceFamily::Cone,
                };
                assert_eq!(faces.result.faces[face.face_id].family, expected);
            }
        }
        assert_eq!(seen.len(), recognition.triangle_faces.len());
        let recalculated = guided.step.recompute_guidance().unwrap();
        assert_eq!(recalculated.report.cad.conical_faces, 1);
        let removed = recalculated.step.remove_guidance(hint.hint_id).unwrap();
        assert!(removed.report.guidance_hints.is_empty());
        assert_eq!(removed.report.cad.faces, base.report.cad.faces);
        assert_eq!(removed.report.cad.conical_faces, 0);
    }
}

#[test]
fn wrong_family_cap_and_unsupported_curved_boundary_do_not_force_a_cone() {
    let cube = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        cube.step
            .guided_surface(&[0], GuidanceFamily::Cone)
            .err()
            .unwrap()
            .code,
        "guidance.cone_unproven"
    );
    for bytes in [
        include_bytes!("../../../fixtures/m4_chamfered_cylinder.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_cone_tip.stl").as_slice(),
    ] {
        let base =
            convert_document_with_provenance(bytes, InputFormat::Stl, &Options::default()).unwrap();
        let seed = cone_seed(&base.step, bytes, 0);
        assert!(base
            .step
            .guided_surface(&[seed], GuidanceFamily::Cone)
            .is_err());
        let recovered = base.step.recompute_guidance().unwrap();
        assert!(recovered.report.guidance_hints.is_empty());
        assert_eq!(recovered.report.cad.faces, base.report.cad.faces);
    }
}

#[test]
fn cone_cancellation_never_returns_a_result() {
    let base =
        convert_document_with_provenance(FRUSTUM, InputFormat::Stl, &Options::default()).unwrap();
    let seed = cone_seed(&base.step, FRUSTUM, 0);
    let error = base
        .step
        .guided_surface_controlled(&[seed], GuidanceFamily::Cone, &mut |checkpoint| {
            if checkpoint.phase == fabstep_app::ConversionPhase::Build {
                Err(fabstep_core::FabError::new("test.cancelled", "cancelled"))
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
    assert_eq!(error.code, "test.cancelled");
}

#[test]
fn placed_3mf_cones_keep_whole_document_provenance_and_independent_hints() {
    let bytes = include_bytes!("../../../fixtures/m4_frustum_instances.3mf");
    let base =
        convert_document_with_provenance(bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    assert_eq!(base.report.cad.solids, 2);
    let seed = cone_seed_format(&base.step, bytes, 1, InputFormat::ThreeMf);
    let first = base
        .step
        .guided_surface(&[seed], GuidanceFamily::Cone)
        .unwrap();
    assert_eq!(first.report.cad.conical_faces, 1);
    assert_eq!(
        first.report.bodies[0].conversion.cad.faces,
        base.report.bodies[0].conversion.cad.faces
    );
    assert_eq!(
        first.report.bodies[0].conversion.cad.volume_mm3,
        base.report.bodies[0].conversion.cad.volume_mm3
    );
    assert_eq!(first.report.bodies[1].conversion.cad.conical_faces, 1);
    assert_eq!(
        first
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    let seed = cone_seed_format(&first.step, bytes, 0, InputFormat::ThreeMf);
    let second = first
        .step
        .guided_surface(&[seed], GuidanceFamily::Cone)
        .unwrap();
    assert_eq!(second.report.cad.conical_faces, 2);
    assert_eq!(
        second
            .report
            .guidance_hints
            .iter()
            .map(|h| h.hint_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
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
    assert_eq!(removed.report.bodies[0].conversion.cad.conical_faces, 1);
    assert_eq!(removed.report.bodies[1].conversion.cad.conical_faces, 0);
    assert_eq!(removed.report.guidance_hints[0].hint_id, 2);
}

fn tapered_boss_source() -> Vec<u8> {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let analysis = fabstep_core::analyze_stl(
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        &options,
    )
    .unwrap();
    let recognized = fabstep_core::recognize_mixed(&analysis, &options).unwrap();
    let mut mesh = analysis.mesh;
    let boss = recognized
        .model
        .cylinders
        .iter()
        .find(|c| !c.inner)
        .unwrap();
    let ring = boss
        .rings
        .iter()
        .map(|&i| &recognized.model.rings[i])
        .max_by(|a, b| a.center[2].total_cmp(&b.center[2]))
        .unwrap();
    let low_z = boss
        .rings
        .iter()
        .map(|&i| recognized.model.rings[i].center[2])
        .fold(f64::INFINITY, f64::min);
    for p in &mut mesh.vertices {
        if p[2] >= low_z - 1e-8
            && p[2] <= ring.center[2] + 1e-8
            && ((p[0] - ring.center[0]).hypot(p[1] - ring.center[1]) - ring.radius_mm).abs() < 1e-8
        {
            let ratio = 1. - 0.3 * (p[2] - low_z) / (ring.center[2] - low_z);
            p[0] = ring.center[0] + ratio * (p[0] - ring.center[0]);
            p[1] = ring.center[1] + ratio * (p[1] - ring.center[1]);
        }
    }
    let mut source = String::from("solid tapered\n");
    for triangle in mesh.triangles {
        source.push_str("facet normal 0 0 0\nouter loop\n");
        for vertex in triangle {
            let [x, y, z] = mesh.vertices[vertex];
            source.push_str(&format!("vertex {x} {y} {z}\n"));
        }
        source.push_str("endloop\nendfacet\n");
    }
    source.push_str("endsolid tapered\n");
    source.into_bytes()
}

#[test]
fn same_body_families_work_in_both_orders_and_remove_independently() {
    let bytes = tapered_boss_source();
    let options = Options::default();
    let analysis = fabstep_core::analyze_stl(&bytes, &options).unwrap();
    let all = fabstep_core::recognize_revolved(
        &analysis,
        &Options {
            reconstruction: ReconstructionMode::Primitives,
            ..options.clone()
        },
    )
    .unwrap();
    let seed = |step: &DocumentStep, family: GuidanceFamily| {
        let triangle = all
            .triangle_faces
            .iter()
            .position(|owner| {
                matches!(
                    (family, owner),
                    (GuidanceFamily::Cone, RevolvedFaceRef::Cone { .. })
                        | (GuidanceFamily::Cylinder, RevolvedFaceRef::Cylinder { .. })
                )
            })
            .unwrap();
        step.result_faces()
            .unwrap()
            .source_correspondence
            .faces
            .iter()
            .find(|face| face.source_triangle_ids.contains(&triangle))
            .unwrap()
            .face_id
    };
    for order in [
        [GuidanceFamily::Cone, GuidanceFamily::Cylinder],
        [GuidanceFamily::Cylinder, GuidanceFamily::Cone],
    ] {
        let base = convert_document_with_provenance(&bytes, InputFormat::Stl, &options).unwrap();
        let first = base
            .step
            .guided_surface(&[seed(&base.step, order[0])], order[0])
            .unwrap();
        assert_eq!(
            first.report.cad.conical_faces,
            usize::from(order[0] == GuidanceFamily::Cone)
        );
        assert_eq!(
            first.report.cad.cylindrical_faces,
            usize::from(order[0] == GuidanceFamily::Cylinder)
        );
        assert!(first.report.cad.planar_faces > all.model.mixed.planes.len());
        let second = first
            .step
            .guided_surface(&[seed(&first.step, order[1])], order[1])
            .unwrap();
        assert_eq!(second.report.cad.conical_faces, 1);
        assert_eq!(second.report.cad.cylindrical_faces, 1);
        assert!((second.report.cad.volume_mm3 - all.report.expected_volume_mm3).abs() < 1e-6);
        assert_eq!(
            second
                .step
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
        for hint_id in [1, 2] {
            let removed = second.step.remove_guidance(hint_id).unwrap();
            let remaining_family = order[if hint_id == 1 { 1 } else { 0 }];
            assert_eq!(
                removed.report.cad.conical_faces,
                usize::from(remaining_family == GuidanceFamily::Cone)
            );
            assert_eq!(
                removed.report.cad.cylindrical_faces,
                usize::from(remaining_family == GuidanceFamily::Cylinder)
            );
            assert_eq!(removed.report.guidance_hints.len(), 1);
            assert_eq!(removed.report.guidance_hints[0].hint_id, 3 - hint_id);
            assert_eq!(
                removed
                    .step
                    .result_faces()
                    .unwrap()
                    .source_correspondence
                    .status,
                "exact"
            );
        }
    }
}
