use super::{
    audit_reimport, triangle_model, validate_source, Analysis, Conversion, ConversionControl,
    ConversionPhase, ConversionReport, FabError, Options, ProcessingSettings, ReconstructionMode,
};
use fabstep_cad::ValidatedStep;
use fabstep_core::unequal_corner::unequal_corner_candidate;

pub(crate) struct Built {
    step: ValidatedStep,
    owners: Vec<usize>,
    bound: f64,
}

pub(crate) fn try_build(
    analysis: &Analysis,
    options: &Options,
    first: usize,
    body: usize,
    control: &mut ConversionControl<'_>,
) -> Result<Option<Built>, FabError> {
    if options.reconstruction != ReconstructionMode::Auto {
        return Ok(None);
    }
    let Some(candidate) = unequal_corner_candidate(analysis, options) else {
        return Ok(None);
    };
    let remaining = options.max_deviation_mm - options.weld_tolerance_mm;
    if remaining <= options.cylinder_fit_tolerance_mm {
        return Ok(None);
    }
    control.run(ConversionPhase::SourceValidation, Some(body), || {
        validate_source(&triangle_model(&analysis.mesh)).map(|_| ())
    })?;
    match control.run(ConversionPhase::Build, Some(body), || {
        fabstep_cad::build_unequal_corner(
            &analysis.mesh,
            &candidate,
            first,
            options.cylinder_fit_tolerance_mm,
            remaining,
        )
    }) {
        Ok((step, owners, bound))
            if bound.is_finite()
                && bound <= remaining
                && step.report.faces == 10
                && step.report.planar_faces == 6
                && step.report.cylindrical_faces == 3 =>
        {
            Ok(Some(Built {
                step,
                owners,
                bound: bound + options.weld_tolerance_mm,
            }))
        }
        Ok(_) => Err(FabError::new(
            "validation.analytic_mismatch",
            "Le coin réimporté ne correspond pas au domaine certifié.",
        )),
        Err(e) if e.code == "cad.corner_unproven" => Ok(None),
        Err(e) => Err(e),
    }
}
impl Built {
    pub(crate) fn append_sources(
        &self,
        preview_id: &str,
        analysis_id: &str,
        source: &[usize],
        output: &mut Vec<crate::document::ResultFaceSource>,
    ) -> Result<(), FabError> {
        let invalid = || {
            FabError::new(
                "step.invalid_source_provenance",
                "Partition incomplète du coin à trois rayons.",
            )
        };
        if self.owners.len() != source.len() {
            return Err(invalid());
        }
        let mut groups: [Vec<usize>; 10] = Default::default();
        for (&owner, &id) in self.owners.iter().zip(source) {
            groups.get_mut(owner).ok_or_else(invalid)?.push(id);
        }
        if groups.iter().any(Vec::is_empty) {
            return Err(invalid());
        }
        output.extend(groups.into_iter().map(|source_triangle_ids| {
            crate::document::ResultFaceSource {
                face_id: 0,
                preview_body_id: preview_id.into(),
                analysis_body_id: analysis_id.into(),
                source_triangle_ids,
            }
        }));
        Ok(())
    }

    pub(crate) fn finish(
        self,
        mut analysis: Analysis,
        options: &Options,
        control: &mut ConversionControl<'_>,
        body: usize,
    ) -> Result<Conversion, FabError> {
        let fidelity = audit_reimport(
            &mut analysis,
            options,
            &self.step,
            Some(self.bound),
            control,
            body,
        )?;
        analysis.report.diagnostics.push(fabstep_core::Diagnostic {
            code: "recognition.unequal_corner_fitted".into(),
            severity: fabstep_core::Severity::Info,
            message: "Les trois congés de rayons distincts et l’intérieur du raccord ont été vérifiés sur le maillage.".into(),
            count: 1,
        });
        Ok(Conversion {
            step_bytes: self.step.bytes,
            report: ConversionReport {
                settings: ProcessingSettings::from(options),
                analysis,
                cad: self.step.report,
                recognition: None,
                mixed_recognition: None,
                revolved_recognition: None,
                rounded_recognition: None,
                rounded_box_recognition: None,
                height_patch_recognition: None,
                cubic_prism_recognition: None,
                oblique_lip_recognition: None,
                fidelity,
            },
        })
    }
}
