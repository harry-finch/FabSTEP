use fabstep_app::{
    convert_document, prepare_source_section, prepare_source_section_controlled, ConversionPhase,
    InputFormat,
};
use fabstep_core::{FabError, InputUnit, Options, ReconstructionMode};

#[test]
fn source_preparation_is_independent_of_recognition_and_planar_segmentation() {
    let bytes = include_bytes!("../../../fixtures/cube.stl");
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        plane_tolerance_mm: 0.2,
        ..Options::default()
    };
    assert!(convert_document(bytes, InputFormat::Stl, &options).is_err());
    let prepared = prepare_source_section(bytes, InputFormat::Stl, &options).unwrap();
    let metadata = prepared.metadata();
    assert_eq!(metadata.bodies.len(), 1);
    assert_eq!(metadata.bodies[0].solid_id, 0);
    assert_eq!(metadata.bodies[0].bounds_mm, [[0.; 3], [1.; 3]]);
    assert_eq!(metadata.bodies[0].triangles, 12);
    assert!(metadata.bodies[0].welded_vertices > 0);
    assert!(metadata
        .diagnostics
        .iter()
        .all(|d| d.code != "validation.native_required"));
    assert!(metadata
        .bodies
        .iter()
        .flat_map(|b| &b.diagnostics)
        .all(|d| d.code != "validation.native_required"));
    assert_eq!(
        metadata.settings.reconstruction,
        ReconstructionMode::Cylinders
    );
    assert!(metadata.bodies[0]
        .analysis_body_id
        .ends_with("/component/0"));
}

#[test]
fn source_preparation_uses_explicit_units_placements_and_authoritative_components() {
    let cube = prepare_source_section(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options {
            input_unit: InputUnit::Inch,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(cube.metadata().bounds_mm, [[0.; 3], [25.4; 3]]);
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    let placed = prepare_source_section(
        include_bytes!("../../../fixtures/m3_instances.3mf"),
        InputFormat::ThreeMf,
        &options,
    )
    .unwrap();
    assert_eq!(placed.metadata().bodies.len(), 2);
    assert_eq!(
        placed.metadata().bodies[1].bounds_mm,
        [[30., 5., 0.], [40., 15., 10.]]
    );
    let split = prepare_source_section(
        include_bytes!("../../../fixtures/m3_disconnected_mesh.3mf"),
        InputFormat::ThreeMf,
        &Options::default(),
    )
    .unwrap();
    assert_eq!(split.metadata().bodies.len(), 2);
    assert_eq!(split.metadata().bodies[1].solid_id, 1);
    assert_eq!(split.metadata().bodies[1].provenance.component_index, 1);
    assert_ne!(
        split.metadata().bodies[0].analysis_body_id,
        split.metadata().bodies[1].analysis_body_id
    );
}

#[test]
fn source_preparation_refuses_invalid_or_unseparated_document_before_any_selection() {
    let error = prepare_source_section(
        include_bytes!("../../../fixtures/v1_intersecting_plate.stl"),
        InputFormat::Stl,
        &Options {
            plane_tolerance_mm: 0.01,
            ..Options::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "cad.invalid_solid");
    assert!(error.body.is_some());
    assert!(prepare_source_section(
        include_bytes!("../../../fixtures/m3_open_mesh.3mf"),
        InputFormat::ThreeMf,
        &Options::default()
    )
    .is_err());
    for bytes in [
        include_bytes!("../../../fixtures/m3_overlap.3mf").as_slice(),
        include_bytes!("../../../fixtures/m3_touching.3mf"),
        include_bytes!("../../../fixtures/m3_nested_solids.3mf"),
    ] {
        assert_eq!(
            prepare_source_section(bytes, InputFormat::ThreeMf, &Options::default())
                .unwrap_err()
                .code,
            "cad.bodies_not_separated"
        );
    }
}

#[test]
fn source_preparation_checks_cancellation_at_native_validation_and_full_separation() {
    for (phase, index) in [
        (ConversionPhase::SourceValidation, Some(1)),
        (ConversionPhase::SourceSeparation, None),
    ] {
        let mut reached = Vec::new();
        let error = prepare_source_section_controlled(
            include_bytes!("../../../fixtures/m3_instances.3mf"),
            InputFormat::ThreeMf,
            &Options::default(),
            &mut |point| {
                reached.push(point);
                if point.phase == phase && point.body_index == index {
                    Err(FabError::new("test.cancelled", "cancel source preparation"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "test.cancelled");
        assert!(error.body.is_none());
        assert!(!reached.iter().any(|p| p.phase == ConversionPhase::Finish));
        assert!(reached
            .iter()
            .any(|p| p.phase == ConversionPhase::SourceValidation && p.body_index == Some(0)));
    }
}
