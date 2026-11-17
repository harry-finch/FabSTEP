//! Local planar guidance. Boundary vertices never move; selected interiors may
//! be represented by a plane only within the explicit geometric budget.
use crate::math::*;
use crate::{
    Analysis, Diagnostic, FabError, GuidanceFamily, MixedLoop, MixedModel, MixedPlane, Options,
    RecognitionReport, RevolvedFaceRef, RevolvedModel, RevolvedRecognition, Severity,
};
use std::collections::BTreeSet;

pub(crate) fn recognize(
    analysis: &Analysis,
    options: &Options,
    requests: &[(GuidanceFamily, Vec<usize>)],
) -> Result<RevolvedRecognition, FabError> {
    for tolerance in [
        options.weld_tolerance_mm,
        options.plane_tolerance_mm,
        options.cylinder_fit_tolerance_mm,
        options.max_deviation_mm,
    ] {
        if !tolerance.is_finite() || tolerance <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    crate::recognition::validate_analysis(analysis)?;
    if requests.is_empty() || requests.len() > 16 {
        return Err(FabError::new(
            "guidance.limit",
            "Au plus 16 indications sont admises.",
        ));
    }
    let invalid = || {
        FabError::new("guidance.invalid_selection", "La sélection doit contenir des triangles source distincts, présents dans le corps analysé.")
    };
    let mut all_selected = BTreeSet::new();
    let mut planes = Vec::new();
    let mut curved = Vec::new();
    let mut total = 0usize;
    for (family, triangles) in requests {
        total = total.saturating_add(triangles.len());
        if triangles.is_empty() || total > analysis.mesh.triangles.len() {
            return Err(invalid());
        }
        let mut selected = BTreeSet::new();
        for &triangle in triangles {
            if triangle >= analysis.mesh.triangles.len() || !selected.insert(triangle) {
                return Err(invalid());
            }
            if !all_selected.insert(triangle) {
                return Err(FabError::new(
                    "guidance.overlapping_indications",
                    "Les indications se recouvrent.",
                ));
            }
        }
        if *family == GuidanceFamily::Plane {
            planes.push(selected);
        } else {
            curved.push((*family, triangles.clone()));
        }
    }
    let adjacency = crate::pipeline::validate_topology(&analysis.mesh)?;
    let mut result = if curved.is_empty() {
        // Reproduce the ordinary planar partition, not the wider circular fit
        // segmentation. There is no implicit curved-surface search in this path.
        let segmentation = crate::planar::build_with_provenance(
            &analysis.mesh,
            &adjacency,
            options.plane_tolerance_mm,
        )?;
        RevolvedRecognition {
            model: RevolvedModel {
                mixed: MixedModel {
                    vertices: segmentation.model.vertices,
                    planes: segmentation
                        .model
                        .faces
                        .into_iter()
                        .map(|face| MixedPlane {
                            normal: face.normal,
                            loops: face
                                .loops
                                .into_iter()
                                .map(|vertices| MixedLoop::Polygon { vertices })
                                .collect(),
                        })
                        .collect(),
                    rings: vec![],
                    cylinders: vec![],
                    arcs: vec![],
                    partial_cylinders: vec![],
                },
                cones: vec![],
            },
            triangle_faces: segmentation
                .triangle_regions
                .into_iter()
                .map(|index| RevolvedFaceRef::Plane { index })
                .collect(),
            report: RecognitionReport {
                cylinders: 0,
                cones: 0,
                spheres: 0,
                tori: 0,
                toroidal_fillets: 0,
                max_radial_residual_mm: 0.,
                max_chord_error_mm: 0.,
                max_wall_residual_mm: 0.,
                max_deviation_bound_mm: 0.,
                expected_volume_mm3: 0.,
                diagnostics: vec![],
            },
        }
    } else {
        crate::mixed::recognize_guided_surfaces(analysis, options, &curved)?
    };
    // Canonical order makes output independent of indication/selection ordering.
    planes.sort();
    let mut checks = 0;
    for selected in &planes {
        let mut replaced = BTreeSet::new();
        for &triangle in selected {
            match result.triangle_faces[triangle] {
                RevolvedFaceRef::Plane { index } => {
                    replaced.insert(index);
                }
                _ => {
                    return Err(FabError::new(
                        "guidance.overlapping_indications",
                        "La zone plane recouvre une paroi courbe indiquée.",
                    ))
                }
            }
        }
        if result
            .triangle_faces
            .iter()
            .enumerate()
            .any(|(triangle, owner)| {
                matches!(owner, RevolvedFaceRef::Plane { index } if replaced.contains(index))
                    && !selected.contains(&triangle)
            })
        {
            return Err(FabError::new("guidance.plane_incomplete", "Sélectionner toutes les facettes des faces à regrouper ; aucune face voisine n’est absorbée implicitement."));
        }
        if replaced.iter().any(|&index| {
            result.model.mixed.planes[index]
                .loops
                .iter()
                .any(|lp| !matches!(lp, MixedLoop::Polygon { .. }))
        }) {
            return Err(FabError::new("guidance.plane_unsupported_boundary", "Cette correction plane ne prend pas encore en charge les contours circulaires ou courbes."));
        }
        let face = crate::planar::guided_face(
            &analysis.mesh,
            &adjacency,
            selected,
            options.plane_tolerance_mm,
            &mut checks,
        )?;
        let first = *replaced.first().expect("nonempty selection");
        let mut mapping = vec![usize::MAX; result.model.mixed.planes.len()];
        let mut merged = Vec::with_capacity(mapping.len() - replaced.len() + 1);
        let mut merged_index = 0;
        for (index, plane) in result.model.mixed.planes.iter().enumerate() {
            if index == first {
                merged_index = merged.len();
                merged.push(MixedPlane {
                    normal: face.normal,
                    loops: face
                        .loops
                        .iter()
                        .cloned()
                        .map(|vertices| MixedLoop::Polygon { vertices })
                        .collect(),
                });
            }
            if replaced.contains(&index) {
                mapping[index] = merged_index;
            } else {
                mapping[index] = merged.len();
                merged.push(plane.clone());
            }
        }
        for owner in &mut result.triangle_faces {
            if let RevolvedFaceRef::Plane { index } = owner {
                *index = mapping[*index];
            }
        }
        result.model.mixed.planes = merged;
    }
    let mut planar_residual: f64 = 0.;
    let mut face_residuals = vec![(0_f64, 0_f64); result.model.mixed.planes.len()];
    for (triangle, owner) in result.triangle_faces.iter().enumerate() {
        if let RevolvedFaceRef::Plane { index } = owner {
            let plane = &result.model.mixed.planes[*index];
            let normal = scale(plane.normal, 1. / norm(plane.normal));
            let origin = crate::partial::loop_origin(&result.model.mixed, &plane.loops[0]);
            for &vertex in &analysis.mesh.triangles[triangle] {
                let offset = sub(analysis.mesh.vertices[vertex], origin);
                let residual = dot(offset, normal).abs();
                planar_residual = planar_residual.max(residual);
                face_residuals[*index].0 = face_residuals[*index].0.max(residual);
                face_residuals[*index].1 = face_residuals[*index].1.max(norm(offset));
            }
        }
    }
    // The curved proof's wall residual already includes its old planes. Keep
    // that conservative contribution and add the final plane residual rather
    // than subtracting a contribution not exposed by the historical report.
    result.report.max_wall_residual_mm += planar_residual;
    result.report.max_deviation_bound_mm = options.weld_tolerance_mm
        + result.report.max_wall_residual_mm
        + result.report.max_radial_residual_mm
        + result.report.max_chord_error_mm;
    if !result.report.max_deviation_bound_mm.is_finite()
        || result.report.max_deviation_bound_mm > options.max_deviation_mm
    {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "La borne d’écart de la correction dépasse le budget explicite.",
        ));
    }
    result.report.expected_volume_mm3 = if result.model.cones.is_empty() {
        crate::mixed::volume(&result.model.mixed)?
    } else {
        crate::revolved::volume(&result.model)?
    };
    result
        .report
        .diagnostics
        .retain(|diagnostic| diagnostic.code != "recognition.mixed_coverage");
    result.report.diagnostics.push(Diagnostic {
        code:"guidance.plane_verified".into(), severity:Severity::Info, count:planes.len(),
        message:"Chaque zone plane indiquée est connectée, bornée par ses contours source et vérifiée sans déplacement de ses sommets.".into(),
    });
    let approximate_faces = face_residuals
        .iter()
        .filter(|(residual, extent)| *residual > extent * f64::EPSILON * 128.)
        .count();
    if approximate_faces > 0 {
        result.report.diagnostics.push(Diagnostic {
            code:"guidance.plane_residual".into(), severity:Severity::Warning, count:approximate_faces,
            message:"Certaines faces planes remplacent un faible relief source ; leurs résidus sont inclus dans la borne d’écart et le solide doit être validé nativement.".into(),
        });
    }
    Ok(result)
}
