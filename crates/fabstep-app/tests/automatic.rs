use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{GuidanceFamily, Options, ReconstructionMode};
fn auto() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn automatic_keeps_exact_provenance_for_planes_cylinders_and_rounded_surfaces() {
    for bytes in [
        include_bytes!("../../../fixtures/cube.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
        include_bytes!("../../../fixtures/m5d_ring_torus.stl"),
    ] {
        let converted = convert_document_with_provenance(bytes, InputFormat::Stl, &auto()).unwrap();
        assert_eq!(converted.report.cad.solids, 1);
        let inspected = converted.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact");
        assert_eq!(
            inspected.source_correspondence.faces.len(),
            inspected.result.faces.len()
        );
        let mut triangles = std::collections::BTreeSet::new();
        for row in &inspected.source_correspondence.faces {
            for t in &row.source_triangle_ids {
                assert!(triangles.insert(*t));
            }
        }
        assert_eq!(
            triangles.len(),
            converted.report.bodies[0]
                .conversion
                .analysis
                .mesh
                .triangles
                .len()
        );
        assert!(converted.report.bodies[0].conversion.fidelity.is_some());
        let replay = converted.step.recompute_guidance().unwrap();
        assert_eq!(replay.report.cad.faces, converted.report.cad.faces);
    }
}

#[test]
fn automatic_local_correction_preserves_other_cylinders_and_removal_restores_baseline() {
    let bytes = include_bytes!("../../../fixtures/m2b_boss_blind.stl");
    let base = convert_document_with_provenance(bytes, InputFormat::Stl, &auto()).unwrap();
    assert_eq!(base.report.cad.cylindrical_faces, 2);
    let faces = base.step.result_faces().unwrap();
    let cylinder = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cylinder)
        .unwrap()
        .id;
    let guided = base
        .step
        .guided_surface(&[cylinder], GuidanceFamily::Cylinder)
        .unwrap();
    assert_eq!(guided.report.cad.cylindrical_faces, 2);
    assert_eq!(
        guided
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    let removed = guided.step.remove_guidance(1).unwrap();
    assert_eq!(removed.report.cad.cylindrical_faces, 2);
    assert!(removed.report.guidance_hints.is_empty());
}

#[test]
fn geometric_selection_is_read_only_bounded_and_cancellable() {
    let bytes = include_bytes!("../../../fixtures/m2b_boss_blind.stl");
    let base =
        convert_document_with_provenance(bytes, InputFormat::Stl, &Options::default()).unwrap();
    let analysis = fabstep_core::analyze_stl(bytes, &Options::default()).unwrap();
    let recognized = fabstep_core::recognize_mixed(
        &analysis,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        },
    )
    .unwrap();
    let wall: std::collections::BTreeSet<_> = recognized
        .triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            matches!(f, fabstep_core::MixedFaceRef::Cylinder { index: 0 }).then_some(i)
        })
        .collect();
    let inspected = base.step.result_faces().unwrap();
    let seed = inspected
        .source_correspondence
        .faces
        .iter()
        .find(|f| f.source_triangle_ids.iter().all(|t| wall.contains(t)))
        .unwrap()
        .face_id;
    let selected = base
        .step
        .suggest_region_controlled(seed, &mut |_| Ok(()))
        .unwrap();
    assert!(selected.len() > 1);
    assert!(selected.len() < inspected.result.faces.len());
    let selected_triangles: std::collections::BTreeSet<_> = selected
        .iter()
        .flat_map(|&id| {
            inspected.source_correspondence.faces[id]
                .source_triangle_ids
                .iter()
                .copied()
        })
        .collect();
    assert_eq!(selected_triangles, wall);
    assert!(base
        .step
        .suggest_region_controlled(usize::MAX, &mut |_| Ok(()))
        .is_err());
    let err = base
        .step
        .suggest_region_controlled(seed, &mut |_| {
            Err(fabstep_core::FabError::new("test.cancelled", "cancelled"))
        })
        .unwrap_err();
    assert_eq!(err.code, "test.cancelled");
    assert_eq!(
        base.step.result_faces().unwrap().result.faces.len(),
        inspected.result.faces.len()
    );
}

#[test]
fn torus_guidance_includes_support_and_survives_replay_in_planar_and_auto_sessions() {
    let bytes = include_bytes!("../../../fixtures/m5_toroidal_fillet.stl");
    for options in [Options::default(), auto()] {
        let base = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let analysis = fabstep_core::analyze_stl(bytes, &options).unwrap();
        let candidate = fabstep_core::recognize_automatic(&analysis, &auto()).unwrap();
        let seed = candidate
            .triangle_faces
            .iter()
            .position(|f| matches!(f, fabstep_core::RoundedFaceRef::Torus { .. }))
            .unwrap();
        let faces = base.step.result_faces().unwrap();
        let face = faces
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.source_triangle_ids.contains(&seed))
            .unwrap()
            .face_id;
        let group = base
            .step
            .suggest_region_controlled(face, &mut |_| Ok(()))
            .unwrap();
        let guided = base
            .step
            .guided_surface(&group, GuidanceFamily::Torus)
            .unwrap();
        assert_eq!(guided.report.cad.toroidal_faces, 1);
        assert_eq!(guided.report.cad.cylindrical_faces, 1);
        assert_eq!(guided.report.guidance_hints[0].family, "torus");
        assert_eq!(
            guided
                .step
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
        let replay = guided.step.recompute_guidance().unwrap();
        assert_eq!(replay.report.cad.toroidal_faces, 1);
        let removed = replay.step.remove_guidance(1).unwrap();
        assert_eq!(
            removed.report.cad.toroidal_faces,
            base.report.cad.toroidal_faces
        );
    }
}

#[test]
fn automatic_multibody_keeps_placements_and_native_identity_and_rejects_contacts() {
    let bytes = include_bytes!("../../../fixtures/m4_frustum_instances.3mf");
    let base = convert_document_with_provenance(bytes, InputFormat::ThreeMf, &auto()).unwrap();
    assert_eq!(base.report.cad.solids, 2);
    assert_eq!(base.report.cad.conical_faces, 2);
    let faces = base.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let bodies: std::collections::BTreeSet<_> = faces
        .source_correspondence
        .faces
        .iter()
        .map(|f| &f.analysis_body_id)
        .collect();
    assert_eq!(bodies.len(), 2);
    let cone = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cone)
        .unwrap()
        .id;
    let corrected = base
        .step
        .guided_surface(&[cone], GuidanceFamily::Cone)
        .unwrap();
    assert_eq!(corrected.report.cad.solids, 2);
    assert_eq!(corrected.report.cad.conical_faces, 2);
    assert_eq!(
        corrected
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    assert_eq!(corrected.report.cad.bounds_mm, base.report.cad.bounds_mm);
    assert!(convert_document_with_provenance(
        include_bytes!("../../../fixtures/m4_frustum_touching.3mf"),
        InputFormat::ThreeMf,
        &auto()
    )
    .is_err());
    assert!(convert_document_with_provenance(
        include_bytes!("../../../fixtures/m5c_open_sphere.stl"),
        InputFormat::Stl,
        &auto()
    )
    .is_err());
}
