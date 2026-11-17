//! Bounded, explicit parameter proposals. Trials never publish an exportable STEP.
use crate::{
    document::convert_document_with_source_proofs, DocumentReport, InputFormat, ProcessingSettings,
    SourceProofCache,
};
use fabstep_core::{FabError, Options, ReconstructionMode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ParameterSearchReport {
    pub settings: ProcessingSettings,
    pub trials: Vec<ParameterTrial>,
    pub best_index: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ParameterTrial {
    pub cylinder_fit_tolerance_mm: f64,
    pub outcome: &'static str,
    pub faces: Option<usize>,
    pub curved_faces: Option<usize>,
    pub max_deviation_bound_mm: Option<f64>,
    pub max_sampled_distance_mm: Option<f64>,
    pub error: Option<FabError>,
}

fn candidates(options: &Options) -> Result<Vec<f64>, FabError> {
    if options.reconstruction == ReconstructionMode::Planar {
        return Err(FabError::new(
            "search.analytic_mode_required",
            "La recherche exige un mode de reconnaissance des formes courbes.",
        ));
    }
    for value in [
        options.weld_tolerance_mm,
        options.plane_tolerance_mm,
        options.cylinder_fit_tolerance_mm,
        options.max_deviation_mm,
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    let current = options.cylinder_fit_tolerance_mm;
    let ceiling = options.max_deviation_mm.min(0.01);
    let mut values: Vec<_> = [0.5, 1.0, 1.2, 1.5, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|factor| current * factor)
        .filter(|v| v.is_finite() && *v > 0.0 && (*v <= ceiling || *v == current))
        .collect();
    values.sort_by(f64::total_cmp);
    values.dedup();
    Ok(values)
}

fn summarize(fit: f64, result: Result<DocumentReport, FabError>) -> ParameterTrial {
    match result {
        Err(error) => ParameterTrial {
            cylinder_fit_tolerance_mm: fit,
            outcome: "rejected",
            faces: None,
            curved_faces: None,
            max_deviation_bound_mm: None,
            max_sampled_distance_mm: None,
            error: Some(error),
        },
        Ok(report) => {
            let bounds = report
                .bodies
                .iter()
                .flat_map(|body| {
                    let c = &body.conversion;
                    [
                        c.recognition.as_ref().map(|r| &r.report),
                        c.mixed_recognition.as_ref().map(|r| &r.report),
                        c.revolved_recognition.as_ref().map(|r| &r.report),
                        c.rounded_recognition.as_ref().map(|r| &r.report),
                        c.rounded_box_recognition.as_ref().map(|r| &r.report),
                    ]
                    .into_iter()
                    .flatten()
                    .map(|r| r.max_deviation_bound_mm)
                })
                .reduce(f64::max);
            let sampled = report
                .bodies
                .iter()
                .filter_map(|b| b.conversion.fidelity.as_ref())
                .map(|f| f.source_to_result_mm.max(f.result_to_source_mm))
                .reduce(f64::max);
            ParameterTrial {
                cylinder_fit_tolerance_mm: fit,
                outcome: "validated",
                faces: Some(report.cad.faces),
                curved_faces: Some(
                    report.cad.cylindrical_faces
                        + report.cad.conical_faces
                        + report.cad.spherical_faces
                        + report.cad.toroidal_faces,
                ),
                max_deviation_bound_mm: bounds,
                max_sampled_distance_mm: sampled,
                error: None,
            }
        }
    }
}

fn best_index(trials: &[ParameterTrial]) -> Option<usize> {
    trials
        .iter()
        .enumerate()
        .filter(|(_, t)| t.error.is_none() && t.faces.is_some())
        .min_by(|(_, a), (_, b)| {
            a.faces.cmp(&b.faces).then_with(|| {
                a.cylinder_fit_tolerance_mm
                    .total_cmp(&b.cylinder_fit_tolerance_mm)
            })
        })
        .map(|(index, _)| index)
}

/// At most seven document conversions, performed sequentially. Successful
/// exact-source native proofs are reused privately within this search; every
/// trial still builds, reimports, checks and audits its own result. Only
/// radial fit changes; the caller's deviation budget and source units remain fixed.
/// `check_current` runs between processing phases and before/after each trial.
/// Cancellation discards the whole proposal, including arbitrary callback
/// failures; it cannot interrupt an individual native CAD call.
pub fn search_parameters(
    bytes: &[u8],
    format: InputFormat,
    options: &Options,
    mut check_current: impl FnMut() -> Result<(), FabError>,
) -> Result<ParameterSearchReport, FabError> {
    search_with_source_proofs(
        bytes,
        format,
        options,
        &mut check_current,
        &mut SourceProofCache::default(),
    )
}

fn search_with_source_proofs(
    bytes: &[u8],
    format: InputFormat,
    options: &Options,
    check_current: &mut dyn FnMut() -> Result<(), FabError>,
    source_proofs: &mut SourceProofCache,
) -> Result<ParameterSearchReport, FabError> {
    let fits = candidates(options)?;
    let mut trials = Vec::with_capacity(fits.len());
    for fit in fits {
        check_current()?;
        let mut trial_options = options.clone();
        trial_options.cylinder_fit_tolerance_mm = fit;
        let mut interruption = None;
        let result = convert_document_with_source_proofs(
            bytes,
            format,
            &trial_options,
            &mut |_| {
                let result = check_current();
                if let Err(error) = &result {
                    interruption = Some(error.clone());
                }
                result
            },
            Some(source_proofs),
        )
        .map(|c| c.report);
        if let Some(error) = interruption {
            return Err(error);
        }
        check_current()?;
        trials.push(summarize(fit, result));
    }
    Ok(ParameterSearchReport {
        settings: options.into(),
        best_index: best_index(&trials),
        trials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{convert_document_controlled, ConversionCheckpoint, ConversionPhase};
    fn options() -> Options {
        Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        }
    }
    #[test]
    fn schedule_is_bounded_and_keeps_current_without_relaxing_other_options() {
        let mut o = options();
        let fits = candidates(&o).unwrap();
        assert_eq!(fits.len(), 7);
        assert_eq!(fits[0], o.cylinder_fit_tolerance_mm * 0.5);
        assert_eq!(fits[6], o.cylinder_fit_tolerance_mm * 10.0);
        o.max_deviation_mm = 1e-5;
        assert_eq!(candidates(&o).unwrap(), vec![5e-6, 1e-5]);
        o.cylinder_fit_tolerance_mm = 0.1;
        assert_eq!(candidates(&o).unwrap(), vec![0.1]);
        o.cylinder_fit_tolerance_mm = f64::NAN;
        assert_eq!(
            candidates(&o).unwrap_err().code,
            "options.invalid_tolerance"
        );
        assert_eq!(
            candidates(&Options::default()).unwrap_err().code,
            "search.analytic_mode_required"
        );
    }
    #[test]
    fn ranking_handles_nonmonotonic_results_and_prefers_stricter_ties() {
        let mut trials: Vec<_> = [1e-5, 1.2e-5, 1.5e-5, 5e-5]
            .into_iter()
            .map(|v| summarize(v, Err(FabError::new("test.refused", "refused"))))
            .collect();
        assert_eq!(best_index(&trials), None);
        for (i, faces) in [(1, 123), (2, 123), (3, 124)] {
            trials[i].error = None;
            trials[i].faces = Some(faces);
        }
        assert_eq!(best_index(&trials), Some(1));
    }
    #[test]
    fn cancellation_before_or_after_trial_never_returns_partial_proposal() {
        for cancel_on in [1, 2, 3] {
            let mut calls = 0;
            let result = search_parameters(&[], InputFormat::Stl, &options(), || {
                calls += 1;
                if calls == cancel_on {
                    Err(FabError::new("engine.stale", "stale"))
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.unwrap_err().code, "engine.stale");
            assert_eq!(calls, cancel_on);
        }
    }

    #[test]
    fn native_source_calls_are_eliminated_but_every_candidate_result_is_revalidated() {
        for (bytes, format, body_count) in [
            (
                include_bytes!("../../../fixtures/short_arcs/convex.stl").as_slice(),
                InputFormat::Stl,
                1,
            ),
            (
                include_bytes!("../../../fixtures/guided_cylinder_instances.3mf").as_slice(),
                InputFormat::ThreeMf,
                2,
            ),
        ] {
            let mut cache = SourceProofCache::default();
            let original = options();
            for fit in candidates(&original).unwrap() {
                let trial = Options {
                    cylinder_fit_tolerance_mm: fit,
                    ..original.clone()
                };
                let mut ordinary_trace = Vec::new();
                let ordinary = convert_document_controlled(bytes, format, &trial, &mut |point| {
                    ordinary_trace.push(point);
                    Ok(())
                })
                .unwrap();
                let mut cached_trace = Vec::new();
                let cached = convert_document_with_source_proofs(
                    bytes,
                    format,
                    &trial,
                    &mut |point| {
                        cached_trace.push(point);
                        Ok(())
                    },
                    Some(&mut cache),
                )
                .unwrap();
                // Full report comparison includes analyzed source meshes,
                // provenance, recognition, bounds, volume, settings and audit.
                assert_eq!(
                    serde_json::to_value(&cached.report).unwrap(),
                    serde_json::to_value(&ordinary.report).unwrap()
                );
                assert_eq!(cached_trace, ordinary_trace);
                assert_eq!(cached.report.cad.solids, body_count);
                for body in &cached.report.bodies {
                    assert!(body.conversion.fidelity.is_some());
                }
                for phase in [
                    ConversionPhase::Build,
                    ConversionPhase::AuditTessellation,
                    ConversionPhase::AuditComparison,
                ] {
                    assert_eq!(
                        cached_trace.iter().filter(|p| p.phase == phase).count(),
                        body_count * 2,
                        "every candidate must run {phase:?} for every body"
                    );
                }
                assert_eq!(
                    cached_trace
                        .iter()
                        .filter(|p| p.phase == ConversionPhase::Combine)
                        .count(),
                    if format == InputFormat::Stl && body_count == 1 {
                        0
                    } else {
                        2
                    }
                );
            }
            assert_eq!(cache.source_calls, body_count);
            assert_eq!(cache.separation_calls, usize::from(body_count > 1));
            let mut search_cache = SourceProofCache::default();
            let search = search_with_source_proofs(
                bytes,
                format,
                &original,
                &mut || Ok(()),
                &mut search_cache,
            )
            .unwrap();
            assert_eq!(search.trials.len(), 7);
            assert!(search
                .trials
                .iter()
                .all(|trial| trial.outcome == "validated"));
            assert_eq!(search_cache.source_calls, body_count);
            assert_eq!(search_cache.separation_calls, usize::from(body_count > 1));
        }
    }

    #[test]
    fn source_units_and_native_failures_invalidate_or_retry_proofs() {
        use fabstep_core::InputUnit;
        let mut cache = SourceProofCache::default();
        let cube = include_bytes!("../../../fixtures/cube.stl");
        for input_unit in [InputUnit::Mm, InputUnit::Inch] {
            let options = Options {
                input_unit,
                ..Options::default()
            };
            let ordinary =
                convert_document_controlled(cube, InputFormat::Stl, &options, &mut |_| Ok(()))
                    .unwrap();
            let cached = convert_document_with_source_proofs(
                cube,
                InputFormat::Stl,
                &options,
                &mut |_| Ok(()),
                Some(&mut cache),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(cached.report).unwrap(),
                serde_json::to_value(ordinary.report).unwrap()
            );
        }
        assert_eq!(
            cache.source_calls, 2,
            "changed millimetre geometry needs a new proof"
        );
        for (bytes, format, separation) in [
            (
                include_bytes!("../../../fixtures/v1_intersecting_plate.stl").as_slice(),
                InputFormat::Stl,
                false,
            ),
            (
                include_bytes!("../../../fixtures/m3_touching.3mf").as_slice(),
                InputFormat::ThreeMf,
                true,
            ),
        ] {
            let mut failed_cache = SourceProofCache::default();
            let options = Options::default();
            let expected = convert_document_controlled(bytes, format, &options, &mut |_| Ok(()))
                .err()
                .unwrap();
            for _ in 0..2 {
                let actual = convert_document_with_source_proofs(
                    bytes,
                    format,
                    &options,
                    &mut |_| Ok(()),
                    Some(&mut failed_cache),
                )
                .err()
                .unwrap();
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    serde_json::to_value(&expected).unwrap()
                );
            }
            assert_eq!(failed_cache.source_calls, if separation { 0 } else { 2 });
            assert_eq!(
                failed_cache.separation_calls,
                if separation { 2 } else { 0 }
            );
        }
    }

    #[test]
    fn recognition_refusals_keep_priority_and_do_not_establish_source_proofs() {
        let bytes = include_bytes!("../../../fixtures/cube.stl");
        let mut cache = SourceProofCache::default();
        let mut trace = Vec::new();
        let ordinary =
            convert_document_controlled(bytes, InputFormat::Stl, &options(), &mut |_| Ok(()))
                .err()
                .unwrap();
        let actual = convert_document_with_source_proofs(
            bytes,
            InputFormat::Stl,
            &options(),
            &mut |point| {
                trace.push(point.phase);
                Ok(())
            },
            Some(&mut cache),
        )
        .err()
        .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(ordinary).unwrap()
        );
        assert_eq!(cache.source_calls, 0);
        assert!(!trace.contains(&ConversionPhase::SourceValidation));
        assert!(!trace.contains(&ConversionPhase::Build));
        // A previously successful source proof still cannot skip recognition.
        convert_document_with_source_proofs(
            bytes,
            InputFormat::Stl,
            &Options::default(),
            &mut |_| Ok(()),
            Some(&mut cache),
        )
        .unwrap();
        let error = convert_document_with_source_proofs(
            bytes,
            InputFormat::Stl,
            &options(),
            &mut |_| Ok(()),
            Some(&mut cache),
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "recognition.surface_unproven");
        assert_eq!(cache.source_calls, 1);
    }

    #[test]
    #[ignore = "opt-in full private report parity; requires FABSTEP_PROFILE_PRIVATE_STL"]
    fn private_search_candidates_preserve_full_reports_errors_and_phase_traces() {
        let path = std::env::var_os("FABSTEP_PROFILE_PRIVATE_STL")
            .expect("FABSTEP_PROFILE_PRIVATE_STL must name the private STL");
        let bytes = std::fs::read(path).expect("read private STL");
        let mut cache = SourceProofCache::default();
        let original = options();
        let mut accepted = 0;
        let mut refused = 0;
        let mut combined_trace = Vec::new();
        for fit in candidates(&original).unwrap() {
            let trial = Options {
                cylinder_fit_tolerance_mm: fit,
                ..original.clone()
            };
            let mut ordinary_trace = Vec::new();
            let ordinary =
                convert_document_controlled(&bytes, InputFormat::Stl, &trial, &mut |point| {
                    ordinary_trace.push(point);
                    Ok(())
                });
            let mut cached_trace = Vec::new();
            let cached = convert_document_with_source_proofs(
                &bytes,
                InputFormat::Stl,
                &trial,
                &mut |point| {
                    cached_trace.push(point);
                    Ok(())
                },
                Some(&mut cache),
            );
            assert_eq!(cached_trace, ordinary_trace);
            match (cached, ordinary) {
                (Ok(cached), Ok(ordinary)) => {
                    assert_eq!(
                        serde_json::to_value(cached.report).unwrap(),
                        serde_json::to_value(ordinary.report).unwrap()
                    );
                    accepted += 1;
                }
                (Err(cached), Err(ordinary)) => {
                    assert!(cached.code.starts_with("recognition."));
                    assert_eq!(
                        serde_json::to_value(cached).unwrap(),
                        serde_json::to_value(ordinary).unwrap()
                    );
                    refused += 1;
                }
                _ => panic!("cached and ordinary candidates differ at fit {fit}"),
            }
            combined_trace.extend(cached_trace);
        }
        assert_eq!((refused, accepted), (2, 5));
        assert_eq!(cache.source_calls, 1);
        assert_eq!(cache.separation_calls, 0);
        for phase in [
            ConversionPhase::SourceValidation,
            ConversionPhase::Build,
            ConversionPhase::AuditTessellation,
            ConversionPhase::AuditComparison,
        ] {
            assert_eq!(
                combined_trace.iter().filter(|p| p.phase == phase).count(),
                accepted * 2,
                "every accepted candidate retains its own {phase:?} boundaries"
            );
        }
    }

    #[test]
    fn cancellation_brackets_cache_hits_and_aborts_search_without_partial_proposal() {
        let bytes = include_bytes!("../../../fixtures/guided_cylinder_instances.3mf");
        let mut cache = SourceProofCache::default();
        let mut trace: Vec<ConversionCheckpoint> = Vec::new();
        convert_document_with_source_proofs(
            bytes,
            InputFormat::ThreeMf,
            &options(),
            &mut |point| {
                trace.push(point);
                Ok(())
            },
            Some(&mut cache),
        )
        .unwrap();
        for phase in [
            ConversionPhase::SourceSeparation,
            ConversionPhase::SourceValidation,
        ] {
            for boundary in 0..2 {
                let stop_at = trace
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.phase == phase)
                    .nth(boundary)
                    .unwrap()
                    .0;
                let mut checks = 0;
                let error = search_with_source_proofs(
                    bytes,
                    InputFormat::ThreeMf,
                    &options(),
                    &mut || {
                        checks += 1;
                        // Search adds a before-trial check before the trace.
                        if checks == stop_at + 2 {
                            Err(FabError::new(
                                "test.host_failure",
                                "stop on cached boundary",
                            ))
                        } else {
                            Ok(())
                        }
                    },
                    &mut cache,
                )
                .unwrap_err();
                assert_eq!(checks, stop_at + 2);
                assert_eq!(error.code, "test.host_failure");
                assert_eq!(error.message, "stop on cached boundary");
                assert!(error.body.is_none());
                assert_eq!(cache.source_calls, 2);
                assert_eq!(cache.separation_calls, 1);
            }
        }
    }
}
