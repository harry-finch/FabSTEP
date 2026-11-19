//! Versioned inspection data for the isolated STL worker.
use crate::{BodyProvenance, BodyReport, DocumentReport, DocumentSource, ProcessingSettings};
use fabstep_cad::CadReport;
use fabstep_core::{Diagnostic, RecognitionReport, Report};
use fabstep_verify::FidelityReport;
use serde::{Deserialize, Serialize};

pub const COMPACT_REPORT_VERSION: u32 = 2;
pub const COMPACT_REPORT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactWorkerReport {
    pub version: u32,
    pub report: CompactDocumentReport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactDocumentReport {
    pub source: DocumentSource,
    pub settings: ProcessingSettings,
    pub cad: CadReport,
    pub diagnostics: Vec<Diagnostic>,
    pub bodies: Vec<CompactBodyReport>,
    /// Isolated conversions are unguided. The wire format always carries [].
    pub guidance_hints: [(); 0],
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactBodyReport {
    pub id: String,
    pub name: String,
    pub provenance: BodyProvenance,
    pub settings: ProcessingSettings,
    pub analysis: CompactAnalysisReport,
    pub cad: CadReport,
    pub fidelity: Option<FidelityReport>,
    pub recognition: Option<CompactRecognition>,
    pub mixed_recognition: Option<CompactPlaneRecognition>,
    pub revolved_recognition: Option<CompactPlaneRecognition>,
    pub rounded_recognition: Option<CompactPlaneRecognition>,
    pub rounded_box_recognition: Option<CompactRecognition>,
    pub height_patch_recognition: Option<CompactRecognition>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactAnalysisReport {
    pub report: Report,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactRecognition {
    pub report: RecognitionReport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactPlaneRecognition {
    pub report: RecognitionReport,
    pub planar_faces: usize,
}

impl From<&DocumentReport> for CompactWorkerReport {
    fn from(report: &DocumentReport) -> Self {
        Self {
            version: COMPACT_REPORT_VERSION,
            report: CompactDocumentReport {
                source: report.source.clone(),
                settings: report.settings.clone(),
                cad: report.cad.clone(),
                diagnostics: report.diagnostics.clone(),
                bodies: report.bodies.iter().map(CompactBodyReport::from).collect(),
                guidance_hints: [],
            },
        }
    }
}

impl From<&BodyReport> for CompactBodyReport {
    fn from(body: &BodyReport) -> Self {
        let data = &body.conversion;
        Self {
            id: body.id.clone(),
            name: body.name.clone(),
            provenance: body.provenance.clone(),
            settings: data.settings.clone(),
            analysis: CompactAnalysisReport {
                report: data.analysis.report.clone(),
            },
            cad: data.cad.clone(),
            fidelity: data.fidelity.clone(),
            recognition: data.recognition.as_ref().map(|r| CompactRecognition {
                report: r.report.clone(),
            }),
            mixed_recognition: data
                .mixed_recognition
                .as_ref()
                .map(|r| CompactPlaneRecognition {
                    report: r.report.clone(),
                    planar_faces: r.model.planes.len(),
                }),
            revolved_recognition: data.revolved_recognition.as_ref().map(|r| {
                CompactPlaneRecognition {
                    report: r.report.clone(),
                    planar_faces: r.model.mixed.planes.len(),
                }
            }),
            rounded_recognition: data.rounded_recognition.as_ref().map(|r| {
                CompactPlaneRecognition {
                    report: r.report.clone(),
                    planar_faces: r.model.revolved.mixed.planes.len(),
                }
            }),
            rounded_box_recognition: data.rounded_box_recognition.as_ref().map(|r| {
                CompactRecognition {
                    report: r.report.clone(),
                }
            }),
            height_patch_recognition: data.height_patch_recognition.as_ref().map(|r| {
                CompactRecognition {
                    report: r.report.clone(),
                }
            }),
        }
    }
}
