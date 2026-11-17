//! Opt-in diagnostic, not a benchmark or a performance regression threshold.
//! Run with: cargo test -p fabstep-app --test parameter_search_profile --offline -- --ignored
//! FABSTEP_PROFILE_REPETITIONS (2..=3) and FABSTEP_PROFILE_PRIVATE_STL are optional.
use fabstep_app::{
    convert_document_controlled, preview_document, search_parameters, ConversionCheckpoint,
    ConversionPhase, DocumentReport, InputFormat, ParameterTrial, ProcessingSettings,
};
use fabstep_core::{FabError, Options, ReconstructionMode};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

// Only .run phases have paired boundaries. Body is an inclusive envelope and
// Finish is a single check; neither belongs in exclusive phase totals.
#[derive(Default)]
struct Recorder {
    open: BTreeMap<String, Instant>,
    durations_ms: BTreeMap<String, f64>,
    calls: BTreeMap<String, usize>,
    body_open: BTreeMap<usize, Instant>,
    body_ms: BTreeMap<usize, f64>,
    finish_checks: usize,
}
impl Recorder {
    fn check(&mut self, checkpoint: ConversionCheckpoint) -> Result<(), FabError> {
        let now = Instant::now();
        match checkpoint.phase {
            ConversionPhase::Body => {
                let body = checkpoint.body_index.unwrap();
                if let Some(start) = self.body_open.remove(&body) {
                    self.body_ms
                        .insert(body, (now - start).as_secs_f64() * 1000.0);
                } else {
                    self.body_open.insert(body, now);
                }
            }
            ConversionPhase::Finish => self.finish_checks += 1,
            phase => {
                let name = format!("{phase:?}");
                if let Some(start) = self.open.remove(&name) {
                    *self.durations_ms.entry(name.clone()).or_default() +=
                        (now - start).as_secs_f64() * 1000.0;
                    *self.calls.entry(name).or_default() += 1;
                } else {
                    // The public pipeline currently has no nested .run phases.
                    assert!(self.open.is_empty(), "exclusive phase unexpectedly nested");
                    self.open.insert(name, now);
                }
            }
        }
        Ok(())
    }
}

fn summary(fit: f64, result: Result<DocumentReport, FabError>) -> ParameterTrial {
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
        .filter(|(_, t)| t.faces.is_some() && t.error.is_none())
        .min_by(|(_, a), (_, b)| {
            a.faces.cmp(&b.faces).then_with(|| {
                a.cylinder_fit_tolerance_mm
                    .total_cmp(&b.cylinder_fit_tolerance_mm)
            })
        })
        .map(|(i, _)| i)
}

fn profile(bytes: &[u8], format: InputFormat, options: &Options) -> (Value, Vec<ParameterTrial>) {
    // Separate import/preview probe: not an instrumented subsection of Analysis.
    let preview_start = Instant::now();
    let preview = preview_document(bytes, format, options).unwrap();
    let standalone_preview_ms = preview_start.elapsed().as_secs_f64() * 1000.0;
    let triangles: usize = preview.bodies.iter().map(|b| b.mesh.triangles.len()).sum();
    let input_bodies = preview.bodies.len();
    drop(preview);
    let search_start = Instant::now();
    let mut records = Vec::new();
    let mut trials = Vec::new();
    // Deliberately reconstructed schedule, checked against real search below.
    for factor in [0.5, 1.0, 1.2, 1.5, 2.0, 5.0, 10.0] {
        let trial_options = Options {
            cylinder_fit_tolerance_mm: options.cylinder_fit_tolerance_mm * factor,
            ..options.clone()
        };
        let mut recorder = Recorder::default();
        let start = Instant::now();
        let result = convert_document_controlled(bytes, format, &trial_options, &mut |point| {
            recorder.check(point)
        });
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(recorder.open.is_empty(), "unpaired .run phase");
        let native_kernel = result.as_ref().ok().map(|c| c.report.cad.kernel.clone());
        if let Ok(conversion) = &result {
            assert_eq!(
                conversion.report.settings,
                ProcessingSettings::from(&trial_options)
            );
            assert_eq!(conversion.report.cad.solids, conversion.report.bodies.len());
            for body in &conversion.report.bodies {
                assert_eq!(
                    body.conversion.settings,
                    ProcessingSettings::from(&trial_options)
                );
            }
            assert_eq!(recorder.finish_checks, 1);
            assert!(recorder.body_open.is_empty());
        }
        let trial = summary(
            trial_options.cylinder_fit_tolerance_mm,
            result.map(|conversion| conversion.report),
        );
        match trial.outcome {
            "validated" => {
                assert!(trial.error.is_none());
                assert!(trial.faces.is_some());
                assert!(trial.max_deviation_bound_mm.unwrap() <= options.max_deviation_mm);
                let sampled = trial.max_sampled_distance_mm.unwrap();
                assert!(sampled.is_finite() && sampled >= 0.0);
            }
            "rejected" => {
                assert!(trial.error.is_some());
                assert!(trial.faces.is_none());
            }
            _ => panic!("unexpected classification"),
        }
        let phase_sum_ms: f64 = recorder.durations_ms.values().sum();
        assert!(phase_sum_ms <= elapsed_ms);
        records.push(json!({
            "trial": trial,
            "native_kernel": native_kernel,
            "elapsed_ms": elapsed_ms,
            "exclusive_phase_ms": recorder.durations_ms,
            "exclusive_phase_calls": recorder.calls,
            "outside_run_phases_residual_ms": elapsed_ms - phase_sum_ms,
            "body_inclusive_ms_do_not_add": recorder.body_ms,
            "incomplete_body_envelopes_on_failure": recorder.body_open.keys().collect::<Vec<_>>(),
            "finish_checks": recorder.finish_checks,
        }));
        trials.push(trial);
    }
    let elapsed_ms = search_start.elapsed().as_secs_f64() * 1000.0;
    (
        json!({
            "placed_triangles": triangles, "imported_bodies": input_bodies,
            "standalone_preview_import_probe_ms_not_additive": standalone_preview_ms,
            "reconstructed_search_elapsed_ms": elapsed_ms,
            "best_index": best_index(&trials), "candidates": records,
        }),
        trials,
    )
}

#[test]
#[ignore = "opt-in native timing diagnostic; writes artifacts/radial-profile/summary.json"]
fn radial_search_phase_profile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repetitions = std::env::var("FABSTEP_PROFILE_REPETITIONS")
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(3);
    assert!((2..=3).contains(&repetitions));
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut inputs = vec![
        (
            "mixed_stl",
            root.join("fixtures/m2b_boss_blind.stl"),
            InputFormat::Stl,
        ),
        (
            "two_instances_3mf",
            root.join("fixtures/guided_cylinder_instances.3mf"),
            InputFormat::ThreeMf,
        ),
    ];
    let private = std::env::var_os("FABSTEP_PROFILE_PRIVATE_STL").map(PathBuf::from);
    let private_available = private.as_ref().is_some_and(|path| path.is_file());
    if let Some(private) = private {
        assert!(
            private_available,
            "FABSTEP_PROFILE_PRIVATE_STL must name a readable file"
        );
        inputs.push(("private_sock_clip_stl", private, InputFormat::Stl));
    }
    let output = root.join("artifacts/radial-profile");
    std::fs::create_dir_all(&output).unwrap();
    let mut datasets = Vec::new();
    for (name, path, format) in inputs {
        eprintln!("radial profile: starting {name}");
        let bytes = std::fs::read(path).unwrap();
        let mut runs = Vec::new();
        let mut reference = None;
        for _ in 0..repetitions {
            let (run, trials) = profile(&bytes, format, &options);
            let classification = json!({ "settings": ProcessingSettings::from(&options),
                "best_index": best_index(&trials), "trials": trials });
            if let Some(expected) = &reference {
                assert_eq!(&classification, expected);
            }
            reference = Some(classification);
            runs.push(run);
        }
        let mut ordinary_ms = None;
        if name == "mixed_stl" {
            let start = Instant::now();
            let ordinary = search_parameters(&bytes, format, &options, || Ok(())).unwrap();
            ordinary_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(serde_json::to_value(ordinary).unwrap(), reference.unwrap());
        }
        let mut totals = runs
            .iter()
            .map(|r| r["reconstructed_search_elapsed_ms"].as_f64().unwrap())
            .collect::<Vec<_>>();
        totals.sort_by(f64::total_cmp);
        datasets.push(
            json!({"name": name, "input_bytes": bytes.len(), "runs": runs,
            "total_ms_min": totals[0], "total_ms_median": if repetitions == 2 {
                (totals[0] + totals[1]) / 2.0 } else {totals[1]},
            "total_ms_max": totals[repetitions - 1],
            "ordinary_search_comparison_performed": ordinary_ms.is_some(),
            "ordinary_search_elapsed_ms_single_run": ordinary_ms }),
        );
        std::fs::write(
            output.join("progress.json"),
            serde_json::to_vec_pretty(&json!({
                "complete": false, "repetitions": repetitions, "completed_datasets": datasets
            }))
            .unwrap(),
        )
        .unwrap();
        eprintln!("radial profile: finished {name}");
    }
    std::fs::write(output.join("summary.json"), serde_json::to_vec_pretty(&json!({
        "schema": 1, "profile": "cargo test default unoptimized/debug; native OCCT dynamically linked",
        "occt_version": "see candidates.native_kernel from validated reports",
        "settings": ProcessingSettings::from(&options), "repetitions": repetitions,
        "private_input_available": private_available,
        "method": [
            "Reconstructed seven-conversion scenario, not direct search_parameters phase instrumentation.",
            "Exact summary/classification/settings/ranking equivalence to real search checked on mixed STL.",
            "Analysis includes import/preview, weld, topology, initial planar segmentation and provenance.",
            "Standalone preview/import probe is independent, warms input, and cannot be subtracted as measured in-search import.",
            "Run phases exclusive; Body inclusive envelopes excluded from their sum; Finish is a point check.",
            "Residual is conversion wall total minus run intervals, before trial summary/report destruction; includes glue and callback overhead. No finer attribution measured.",
            "Build includes native construction and obligatory STEP reimport/validation; not separated by current callbacks.",
            "Repeated planar segmentation in analysis and recognizers observed in source; its isolated time and safe reuse are unmeasured.",
            "No warmup discarded, serial candidate order, debug build, no speedup measured or promised."
        ], "datasets": datasets
    })).unwrap()).unwrap();
    std::fs::write(
        output.join("progress.json"),
        serde_json::to_vec_pretty(&json!({
            "complete": true, "repetitions": repetitions, "completed_datasets": datasets
        }))
        .unwrap(),
    )
    .unwrap();
}
