use fabstep_app::{
    convert_document_with_provenance, preview_document, result_faces, DocumentResultFaces,
    DocumentStep, InputFormat,
};
use fabstep_core::{InputUnit, Options, ReconstructionMode};
use std::{collections::BTreeMap, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
    .unwrap()
}

fn check_exact(bytes: &[u8], format: InputFormat, options: &Options) -> DocumentResultFaces {
    let preview = preview_document(bytes, format, options).unwrap();
    let converted = convert_document_with_provenance(bytes, format, options).unwrap();
    let inspected = converted.step.result_faces().unwrap();
    let correspondence = &inspected.source_correspondence;
    assert_eq!(correspondence.status, "exact");
    assert_eq!(correspondence.faces.len(), inspected.result.faces.len());
    let mut partition = BTreeMap::<&str, Vec<usize>>::new();
    let mut solid_bodies = BTreeMap::new();
    for (face, source) in inspected.result.faces.iter().zip(&correspondence.faces) {
        assert_eq!(source.face_id, face.id);
        assert!(!source.source_triangle_ids.is_empty());
        assert!(source.source_triangle_ids.windows(2).all(|p| p[0] < p[1]));
        assert!(source
            .analysis_body_id
            .starts_with(&format!("{}/component/", source.preview_body_id)));
        if let Some(previous) = solid_bodies.insert(face.solid_id, &source.analysis_body_id) {
            assert_eq!(previous, &source.analysis_body_id);
        }
        let body = preview
            .bodies
            .iter()
            .find(|body| body.id == source.preview_body_id)
            .unwrap();
        partition
            .entry(&source.preview_body_id)
            .or_default()
            .extend(&source.source_triangle_ids);
        // Independent geometric oracle for these planar fixtures: the native
        // face must lie on the exact assigned source triangles' plane. This is
        // a test only; production provenance never matches by coordinates.
        for &triangle in &source.source_triangle_ids {
            let points = body.mesh.triangles[triangle].map(|i| body.mesh.vertices[i]);
            let a: [f64; 3] = std::array::from_fn(|i| points[1][i] - points[0][i]);
            let b: [f64; 3] = std::array::from_fn(|i| points[2][i] - points[0][i]);
            let n = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let length = n[0].hypot(n[1]).hypot(n[2]);
            for native_triangle in &inspected.result.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in native_triangle {
                    let p = inspected.result.mesh.vertices[vertex];
                    let distance = (0..3)
                        .map(|i| n[i] * (p[i] - points[0][i]))
                        .sum::<f64>()
                        .abs()
                        / length;
                    assert!(distance < 1e-6, "wrong source/native identity: {distance}");
                }
            }
        }
    }
    assert_eq!(partition.len(), preview.bodies.len());
    for body in &preview.bodies {
        let ids = partition.get_mut(body.id.as_str()).unwrap();
        ids.sort_unstable();
        assert_eq!(*ids, (0..body.mesh.triangles.len()).collect::<Vec<_>>());
    }
    assert_eq!(solid_bodies.len(), converted.report.cad.solids);
    let json = serde_json::to_value(correspondence).unwrap();
    assert!(json.get("reason").is_none());
    // Names alone are not trusted when the in-memory conversion record is lost.
    assert_eq!(
        result_faces(converted.step.bytes())
            .unwrap()
            .source_correspondence
            .status,
        "unavailable"
    );
    assert_eq!(
        DocumentStep::untracked(converted.step.bytes().to_vec())
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "unavailable"
    );
    inspected
}

#[test]
fn planar_step_faces_have_exact_source_partition_for_stl_and_placed_3mf() {
    for (name, format) in [
        ("cube.stl", InputFormat::Stl),
        ("rectangular_hole.stl", InputFormat::Stl),
        ("m3_instances.3mf", InputFormat::ThreeMf),
        ("m3_nested_components.3mf", InputFormat::ThreeMf),
        ("m3_mirror.3mf", InputFormat::ThreeMf),
        ("m3_disconnected_mesh.3mf", InputFormat::ThreeMf),
    ] {
        check_exact(&fixture(name), format, &Options::default());
    }
    check_exact(
        &fixture("cube.stl"),
        InputFormat::Stl,
        &Options {
            input_unit: InputUnit::Inch,
            ..Options::default()
        },
    );
}

#[test]
fn interleaved_components_keep_source_indices_through_weld_and_inversion() {
    let cube = fabstep_core::read_stl_mesh(&fixture("cube.stl"), InputUnit::Mm).unwrap();
    let mut stl = String::from("solid interleaved\n");
    for (i, triangle) in cube.triangles.iter().enumerate() {
        for body in 0..2 {
            let mut points = triangle.map(|v| cube.vertices[v]);
            for p in &mut points {
                p[0] += body as f64 * 3.;
            }
            if body == 1 {
                points.swap(1, 2);
            }
            if body == 0 && i == 0 {
                points[0][0] += 1e-8;
            }
            stl.push_str("facet normal 0 0 0\nouter loop\n");
            for p in points {
                stl.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            stl.push_str("endloop\nendfacet\n");
        }
    }
    stl.push_str("endsolid interleaved\n");
    let inspected = check_exact(stl.as_bytes(), InputFormat::Stl, &Options::default());
    for face in inspected.source_correspondence.faces {
        let component = if face.analysis_body_id.ends_with("/0") {
            0
        } else {
            1
        };
        assert!(face.source_triangle_ids.iter().all(|i| i % 2 == component));
    }
}

#[test]
fn analytic_result_retains_explicitly_unavailable_source_correspondence() {
    let converted = convert_document_with_provenance(
        &fixture("m4_chamfered_cylinder.stl"),
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        },
    )
    .unwrap();
    let inspected = converted.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "unavailable");
    assert_eq!(
        inspected.source_correspondence.reason,
        "step.provenance_unavailable"
    );
    assert!(inspected.source_correspondence.faces.is_empty());
    assert!(serde_json::to_value(inspected.source_correspondence)
        .unwrap()
        .get("faces")
        .is_none());
}
