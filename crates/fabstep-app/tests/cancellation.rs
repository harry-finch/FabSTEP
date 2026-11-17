use fabstep_app::{
    convert_document, convert_document_controlled, convert_document_with_provenance,
    search_parameters, ConversionPhase, InputFormat,
};
use fabstep_core::{FabError, Options, ReconstructionMode};

#[test]
fn named_boundaries_stop_before_next_body_and_document_combination() {
    let bytes = include_bytes!("../../../fixtures/m3_instances.3mf");
    for (phase, body_index) in [
        (ConversionPhase::SourceSeparation, None),
        (ConversionPhase::Body, Some(1)),
        (ConversionPhase::SourceValidation, Some(0)),
        (ConversionPhase::Build, Some(0)),
        (ConversionPhase::Combine, None),
    ] {
        let mut reached = Vec::new();
        let error = convert_document_controlled(
            bytes,
            InputFormat::ThreeMf,
            &Options::default(),
            &mut |point| {
                reached.push(point);
                if point.phase == phase && point.body_index == body_index {
                    Err(FabError::new("test.cancelled", "controlled stop"))
                } else {
                    Ok(())
                }
            },
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "test.cancelled");
        assert_eq!(error.message, "controlled stop");
        assert!(
            error.body.is_none(),
            "callback errors keep their original context"
        );
        let last = reached.last().unwrap();
        assert_eq!((last.phase, last.body_index), (phase, body_index));
        assert!(!reached
            .iter()
            .any(|point| point.phase == ConversionPhase::Finish));
        if phase == ConversionPhase::Body {
            assert!(reached
                .iter()
                .any(|point| point.phase == ConversionPhase::Build && point.body_index == Some(0)));
            assert!(!reached
                .iter()
                .any(|point| point.phase == ConversionPhase::Build && point.body_index == Some(1)));
        }
    }
}

#[test]
fn analytic_phases_are_independently_cancellable_and_noop_control_preserves_report() {
    let bytes = include_bytes!("../../../fixtures/m2_plate_hole.stl");
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    for phase in [
        ConversionPhase::Recognition,
        ConversionPhase::AuditTessellation,
        ConversionPhase::AuditComparison,
    ] {
        let mut reached = Vec::new();
        let error = convert_document_controlled(bytes, InputFormat::Stl, &options, &mut |point| {
            reached.push(point.phase);
            if point.phase == phase {
                Err(FabError::new("engine.stale", "stale"))
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
        assert_eq!(error.code, "engine.stale");
        assert_eq!(reached.last(), Some(&phase));
        assert!(!reached.contains(&ConversionPhase::Finish));
    }
    let ordinary = convert_document(bytes, InputFormat::Stl, &options).unwrap();
    let controlled =
        convert_document_controlled(bytes, InputFormat::Stl, &options, &mut |_| Ok(())).unwrap();
    assert_eq!(
        serde_json::to_value(ordinary.report).unwrap(),
        serde_json::to_value(controlled.report).unwrap()
    );
}

#[test]
fn callback_failure_after_failed_analysis_aborts_the_entire_search_verbatim() {
    // Empty input fails analysis. The after-phase callback must still run and
    // its arbitrary failure must not become a rejected candidate.
    let mut before_analysis = true;
    let error =
        convert_document_controlled(&[], InputFormat::Stl, &Options::default(), &mut |point| {
            assert_eq!(point.phase, ConversionPhase::Analysis);
            if std::mem::take(&mut before_analysis) {
                Ok(())
            } else {
                Err(FabError::new("test.host_unavailable", "host failure"))
            }
        })
        .err()
        .unwrap();
    assert_eq!(error.code, "test.host_unavailable");
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut trial_started = false;
    let error = search_parameters(&[], InputFormat::Stl, &options, || {
        if std::mem::replace(&mut trial_started, true) {
            Err(FabError::new("test.host_unavailable", "host failure"))
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(error.code, "test.host_unavailable");
    assert_eq!(error.message, "host failure");
}

#[test]
fn guided_mutations_stop_before_rebuilding_and_preserve_the_accepted_source() {
    let planar = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    let original = planar.step.bytes().to_vec();
    let mut cancel = |_| Err(FabError::new("engine.stale", "stale"));
    assert_eq!(
        planar
            .step
            .guided_cylinder_controlled(&[0], &mut cancel)
            .err()
            .unwrap()
            .code,
        "engine.stale"
    );
    assert_eq!(
        planar
            .step
            .recompute_guidance_controlled(&mut cancel)
            .err()
            .unwrap()
            .code,
        "engine.stale"
    );
    assert_eq!(
        planar
            .step
            .remove_guidance_controlled(1, &mut cancel)
            .err()
            .unwrap()
            .code,
        "engine.stale"
    );
    assert_eq!(planar.step.bytes(), original);
}
