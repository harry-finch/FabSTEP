use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{analyze_stl, recognize_cubic_prism, Options, ReconstructionMode};
#[test]
fn source_certifies_both_caps_and_the_whole_cubic_wall() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (bytes, hole, vol) in [
        (
            include_bytes!("../../../fixtures/cubic_extrusion/cam-coarse.stl").as_slice(),
            false,
            267.,
        ),
        (
            include_bytes!("../../../fixtures/cubic_extrusion/cam-fine.stl").as_slice(),
            false,
            267.,
        ),
        (
            include_bytes!("../../../fixtures/cubic_extrusion/guide-coarse.stl").as_slice(),
            true,
            490.9933644457817,
        ),
        (
            include_bytes!("../../../fixtures/cubic_extrusion/guide-fine.stl").as_slice(),
            true,
            490.9933644457817,
        ),
    ] {
        let a = analyze_stl(bytes, &o).unwrap();
        let cert = recognize_cubic_prism(&a, &o)
            .unwrap()
            .expect("expected cubic profile");
        assert_eq!(cert.model.bore.is_some(), hole);
        let converted = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
        let compact = fabstep_app::isolated_report::CompactWorkerReport::from(&converted.report);
        assert_eq!(
            compact.report.bodies[0]
                .recognition
                .as_ref()
                .unwrap()
                .report
                .diagnostics[0]
                .code,
            "recognition.cubic_extrusion_assumed"
        );
        let cad = &converted.report.cad;
        assert!((cad.volume_mm3 - vol).abs() < 0.0001);
        assert_eq!(
            (cad.faces, cad.planar_faces, cad.cylindrical_faces),
            (6 + usize::from(hole), 5, usize::from(hole))
        );
        let body = &converted.report.bodies[0].conversion;
        let recon = body.cubic_prism_recognition.as_ref().unwrap();
        assert!((recon.model.height_mm - 5.).abs() < 1e-6);
        let fidelity = body.fidelity.as_ref().unwrap();
        assert!(
            fidelity.source_samples > 0
                && fidelity.result_samples > 0
                && fidelity
                    .source_to_result_mm
                    .max(fidelity.result_to_source_mm)
                    <= o.max_deviation_mm
        );
        let inspected = converted.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact");
        let all = inspected
            .source_correspondence
            .faces
            .iter()
            .flat_map(|f| f.source_triangle_ids.iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(all.len(), a.mesh.triangles.len());
        assert!((0..all.len()).eq(all.into_iter().collect::<std::collections::BTreeSet<_>>()));
        let swept = inspected
            .result
            .faces
            .iter()
            .find(|f| f.family == fabstep_cad::ResultFaceFamily::Extrusion)
            .unwrap();
        assert_eq!(swept.adjacent_face_ids.len(), 4);
        assert!((cert.report.expected_volume_mm3 - vol).abs() < 0.0001);
        assert_eq!(cert.triangle_faces.len(), a.mesh.triangles.len());
        assert!(cert.report.max_deviation_bound_mm < o.max_deviation_mm);
    }
}

#[test]
fn bent_twisted_and_drafted_bodies_refuse_the_straight_sweep() {
    let original = include_bytes!("../../../fixtures/cubic_extrusion/cam-coarse.stl");
    let options = Options::default();
    for deform in 0..3 {
        let mut data = original.to_vec();
        for tri in data[84..].chunks_exact_mut(50) {
            for vertex in tri[12..48].chunks_exact_mut(12) {
                let read =
                    |i: usize| f32::from_le_bytes(vertex[i..i + 4].try_into().unwrap()) as f64;
                let (x, y, t) = (read(0), read(4), read(8) / 5.);
                let (x, y) = match deform {
                    0 => (x, y + 0.35 * 4. * t * (1. - t)),
                    1 => {
                        let (s, c) = (0.06 * t).sin_cos();
                        (6. + (x - 6.) * c - y * s, (x - 6.) * s + y * c)
                    }
                    _ => (6. + (x - 6.) * (1. + 0.005 * t), y * (1. + 0.005 * t)),
                };
                vertex[..4].copy_from_slice(&(x as f32).to_le_bytes());
                vertex[4..8].copy_from_slice(&(y as f32).to_le_bytes());
            }
        }
        let analysis = analyze_stl(&data, &options).unwrap();
        assert!(recognize_cubic_prism(&analysis, &options)
            .unwrap()
            .is_none());
    }
}

#[test]
fn native_domain_and_reimport_reject_unverified_profiles() {
    let source = include_bytes!("../../../fixtures/cubic_extrusion/cam-coarse.stl");
    let options = Options::default();
    let analysis = analyze_stl(source, &options).unwrap();
    let prism = recognize_cubic_prism(&analysis, &options)
        .unwrap()
        .unwrap()
        .model;
    let mut invalid = prism.clone();
    invalid.bore = Some([0., 0., 4.]);
    assert_eq!(
        fabstep_cad::build_cubic_prism(&invalid, None)
            .err()
            .unwrap()
            .code,
        "cad.invalid_model"
    );
    assert!(fabstep_cad::build_cubic_prism(&prism, Some(&[1, 2, 3, 4, 4, 6])).is_err());
    let native = fabstep_cad::build_cubic_prism(&prism, None).unwrap();
    let mut step = std::str::from_utf8(&native.bytes).unwrap().to_owned();
    // Shorten only the actual sweep vector. The trim wires remain on both
    // disks, so their two translated control polygons cannot agree with it.
    let extrusion = step
        .lines()
        .find(|row| row.contains("SURFACE_OF_LINEAR_EXTRUSION("))
        .unwrap();
    let vector = extrusion
        .rsplit_once('#')
        .unwrap()
        .1
        .split(')')
        .next()
        .unwrap();
    let entity = format!("#{vector} = VECTOR(");
    let start = step.find(&entity).unwrap();
    let line = &step[start..];
    let stop = line.find(");").unwrap();
    let at = line[..stop].rfind(',').unwrap() + 1;
    step.replace_range(start + at..start + stop, "0.");
    assert!(fabstep_cad::result_faces(step.as_bytes()).is_err());
}
