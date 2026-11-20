//! Complete large-STL fallback. Source identities have already been remapped by
//! document analysis and must survive native face labeling and STEP reimport.
use super::{BodySourceTriangles, ResultFaceSource};
use crate::{
    audit_reimport, Conversion, ConversionControl, ConversionPhase, ConversionReport,
    ProcessingSettings,
};
use fabstep_core::torus_atlas::{residual_planes, ResidualPlane, TorusAtlas};
use fabstep_core::{Analysis, Diagnostic, FabError, Options, Severity};

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

// Geometry operates on the welded mesh. A barycentric point of a raw
// triangle can move by up to the weld tolerance before this proof starts.
pub(super) fn available_deviation(options: &Options) -> Result<f64, FabError> {
    let remaining = options.max_deviation_mm - options.weld_tolerance_mm;
    if remaining.is_finite() && remaining > 0. {
        Ok(remaining)
    } else {
        Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le soudage consomme le budget d’écart disponible pour les zones toriques.",
        ))
    }
}

fn source_deviation(options: &Options, native: f64) -> Result<f64, FabError> {
    let total = options.weld_tolerance_mm + native;
    if native.is_finite()
        && (0.0..=available_deviation(options)?).contains(&native)
        && total <= options.max_deviation_mm
    {
        Ok(total)
    } else {
        Err(FabError::new(
            "validation.analytic_mismatch",
            "La borne composée de l’atlas torique dépasse la tolérance demandée.",
        ))
    }
}

fn source_faces(
    body_id: &str,
    source: &BodySourceTriangles,
    atlas: &TorusAtlas,
    planes: &[ResidualPlane<'_>],
    triangles: usize,
    remaining: usize,
    mut destination: Option<&mut Vec<ResultFaceSource>>,
) -> Result<usize, FabError> {
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
    let face = |source_triangle_ids| ResultFaceSource {
        face_id: 0,
        preview_body_id: source.preview_body_id.clone(),
        analysis_body_id: body_id.into(),
        source_triangle_ids,
    };
    for owners in atlas
        .charts
        .iter()
        .map(|chart| &chart.triangles)
        .chain(planes.iter().map(|plane| &plane.triangles))
    {
        if owners.is_empty() {
            return Err(invalid());
        }
        for &i in owners {
            if std::mem::replace(used.get_mut(i).ok_or_else(invalid)?, true) {
                return Err(invalid());
            }
        }
        if let Some(out) = &mut destination {
            let mut ids: Vec<_> = owners
                .iter()
                .map(|&i| source.analysis_to_source_triangles[i])
                .collect();
            ids.sort_unstable();
            out.push(face(ids));
        }
    }
    let count =
        atlas.charts.len() + planes.len() + used.iter().filter(|&&claimed| !claimed).count();
    if count > remaining {
        return Err(invalid());
    }
    if let Some(out) = destination {
        for (i, claimed) in used.into_iter().enumerate() {
            if !claimed {
                out.push(face(vec![source.analysis_to_source_triangles[i]]));
            }
        }
    }
    Ok(count)
}

pub(super) fn convert(
    mut analysis: Analysis,
    source: (&str, &BodySourceTriangles),
    options: &Options,
    atlas: &TorusAtlas,
    mut document_faces: Option<&mut Vec<ResultFaceSource>>,
    control: &mut ConversionControl<'_>,
    body_index: usize,
) -> Result<Conversion, FabError> {
    let previous = document_faces.as_ref().map_or(0, |faces| faces.len());
    let first = previous + 1;
    let mut planes = residual_planes(&analysis, &source.1.planar_triangle_faces, atlas);
    control.run(ConversionPhase::SourceValidation, Some(body_index), || {
        fabstep_verify::certify_source_embedding(&analysis.mesh)
    })?;
    let remaining = available_deviation(options)?;
    let (step, native_bound, count) = loop {
        let count = source_faces(
            source.0,
            source.1,
            atlas,
            &planes,
            analysis.mesh.triangles.len(),
            100_000usize.saturating_sub(previous),
            document_faces.as_deref_mut(),
        )?;
        let tokens = (first..first + count).collect::<Vec<_>>();
        match control.run(ConversionPhase::Build, Some(body_index), || {
            fabstep_cad::build_torus_atlas_regions(
                &analysis.mesh,
                atlas,
                &planes,
                &tokens,
                remaining,
            )
        }) {
            Ok((step, bound)) => break (step, bound, count),
            Err(e) if !planes.is_empty() && e.code == "cad.atlas_plane_unproven" => {
                // Only a declined plane certificate can retry. Roll back its
                // source identities before issuing one token per residual triangle.
                if let Some(faces) = &mut document_faces {
                    faces.truncate(previous);
                }
                planes.clear();
            }
            Err(e) => return Err(e),
        }
    };
    let merged = planes.len();
    drop(planes);
    let bound = source_deviation(options, native_bound)?;
    if step.report.toroidal_faces != atlas.charts.len() || step.report.faces != count {
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
    if merged > 0 {
        analysis.report.diagnostics.push(Diagnostic {
            code: "recognition.atlas_planes_merged".into(),
            severity: Severity::Info,
            message: "Les triangles coplanaires ont été réunis en faces planes avec leurs contours vérifiés.".into(),
            count: merged,
        });
    }
    analysis.report.diagnostics.push(Diagnostic {
        code: "recognition.trimmed_tori_fitted".into(),
        severity: Severity::Info,
        message: format!(
            "Les zones toriques et leurs bords partagés sont reconstruits avec une borne de {bound:.9} mm, dont {:.9} mm pour le soudage.",
            options.weld_tolerance_mm
        ),
        count: atlas.charts.len(),
    });
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
            cubic_prism_recognition: None,
            oblique_lip_recognition: None,
            oblique_band_recognition: None,
            fidelity,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabstep_core::torus_atlas::TorusChart;

    #[test]
    fn the_weld_and_native_errors_share_one_source_budget() {
        let options = Options {
            max_deviation_mm: 0.05,
            weld_tolerance_mm: 0.0125,
            ..Options::default()
        };
        let cube = include_str!("../../../../fixtures/cube.stl");
        // Only one corner occurrence moves. The others weld onto its
        // representative, so the core supplies its continuous bound.
        let raw = cube.replacen("vertex 1 1 0", "vertex 1.01 1 0", 1);
        let source = fabstep_core::analyze_stl(raw.as_bytes(), &options).unwrap();
        assert!(source
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "mesh.vertices_moved"));
        let remaining = available_deviation(&options).unwrap();
        assert_eq!(
            source_deviation(&options, remaining).unwrap(),
            options.max_deviation_mm
        );
        let larger = f64::from_bits(remaining.to_bits() + 1);
        assert!(larger < options.max_deviation_mm);
        assert_eq!(
            source_deviation(&options, larger).unwrap_err().code,
            "validation.analytic_mismatch"
        );
        assert_eq!(
            available_deviation(&Options {
                weld_tolerance_mm: 0.05,
                ..options
            })
            .unwrap_err()
            .code,
            "recognition.deviation_exceeded"
        );
    }

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
        let mut faces = Vec::new();
        assert_eq!(
            source_faces("body", &source, &atlas, &[], 4, 3, Some(&mut faces)).unwrap(),
            3
        );
        assert_eq!(
            source_faces("body", &source, &atlas, &[], 4, 3, None).unwrap(),
            3
        );
        let owners: Vec<_> = faces.into_iter().map(|f| f.source_triangle_ids).collect();
        assert_eq!(owners, [vec![11, 31], vec![27], vec![33]]);
        let face = fabstep_core::PlanarFace {
            loops: Vec::new(),
            normal: [0., 0., 1.],
        };
        let plane = ResidualPlane {
            face: &face,
            triangles: vec![3, 1],
        };
        let mut faces = Vec::new();
        assert_eq!(
            source_faces("body", &source, &atlas, &[plane], 4, 2, Some(&mut faces)).unwrap(),
            2
        );
        assert_eq!(faces[1].source_triangle_ids, vec![27, 33]);
        assert!(source_faces("body", &source, &atlas, &[], 4, 2, None).is_err());
        for owners in [vec![2, 0, 2], vec![4], vec![]] {
            atlas.charts[0].triangles = owners;
            for destination in [None, Some(&mut Vec::new())] {
                let error =
                    source_faces("body", &source, &atlas, &[], 4, 3, destination).unwrap_err();
                assert_eq!(error.code, "step.invalid_source_provenance");
            }
        }
    }
}
