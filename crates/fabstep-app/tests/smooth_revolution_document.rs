use fabstep_app::{convert_document_with_provenance, prepare_source_section, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use std::collections::BTreeSet;

const BULGE: &[u8] = include_bytes!("../../../fixtures/smooth_revolution/bulge.stl");
const NECK: &[u8] = include_bytes!("../../../fixtures/smooth_revolution/neck.stl");
const PHASED: &[u8] = include_bytes!("../../../fixtures/smooth_revolution/retessellated.stl");

fn interleaved_stl(first: &[u8], second: &[u8], offset: [f64; 3]) -> Vec<u8> {
    let a = read_stl_mesh(first, InputUnit::Mm).unwrap();
    let b = read_stl_mesh(second, InputUnit::Mm).unwrap();
    let mut stl = vec![0; 80];
    stl.extend(
        u32::try_from(a.triangles.len() + b.triangles.len())
            .unwrap()
            .to_le_bytes(),
    );
    for i in 0..a.triangles.len().max(b.triangles.len()) {
        for (mesh, shift) in [(&a, [0.; 3]), (&b, offset)] {
            let Some(triangle) = mesh.triangles.get(i) else {
                continue;
            };
            let points = triangle.map(|i| mesh.vertices[i]);
            stl.extend([0; 12]); // Orientation comes from vertex order.
            for point in points {
                for (coordinate, delta) in point.into_iter().zip(shift) {
                    stl.extend(((coordinate + delta) as f32).to_le_bytes());
                }
            }
            stl.extend([0; 2]);
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
fn disjoint_revolutions_keep_two_roots_and_interleaved_source_ownership() {
    check_two_roots(BULGE, NECK, 4224);
}

#[test]
fn two_span_revolutions_keep_both_internal_knots_in_a_multiroot_export() {
    let source = include_bytes!("../../../fixtures/smooth_revolution/two_span.stl");
    check_two_roots(source, source, 6272);
}

fn check_two_roots(first: &[u8], second: &[u8], count: usize) {
    let bytes = interleaved_stl(first, second, [25., -2., 3.]);
    let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
    let report = &converted.report;
    assert_eq!(
        (
            report.cad.solids,
            report.cad.faces,
            report.cad.planar_faces,
            report.cad.revolution_faces
        ),
        (2, 6, 4, 2)
    );
    let inspected = converted.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    assert_eq!(inspected.source_correspondence.faces.len(), 6);
    let mut claimed_faces = BTreeSet::new();
    let mut solid_ids = BTreeSet::new();
    for (parity, expected_center_x) in [0., 25.].into_iter().enumerate() {
        let body = report
            .bodies
            .iter()
            .find(|body| {
                let [[low, _, _], [high, _, _]] = body.conversion.cad.bounds_mm;
                ((low + high) * 0.5 - expected_center_x).abs() < 2e-4
            })
            .unwrap();
        let mut body_solids = BTreeSet::new();
        for (family, start, end) in [
            (ResultFaceFamily::Plane, 0, 64),
            (ResultFaceFamily::Revolution, 64, count - 64),
            (ResultFaceFamily::Plane, count - 64, count),
        ] {
            let row = inspected
                .source_correspondence
                .faces
                .iter()
                .find(|row| {
                    row.source_triangle_ids
                        .iter()
                        .copied()
                        .eq((start..end).map(|i| 2 * i + parity))
                })
                .unwrap();
            assert_eq!(row.analysis_body_id, body.id);
            assert!(claimed_faces.insert(row.face_id));
            let face = &inspected.result.faces[row.face_id];
            assert_eq!(face.family, family);
            body_solids.insert(face.solid_id);
            assert!(face
                .adjacent_face_ids
                .iter()
                .all(|&neighbor| { inspected.result.faces[neighbor].solid_id == face.solid_id }));
        }
        assert_eq!(body_solids.len(), 1);
        assert!(solid_ids.insert(*body_solids.first().unwrap()));
    }
}

#[test]
fn contact_in_the_source_or_only_after_reconstruction_blocks_the_whole_document() {
    // Caps share z=20 and overlap in XY, without coinciding source vertices.
    let touching_source = interleaved_stl(BULGE, NECK, [0.375, 0.625, 20.]);
    let error = convert_document_with_provenance(&touching_source, InputFormat::Stl, &options())
        .err()
        .expect("touching sources accepted");
    // The touching source bounds require 4,224² face pairs. The native
    // source admission budget is ten million pairs and blocks this document.
    assert_eq!(error.code, "cad.bodies_separation_budget");

    // The phased 8 mm cap polygons remain separated along X, while the
    // circular caps overlap by 0.0005 mm after rebuilding both solid roots.
    let touching_result = interleaved_stl(PHASED, PHASED, [15.9995, 0., 0.]);
    let source = prepare_source_section(&touching_result, InputFormat::Stl, &options()).unwrap();
    assert_eq!(source.metadata().bodies.len(), 2);
    let error = convert_document_with_provenance(&touching_result, InputFormat::Stl, &options())
        .err()
        .expect("overlapping reconstructed solids accepted");
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn a_pole_and_a_two_disk_solid_preserve_all_five_faces_and_both_owners() {
    let tip = include_bytes!("../../../fixtures/smooth_revolution/axial_tip.stl");
    let bytes = interleaved_stl(tip, NECK, [32., 0., 0.]);
    let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
    let r = &converted.report.cad;
    assert_eq!(
        (r.solids, r.faces, r.planar_faces, r.revolution_faces),
        (2, 5, 3, 2)
    );
    let inspected = converted.step.result_faces().unwrap();
    let correspondence = inspected.source_correspondence;
    assert_eq!(correspondence.status, "exact");
    assert_eq!(correspondence.faces.len(), 5);
    let mut used = BTreeSet::new();
    for row in &correspondence.faces {
        let face = &inspected.result.faces[row.face_id];
        let body = converted
            .report
            .bodies
            .iter()
            .find(|b| b.id == row.analysis_body_id)
            .unwrap();
        let is_tip = body.conversion.cad.planar_faces == 1;
        assert!(face
            .adjacent_face_ids
            .iter()
            .all(|&i| inspected.result.faces[i].solid_id == face.solid_id));
        if face.family == ResultFaceFamily::Revolution {
            assert_eq!(face.adjacent_face_ids.len(), if is_tip { 1 } else { 2 });
        }
        for &id in &row.source_triangle_ids {
            assert!(used.insert(id));
            assert_eq!(id < 8192 && id % 2 == 0, is_tip);
        }
    }
    assert_eq!(used.len(), 4096 + 4224);
}
