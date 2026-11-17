//! Independent half-solid with cylinder / chamfer / cylinder bands.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{GuidanceFamily, Options, ReconstructionMode};
use std::collections::BTreeSet;

fn source() -> Vec<u8> {
    const N: usize = 48;
    let levels = [(0., 1.), (2., 1.), (3., 2.), (4., 2.)];
    let mut vertices = Vec::new();
    for (z, radius) in levels {
        for i in 0..=N {
            let angle = std::f64::consts::PI * i as f64 / N as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), z]);
        }
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
    let mut text = String::from("solid independent_partial_chain\n");
    for triangle in triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for v in triangle {
            let [x, y, z] = vertices[v];
            text.push_str(&format!("vertex {x:.17} {y:.17} {z:.17}\n"));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid independent_partial_chain\n");
    text.into_bytes()
}

#[test]
fn automatic_partial_chain_has_exact_provenance_and_guided_replay() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let converted =
        convert_document_with_provenance(&source(), InputFormat::Stl, &options).unwrap();
    assert_eq!(converted.report.cad.solids, 1);
    assert_eq!(converted.report.cad.cylindrical_faces, 2);
    assert_eq!(converted.report.cad.conical_faces, 1);
    assert!((converted.report.cad.volume_mm3 - 25. * std::f64::consts::PI / 6.).abs() < 1e-7);
    assert!(converted.report.bodies[0].conversion.fidelity.is_some());
    let faces = converted.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let mut partition = BTreeSet::new();
    for face in &faces.source_correspondence.faces {
        assert!(!face.source_triangle_ids.is_empty());
        for &triangle in &face.source_triangle_ids {
            assert!(partition.insert(triangle));
        }
    }
    assert_eq!(
        partition.len(),
        converted.report.bodies[0]
            .conversion
            .analysis
            .mesh
            .triangles
            .len()
    );
    let seed = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cylinder)
        .unwrap()
        .id;
    let group = converted
        .step
        .suggest_region_controlled(seed, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(group.len(), 3);
    assert!(group.contains(&seed));
    assert!(group.iter().all(|id| faces
        .result
        .faces
        .iter()
        .find(|face| face.id == *id)
        .is_some_and(|face| matches!(
            face.family,
            fabstep_cad::ResultFaceFamily::Cylinder | fabstep_cad::ResultFaceFamily::Cone
        ))));
    let cone = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cone)
        .unwrap()
        .id;
    let cone_guided = converted
        .step
        .guided_surface(&[cone], GuidanceFamily::Cone)
        .unwrap();
    assert_eq!(cone_guided.report.cad.faces, converted.report.cad.faces);
    assert_eq!(cone_guided.report.guidance_hints[0].family, "cone");
    let mut cone_first = group.clone();
    cone_first.sort_by_key(|id| usize::from(*id != cone));
    let grouped = converted
        .step
        .guided_surface(&cone_first, GuidanceFamily::Cylinder)
        .unwrap();
    assert_eq!(grouped.report.cad.faces, converted.report.cad.faces);
    let mut cylinder_first = group.clone();
    cylinder_first.sort_by_key(|id| usize::from(*id != seed));
    let grouped = converted
        .step
        .guided_surface(&cylinder_first, GuidanceFamily::Cone)
        .unwrap();
    assert_eq!(grouped.report.cad.faces, converted.report.cad.faces);
    let guided = converted
        .step
        .guided_surface(&[seed], GuidanceFamily::Cylinder)
        .unwrap();
    assert_eq!(guided.report.cad.cylindrical_faces, 2);
    assert_eq!(guided.report.cad.conical_faces, 1);
    let hint = &guided.report.guidance_hints[0];
    assert!(hint.reconstructed_source_triangle_ids.len() > hint.selected_source_triangle_ids.len());
    let replay = guided.step.recompute_guidance().unwrap();
    assert_eq!(
        replay.report.guidance_hints[0].reconstructed_source_triangle_ids,
        hint.reconstructed_source_triangle_ids
    );
    let removed = replay.step.remove_guidance(hint.hint_id).unwrap();
    assert!(removed.report.guidance_hints.is_empty());
    assert_eq!(removed.report.cad.faces, converted.report.cad.faces);
    assert!((removed.report.cad.volume_mm3 - converted.report.cad.volume_mm3).abs() < 1e-8);
}
