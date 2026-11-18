use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use serde_json::Value;
use std::collections::BTreeSet;

const CASES: [&str; 2] = ["f_nominal", "f_mirror"];
const NOMINAL: &[u8] = include_bytes!("../../../fixtures/cross_drill/f_nominal.stl");
const MIRROR: &[u8] = include_bytes!("../../../fixtures/cross_drill/f_mirror.stl");

fn interleaved_stl(offset: [f64; 3]) -> Vec<u8> {
    let a = read_stl_mesh(NOMINAL, InputUnit::Mm).unwrap();
    let b = read_stl_mesh(MIRROR, InputUnit::Mm).unwrap();
    assert_eq!(a.triangles.len(), b.triangles.len());
    let count = u32::try_from(a.triangles.len() + b.triangles.len()).unwrap();
    let mut stl = vec![0; 80];
    stl.extend(count.to_le_bytes());
    for (first, second) in a.triangles.iter().zip(&b.triangles) {
        for (points, shift) in [
            (first.map(|i| a.vertices[i]), [0.; 3]),
            (second.map(|i| b.vertices[i]), offset),
        ] {
            stl.extend([0; 12]); // Facet orientation comes from vertex order.
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
fn interleaved_cross_drills_retain_their_native_solids_and_source_faces() {
    let offset = [32., -5., 3.];
    let bytes = interleaved_stl(offset);
    let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
    let report = &converted.report;
    assert_eq!((report.cad.solids, report.cad.faces), (2, 18));
    assert_eq!(
        (report.cad.planar_faces, report.cad.cylindrical_faces),
        (12, 6)
    );
    assert_eq!(report.bodies.len(), 2);

    let inspected = converted.step.result_faces().unwrap();
    let rows = &inspected.source_correspondence.faces;
    let result = &inspected.result;
    assert_eq!(inspected.source_correspondence.status, "exact");
    assert_eq!(rows.len(), 18);
    assert_eq!(result.faces.len(), 18);

    let manifest: Value =
        serde_json::from_str(include_str!("../../../fixtures/cross_drill/manifest.json")).unwrap();
    let tolerance = 2e-5;
    let mut matched = BTreeSet::new();
    let mut solids = BTreeSet::new();
    for (i, (name, shift)) in CASES.into_iter().zip([[0.; 3], offset]).enumerate() {
        let case = &manifest["cases"][name];
        let expected: [[f64; 3]; 2] = std::array::from_fn(|end| {
            std::array::from_fn(|axis| {
                case["analytic_bounds_mm"][end][axis].as_f64().unwrap() + shift[axis]
            })
        });
        let body = report
            .bodies
            .iter()
            .find(|body| (body.conversion.cad.bounds_mm[0][0] - expected[0][0]).abs() < tolerance)
            .unwrap();
        let bounds = body.conversion.cad.bounds_mm;
        for (measured, reference) in bounds
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert!(
                (measured - reference).abs() < tolerance,
                "{name}: {bounds:?}"
            );
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
            assert!(matched.insert(row.face_id));

            let face = &result.faces[row.face_id];
            assert_eq!(face.id, row.face_id);
            let family = if owner.starts_with("plane_") {
                ResultFaceFamily::Plane
            } else {
                ResultFaceFamily::Cylinder
            };
            assert_eq!(face.family, family);
            body_solids.insert(face.solid_id);
            assert!(face
                .adjacent_face_ids
                .iter()
                .all(|&id| result.faces[id].solid_id == face.solid_id));
            assert!(face.triangle_count > 0);
            for triangle in &result.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    assert!(result.mesh.vertices[vertex]
                        .iter()
                        .enumerate()
                        .all(|(axis, p)| {
                            (bounds[0][axis] - tolerance..=bounds[1][axis] + tolerance).contains(p)
                        }));
                }
            }
        }
        assert_eq!(body_solids.len(), 1);
        assert!(solids.insert(*body_solids.first().unwrap()));
    }
    assert_eq!(matched.len(), 18);
}

#[test]
fn touching_cross_drill_bodies_are_refused() {
    // The x mouth planes meet at x=12 with material surrounding both holes.
    // Offsetting in their plane keeps all vertices distinct between bodies.
    let bytes = interleaved_stl([20., 0.375, 0.625]);
    let error = convert_document_with_provenance(&bytes, InputFormat::Stl, &options())
        .err()
        .expect("touching sources accepted");
    assert_eq!(error.code, "cad.bodies_not_separated");
}
