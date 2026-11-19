use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{read_stl_mesh, InputUnit, Options, ProfileSegment, ReconstructionMode};
use std::collections::BTreeMap;

const BLOCK: &[u8] = include_bytes!("../../../fixtures/r115-rotated-block.stl");
const CREASE: &[u8] = include_bytes!("../../../fixtures/r115-rotated-crease.stl");

// Known STL record order from scripts/generate-r115-planes.py, independent of
// welding, visitation order, normals and final OCCT face ordering.
fn source_plane(id: usize, folded: bool) -> &'static str {
    match id {
        0..64 if id % 4 >= 2 => "bottom",
        48..64 if folded => "slope",
        0..64 => "top",
        64..80 if id % 4 < 2 => "front",
        64..80 => "back",
        80..96 if id % 4 < 2 => "left",
        80..96 => "right",
        _ => panic!("unexpected source triangle {id}"),
    }
}

fn inspect(
    bytes: &[u8],
    folded: bool,
    mode: ReconstructionMode,
) -> Vec<fabstep_app::ResultFaceSource> {
    let options = Options {
        reconstruction: mode,
        ..Options::default()
    };
    let converted = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
    let cad = &converted.report.cad;
    let expected_faces = if folded { 7 } else { 6 };
    assert_eq!(
        (cad.solids, cad.faces, cad.planar_faces),
        (1, expected_faces, expected_faces)
    );
    let wedge = if folded {
        0.5 * 7. * (7. / 8.) * 10.
    } else {
        0.
    };
    assert!((cad.volume_mm3 - (14. * 10. * 2. + wedge) / 16_f64.powi(3)).abs() < 3e-8);

    let source = read_stl_mesh(bytes, InputUnit::Mm).unwrap();
    let inspected = converted.step.result_faces().unwrap();
    let correspondence = inspected.source_correspondence;
    assert_eq!(correspondence.status, "exact");
    assert_eq!(correspondence.faces.len(), expected_faces);
    let mut owners = BTreeMap::new();
    for owner in &correspondence.faces {
        let ids = &owner.source_triangle_ids;
        let label = source_plane(ids[0], folded);
        let expected: Vec<_> = (0..96)
            .filter(|&i| source_plane(i, folded) == label)
            .collect();
        assert_eq!(
            *ids, expected,
            "incomplete region or crossed crease at {label}"
        );
        assert!(owners.insert(label, owner.face_id).is_none());
        let profile = converted.step.planar_face_profile(owner.face_id).unwrap();
        assert_eq!(profile.loops.len(), 1);
        let boundary = &profile.loops[0];
        assert!(boundary.is_outer && boundary.segments.len() >= 4);
        assert!(boundary
            .segments
            .iter()
            .all(|s| matches!(s, ProfileSegment::Line { .. })));
        // Compare each assigned triangle with the actual plane of the reimported face.
        for &triangle in ids {
            for vertex in source.triangles[triangle] {
                let p = source.vertices[vertex];
                let distance: f64 = (0..3)
                    .map(|i| (p[i] - profile.frame.origin_mm[i]) * profile.frame.normal[i])
                    .sum();
                assert!(
                    distance.abs() < 1e-7,
                    "wrong STEP face for triangle {triangle}"
                );
            }
        }
    }
    if folded {
        let top = owners["top"];
        let slope = owners["slope"];
        assert!(inspected.result.faces[top]
            .adjacent_face_ids
            .contains(&slope));
    }
    let fidelity = fabstep_verify::compare_meshes(
        &source,
        &inspected.result.mesh,
        inspected.result.deflection_mm,
    )
    .unwrap();
    assert!(fidelity.source_to_result_mm < 1e-7 && fidelity.result_to_source_mm < 1e-7);
    correspondence.faces
}

#[test]
fn planar_and_automatic_recover_the_nominal_faces_and_source_regions() {
    for (bytes, folded) in [(BLOCK, false), (CREASE, true)] {
        for mode in [ReconstructionMode::Planar, ReconstructionMode::Auto] {
            assert_eq!(inspect(bytes, folded, mode), inspect(bytes, folded, mode));
        }
    }
}

#[test]
fn large_quantization_residual_preserves_the_native_refusal() {
    let mut bytes = CREASE.to_vec();
    for record in bytes[84..].chunks_exact_mut(50) {
        for coord in record[12..48].chunks_exact_mut(4) {
            let v = f32::from_le_bytes(coord.try_into().unwrap());
            coord.copy_from_slice(&(16. * v).to_le_bytes());
        }
    }
    let refusal = convert_document_with_provenance(&bytes, InputFormat::Stl, &Options::default())
        .err()
        .unwrap();
    assert_eq!(refusal.code, "cad.invalid_model");
}
