use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use serde_json::Value;
use std::collections::BTreeSet;

const CASES: [&str; 2] = ["d_corner", "d_corner_mirror"];
const NOMINAL: &[u8] = include_bytes!("../../../fixtures/local_blends/d_corner.stl");
const MIRROR: &[u8] = include_bytes!("../../../fixtures/local_blends/d_corner_mirror.stl");

fn interleaved_stl(offset: [f64; 3]) -> Vec<u8> {
    let a = read_stl_mesh(NOMINAL, InputUnit::Mm).unwrap();
    let b = read_stl_mesh(MIRROR, InputUnit::Mm).unwrap();
    assert_eq!(a.triangles.len(), b.triangles.len());
    let triangle_count = u32::try_from(a.triangles.len() + b.triangles.len()).unwrap();
    let mut stl = vec![0; 80];
    stl.extend(triangle_count.to_le_bytes());
    for (first, second) in a.triangles.iter().zip(&b.triangles) {
        for (points, shift) in [
            (first.map(|i| a.vertices[i]), [0.; 3]),
            (second.map(|i| b.vertices[i]), offset),
        ] {
            stl.extend([0; 12]); // STL facet normal, inferred from vertex order
            for point in points {
                for (coordinate, delta) in point.into_iter().zip(shift) {
                    stl.extend(((coordinate + delta) as f32).to_le_bytes());
                }
            }
            stl.extend([0; 2]); // STL attribute byte count
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
fn interleaved_trihedral_bodies_keep_placements_and_exact_native_face_owners() {
    let offset = [32., -8., 4.];
    let source = interleaved_stl(offset);
    let converted =
        convert_document_with_provenance(&source, InputFormat::Stl, &options()).unwrap();
    let report = &converted.report;
    assert_eq!((report.cad.solids, report.cad.faces), (2, 20));
    assert_eq!(
        (
            report.cad.planar_faces,
            report.cad.cylindrical_faces,
            report.cad.spherical_faces
        ),
        (12, 6, 2)
    );
    assert_eq!(report.bodies.len(), 2);

    let inspected = converted.step.result_faces().unwrap();
    let rows = &inspected.source_correspondence.faces;
    assert_eq!(inspected.source_correspondence.status, "exact");
    assert_eq!(rows.len(), 20);
    assert_eq!(inspected.result.faces.len(), 20);

    let manifest: Value =
        serde_json::from_str(include_str!("../../../fixtures/local_blends/manifest.json")).unwrap();
    let mut matched_faces = BTreeSet::new();
    let mut solids = BTreeSet::new();
    for (i, (name, shift)) in CASES.into_iter().zip([[0.; 3], offset]).enumerate() {
        let case = &manifest["cases"][name];
        let tolerance = 2e-5;
        let body = report
            .bodies
            .iter()
            .find(|body| (body.conversion.cad.bounds_mm[0][0] - shift[0]).abs() < tolerance)
            .unwrap();
        let bounds = body.conversion.cad.bounds_mm;
        for (end, coordinates) in bounds.into_iter().enumerate() {
            for (axis, (actual, delta)) in coordinates.into_iter().zip(shift).enumerate() {
                let expected = case["analytic_bounds_mm"][end][axis].as_f64().unwrap() + delta;
                assert!((actual - expected).abs() < tolerance, "{name}: {bounds:?}");
            }
        }
        let mut body_solids = BTreeSet::new();
        for patch in case["patches"].as_array().unwrap() {
            let owner = patch["owner"].as_str().unwrap();
            let family = match owner.split('_').next().unwrap() {
                "plane" => ResultFaceFamily::Plane,
                "cylinder" => ResultFaceFamily::Cylinder,
                "sphere" => ResultFaceFamily::Sphere,
                _ => panic!("unknown source patch {owner}"),
            };
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
            assert_eq!(face.family, family);
            body_solids.insert(face.solid_id);
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
    assert_eq!(matched_faces.len(), 20);
}

#[test]
fn touching_trihedral_bodies_are_refused_before_reconstruction() {
    // The end planes meet at x = 20. The in-plane offset keeps their vertices
    // distinct so the source-body separation check handles the contact.
    let source = interleaved_stl([20., 0.375, 0.625]);
    let error = convert_document_with_provenance(&source, InputFormat::Stl, &options())
        .err()
        .expect("touching source accepted");
    assert_eq!(error.code, "cad.bodies_not_separated");
}
