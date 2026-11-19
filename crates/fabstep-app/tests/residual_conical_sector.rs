//! Half frustum drawn independently from its radii and height. Four
//! distinct axial levels on its wall must resolve to one shared conical face.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;

const N: usize = 32;
const ROWS: usize = 4;

fn half_frustum(map: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<u8> {
    let mut stl = include_bytes!("../../../fixtures/r117-half-frustum.stl").to_vec();
    for record in stl[84..].chunks_exact_mut(50) {
        for point in record[12..48].chunks_exact_mut(12) {
            let xyz = std::array::from_fn(|i| {
                f32::from_le_bytes(point[4 * i..4 * i + 4].try_into().unwrap())
            });
            for (target, v) in point.chunks_exact_mut(4).zip(map(xyz)) {
                target.copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    stl
}

fn convert(source: &[u8]) -> fabstep_app::ProvenancedDocumentConversion {
    convert_document_with_provenance(
        source,
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        },
    )
    .unwrap()
}

#[test]
fn normal_nominated_half_frustum_has_a_single_trimmed_cone() {
    for bytes in [half_frustum(|p| p), half_frustum(|[x, y, z]| [z, x, y])] {
        let doc = convert(&bytes);
        assert_eq!(
            (
                doc.report.cad.solids,
                doc.report.cad.faces,
                doc.report.cad.conical_faces
            ),
            (1, 4, 1)
        );
        let expected = std::f64::consts::PI / 2. * (4. + 7. + 12.25);
        assert!((doc.report.cad.volume_mm3 - expected).abs() < 1e-6);
        let fidelity = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(fidelity.source_to_result_mm < 0.009 && fidelity.result_to_source_mm < 0.009);
        let result = doc.step.result_faces().unwrap();
        assert_eq!(result.source_correspondence.status, "exact");
        let id = result
            .result
            .faces
            .iter()
            .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cone)
            .unwrap()
            .id;
        let owner = result
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.face_id == id)
            .unwrap();
        let expected: BTreeSet<_> = (0..ROWS - 1)
            .flat_map(|k| 2 * (N + 1) * k..2 * (N + 1) * k + 2 * N)
            .collect();
        assert_eq!(
            owner
                .source_triangle_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
        );
    }
}

#[test]
fn elliptic_band_and_bulged_middle_row_keep_their_facets() {
    for bytes in [
        half_frustum(|[x, y, z]| [x, 0.85 * y, z]),
        half_frustum(|[x, y, z]| [x, if z == 1. { y * 1.015 } else { y }, z]),
    ] {
        let doc = convert(&bytes);
        assert_eq!(
            (doc.report.cad.solids, doc.report.cad.conical_faces),
            (1, 0)
        );
    }
}
