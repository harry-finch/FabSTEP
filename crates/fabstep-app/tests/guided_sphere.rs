use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{
    analyze_stl, recognize_automatic, GuidanceFamily, Options, ReconstructionMode, RoundedFaceRef,
};

#[test]
fn spherical_wall_is_verified_from_one_step_face_and_can_be_removed() {
    for (bytes, mode) in [
        (
            &include_bytes!("../../../fixtures/m5_hemisphere.stl")[..],
            ReconstructionMode::Planar,
        ),
        (
            &include_bytes!("../../../fixtures/m5b_spherical_zone.stl")[..],
            ReconstructionMode::Auto,
        ),
        (
            &include_bytes!("../../../fixtures/m5c_icosphere.stl")[..],
            ReconstructionMode::Auto,
        ),
    ] {
        let options = Options {
            reconstruction: mode,
            ..Options::default()
        };
        let base = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let analysis = analyze_stl(bytes, &options).unwrap();
        let r = recognize_automatic(&analysis, &options).unwrap();
        let expected: Vec<_> = r
            .triangle_faces
            .iter()
            .enumerate()
            .filter_map(|(t, f)| matches!(f, RoundedFaceRef::Sphere { .. }).then_some(t))
            .collect();
        assert!(!expected.is_empty());
        let faces = base.step.result_faces().unwrap();
        let row = faces
            .source_correspondence
            .faces
            .iter()
            .find(|r| r.source_triangle_ids.contains(&expected[0]))
            .unwrap();
        let guided = base
            .step
            .guided_surface(&[row.face_id], GuidanceFamily::Sphere)
            .unwrap();
        assert_eq!(guided.report.cad.spherical_faces, 1);
        assert_eq!(guided.report.guidance_hints[0].family, "sphere");
        assert_eq!(
            guided.report.guidance_hints[0].reconstructed_source_triangle_ids,
            expected
        );
        assert_eq!(
            guided
                .step
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
        assert!(guided
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "guidance.sphere_applied"));
        let replay = guided.step.recompute_guidance().unwrap();
        let removed = replay.step.remove_guidance(1).unwrap();
        assert_eq!(removed.report.cad.faces, base.report.cad.faces);
        if let Some(other) = faces.source_correspondence.faces.iter().find(|r| {
            r.source_triangle_ids
                .iter()
                .all(|id| !expected.contains(id))
        }) {
            assert_eq!(
                base.step
                    .guided_surface(&[row.face_id, other.face_id], GuidanceFamily::Sphere)
                    .err()
                    .unwrap()
                    .code,
                "guidance.family_unproven"
            );
        }
    }
}

#[test]
fn explicit_sphere_on_a_cylinder_is_refused() {
    let bytes = include_bytes!("../../../fixtures/m2_cylinder.stl");
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let base = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
    let faces = base.step.result_faces().unwrap();
    let face = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cylinder)
        .unwrap();
    assert_eq!(
        base.step
            .guided_surface(&[face.id], GuidanceFamily::Sphere)
            .err()
            .unwrap()
            .code,
        "guidance.family_unproven"
    );
}
