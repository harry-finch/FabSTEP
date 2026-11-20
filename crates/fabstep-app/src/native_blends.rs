use super::{
    audit_reimport, validate_mesh_source, Analysis, Conversion, ConversionControl, ConversionPhase,
    ConversionReport, FabError, Options, ProcessingSettings, ReconstructionMode, SourceProofCache,
};
use fabstep_cad::ValidatedStep;
use fabstep_core::{rolling_bore, unequal_corner::unequal_corner_candidate};
enum Blend {
    Corner(fabstep_core::unequal_corner::UnequalCorner),
    Bore(rolling_bore::RollingBore),
}
#[derive(Clone, Copy)]
enum Kind {
    Corner,
    Bore,
}
impl Kind {
    fn counts(self) -> (usize, usize, usize) {
        match self {
            Self::Corner => (10, 6, 3),
            Self::Bore => (5, 2, 2),
        }
    }
    // A rejected candidate can fall back only for the geometric proofs owned
    // by its domain. Resource limits, cancellation and I/O still propagate.
    fn rejects_geometry(self, code: &str) -> bool {
        match self {
            Self::Corner => code == "cad.corner_unproven",
            Self::Bore => matches!(
                code,
                "cad.rolling_bore_unproven"
                    | "cad.invalid_solid"
                    | "cad.roundtrip_mismatch"
                    | "validation.bounds_mismatch"
                    | "validation.volume_mismatch"
                    | "validation.distance_exceeded"
                    | "cad.tessellation_failed"
            ),
        }
    }
    fn diagnostic(self) -> (&'static str, &'static str) {
        match self {
            Self::Corner => (
                "recognition.unequal_corner_fitted",
                "Les trois congés de rayons distincts et l’intérieur du raccord ont été vérifiés sur le maillage.",
            ),
            Self::Bore => (
                "recognition.rolling_bore_fitted",
                "Les deux parois cylindriques et le raccord arrondi de l’ouverture ont été vérifiés sur le maillage.",
            ),
        }
    }
}

pub(crate) struct Built {
    step: ValidatedStep,
    owners: Vec<usize>,
    fidelity: Option<fabstep_verify::FidelityReport>,
    kind: Kind,
}

pub(crate) fn try_build(
    analysis: &mut Analysis,
    options: &Options,
    first: usize,
    body: usize,
    control: &mut ConversionControl<'_>,
    source_proofs: Option<&mut SourceProofCache>,
) -> Result<Option<Built>, FabError> {
    if options.reconstruction != ReconstructionMode::Auto {
        return Ok(None);
    }
    let candidate = control.run(ConversionPhase::Recognition, Some(body), || {
        Ok(unequal_corner_candidate(analysis, options)
            .map(Blend::Corner)
            .or_else(|| rolling_bore::candidate(analysis, options).map(Blend::Bore)))
    })?;
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let remaining = options.max_deviation_mm - options.weld_tolerance_mm;
    if remaining <= options.cylinder_fit_tolerance_mm {
        return Ok(None);
    }
    validate_mesh_source(&analysis.mesh, body, control, source_proofs)?;
    let fit = options.cylinder_fit_tolerance_mm;
    let kind = match &candidate {
        Blend::Corner(_) => Kind::Corner,
        Blend::Bore(_) => Kind::Bore,
    };
    let result = control.run(ConversionPhase::Build, Some(body), || match candidate {
        Blend::Corner(c) => {
            fabstep_cad::build_unequal_corner(&analysis.mesh, &c, first, fit, remaining)
        }
        Blend::Bore(b) => fabstep_cad::build_rolling_bore_with_patch_bound(
            &analysis.mesh,
            &b,
            first,
            fit.min(1e-5),
            remaining,
        )
        .map(|(step, patch_bound)| (step, b.owners, b.support_bound_mm.max(patch_bound))),
    });
    match result {
        Ok((step, owners, bound))
            if bound.is_finite()
                && bound >= 0.
                && bound <= remaining
                && step.report.solids == 1
                && (
                    step.report.faces,
                    step.report.planar_faces,
                    step.report.cylindrical_faces,
                ) == kind.counts() =>
        {
            let fidelity = match audit_reimport(
                analysis,
                options,
                &step,
                Some(bound + options.weld_tolerance_mm),
                control,
                body,
            ) {
                Ok(f) => f,
                Err(e) if kind.rejects_geometry(&e.code) && control.interruption().is_none() => {
                    return Ok(None)
                }
                Err(e) => return Err(e),
            };
            Ok(Some(Built {
                kind,
                step,
                owners,
                fidelity,
            }))
        }
        Ok(_) if matches!(kind, Kind::Bore) => Ok(None),
        Ok(_) => Err(FabError::new(
            "validation.analytic_mismatch",
            "Le raccord réimporté ne correspond pas au domaine certifié.",
        )),
        Err(e) if kind.rejects_geometry(&e.code) && control.interruption().is_none() => Ok(None),
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
                "Partition incomplète des faces du raccord.",
            )
        };
        if self.owners.len() != source.len() {
            return Err(invalid());
        }
        let mut groups = vec![Vec::new(); self.kind.counts().0];
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

    pub(crate) fn finish(self, mut analysis: Analysis, options: &Options) -> Conversion {
        let (code, message) = self.kind.diagnostic();
        analysis.report.diagnostics.push(fabstep_core::Diagnostic {
            code: code.into(),
            severity: fabstep_core::Severity::Info,
            message: message.into(),
            count: 1,
        });
        Conversion {
            step_bytes: self.step.bytes,
            report: ConversionReport {
                auto_adjustment: None,
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
                oblique_band_recognition: None,
                fidelity: self.fidelity,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;

    #[test]
    fn parameter_trials_reuse_the_exact_source_proof_for_native_blends() {
        let options = crate::Options {
            reconstruction: crate::ReconstructionMode::Auto,
            ..Default::default()
        };
        let bytes = include_bytes!("../../../fixtures/unequal_corner/corner-0-coarse.stl");
        let mut cache = crate::SourceProofCache::default();
        for _ in 0..2 {
            let result = crate::document::convert_document_with_source_proofs(
                bytes,
                crate::InputFormat::Stl,
                &options,
                &mut |_| Ok(()),
                Some(&mut cache),
            )
            .unwrap();
            assert_eq!(result.report.cad.faces, 10);
            let report = &result.report.bodies[0].conversion;
            assert!(report.fidelity.is_some());
            assert!(report
                .analysis
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "recognition.unequal_corner_fitted"));
            assert_eq!(cache.source_calls, 1);
        }
    }

    #[test]
    fn resource_and_operational_errors_propagate_for_both_domains() {
        for code in [
            "cad.tessellation_too_complex",
            "validation.distance_budget",
            "validation.distance_resolution",
            "engine.stale",
            "engine.cancelled",
            "cad.read_failed",
            "cad.write_failed",
            "validation.numeric_range",
            "step.invalid_source_provenance",
        ] {
            assert!(!Kind::Bore.rejects_geometry(code), "{code}");
            assert!(!Kind::Corner.rejects_geometry(code), "{code}");
        }
        for code in ["cad.invalid_solid", "validation.distance_exceeded"] {
            assert!(Kind::Bore.rejects_geometry(code));
            assert!(!Kind::Corner.rejects_geometry(code));
        }
    }
}
