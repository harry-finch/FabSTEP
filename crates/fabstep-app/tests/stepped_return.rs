use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;
#[test]
fn measured_lands_bound_the_returning_wall() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for source in [
        include_bytes!("../../../fixtures/stepped_return/shaft-coarse.stl").as_slice(),
        include_bytes!("../../../fixtures/stepped_return/shaft-fine.stl").as_slice(),
    ] {
        let doc = convert_document_with_provenance(source, InputFormat::Stl, &o).unwrap();
        assert_eq!(
            (
                doc.report.cad.faces,
                doc.report.cad.cylindrical_faces,
                doc.report.cad.revolution_faces
            ),
            (5, 2, 1)
        );
        let display = doc.step.result_faces().unwrap();
        assert_eq!(display.source_correspondence.status, "exact");
        let report = &doc.report.bodies[0].conversion;
        let model = &report.rounded_recognition.as_ref().unwrap().model;
        let m = &model.revolved.mixed;
        let mut rings = m
            .rings
            .iter()
            .map(|r| (r.center[2], r.radius_mm))
            .collect::<Vec<_>>();
        rings.sort_by(|a, b| a.0.total_cmp(&b.0));
        for ((z, r), (height, radius)) in
            rings
                .into_iter()
                .zip([(-2., 3.), (0., 3.), (8., 10.5), (11., 10.5)])
        {
            assert!((z - height).abs() < 1e-5 && (r - radius).abs() < 1e-5);
        }
        let owners = &display.source_correspondence.faces;
        let mut ids = BTreeSet::new();
        for row in owners {
            let face = &display.result.faces[row.face_id];
            if face.family == ResultFaceFamily::Revolution {
                assert_eq!(face.adjacent_face_ids.len(), 2);
                assert!(face
                    .adjacent_face_ids
                    .iter()
                    .all(|&j| display.result.faces[j].family == ResultFaceFamily::Cylinder));
            }
            for &id in &row.source_triangle_ids {
                assert!(ids.insert(id));
                let z =
                    report.analysis.mesh.triangles[id].map(|i| report.analysis.mesh.vertices[i][2]);
                let kind = if [-2., 11.]
                    .into_iter()
                    .any(|h| z.iter().all(|v| (v - h).abs() < 1e-6))
                {
                    ResultFaceFamily::Plane
                } else if z.iter().all(|&v| v <= 1e-6) || z.iter().all(|&v| v >= 8. - 1e-6) {
                    ResultFaceFamily::Cylinder
                } else {
                    ResultFaceFamily::Revolution
                };
                assert_eq!(kind, face.family);
            }
        }
        assert!(ids.into_iter().eq(0..report.analysis.mesh.triangles.len()));
        assert!(model
            .smooth_revolution
            .as_ref()
            .unwrap()
            .axial_controls_mm
            .is_some());
        assert!((report.cad.volume_mm3 - 2541.1557576724435).abs() < 0.001);
        let check = report.fidelity.as_ref().unwrap();
        assert!(check.source_to_result_mm.max(check.result_to_source_mm) <= o.max_deviation_mm);
    }
}
