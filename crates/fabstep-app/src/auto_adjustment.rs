//! A single native candidate per automatic conversion. Failed proposals do not
//! publish a STEP or its provenance. The original core result remains available.
use crate::{Analysis, Conversion, ConversionControl, ConversionPhase, FabError, Options};
use fabstep_core::{AutomaticCandidate, AutomaticTuningBudget, RoundedRecognition};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AutoAdjustment {
    pub requested_plane_tolerance_mm: f64,
    pub requested_cylinder_fit_tolerance_mm: f64,
    pub plane_tolerance_mm: f64,
    pub cylinder_fit_tolerance_mm: f64,
}

impl AutoAdjustment {
    pub(crate) fn effective(&self, requested: &Options) -> Options {
        Options {
            plane_tolerance_mm: self.plane_tolerance_mm,
            cylinder_fit_tolerance_mm: self.cylinder_fit_tolerance_mm,
            auto_tune: false,
            ..requested.clone()
        }
    }
}

pub(crate) type Choices = (RoundedRecognition, Option<AutomaticCandidate>);

pub(crate) fn candidates(
    analysis: &Analysis,
    options: &Options,
    budget: &mut AutomaticTuningBudget,
    control: &mut ConversionControl<'_>,
    body: usize,
) -> Result<Choices, FabError> {
    let mut check = || control.check(ConversionPhase::Recognition, Some(body));
    let result =
        fabstep_core::recognize_automatic_candidates(analysis, options, budget, &mut check);
    if result.is_err() && control.interruption().is_none() {
        // A geometry failure and a replacement request can arrive during the
        // same recognition. Give the caller its cancellation verbatim.
        control.check(ConversionPhase::Recognition, Some(body))?;
    }
    result
}

pub(crate) fn build(
    analysis: Analysis,
    options: &Options,
    (baseline, candidate): Choices,
    control: &mut ConversionControl<'_>,
    mut validate: impl FnMut(
        Analysis,
        &Options,
        RoundedRecognition,
        &mut ConversionControl<'_>,
    ) -> Result<Conversion, FabError>,
) -> Result<Conversion, FabError> {
    if let Some(candidate) = candidate {
        // A copy is needed only for a strict, bounded core improvement. Retain
        // the original analysis for fallback if the native proof rejects it.
        let selected = &candidate.options;
        let attempt = validate(analysis.clone(), selected, candidate.recognition, control);
        if let Some(stopped) = control.interruption() {
            return Err(stopped);
        }
        let baseline_faces: usize = crate::rounded_face_counts(&baseline.model).iter().sum();
        if let Some(mut conversion) = attempt.ok().filter(|c| {
            c.report.cad.faces < baseline_faces
                && c.report.cad.faces > c.report.cad.planar_faces
                && c.report.fidelity.as_ref().is_some_and(|f| {
                    f.source_to_result_mm <= options.max_deviation_mm
                        && f.result_to_source_mm <= options.max_deviation_mm
                })
        }) {
            conversion.report.auto_adjustment = Some(AutoAdjustment {
                requested_plane_tolerance_mm: options.plane_tolerance_mm,
                requested_cylinder_fit_tolerance_mm: options.cylinder_fit_tolerance_mm,
                plane_tolerance_mm: selected.plane_tolerance_mm,
                cylinder_fit_tolerance_mm: selected.cylinder_fit_tolerance_mm,
            });
            return Ok(conversion);
        }
    }
    validate(analysis, options, baseline, control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{convert_analysis_with_recognized, Recognized};
    use fabstep_core::ReconstructionMode;

    #[test]
    fn a_native_model_mismatch_rejects_the_candidate_and_revalidates_the_baseline() {
        let bytes = include_bytes!("../../../fixtures/r117-cut-sphere.stl");
        let options = Options {
            reconstruction: ReconstructionMode::Auto,
            ..Default::default()
        };
        let analysis = fabstep_core::analyze_stl(bytes, &options).unwrap();
        let baseline = fabstep_core::recognize_automatic(&analysis, &options).unwrap();
        let mut wrong = baseline.clone();
        wrong.report.expected_volume_mm3 *= 2.;
        let winner = AutomaticCandidate {
            recognition: wrong,
            options: Options {
                plane_tolerance_mm: 5e-6,
                ..options.clone()
            },
        };
        let mut builds = 0;
        let mut observe = |p: crate::ConversionCheckpoint| {
            builds += usize::from(p.phase == ConversionPhase::Build);
            Ok(())
        };
        let mut control = ConversionControl::new(&mut observe);
        let mut failures = vec![];
        let conversion = build(
            analysis,
            &options,
            (baseline, Some(winner)),
            &mut control,
            |a, o, r, c| {
                let native = convert_analysis_with_recognized(
                    a,
                    o,
                    None,
                    Some(Recognized::Rounded(r)),
                    c,
                    0,
                    None,
                );
                if let Err(e) = &native {
                    failures.push(e.code.clone());
                }
                native
            },
        )
        .unwrap();
        assert_eq!(builds, 4);
        assert_eq!(failures, ["validation.analytic_mismatch"]);
        assert!(conversion.report.auto_adjustment.is_none());
        assert_eq!(
            conversion.report.settings.plane_tolerance_mm,
            options.plane_tolerance_mm
        );
        assert_eq!(conversion.report.cad.faces, 4);
    }
}
