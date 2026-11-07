use fabstep_core::{Options, ReconstructionMode};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}

#[test]
fn recognized_extrusions_roundtrip_as_cylindrical_step_surfaces() {
    let pi = std::f64::consts::PI;
    for (bytes, faces, cylinders, volume) in [
        (
            include_bytes!("../../../fixtures/m2_cylinder.stl").as_slice(),
            3,
            1,
            pi * 100.0 * 20.0,
        ),
        (
            include_bytes!("../../../fixtures/m2_cylinder_subdivided.stl").as_slice(),
            3,
            1,
            pi * 100.0 * 20.0,
        ),
        (
            include_bytes!("../../../fixtures/m2_cylinder_rotated.stl").as_slice(),
            3,
            1,
            pi * 100.0 * 20.0,
        ),
        (
            include_bytes!("../../../fixtures/m2_tube.stl").as_slice(),
            4,
            2,
            pi * 75.0 * 20.0,
        ),
        (
            include_bytes!("../../../fixtures/m2_plate_hole.stl").as_slice(),
            7,
            1,
            (600.0 - pi * 9.0) * 5.0,
        ),
    ] {
        let result = fabstep_app::convert_stl(bytes, &options()).unwrap();
        assert_eq!(result.report.cad.faces, faces);
        assert_eq!(result.report.cad.cylindrical_faces, cylinders);
        assert_eq!(result.report.cad.planar_faces, faces - cylinders);
        assert!((result.report.cad.volume_mm3 - volume).abs() < volume * 1e-8);
        let recognition = result.report.recognition.unwrap();
        assert!(recognition.report.max_deviation_bound_mm <= options().max_deviation_mm);
        assert!(recognition.report.max_deviation_bound_mm > 0.0);
        assert_eq!(recognition.report.cylinders, cylinders);
        let step = std::str::from_utf8(&result.step_bytes).unwrap();
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), cylinders);
        assert!(!step.contains("B_SPLINE_SURFACE"));
    }
}

#[test]
fn planar_default_preserves_facets_and_analytic_mode_never_silently_falls_back() {
    let bytes = include_bytes!("../../../fixtures/m2_cylinder.stl");
    let planar = fabstep_app::convert_stl(bytes, &Options::default()).unwrap();
    assert_eq!(planar.report.cad.cylindrical_faces, 0);
    assert_eq!(planar.report.cad.faces, 66);
    assert!(planar.report.recognition.is_none());
    for bytes in [
        include_bytes!("../../../fixtures/cube.stl").as_slice(),
        include_bytes!("../../../fixtures/m2_conical_frustum.stl").as_slice(),
        include_bytes!("../../../fixtures/m2_waisted.stl").as_slice(),
    ] {
        let error = fabstep_app::convert_stl(bytes, &options())
            .err()
            .expect("must reject unsupported extrusion");
        assert!(error.code.starts_with("recognition."), "{error}");
    }
}

#[test]
fn insufficient_deviation_budget_refuses_the_inscribed_polygon() {
    let strict = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    assert!(
        fabstep_app::convert_stl(include_bytes!("../../../fixtures/m2_cylinder.stl"), &strict)
            .is_err()
    );
}

#[test]
fn rotated_binary_f32_keeps_source_validation_and_recognizes_the_extrusion() {
    // Binary STL uses f32 coordinates. Rounding makes merged cap polygons
    // slightly nonplanar, while the original triangles still form a valid solid.
    let analysis = fabstep_core::analyze_stl(
        include_bytes!("../../../fixtures/m2_cylinder_rotated.stl"),
        &Options::default(),
    )
    .unwrap();
    let mut bytes = vec![0_u8; 80];
    bytes.extend_from_slice(&(analysis.mesh.triangles.len() as u32).to_le_bytes());
    for triangle in &analysis.mesh.triangles {
        bytes.extend_from_slice(&[0_u8; 12]);
        for &index in triangle {
            for value in analysis.mesh.vertices[index] {
                bytes.extend_from_slice(&(value as f32).to_le_bytes());
            }
        }
        bytes.extend_from_slice(&[0_u8; 2]);
    }
    let result = fabstep_app::convert_stl(&bytes, &options()).unwrap();
    assert_eq!(result.report.cad.faces, 3);
    assert_eq!(result.report.cad.cylindrical_faces, 1);
    assert!((result.report.cad.volume_mm3 - std::f64::consts::PI * 2000.0).abs() < 0.001);
    assert!(
        result
            .report
            .recognition
            .unwrap()
            .report
            .max_deviation_bound_mm
            < 0.05
    );
}
