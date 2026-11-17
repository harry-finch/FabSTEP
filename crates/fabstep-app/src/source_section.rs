//! Trusted source-only section preparation; independent of STEP reconstruction.
use crate::{
    controlled, preview_document, triangle_model, BodyProvenance, ConversionCheckpoint,
    ConversionPhase, DocumentSource, InputFormat, ProcessingSettings,
};
use fabstep_core::{Diagnostic, FabError, Mesh, Options, SectionProfile, SectionRequest};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SourceSectionBody {
    pub solid_id: usize,
    pub analysis_body_id: String,
    pub name: String,
    pub provenance: BodyProvenance,
    pub bounds_mm: [[f64; 3]; 2],
    pub triangles: usize,
    pub input_triangles: usize,
    pub welded_vertices: usize,
    pub volume_mm3: f64,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSectionMetadata {
    pub source: DocumentSource,
    pub settings: ProcessingSettings,
    pub bounds_mm: [[f64; 3]; 2],
    pub bodies: Vec<SourceSectionBody>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Constructed only after every exact source mesh and the complete document
/// separation pass. No public constructor, mutable meshes, or OCCT objects.
#[derive(Debug)]
pub struct PreparedSourceSection {
    metadata: SourceSectionMetadata,
    meshes: Vec<Mesh>,
}

impl PreparedSourceSection {
    pub fn metadata(&self) -> &SourceSectionMetadata {
        &self.metadata
    }

    pub fn section_profile(&self, request: &SectionRequest) -> Result<SectionProfile, FabError> {
        self.section_profile_controlled(request, &mut || Ok(()))
    }

    pub fn section_profile_controlled(
        &self,
        request: &SectionRequest,
        check_current: &mut impl FnMut() -> Result<(), FabError>,
    ) -> Result<SectionProfile, FabError> {
        check_current()?;
        let meshes = self.meshes.iter().collect::<Vec<_>>();
        let result = fabstep_core::section_meshes_controlled(&meshes, request, check_current);
        check_current()?;
        result
    }
}

pub fn prepare_source_section(
    bytes: &[u8],
    format: InputFormat,
    options: &Options,
) -> Result<PreparedSourceSection, FabError> {
    prepare_source_section_controlled(bytes, format, options, &mut |_| Ok(()))
}

pub fn prepare_source_section_controlled(
    bytes: &[u8],
    format: InputFormat,
    options: &Options,
    check_current: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
) -> Result<PreparedSourceSection, FabError> {
    controlled(check_current, |control| {
        let preview = control.run(ConversionPhase::Analysis, None, || {
            preview_document(bytes, format, options)
        })?;
        let mut metadata = SourceSectionMetadata {
            source: preview.source,
            settings: ProcessingSettings::from(options),
            bounds_mm: [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]],
            bodies: Vec::new(),
            diagnostics: preview.diagnostics,
        };
        let mut meshes = Vec::new();
        let mut triangles = 0usize;
        let mut vertices = 0usize;
        for body in preview.bodies {
            control.check(ConversionPhase::Analysis, None)?;
            let prepared = fabstep_core::prepare_mesh_bodies_for_sections_controlled(
                &body.mesh,
                options,
                &mut || control.check(ConversionPhase::Analysis, None),
            );
            control.check(ConversionPhase::Analysis, None)?;
            let prepared = prepared.map_err(|e| e.with_body(&body.id, &body.name))?;
            let count = prepared.len();
            if meshes.len() + count > 32 {
                return Err(FabError::new(
                    "input.too_many_bodies",
                    "La préparation source est limitée à 32 corps.",
                ));
            }
            for (component_index, prepared) in prepared.into_iter().enumerate() {
                triangles = triangles
                    .checked_add(prepared.mesh.triangles.len())
                    .ok_or_else(budget)?;
                vertices = vertices
                    .checked_add(prepared.mesh.vertices.len())
                    .ok_or_else(budget)?;
                if triangles > 100_000 || vertices > 300_000 {
                    return Err(budget());
                }
                let solid_id = meshes.len();
                let analysis_body_id = format!("{}/component/{component_index}", body.id);
                let name = if count == 1 {
                    body.name.clone()
                } else {
                    format!("{} · {}", body.name, component_index + 1)
                };
                control.run(ConversionPhase::SourceValidation, Some(solid_id), || {
                    fabstep_cad::validate_source(&triangle_model(&prepared.mesh))
                        .map(|_| ())
                        .map_err(|e| e.with_body(&analysis_body_id, &name))
                })?;
                for axis in 0..3 {
                    metadata.bounds_mm[0][axis] =
                        metadata.bounds_mm[0][axis].min(prepared.bounds_mm[0][axis]);
                    metadata.bounds_mm[1][axis] =
                        metadata.bounds_mm[1][axis].max(prepared.bounds_mm[1][axis]);
                }
                if metadata.diagnostics.len() + prepared.diagnostics.len() > 4096 {
                    return Err(budget());
                }
                metadata
                    .diagnostics
                    .extend(prepared.diagnostics.iter().cloned());
                let mut provenance = body.provenance.clone();
                provenance.component_index = component_index;
                metadata.bodies.push(SourceSectionBody {
                    solid_id,
                    analysis_body_id,
                    name,
                    provenance,
                    bounds_mm: prepared.bounds_mm,
                    triangles: prepared.mesh.triangles.len(),
                    input_triangles: prepared.input_triangles,
                    welded_vertices: prepared.welded_vertices,
                    volume_mm3: prepared.volume_mm3,
                    diagnostics: prepared.diagnostics,
                });
                meshes.push(prepared.mesh);
            }
        }
        if meshes.is_empty() {
            return Err(FabError::new(
                "mesh.empty",
                "La préparation ne contient aucun corps source.",
            ));
        }
        if meshes.len() > 1 {
            control.run(ConversionPhase::SourceSeparation, None, || {
                let models = meshes.iter().map(triangle_model).collect::<Vec<_>>();
                fabstep_cad::validate_sources_separated(&models)
            })?;
        }
        // The preparation has now proved every source body and their complete
        // separation. Preserve all preprocessing diagnostics; only this fulfilled
        // prerequisite from the topology-only stage is no longer applicable.
        metadata
            .diagnostics
            .retain(|diagnostic| diagnostic.code != "validation.native_required");
        for body in &mut metadata.bodies {
            body.diagnostics
                .retain(|diagnostic| diagnostic.code != "validation.native_required");
        }
        control.check(ConversionPhase::Finish, None)?;
        Ok(PreparedSourceSection { metadata, meshes })
    })
}

fn budget() -> FabError {
    FabError::new(
        "dxf.too_complex",
        "La préparation source dépasse son budget de géométrie ou de diagnostics.",
    )
}
