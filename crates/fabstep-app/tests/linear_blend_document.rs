use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ProfileSegment, ReconstructionMode};
use serde_json::Value;
use std::collections::BTreeSet;

const CASES: [&str; 2] = ["g_increasing", "g_decreasing"];
const INCREASING: &[u8] = include_bytes!("../../../fixtures/local_blends/g_increasing.stl");
const DECREASING: &[u8] = include_bytes!("../../../fixtures/local_blends/g_decreasing.stl");

fn interleaved_stl(offset: [f64; 3]) -> Vec<u8> {
    let first = read_stl_mesh(INCREASING, InputUnit::Mm).unwrap();
    let second = read_stl_mesh(DECREASING, InputUnit::Mm).unwrap();
    assert_eq!(first.triangles.len(), second.triangles.len());
    let triangle_count = u32::try_from(first.triangles.len() + second.triangles.len()).unwrap();
    let mut stl = vec![0; 80];
    stl.extend(triangle_count.to_le_bytes());
    for (a, b) in first.triangles.iter().zip(&second.triangles) {
        for (points, shift) in [
            (a.map(|i| first.vertices[i]), [0.; 3]),
            (b.map(|i| second.vertices[i]), offset),
        ] {
            stl.extend([0; 12]); // Orientation comes from the vertex order.
            for point in points {
                for (coordinate, delta) in point.into_iter().zip(shift) {
                    stl.extend(((coordinate + delta) as f32).to_le_bytes());
                }
            }
            stl.extend([0; 2]); // STL attribute byte count.
        }
    }
    stl
}

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn interleaved_linear_blends_keep_their_native_faces_owners_and_placements() {
    let offset = [32., -8., 4.];
    let source = interleaved_stl(offset);
    let converted =
        convert_document_with_provenance(&source, InputFormat::Stl, &options()).unwrap();
    let report = &converted.report;
    assert_eq!((report.cad.solids, report.cad.faces), (2, 14));
    assert_eq!((report.cad.planar_faces, report.cad.conical_faces), (12, 2));
    assert_eq!(report.bodies.len(), 2);

    let inspected = converted.step.result_faces().unwrap();
    let rows = &inspected.source_correspondence.faces;
    assert_eq!(inspected.source_correspondence.status, "exact");
    assert_eq!(rows.len(), 14);
    assert_eq!(inspected.result.faces.len(), 14);

    let manifest: Value =
        serde_json::from_str(include_str!("../../../fixtures/local_blends/manifest.json")).unwrap();
    let tolerance = 2e-5;
    let mut matched_faces = BTreeSet::new();
    let mut solids = BTreeSet::new();
    for (i, (name, shift)) in CASES.into_iter().zip([[0.; 3], offset]).enumerate() {
        let case = &manifest["cases"][name];
        let body = report
            .bodies
            .iter()
            .find(|body| (body.conversion.cad.bounds_mm[0][0] - shift[0]).abs() < tolerance)
            .unwrap();
        let cad = &body.conversion.cad;
        let bounds = cad.bounds_mm;
        assert_eq!((cad.faces, cad.conical_faces), (7, 1));
        for (end, coordinates) in bounds.into_iter().enumerate() {
            for (axis, (actual, delta)) in coordinates.into_iter().zip(shift).enumerate() {
                let expected = case["analytic_bounds_mm"][end][axis].as_f64().unwrap() + delta;
                assert!((actual - expected).abs() < tolerance, "{name}: {bounds:?}");
            }
        }

        let mut body_solids = BTreeSet::new();
        for patch in case["patches"].as_array().unwrap() {
            let owner = patch["owner"].as_str().unwrap();
            let first = patch["first_source_triangle"].as_u64().unwrap() as usize;
            let count = patch["triangles"].as_u64().unwrap() as usize;
            let expected_ids = (first..first + count).map(|triangle| 2 * triangle + i);
            let row = rows
                .iter()
                .find(|row| {
                    row.source_triangle_ids
                        .iter()
                        .copied()
                        .eq(expected_ids.clone())
                })
                .unwrap_or_else(|| panic!("missing source patch {name}/{owner}"));
            assert_eq!(row.analysis_body_id, body.id);
            assert!(matched_faces.insert(row.face_id));
            let face = &inspected.result.faces[row.face_id];
            assert_eq!(face.id, row.face_id);
            body_solids.insert(face.solid_id);
            if owner == "inclined_cone" {
                assert_eq!(face.family, ResultFaceFamily::Cone);
                assert_eq!(face.adjacent_face_ids.len(), 4);
                let ellipse_count = face
                    .adjacent_face_ids
                    .iter()
                    .map(|&id| {
                        let profile = converted.step.planar_face_profile(id).unwrap();
                        assert_eq!(profile.solid_id, face.solid_id);
                        profile
                            .loops
                            .iter()
                            .flat_map(|wire| &wire.segments)
                            .filter(|segment| matches!(segment, ProfileSegment::Ellipse { .. }))
                            .count()
                    })
                    .sum::<usize>();
                assert_eq!(ellipse_count, 2, "{name}");
            } else {
                assert_eq!(face.family, ResultFaceFamily::Plane);
            }
            assert!(face.triangle_count > 0);
            for triangle in &inspected.result.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    let point = inspected.result.mesh.vertices[vertex];
                    assert!(point.iter().enumerate().all(|(axis, coordinate)| {
                        (bounds[0][axis] - tolerance..=bounds[1][axis] + tolerance)
                            .contains(coordinate)
                    }));
                }
            }
        }
        assert_eq!(body_solids.len(), 1);
        assert!(solids.insert(*body_solids.first().unwrap()));
    }
    assert_eq!(matched_faces.len(), 14);
}

#[test]
fn touching_linear_blend_bodies_are_refused() {
    // The outer planes meet at x = 20. The in-plane offset keeps all vertices
    // distinct so the contact is detected between the two source bodies.
    let source = interleaved_stl([20., 0.375, 0.625]);
    let error = convert_document_with_provenance(&source, InputFormat::Stl, &options())
        .err()
        .expect("touching source accepted");
    assert_eq!(error.code, "cad.bodies_not_separated");
}
