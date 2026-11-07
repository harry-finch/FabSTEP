use fabstep_core::{Options, ReconstructionMode};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}

#[test]
fn blind_holes_bosses_and_mixed_parts_keep_real_analytic_surfaces() {
    let pi = std::f64::consts::PI;
    for (bytes, faces, cylinders, volume) in [
        (
            include_bytes!("../../../fixtures/m2b_orthogonal_freecad.stl").as_slice(),
            10,
            2,
            6000. - 74. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_blind.stl").as_slice(),
            8,
            1,
            6000. - 54. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_blind_subdivided.stl").as_slice(),
            8,
            1,
            6000. - 54. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_blind_rotated.stl").as_slice(),
            8,
            1,
            6000. - 54. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_two_holes.stl").as_slice(),
            9,
            2,
            6000. - 144. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_boss.stl").as_slice(),
            8,
            1,
            6000. + 45. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_boss_blind.stl").as_slice(),
            10,
            2,
            6000. - 9. * pi,
        ),
        (
            include_bytes!("../../../fixtures/m2b_boss_blind_rotated.stl").as_slice(),
            10,
            2,
            6000. - 9. * pi,
        ),
    ] {
        let result = fabstep_app::convert_stl(bytes, &options()).unwrap();
        assert!(result.report.recognition.is_none());
        let mixed = result.report.mixed_recognition.unwrap();
        assert_eq!(
            mixed.triangle_faces.len(),
            result.report.analysis.mesh.triangles.len()
        );
        assert_eq!(mixed.model.cylinders.len(), cylinders);
        assert_eq!(result.report.cad.faces, faces);
        assert_eq!(result.report.cad.cylindrical_faces, cylinders);
        assert_eq!(result.report.cad.planar_faces, faces - cylinders);
        assert!((result.report.cad.volume_mm3 - volume).abs() < volume * 1e-8);
        let fidelity = result.report.fidelity.unwrap();
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(fidelity.source_to_result_mm < 0.05);
        assert!(fidelity.result_to_source_mm < 0.05);
        let step = std::str::from_utf8(&result.step_bytes).unwrap();
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), cylinders);
    }
}

#[test]
fn unsupported_hole_walls_and_floors_are_not_regularized_silently() {
    for bytes in [
        include_bytes!("../../../fixtures/m2b_conical_blind.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_nonflat_floor.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_bulged_boss.stl").as_slice(),
        include_bytes!("../../../fixtures/m2b_low_sample_blind.stl").as_slice(),
    ] {
        let error = fabstep_app::convert_stl(bytes, &options())
            .err()
            .expect("unsupported geometry must fail");
        assert!(error.code.starts_with("recognition."), "{error}");
    }
}

#[test]
fn a_missing_blind_floor_is_never_invented() {
    let bytes = include_bytes!("../../../fixtures/m2b_blind.stl");
    let source = std::str::from_utf8(bytes).unwrap();
    // Remove an actual source triangle; the topology must fail before any
    // analytic reconstruction can propose a replacement face.
    let (start, end) = source
        .match_indices("facet normal")
        .find_map(|(start, _)| {
            let end = start + source[start..].find("endfacet")? + "endfacet".len();
            let z: Vec<f64> = source[start..end]
                .lines()
                .filter_map(|line| {
                    let mut words = line.split_whitespace();
                    if words.next()? != "vertex" {
                        return None;
                    }
                    words.nth(2)?.parse().ok()
                })
                .collect();
            (z.len() == 3 && z.iter().all(|z| (*z - 4.0).abs() < 1e-10)).then_some((start, end))
        })
        .expect("fixture contains a real flat floor at z=4");
    let open = format!("{}{}", &source[..start], &source[end..]);
    let error = fabstep_app::convert_stl(open.as_bytes(), &options())
        .err()
        .expect("open source must fail");
    assert_eq!(error.code, "mesh.open");
}
