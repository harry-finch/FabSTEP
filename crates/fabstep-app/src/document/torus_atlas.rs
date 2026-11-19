//! Complete large-STL fallback. Source identities have already been remapped by
//! document analysis and must survive native face labeling and STEP reimport.
use super::{BodySourceTriangles, ResultFaceSource};
use crate::{
    audit_reimport, Conversion, ConversionControl, ConversionPhase, ConversionReport,
    ProcessingSettings,
};
use fabstep_core::{torus_atlas::TorusAtlas, Analysis, Diagnostic, FabError, Options, Severity};

// Keep any ordinary reconstruction that fits the result limit. Work exhaustion
// in that recognizer can start this independent, bounded proof. A failed
// geometric check or an interrupted job must propagate unchanged.
pub(super) fn should_retry(candidate: &Result<fabstep_core::RoundedRecognition, FabError>) -> bool {
    match candidate {
        Ok(model) => {
            crate::rounded_face_counts(&model.model)
                .iter()
                .sum::<usize>()
                > 100_000
        }
        Err(e) => e.code == "recognition.too_complex",
    }
}

fn source_faces(
    body_id: &str,
    source: &BodySourceTriangles,
    atlas: &TorusAtlas,
    triangles: usize,
    remaining: usize,
) -> Result<Vec<ResultFaceSource>, FabError> {
    let invalid = || {
        FabError::new(
            "step.invalid_source_provenance",
            "Partition incomplète ou ambiguë des zones toriques.",
        )
    };
    if atlas.charts.is_empty()
        || atlas.charts.len() > remaining
        || source.analysis_to_source_triangles.len() != triangles
    {
        return Err(invalid());
    }
    let mut used = vec![false; triangles];
    let mut faces = Vec::new();
    let face = |source_triangle_ids| ResultFaceSource {
        face_id: 0,
        preview_body_id: source.preview_body_id.clone(),
        analysis_body_id: body_id.into(),
        source_triangle_ids,
    };
    for chart in &atlas.charts {
        if chart.triangles.is_empty() {
            return Err(invalid());
        }
        let mut ids = Vec::with_capacity(chart.triangles.len().min(triangles));
        for &i in &chart.triangles {
            if std::mem::replace(used.get_mut(i).ok_or_else(invalid)?, true) {
                return Err(invalid());
            }
            ids.push(source.analysis_to_source_triangles[i]);
        }
        ids.sort_unstable();
        faces.push(face(ids));
    }
    if used.iter().filter(|&&claimed| !claimed).count() > remaining - faces.len() {
        return Err(invalid());
    }
    for (i, claimed) in used.into_iter().enumerate() {
        if !claimed {
            faces.push(face(vec![source.analysis_to_source_triangles[i]]));
        }
    }
    Ok(faces)
}

pub(super) fn convert(
    mut analysis: Analysis,
    source: (&str, &BodySourceTriangles),
    options: &Options,
    atlas: &TorusAtlas,
    document_faces: &mut Vec<ResultFaceSource>,
    control: &mut ConversionControl<'_>,
    body_index: usize,
) -> Result<Conversion, FabError> {
    let first = document_faces.len() + 1;
    let faces = source_faces(
        source.0,
        source.1,
        atlas,
        analysis.mesh.triangles.len(),
        100_000usize.saturating_sub(document_faces.len()),
    )?;
    let tokens = (first..first + faces.len()).collect::<Vec<_>>();
    control.run(ConversionPhase::SourceValidation, Some(body_index), || {
        fabstep_verify::certify_source_embedding(&analysis.mesh)
    })?;
    let (step, bound) = control.run(ConversionPhase::Build, Some(body_index), || {
        fabstep_cad::build_torus_atlas_step_with_provenance(
            &analysis.mesh,
            atlas,
            &tokens,
            options.max_deviation_mm,
        )
    })?;
    if step.report.toroidal_faces != atlas.charts.len()
        || step.report.faces != faces.len()
        || !bound.is_finite()
        || bound < 0.
        || bound > options.max_deviation_mm
    {
        return Err(FabError::new(
            "validation.analytic_mismatch",
            "Le résultat réimporté ne correspond pas aux domaines toriques certifiés.",
        ));
    }
    let fidelity = audit_reimport(
        &mut analysis,
        options,
        &step,
        Some(bound),
        control,
        body_index,
    )?;
    analysis.report.diagnostics.push(Diagnostic {
        code: "recognition.trimmed_tori_fitted".into(),
        severity: Severity::Info,
        message: "Les zones toriques découpées ont été reconstruites avec leurs bords partagés."
            .into(),
        count: atlas.charts.len(),
    });
    document_faces.extend(faces);
    Ok(Conversion {
        step_bytes: step.bytes,
        report: ConversionReport {
            settings: ProcessingSettings::from(options),
            analysis,
            cad: step.report,
            recognition: None,
            mixed_recognition: None,
            revolved_recognition: None,
            rounded_recognition: None,
            rounded_box_recognition: None,
            height_patch_recognition: None,
            fidelity,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabstep_core::torus_atlas::TorusChart;

    #[test]
    fn partition_preserves_source_ids_and_refuses_overlap_or_excess_faces() {
        let source = BodySourceTriangles {
            preview_body_id: "stl".into(),
            analysis_to_source_triangles: vec![11, 27, 31, 33],
            planar_triangle_faces: Vec::new(),
        };
        let mut atlas = TorusAtlas {
            axis: [0., 0., 1.],
            radial: [1., 0., 0.],
            charts: vec![TorusChart {
                center_mm: [0.; 3],
                major_radius_mm: 8.,
                minor_radius_mm: 1.,
                orientation: 1,
                triangles: vec![2, 0],
                loops: Vec::new(),
                bound_mm: 0.01,
            }],
        };
        let faces = source_faces("body", &source, &atlas, 4, 3).unwrap();
        let owners: Vec<_> = faces.into_iter().map(|f| f.source_triangle_ids).collect();
        assert_eq!(owners, [vec![11, 31], vec![27], vec![33]]);
        assert!(source_faces("body", &source, &atlas, 4, 2).is_err());
        atlas.charts[0].triangles.push(2);
        assert!(source_faces("body", &source, &atlas, 4, 3).is_err());
    }
}
