use fabstep_app::{search_parameters, InputFormat, ProcessingSettings};
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn proposals_require_complete_validated_document_and_preserve_settings() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let bytes = include_bytes!("../../../fixtures/short_arcs/convex.stl");
    let report = search_parameters(bytes, InputFormat::Stl, &options, || Ok(())).unwrap();
    assert_eq!(report.settings, ProcessingSettings::from(&options));
    assert_eq!(report.trials.len(), 7);
    let best = &report.trials[report.best_index.unwrap()];
    assert_eq!(best.faces, Some(6));
    assert_eq!(best.curved_faces, Some(3));
    assert_eq!(best.cylinder_fit_tolerance_mm, 5e-6);
    for trial in &report.trials {
        assert_eq!(trial.outcome, "validated");
        assert!(trial.error.is_none());
        assert!(trial.max_deviation_bound_mm.unwrap() <= options.max_deviation_mm);
        assert!(trial.max_sampled_distance_mm.unwrap() <= options.max_deviation_mm);
    }
    let rejected = search_parameters(&[], InputFormat::Stl, &options, || Ok(())).unwrap();
    assert!(rejected.best_index.is_none());
    assert_eq!(rejected.trials.len(), 7);
    assert!(rejected
        .trials
        .iter()
        .all(|t| t.outcome == "rejected" && t.faces.is_none() && t.error.is_some()));
}
