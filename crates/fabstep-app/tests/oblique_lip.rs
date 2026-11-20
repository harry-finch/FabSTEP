use fabstep_app::{
    convert_document_with_provenance, isolated_report::CompactWorkerReport, InputFormat,
};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn auto_exports_the_interrupted_cylinder_and_both_torus_cut_branches() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (stl, digest) in [
        (
            include_bytes!("../../../fixtures/oblique_lip/cap-coarse.stl").as_slice(),
            0xfb3f33101176a1de_u64,
        ),
        (
            include_bytes!("../../../fixtures/oblique_lip/cap-fine.stl").as_slice(),
            0x1d3d3a33d9bb79dd,
        ),
        (
            include_bytes!("../../../fixtures/oblique_lip/wide-coarse.stl").as_slice(),
            0x019554420ac0d6dc,
        ),
        (
            include_bytes!("../../../fixtures/oblique_lip/wide-fine.stl").as_slice(),
            0xe5b66961de5e5340,
        ),
    ] {
        let result = convert_document_with_provenance(stl, InputFormat::Stl, &options).unwrap();
        let body = &result.report.bodies[0].conversion;
        let fit = body.oblique_lip_recognition.as_ref().unwrap();
        let cad = &body.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces,
                cad.toroidal_faces
            ),
            (1, 5, 3, 1, 1)
        );
        let summary = CompactWorkerReport::from(&result.report);
        assert_eq!(
            summary.report.bodies[0]
                .rounded_recognition
                .as_ref()
                .unwrap()
                .planar_faces,
            3
        );
        let f = body.fidelity.as_ref().unwrap();
        assert!(
            f.source_samples > 0
                && f.result_samples > 0
                && f.source_to_result_mm.max(f.result_to_source_mm) <= options.max_deviation_mm
        );
        let faces = result.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let mut published = vec![usize::MAX; fit.triangle_faces.len()];
        for face in &faces.result.faces {
            let src = faces
                .source_correspondence
                .faces
                .iter()
                .find(|s| s.face_id == face.id)
                .unwrap();
            let first = src.source_triangle_ids[0];
            let slot = fit.triangle_faces[first];
            assert_eq!(
                face.family,
                match slot {
                    0..=2 => ResultFaceFamily::Plane,
                    3 => ResultFaceFamily::Cylinder,
                    4 => ResultFaceFamily::Torus,
                    _ => unreachable!(),
                }
            );
            for &i in &src.source_triangle_ids {
                assert_eq!(std::mem::replace(&mut published[i], slot), usize::MAX);
            }
        }
        assert_eq!(
            published
                .iter()
                .fold(0xcbf29ce484222325u64, |h, &v| (h ^ v as u64)
                    .wrapping_mul(0x100000001b3)),
            digest
        );
    }
}
