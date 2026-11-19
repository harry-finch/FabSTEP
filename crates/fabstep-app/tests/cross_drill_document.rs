use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use serde_json::Value;
use std::collections::BTreeSet;

const NOMINAL: &[u8] = include_bytes!("../../../fixtures/cross_drill/f_nominal.stl");
const WIDER_RATIO: &[u8] = include_bytes!("../../../fixtures/cross_drill/f_ratio.stl");
const MIRROR: &[u8] = include_bytes!("../../../fixtures/cross_drill/f_mirror.stl");

fn interleaved_stl(first: &[u8], offset: [f64; 3]) -> (Vec<u8>, [Vec<usize>; 2]) {
    let meshes = [first, MIRROR].map(|bytes| read_stl_mesh(bytes, InputUnit::Mm).unwrap());
    let counts = meshes.each_ref().map(|mesh| mesh.triangles.len());
    let count = u32::try_from(counts.iter().sum::<usize>()).unwrap();
    let mut stl = vec![0; 80];
    stl.extend(count.to_le_bytes());
    let mut source_ids = [Vec::new(), Vec::new()];
    for row in 0..counts[0].max(counts[1]) {
        for (body, mesh) in meshes.iter().enumerate() {
            let Some(triangle) = mesh.triangles.get(row) else {
                continue;
            };
            source_ids[body].push((stl.len() - 84) / 50);
            let shift = if body == 0 { [0.; 3] } else { offset };
            stl.extend([0; 12]); // Facet orientation comes from vertex order.
            for point in triangle.map(|i| mesh.vertices[i]) {
                for (coordinate, delta) in point.into_iter().zip(shift) {
                    stl.extend(((coordinate + delta) as f32).to_le_bytes());
                }
            }
            stl.extend([0; 2]); // STL attribute byte count.
        }
    }
    (stl, source_ids)
}

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn interleaved_cross_drills_retain_their_native_solids_and_source_faces() {
    check_document("f_nominal", NOMINAL);
}

#[test]
fn wider_ratio_and_historical_bores_keep_separate_source_faces() {
    check_document("f_ratio", WIDER_RATIO);
}

fn check_document(first_name: &str, first: &[u8]) {
    let offset = [32., -5., 3.];
    let (bytes, source_ids) = interleaved_stl(first, offset);
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
    for (i, (name, shift)) in [first_name, "f_mirror"]
        .into_iter()
        .zip([[0.; 3], offset])
        .enumerate()
    {
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
            let expected_ids = source_ids[i][first..first + count].iter().copied();
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
    let (bytes, _) = interleaved_stl(NOMINAL, [20., 0.375, 0.625]);
    let error = convert_document_with_provenance(&bytes, InputFormat::Stl, &options())
        .err()
        .expect("touching sources accepted");
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn an_inclined_and_an_orthogonal_bore_pair_keep_independent_native_frames() {
    let inclined = include_bytes!("../../../fixtures/oblique_cross_drill/angle-75-coarse.stl");
    let (input, source) = interleaved_stl(inclined, [72., 0., 0.]);
    let result = convert_document_with_provenance(&input, InputFormat::Stl, &options()).unwrap();
    let cad = &result.report.cad;
    assert_eq!(
        (
            cad.solids,
            cad.faces,
            cad.planar_faces,
            cad.cylindrical_faces
        ),
        (2, 18, 12, 6)
    );
    let read = result.step.result_faces().unwrap();
    assert_eq!(read.source_correspondence.status, "exact");
    for (i, body) in result.report.bodies.iter().enumerate() {
        let drill = body
            .conversion
            .rounded_recognition
            .as_ref()
            .unwrap()
            .model
            .cross_drill
            .as_ref()
            .unwrap();
        if i == 0 {
            assert!((drill.large_axis_cosine.abs() - 75_f64.to_radians().cos()).abs() < 2e-6);
        } else {
            assert_eq!(drill.large_axis_cosine, 0.);
        }
        let mut source_faces = Vec::new();
        let mut solids = BTreeSet::new();
        for row in read
            .source_correspondence
            .faces
            .iter()
            .filter(|r| r.analysis_body_id == body.id)
        {
            source_faces.extend(row.source_triangle_ids.iter().copied());
            solids.insert(read.result.faces[row.face_id].solid_id);
        }
        source_faces.sort_unstable();
        assert_eq!(source_faces, source[i]);
        assert_eq!(solids.len(), 1);
    }
}
