use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{read_stl_mesh, InputUnit, Options, ReconstructionMode};
use std::collections::BTreeSet;

#[test]
fn auto_exports_one_solid_and_preserves_the_four_distinct_wall_owners() {
    for (source, expected) in [
        (
            include_bytes!("../../../fixtures/composed_cross_drill/disjoint-coarse.stl").as_slice(),
            678,
        ),
        (
            include_bytes!("../../../fixtures/composed_cross_drill/disjoint-fine.stl").as_slice(),
            1290,
        ),
    ] {
        let options = Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        };
        let doc = convert_document_with_provenance(source, InputFormat::Stl, &options).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces
            ),
            (1, 10, 6, 4)
        );
        assert!((cad.volume_mm3 - 5733.466076).abs() < 1e-4);
        let fidelity = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(fidelity.source_to_result_mm <= options.max_deviation_mm);
        assert!(fidelity.result_to_source_mm <= options.max_deviation_mm);
        let mesh = read_stl_mesh(source, InputUnit::Mm).unwrap();
        let inspected = doc.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact");
        assert_eq!(inspected.source_correspondence.faces.len(), 10);
        let mut seen = BTreeSet::new();
        let mut cylinder_triangles = 0;
        let mut fillets = 0;
        for owner in &inspected.source_correspondence.faces {
            assert!(!owner.source_triangle_ids.is_empty());
            for &id in &owner.source_triangle_ids {
                assert!(seen.insert(id));
            }
            let face = &inspected.result.faces[owner.face_id];
            if face.family != ResultFaceFamily::Cylinder {
                continue;
            }
            cylinder_triangles += owner.source_triangle_ids.len();
            let adjacent = &face.adjacent_face_ids;
            if adjacent
                .iter()
                .all(|&n| inspected.result.faces[n].family == ResultFaceFamily::Plane)
            {
                assert_eq!(adjacent.len(), 4);
                fillets += 1;
                for &id in &owner.source_triangle_ids {
                    for v in mesh.triangles[id] {
                        let [_, y, z] = mesh.vertices[v];
                        assert!(((y + 8.5).hypot(z - 7.5) - 1.5).abs() < 1e-5);
                    }
                }
            }
        }
        assert_eq!(fillets, 1);
        assert_eq!(cylinder_triangles, expected);
        assert_eq!(seen.len(), mesh.triangles.len());
    }
}
