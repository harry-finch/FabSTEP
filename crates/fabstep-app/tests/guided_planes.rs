use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{GuidanceFamily, Options};

// Thin prism with two distinct, almost coplanar roof faces. Their normals
// exceed automatic angular grouping; their shared vertices must not move.
fn roof_source(height: f64) -> Vec<u8> {
    let section = [
        [-0.001, 0.0],
        [0.001, 0.0],
        [0.001, 1.0],
        [0.0, 1.0 + height],
        [-0.001, 1.0],
    ];
    let mut vertices = Vec::new();
    for y in [-1.0, 1.0] {
        for [x, z] in section {
            vertices.push([x, y, z]);
        }
    }
    let mut triangles = Vec::new();
    for i in 1..4 {
        triangles.push([0, i, i + 1]);
        triangles.push([5, i + 6, i + 5]);
    }
    for i in 0..5 {
        let j = (i + 1) % 5;
        triangles.push([i, j + 5, j]);
        triangles.push([i, i + 5, j + 5]);
    }
    let mut text = String::from("solid roof\n");
    for t in triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for v in t {
            let [x, y, z] = vertices[v];
            text.push_str(&format!("vertex {x} {y} {z}\n"));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid roof\n");
    text.into_bytes()
}

#[test]
fn plane_merges_real_distinct_faces_and_retains_validated_session() {
    let bytes = roof_source(2e-8);
    let options = Options::default();
    let analysis = fabstep_core::analyze_stl(&bytes, &options).unwrap();
    let roof: std::collections::BTreeSet<_> = analysis
        .mesh
        .triangles
        .iter()
        .enumerate()
        .filter(|(_, t)| t.iter().all(|&v| analysis.mesh.vertices[v][2] >= 1.0))
        .map(|(i, _)| i)
        .collect();
    let base = convert_document_with_provenance(&bytes, InputFormat::Stl, &options).unwrap();
    let faces = base.step.result_faces().unwrap();
    let selected: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .filter(|f| f.source_triangle_ids.iter().all(|t| roof.contains(t)))
        .map(|f| f.face_id)
        .collect();
    assert_eq!(selected.len(), 2, "fixture must exercise an actual merge");
    let guided = base
        .step
        .guided_surface(&selected, GuidanceFamily::Plane)
        .unwrap();
    assert_eq!(guided.report.cad.solids, 1);
    assert_eq!(guided.report.cad.faces + 1, base.report.cad.faces);
    assert_eq!(guided.report.guidance_hints[0].family, "plane");
    assert_eq!(
        guided.report.guidance_hints[0].selected_source_triangle_ids,
        guided.report.guidance_hints[0].reconstructed_source_triangle_ids
    );
    assert!(guided.report.bodies[0].conversion.fidelity.is_some());
    let faces = guided.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let all: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .flat_map(|f| f.source_triangle_ids.iter().copied())
        .collect();
    assert_eq!(all.len(), analysis.mesh.triangles.len());
    assert_eq!(
        all.iter().collect::<std::collections::BTreeSet<_>>().len(),
        all.len()
    );
    let replay = guided.step.recompute_guidance().unwrap();
    assert_eq!(replay.report.cad.faces, guided.report.cad.faces);
    let removed = replay.step.remove_guidance(1).unwrap();
    assert_eq!(removed.report.cad.faces, base.report.cad.faces);
    assert!(removed.report.guidance_hints.is_empty());
}

#[test]
fn nonplanar_selection_is_refused_and_cancelled_plane_never_publishes() {
    let base = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        base.step
            .guided_surface(&[0, 1, 2], GuidanceFamily::Plane)
            .err()
            .unwrap()
            .code,
        "guidance.plane_unproven"
    );
    let error = base
        .step
        .guided_surface_controlled(&[0], GuidanceFamily::Plane, &mut |checkpoint| {
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
fn plane_and_cylinder_coexist_and_remove_independently() {
    let bytes = include_bytes!("../../../fixtures/m2b_boss_blind.stl");
    let options = Options::default();
    let analysis = fabstep_core::analyze_stl(bytes, &options).unwrap();
    let recognized = fabstep_core::recognize_mixed(
        &analysis,
        &Options {
            reconstruction: fabstep_core::ReconstructionMode::Cylinders,
            ..options.clone()
        },
    )
    .unwrap();
    let cylinder = recognized
        .triangle_faces
        .iter()
        .position(|f| matches!(f, fabstep_core::MixedFaceRef::Cylinder { .. }))
        .unwrap();
    let min_z = analysis
        .mesh
        .vertices
        .iter()
        .map(|v| v[2])
        .fold(f64::INFINITY, f64::min);
    let plane = analysis
        .mesh
        .triangles
        .iter()
        .position(|t| {
            t.iter()
                .all(|&v| (analysis.mesh.vertices[v][2] - min_z).abs() < 1e-10)
        })
        .unwrap();
    let seed = |step: &fabstep_app::DocumentStep, triangle| {
        step.result_faces()
            .unwrap()
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.source_triangle_ids.contains(&triangle))
            .unwrap()
            .face_id
    };
    for order in [
        [GuidanceFamily::Plane, GuidanceFamily::Cylinder],
        [GuidanceFamily::Cylinder, GuidanceFamily::Plane],
    ] {
        let base = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let triangle = |family| {
            if family == GuidanceFamily::Plane {
                plane
            } else {
                cylinder
            }
        };
        let first = base
            .step
            .guided_surface(&[seed(&base.step, triangle(order[0]))], order[0])
            .unwrap();
        let second = first
            .step
            .guided_surface(&[seed(&first.step, triangle(order[1]))], order[1])
            .unwrap();
        assert_eq!(second.report.guidance_hints.len(), 2);
        assert_eq!(second.report.cad.cylindrical_faces, 1);
        assert_eq!(
            second
                .step
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
        for id in [1, 2] {
            let removed = second.step.remove_guidance(id).unwrap();
            assert_eq!(removed.report.guidance_hints.len(), 1);
            let remaining = order[if id == 1 { 1 } else { 0 }];
            assert_eq!(
                removed.report.cad.cylindrical_faces,
                usize::from(remaining == GuidanceFamily::Cylinder)
            );
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
