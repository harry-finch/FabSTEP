use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;
const CASES: [(&str, &[u8], f64); 3] = [
    (
        "bulged-coarse",
        include_bytes!("../../../fixtures/irregular_revolution/bulged-coarse.stl"),
        3129.026282975438,
    ),
    (
        "bulged-fine",
        include_bytes!("../../../fixtures/irregular_revolution/bulged-fine.stl"),
        3129.026282975438,
    ),
    (
        "inflected-coarse",
        include_bytes!("../../../fixtures/irregular_revolution/inflected-coarse.stl"),
        2747.7266946197547,
    ),
];
#[test]
fn irregular_cad_remeshes_keep_two_real_disks_one_wall_and_exact_provenance() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..Options::default()
        };
        for (name, raw, reference) in CASES {
            let converted = convert_document_with_provenance(raw, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let cad = &converted.report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.planar_faces,
                    cad.revolution_faces,
                    cad.faces
                ),
                (1, 2, 1, 3)
            );
            assert!((cad.volume_mm3 - reference).abs() < reference * 1e-7);
            let r = &converted.report.bodies[0].conversion;
            let proof = r.rounded_recognition.as_ref().unwrap();
            assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
            let f = r.fidelity.as_ref().unwrap();
            assert!(
                f.source_to_result_mm <= o.max_deviation_mm
                    && f.result_to_source_mm <= o.max_deviation_mm
            );
            let result = converted.step.result_faces().unwrap();
            let wall = result
                .result
                .faces
                .iter()
                .find(|f| f.family == ResultFaceFamily::Revolution)
                .unwrap();
            assert_eq!(wall.adjacent_face_ids.len(), 2);
            if mode == ReconstructionMode::Auto {
                let map = &result.source_correspondence;
                assert_eq!(map.status, "exact");
                assert_eq!(map.faces.len(), 3);
                let mut source = BTreeSet::new();
                for row in &map.faces {
                    for &id in &row.source_triangle_ids {
                        assert!(source.insert(id));
                    }
                }
                assert!(source.into_iter().eq(0..r.analysis.mesh.triangles.len()));
            }
        }
    }
}
