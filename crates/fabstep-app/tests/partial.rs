use fabstep_core::{analyze_stl, recognize_extrusion, MixedFaceRef, Options, ReconstructionMode};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}

#[test]
fn open_arcs_roundtrip_with_exact_endpoints_and_analytic_volumes() {
    // Independent geometric references from fixtures/generate_m2c.py.
    for (bytes, radius, volume, complete, planes) in [
        (
            include_bytes!("../../../fixtures/m2c_open_arc.stl").as_slice(),
            18.,
            53857.34658341286,
            0,
            6,
        ),
        (
            include_bytes!("../../../fixtures/m2c_open_arc_subdivided.stl").as_slice(),
            18.,
            53857.34658341286,
            0,
            6,
        ),
        (
            include_bytes!("../../../fixtures/m2c_open_arc_rotated.stl").as_slice(),
            18.,
            53857.34658341286,
            0,
            6,
        ),
        (
            include_bytes!("../../../fixtures/m2c_open_arc_mixed.stl").as_slice(),
            18.,
            53345.26698087772,
            2,
            7,
        ),
        (
            include_bytes!("../../../fixtures/m2c_open_arc_mixed_rotated.stl").as_slice(),
            18.,
            53345.26698087772,
            2,
            7,
        ),
        (
            include_bytes!("../../../fixtures/m2c_arc_other_angle.stl").as_slice(),
            21.,
            51282.20548677901,
            0,
            6,
        ),
    ] {
        let conversion = fabstep_app::convert_stl(bytes, &options()).unwrap();
        let report = &conversion.report;
        let mixed = report.mixed_recognition.as_ref().unwrap();
        assert_eq!(mixed.model.partial_cylinders.len(), 1);
        assert_eq!(mixed.model.cylinders.len(), complete);
        assert_eq!(report.cad.cylindrical_faces, complete + 1);
        assert_eq!(report.cad.planar_faces, planes);
        assert_eq!(report.cad.faces, planes + complete + 1);
        assert!((report.cad.volume_mm3 - volume).abs() < volume * 1e-8);
        assert_eq!(
            mixed.triangle_faces.len(),
            report.analysis.mesh.triangles.len()
        );
        assert!(mixed
            .triangle_faces
            .iter()
            .any(|owner| matches!(owner, MixedFaceRef::PartialCylinder { .. })));
        for arc in &mixed.model.arcs {
            assert!((arc.radius_mm - radius).abs() < 1e-8);
            for &index in &arc.vertices {
                assert_eq!(
                    mixed.model.vertices[index],
                    report.analysis.mesh.vertices[index]
                );
            }
        }
        let fidelity = report.fidelity.as_ref().unwrap();
        assert!(fidelity.source_to_result_mm < 0.05);
        assert!(fidelity.result_to_source_mm < 0.05);
        let step = std::str::from_utf8(&conversion.step_bytes).unwrap();
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), complete + 1);
    }
}

#[test]
fn invalid_partial_walls_and_inadequate_budgets_never_export() {
    for bytes in [
        include_bytes!("../../../fixtures/m2c_arc_conical.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_arc_bulge.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_arc_mismatch.stl").as_slice(),
    ] {
        let error = fabstep_app::convert_stl(bytes, &options()).err().unwrap();
        assert_eq!(error.code, "recognition.partial_unproven", "{error}");
    }
    let mut strict = options();
    strict.max_deviation_mm = 0.001;
    let error = fabstep_app::convert_stl(
        include_bytes!("../../../fixtures/m2c_open_arc.stl"),
        &strict,
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");

    let mut generous = options();
    generous.max_deviation_mm = 1.;
    let error = fabstep_app::convert_stl(
        include_bytes!("../../../fixtures/m2c_arc_few_segments.stl"),
        &generous,
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "recognition.unsupported");
}

#[test]
fn a_partial_arc_refines_an_already_recognized_extrusion_without_masking_budget_errors() {
    let bytes = include_bytes!("../../../fixtures/m2c_open_arc_hole.stl");
    let options = options();
    let analysis = analyze_stl(bytes, &options).unwrap();
    // M2a can prove this entire extrusion while keeping the open arc faceted.
    assert_eq!(
        recognize_extrusion(&analysis, &options)
            .unwrap()
            .report
            .cylinders,
        1
    );
    let converted = fabstep_app::convert_stl(bytes, &options).unwrap();
    assert!(converted.report.recognition.is_none());
    let mixed = converted.report.mixed_recognition.as_ref().unwrap();
    assert_eq!(mixed.model.partial_cylinders.len(), 1);
    assert_eq!(converted.report.cad.faces, 8);
    assert_eq!(converted.report.cad.cylindrical_faces, 2);
    assert!((converted.report.cad.volume_mm3 - 52952.5678991782).abs() < 1e-5);

    let strict = Options {
        max_deviation_mm: 0.009,
        ..options
    };
    assert!(recognize_extrusion(&analysis, &strict).is_ok());
    let error = fabstep_app::recognize_cylinders(&analysis, &strict)
        .err()
        .unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
