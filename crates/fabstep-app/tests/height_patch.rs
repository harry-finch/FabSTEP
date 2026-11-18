use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;

const BOSS: &[u8] = include_bytes!("../../../fixtures/height_patch/boss-1.0.stl");
const SADDLE: &[u8] = include_bytes!("../../../fixtures/height_patch/saddle-1.0.stl");

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::HeightPatch,
        ..Options::default()
    }
}

#[test]
fn reliefs_preserve_every_source_owner_and_the_volume_after_transfer() {
    for (source, triangles, volume) in [(BOSS, 11598, 2090.), (SADDLE, 5138, 1920.)] {
        let result =
            convert_document_with_provenance(source, InputFormat::Stl, &options()).unwrap();
        let m = &result.report.cad;
        assert_eq!((m.solids, m.faces, m.planar_faces), (1, 6, 5));
        assert!((m.volume_mm3 - volume).abs() < 1e-5);
        let c = &result.report.bodies[0].conversion;
        assert!(
            c.height_patch_recognition
                .as_ref()
                .unwrap()
                .report
                .max_deviation_bound_mm
                < 0.002
        );
        let f = c.fidelity.as_ref().unwrap();
        assert!(f.source_to_result_mm.max(f.result_to_source_mm) < 0.02);
        let inspected = result.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact");
        let roof = inspected
            .result
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::HeightPatch)
            .unwrap();
        assert_eq!(roof.adjacent_face_ids.len(), 4);
        let rows = &inspected.source_correspondence.faces;
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| !r.source_triangle_ids.is_empty()));
        let all: Vec<_> = rows
            .iter()
            .flat_map(|r| r.source_triangle_ids.iter().copied())
            .collect();
        assert_eq!(all.len(), triangles);
        assert!((0..triangles).eq(all.into_iter().collect::<BTreeSet<_>>()));
    }
}

#[test]
fn cuboid_and_overbudget_relief_do_not_produce_an_approximate_export() {
    let cube = include_bytes!("../../../fixtures/cube.stl");
    let rejected = convert_document_with_provenance(cube, InputFormat::Stl, &options())
        .err()
        .unwrap();
    assert_eq!(rejected.code, "recognition.height_patch_unproven");
    let tight = Options {
        max_deviation_mm: 0.0005,
        ..options()
    };
    let error = convert_document_with_provenance(BOSS, InputFormat::Stl, &tight)
        .err()
        .unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");
}

#[test]
fn two_translated_reliefs_remain_separate_in_the_same_step() {
    let mut bytes = BOSS.to_vec();
    let count = (BOSS.len() + SADDLE.len() - 168) / 50;
    bytes[80..84].copy_from_slice(&(count as u32).to_le_bytes());
    for triangle in SADDLE[84..].chunks_exact(50) {
        let start = bytes.len();
        bytes.extend_from_slice(triangle);
        for i in [12, 24, 36] {
            let coordinate = &mut bytes[start + i..start + i + 4];
            let x = f32::from_le_bytes(coordinate.try_into().unwrap()) + 32.;
            coordinate.copy_from_slice(&x.to_le_bytes());
        }
    }
    let result = convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
    let m = &result.report.cad;
    assert_eq!((m.solids, m.faces, m.planar_faces), (2, 12, 10));
    assert!((m.volume_mm3 - 4010.).abs() < 1e-5);
    let inspected = result.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    assert_eq!(
        inspected
            .result
            .faces
            .iter()
            .filter(|f| f.family == ResultFaceFamily::HeightPatch)
            .count(),
        2
    );
}

#[test]
fn native_builder_refuses_invalid_control_nets_and_duplicate_tokens() {
    let mut patch = fabstep_core::HeightPatch {
        origin: [0.; 3],
        size: [20., 16.],
        heights: [[6.; 4]; 4],
    };
    patch.heights[1][1] = 7.;
    for bad in [f64::NAN, -1., 0.] {
        let mut wrong = patch.clone();
        wrong.heights[2][2] = bad;
        assert_eq!(
            fabstep_cad::build_height_patch(&wrong, None)
                .err()
                .unwrap()
                .code,
            "cad.invalid_model"
        );
    }
    let tokens = [1, 2, 3, 4, 5, 5];
    assert_eq!(
        fabstep_cad::build_height_patch(&patch, Some(&tokens))
            .err()
            .unwrap()
            .code,
        "step.invalid_source_provenance"
    );
    patch.heights[0][1] = 6.5;
    assert_eq!(
        fabstep_cad::build_height_patch(&patch, None)
            .err()
            .unwrap()
            .code,
        "cad.invalid_model"
    );
}
