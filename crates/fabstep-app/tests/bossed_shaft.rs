//! FreeCAD union of an R5 shaft and a 6 mm wide longitudinal boss. The
//! cylindrical wall ends on two flanks, with a major arc on each cap.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ProfileSegment, ReconstructionMode};
use std::collections::BTreeSet;

const MESHES: [&[u8]; 2] = [
    include_bytes!("../../../fixtures/r117-bossed-shaft-coarse.stl"),
    include_bytes!("../../../fixtures/r117-bossed-shaft-fine.stl"),
];

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

fn source_wall(bytes: &[u8]) -> BTreeSet<usize> {
    bytes[84..]
        .chunks_exact(50)
        .enumerate()
        .filter_map(|(id, record)| {
            let p = |vertex: usize, axis: usize| {
                let start = 12 + 12 * vertex + 4 * axis;
                f64::from(f32::from_le_bytes(
                    record[start..start + 4].try_into().unwrap(),
                ))
            };
            let vertical = (1..3).any(|v| p(v, 2) != p(0, 2));
            (vertical && (0..3).all(|v| (p(v, 0).hypot(p(v, 1)) - 5.).abs() < 1e-6)).then_some(id)
        })
        .collect()
}

#[test]
fn shaft_interrupted_by_boss_has_two_major_arcs_and_exact_wall_provenance_at_both_densities() {
    let angle = 2. * 3.0_f64.atan2(4.);
    let expected_sweep = std::f64::consts::TAU - angle;
    let expected_volume = (25. * std::f64::consts::PI - 12.5 * (angle - angle.sin()) + 24.) * 3.5;
    for bytes in MESHES {
        let result = convert_document_with_provenance(bytes, InputFormat::Stl, &options()).unwrap();
        let report = &result.report;
        assert_eq!(
            (
                report.cad.solids,
                report.cad.cylindrical_faces,
                report.cad.planar_faces
            ),
            (1, 1, 5)
        );
        assert!((report.cad.volume_mm3 - expected_volume).abs() < 2e-5);
        let fid = report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(fid.source_to_result_mm < 0.011 && fid.result_to_source_mm < 0.011);
        let faces = result.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let expected = source_wall(bytes);
        assert!(expected.len() > 50);
        let mut caps = 0;
        for face in &faces.result.faces {
            if face.family == ResultFaceFamily::Cylinder {
                assert_eq!(face.adjacent_face_ids.len(), 4);
                let row = faces
                    .source_correspondence
                    .faces
                    .iter()
                    .find(|row| row.face_id == face.id)
                    .unwrap();
                assert_eq!(
                    row.source_triangle_ids
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>(),
                    expected
                );
                continue;
            }
            let profile = result.step.planar_face_profile(face.id).unwrap();
            let [outer] = profile.loops.as_slice() else {
                panic!("extra loop")
            };
            assert_eq!(outer.segments.len(), 4);
            for segment in &outer.segments {
                if let ProfileSegment::Arc {
                    radius_mm,
                    sweep_angle_rad,
                    ..
                } = segment
                {
                    assert!(
                        (radius_mm - 5.).abs() < 1e-6
                            && (sweep_angle_rad.abs() - expected_sweep).abs() < 1e-6
                    );
                    caps += 1;
                }
            }
        }
        assert_eq!(caps, 2);
    }
}

#[test]
fn elliptic_shaft_with_the_same_boss_stays_faceted() {
    for mesh in MESHES {
        let mut bytes = mesh.to_vec();
        for triangle in bytes[84..].chunks_exact_mut(50) {
            for v in triangle[12..48].chunks_exact_mut(12) {
                let y = f32::from_le_bytes(v[4..8].try_into().unwrap());
                v[4..8].copy_from_slice(&(y * 1.08).to_le_bytes());
            }
        }
        let result =
            convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
        assert_eq!(result.report.cad.solids, 1);
        assert_eq!(result.report.cad.cylindrical_faces, 0);
        assert_eq!(result.report.cad.faces, result.report.cad.planar_faces);
    }
}
