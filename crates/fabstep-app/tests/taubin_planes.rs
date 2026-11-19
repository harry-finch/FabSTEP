//! The upper interior vertices of this rotated f32 block carry a measured
//! 0.000000175 mm perturbation. Its four sides and the roof rim are planar.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use std::collections::BTreeSet;

const ROOF: &[u8] = include_bytes!("../../../fixtures/r116-quantized-roof.stl");

#[test]
fn stable_source_seeds_reimport_six_planes_with_exact_ownership() {
    for reconstruction in [ReconstructionMode::Auto, ReconstructionMode::Planar] {
        let options = Options {
            reconstruction,
            ..Options::default()
        };
        let doc = convert_document_with_provenance(ROOF, InputFormat::Stl, &options).unwrap();
        let cad = &doc.report.cad;
        assert_eq!((cad.solids, cad.faces, cad.planar_faces), (1, 6, 6));
        assert!((cad.volume_mm3 - 0.48 * 0.32 * 0.04).abs() < 3e-9);
        let result = doc.step.result_faces().unwrap();
        let map = &result.source_correspondence;
        assert_eq!(map.status, "exact");
        assert_eq!(map.faces.len(), 6);
        let mut seen = BTreeSet::new();
        for f in &map.faces {
            let id = f.source_triangle_ids[0] % 40;
            let expected: Vec<_> = (0..320)
                .filter(|t| {
                    let record = t % 40;
                    if id < 32 {
                        record < 32 && (record % 4) / 2 == (id % 4) / 2
                    } else {
                        record >= 32 && (record - 32) / 2 == (id - 32) / 2
                    }
                })
                .collect();
            assert_eq!(f.source_triangle_ids, expected);
            assert!(seen.insert(expected[0]));
        }
        let source = read_stl_mesh(ROOF, InputUnit::Mm).unwrap();
        let f = fabstep_verify::compare_meshes(
            &source,
            &result.result.mesh,
            result.result.deflection_mm,
        )
        .unwrap();
        assert!(f.source_to_result_mm < 2e-7 && f.result_to_source_mm < 2e-7);
    }
}
