//! Read-only distances on the native tessellation published by the desktop.
use super::*;

#[derive(Debug, Serialize)]
pub struct DisplayDeviationMap {
    pub vertex_distances_mm: Vec<f64>,
    pub face_maxima_mm: Vec<f64>,
    pub min_mm: f64,
    pub max_mm: f64,
    pub budget_mm: f64,
}

impl DocumentStep {
    /// The caller supplies its native inspection of this exact immutable STEP.
    /// Source coordinates and units come only from the conversion's original input.
    pub fn display_deviations_controlled(
        &self,
        display: &DocumentResultFaces,
        check_current: &mut impl FnMut() -> Result<(), FabError>,
    ) -> Result<DisplayDeviationMap, FabError> {
        check_current()?;
        let source = self.source.as_ref().ok_or_else(|| {
            FabError::new(
                "step.deviation_unavailable",
                "Le maillage source de ce résultat est indisponible.",
            )
        })?;
        if !source.options.max_deviation_mm.is_finite() || source.options.max_deviation_mm <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Le seuil de la carte doit être fini et strictement positif.",
            ));
        }
        let result = &display.result;
        let invalid_display = || {
            FabError::new(
                "cad.tessellation_failed",
                "La tessellation d’affichage est incohérente.",
            )
        };
        if result.mesh.triangles.is_empty()
            || result.mesh.triangles.len() > 100_000
            || result.faces.is_empty()
            || result.faces.len() > result.mesh.triangles.len()
        {
            return Err(invalid_display());
        }
        let preview = preview_document(&source.bytes, source.format, &source.options)?;
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        for body in preview.bodies {
            // Same welding, component split and orientation as conversion. All
            // bodies already include their placements and explicit unit in mm.
            for component in fabstep_core::prepare_mesh_bodies_for_sections_controlled(
                &body.mesh,
                &source.options,
                check_current,
            )? {
                let offset = mesh.vertices.len();
                let mut part = component.mesh;
                if part.triangles.len() > 100_000 - mesh.triangles.len()
                    || part.vertices.len() > 300_000 - offset
                {
                    return Err(FabError::new(
                        "validation.distance_budget",
                        "Le maillage source dépasse le plafond de la carte d’écart.",
                    ));
                }
                if offset == 0 {
                    mesh = part;
                } else {
                    mesh.vertices.append(&mut part.vertices);
                    mesh.triangles
                        .extend(part.triangles.into_iter().map(|t| t.map(|i| i + offset)));
                }
            }
        }
        let distances = fabstep_verify::vertex_distances_controlled(
            &mesh,
            &result.mesh.vertices,
            check_current,
        )?;
        let mut face_maxima = Vec::with_capacity(result.faces.len());
        let mut end = 0;
        for face in &result.faces {
            if face.id != face_maxima.len()
                || face.triangle_start != end
                || face.triangle_count == 0
            {
                return Err(invalid_display());
            }
            end = end
                .checked_add(face.triangle_count)
                .ok_or_else(invalid_display)?;
            let triangles = result
                .mesh
                .triangles
                .get(face.triangle_start..end)
                .ok_or_else(invalid_display)?;
            let mut maximum = 0_f64;
            for &i in triangles.iter().flatten() {
                maximum = maximum.max(*distances.get(i).ok_or_else(invalid_display)?);
            }
            face_maxima.push(maximum);
        }
        if end != result.mesh.triangles.len() {
            return Err(invalid_display());
        }
        check_current()?;
        Ok(DisplayDeviationMap {
            min_mm: distances.iter().copied().fold(f64::INFINITY, f64::min),
            max_mm: distances.iter().copied().fold(0., f64::max),
            budget_mm: source.options.max_deviation_mm,
            vertex_distances_mm: distances,
            face_maxima_mm: face_maxima,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_distances_use_all_placed_source_components() {
        let source = include_bytes!("../../../../fixtures/m3_instances.3mf");
        let conversion =
            convert_document_with_provenance(source, InputFormat::ThreeMf, &Options::default())
                .unwrap();
        assert!(conversion.report.bodies.len() > 1);
        let display = conversion.step.result_faces().unwrap();
        let report = conversion
            .step
            .display_deviations_controlled(&display, &mut || Ok(()))
            .unwrap();
        assert!(report.max_mm < 1e-8);
        assert!(display.result.faces.iter().any(|f| f.solid_id > 0));
    }

    #[test]
    fn analytic_display_reports_curvature_and_exact_face_maxima_with_current_budget() {
        let options = Options {
            reconstruction: ReconstructionMode::Auto,
            max_deviation_mm: 0.1,
            ..Options::default()
        };
        let bytes = include_bytes!("../../../../fixtures/m2b_boss_blind.stl");
        let conversion =
            convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let display = conversion.step.result_faces().unwrap();
        let report = conversion
            .step
            .display_deviations_controlled(&display, &mut || Ok(()))
            .unwrap();
        let source = &conversion.report.bodies[0].conversion.analysis.mesh;
        let expected = fabstep_verify::vertex_distances_controlled(
            source,
            &display.result.mesh.vertices,
            &mut || Ok(()),
        )
        .unwrap();
        assert_eq!(expected, report.vertex_distances_mm);
        assert!(report.min_mm < 1e-8 && report.max_mm > 1e-5 && report.max_mm <= report.budget_mm);
        assert_eq!(report.budget_mm, options.max_deviation_mm);
        assert_eq!(
            report.max_mm,
            report.face_maxima_mm.iter().copied().fold(0., f64::max)
        );
        assert_eq!(report.face_maxima_mm.len(), display.result.faces.len());
        for face in &display.result.faces {
            let max = display.result.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
                .iter()
                .flatten()
                .map(|&i| expected[i])
                .fold(0., f64::max);
            assert_eq!(max, report.face_maxima_mm[face.id]);
        }
        let isolated = DocumentStep::untracked(conversion.step.bytes().to_vec());
        assert_eq!(
            isolated
                .display_deviations_controlled(&display, &mut || Ok(()))
                .unwrap_err()
                .code,
            "step.deviation_unavailable"
        );
    }
}
