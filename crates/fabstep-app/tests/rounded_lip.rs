use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode};
use std::collections::BTreeSet;

const CUP: &[u8] = include_bytes!("../../../fixtures/rounded_lip/cup.stl");

#[test]
fn two_distinct_toroidal_rims_bound_the_open_cup_in_both_orientations() {
    let mut inverted = CUP.to_vec();
    for triangle in inverted[84..].chunks_exact_mut(50) {
        for point in triangle[0..48].chunks_exact_mut(12) {
            for offset in [4, 8] {
                let coord: [u8; 4] = point[offset..offset + 4].try_into().unwrap();
                point[offset..offset + 4]
                    .copy_from_slice(&(-f32::from_le_bytes(coord)).to_le_bytes());
            }
        }
    }
    for (bytes, mode) in [
        (CUP, ReconstructionMode::Auto),
        (inverted.as_slice(), ReconstructionMode::Auto),
    ] {
        let options = Options {
            reconstruction: mode,
            ..Options::default()
        };
        let r = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let m = &r.report.cad;
        assert_eq!(
            (
                m.solids,
                m.planar_faces,
                m.cylindrical_faces,
                m.toroidal_faces,
                m.faces
            ),
            (1, 3, 2, 2, 7)
        );
        assert!((m.volume_mm3 - 1978.143279).abs() < 0.002);
        let faces = r.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let walls: Vec<_> = faces
            .result
            .faces
            .iter()
            .filter(|f| f.family == ResultFaceFamily::Torus)
            .collect();
        assert_eq!(walls.len(), 2);
        let mut owners = BTreeSet::new();
        for face in walls {
            let row = faces
                .source_correspondence
                .faces
                .iter()
                .find(|r| r.face_id == face.id)
                .unwrap();
            assert!(row.source_triangle_ids.len() > 100);
            assert!(row.source_triangle_ids.iter().all(|t| owners.insert(*t)));
            let neighbors: Vec<_> = face
                .adjacent_face_ids
                .iter()
                .map(|i| &faces.result.faces[*i].family)
                .collect();
            assert!(neighbors.contains(&&ResultFaceFamily::Plane));
            assert!(neighbors.contains(&&ResultFaceFamily::Cylinder));
        }
    }
}

#[test]
fn an_inner_lip_with_a_warped_meridian_fails_the_surface_proof() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut source = analyze_stl(CUP, &o).unwrap();
    let original = recognize_rounded(&source, &o).unwrap();
    assert_eq!(original.report.tori, 2);
    let v = source
        .mesh
        .vertices
        .iter_mut()
        .find(|v| v[2] > 13.5 && v[2] < 13.8 && v[0].hypot(v[1]) < 8.5)
        .unwrap();
    v[2] += 0.04;
    assert!(recognize_rounded(&source, &o).is_err());
}
