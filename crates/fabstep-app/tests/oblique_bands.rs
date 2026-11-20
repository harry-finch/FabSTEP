use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{
    oblique_band::{Family, ObliqueBand},
    oblique_lip::ObliqueLip,
    Options, ReconstructionMode,
};

#[test]
fn native_cut_branches_persist_on_both_families() {
    for (family, rb, rt, h, offset, slope) in [
        (Family::Cone, 12., 8., 8., -3., 0.23),
        (
            Family::Sphere,
            (144_f64 - (7_f64 * 0.38).powi(2)).sqrt(),
            (144_f64 - (7_f64 * 0.62).powi(2)).sqrt(),
            7.,
            -3.,
            0.24,
        ),
    ] {
        let model = ObliqueBand {
            family,
            frame: ObliqueLip {
                origin: [3., -2., 1.],
                axis: [0., 0.6, 0.8],
                radial: [1., 0., 0.],
                outer_radius_mm: rb,
                major_radius_mm: rt,
                height_mm: h,
                cut_offset_mm: offset,
                slope,
            },
        };
        let step = fabstep_cad::build_oblique_band(&model, Some(&[4, 3, 2, 1])).unwrap();
        assert_eq!(
            (
                step.report.solids,
                step.report.faces,
                step.report.planar_faces
            ),
            (1, 4, 3)
        );
        assert!((step.report.volume_mm3 - model.volume()).abs() < model.volume() * 1e-8);
    }
}

#[test]
fn auto_exports_four_source_owned_faces_and_the_expected_volume() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (name, cone, volume) in [
        ("cone1", true, 933.4868103465583),
        ("cone2", true, 1486.0918540042196),
        ("sphere1", false, 1175.27733667),
        ("sphere3", false, 2298.292477105),
    ] {
        for density in ["coarse", "fine"] {
            let key = format!("../../fixtures/oblique_bands/{name}-{density}");
            let input = std::fs::read(format!("{key}.stl")).unwrap();
            let owners = std::fs::read(format!("{key}.owners")).unwrap();
            let out = convert_document_with_provenance(&input, InputFormat::Stl, &options)
                .unwrap_or_else(|e| panic!("{key}: {e:?}"));
            let body = &out.report.bodies[0].conversion;
            let fit = body
                .oblique_band_recognition
                .as_ref()
                .unwrap_or_else(|| panic!("{key}"));
            let cad = &body.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.conical_faces,
                    cad.spherical_faces
                ),
                (1, 4, 3, usize::from(cone), usize::from(!cone))
            );
            assert!((cad.volume_mm3 - volume).abs() < 0.001);
            let fidelity = body.fidelity.as_ref().unwrap();
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(
                fidelity
                    .source_to_result_mm
                    .max(fidelity.result_to_source_mm)
                    <= options.max_deviation_mm
            );
            let faces = out.step.result_faces().unwrap();
            assert_eq!(faces.source_correspondence.status, "exact");
            let mut published = vec![u8::MAX; owners.len()];
            for face in &faces.result.faces {
                let source = faces
                    .source_correspondence
                    .faces
                    .iter()
                    .find(|s| s.face_id == face.id)
                    .unwrap();
                let slot = fit.triangle_faces[source.source_triangle_ids[0]];
                assert_eq!(
                    face.family,
                    match slot {
                        0..=2 => fabstep_cad::ResultFaceFamily::Plane,
                        3 if cone => fabstep_cad::ResultFaceFamily::Cone,
                        3 => fabstep_cad::ResultFaceFamily::Sphere,
                        _ => unreachable!(),
                    }
                );
                for &i in &source.source_triangle_ids {
                    assert_eq!(std::mem::replace(&mut published[i], slot as u8), u8::MAX);
                }
            }
            assert_eq!(published, owners);
        }
    }
}

#[test]
fn tangent_domains_and_duplicate_face_tokens_are_declined_natively() {
    let frame = ObliqueLip {
        origin: [0.; 3],
        axis: [0., 0., 1.],
        radial: [1., 0., 0.],
        outer_radius_mm: 12.,
        major_radius_mm: 8.,
        height_mm: 8.,
        cut_offset_mm: -3.,
        slope: 0.23,
    };
    let mut model = ObliqueBand {
        family: Family::Cone,
        frame,
    };
    let duplicate = fabstep_cad::build_oblique_band(&model, Some(&[4, 3, 3, 1]));
    assert_eq!(duplicate.err().unwrap().code, "cad.invalid_model");
    model.frame.slope = 0.5;
    assert_eq!(
        fabstep_cad::build_oblique_band(&model, None)
            .err()
            .unwrap()
            .code,
        "cad.invalid_model"
    );
    model.family = Family::Sphere;
    model.frame.cut_offset_mm = -12.;
    assert_eq!(
        fabstep_cad::build_oblique_band(&model, None)
            .err()
            .unwrap()
            .code,
        "cad.invalid_model"
    );
}
