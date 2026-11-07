//! Orchestration commune à la CLI et au bureau, sans accès aux fichiers utilisateur.
use fabstep_cad::{
    build_extrusion_step, build_mixed_step, build_revolved_step, build_rounded_box_step,
    build_rounded_step, build_step, tessellate_step, validate_source, CadReport,
};
use fabstep_core::{
    analyze_stl, recognize_complete_sphere, recognize_complete_torus, recognize_extrusion,
    recognize_mixed, recognize_revolved, recognize_rounded, recognize_rounded_box, Analysis,
    FabError, Mesh, MixedRecognition, Options, PlanarFace, PlanarModel, Recognition,
    RecognitionReport, ReconstructionMode, RevolvedRecognition, RoundedBoxRecognition,
    RoundedRecognition,
};
use fabstep_verify::{compare_meshes, FidelityReport};
use serde::Serialize;

mod document;
pub use document::*;

/// Snapshot des réglages géométriques de la requête. L'unité effective est
/// portée séparément par DocumentSource : une préférence STL ne décrit pas un 3MF.
/// Les deux seuils analytiques sont actifs dans tous les modes analytiques.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessingSettings {
    pub reconstruction: ReconstructionMode,
    pub weld_tolerance_mm: f64,
    pub plane_tolerance_mm: f64,
    pub cylinder_fit_tolerance_mm: f64,
    pub max_deviation_mm: f64,
}

impl From<&Options> for ProcessingSettings {
    fn from(options: &Options) -> Self {
        Self {
            reconstruction: options.reconstruction,
            weld_tolerance_mm: options.weld_tolerance_mm,
            plane_tolerance_mm: options.plane_tolerance_mm,
            cylinder_fit_tolerance_mm: options.cylinder_fit_tolerance_mm,
            max_deviation_mm: options.max_deviation_mm,
        }
    }
}

fn precision_diagnostic(options: &Options) -> Option<fabstep_core::Diagnostic> {
    let defaults = Options::default();
    let custom = options.weld_tolerance_mm != defaults.weld_tolerance_mm
        || options.plane_tolerance_mm != defaults.plane_tolerance_mm
        || (options.reconstruction != ReconstructionMode::Planar
            && (options.cylinder_fit_tolerance_mm != defaults.cylinder_fit_tolerance_mm
                || options.max_deviation_mm != defaults.max_deviation_mm));
    custom.then(|| fabstep_core::Diagnostic {
        code: "precision.custom_tolerances".into(),
        severity: fabstep_core::Severity::Info,
        message: "Des tolérances choisies diffèrent des valeurs initiales. Les réglages sont consignés dans le rapport ; aucune adaptation automatique.".into(),
        count: 1,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversionReport {
    pub settings: ProcessingSettings,
    pub analysis: Analysis,
    pub cad: CadReport,
    pub recognition: Option<Recognition>,
    pub mixed_recognition: Option<MixedRecognition>,
    pub revolved_recognition: Option<RevolvedRecognition>,
    pub rounded_recognition: Option<RoundedRecognition>,
    pub rounded_box_recognition: Option<RoundedBoxRecognition>,
    pub fidelity: Option<FidelityReport>,
}

pub struct Conversion {
    pub report: ConversionReport,
    pub step_bytes: Vec<u8>,
}

/// Les représentations analytiques restent distinctes. La reconnaissance
/// locale peut aussi enrichir une extrusion complète dont le profil polygonal
/// contient un arc ouvert. Un budget dépassé ou une preuve invalide ne sont
/// jamais masqués par le résultat moins détaillé de l'extrusion.
pub enum Recognized {
    Extrusion(Recognition),
    Mixed(MixedRecognition),
    Revolved(RevolvedRecognition),
    Rounded(RoundedRecognition),
    RoundedBox(RoundedBoxRecognition),
}
impl Recognized {
    pub fn report(&self) -> &RecognitionReport {
        match self {
            Self::Extrusion(r) => &r.report,
            Self::Mixed(r) => &r.report,
            Self::Revolved(r) => &r.report,
            Self::Rounded(r) => &r.report,
            Self::RoundedBox(r) => &r.report,
        }
    }
}

/// Chaque mode ajoute une preuve sans modifier les modes précédents. Seule
/// l'absence de candidat autorise leur parcours historique ; une preuve
/// échouée ou un budget dépassé ne sont jamais masqués par un repli.
pub fn recognize_surfaces(analysis: &Analysis, options: &Options) -> Result<Recognized, FabError> {
    match options.reconstruction {
        ReconstructionMode::Planar => Err(FabError::new(
            "recognition.disabled",
            "Activer un mode de reconstruction analytique.",
        )),
        ReconstructionMode::Cylinders => recognize_cylinders(analysis, options),
        ReconstructionMode::Primitives => match recognize_revolved(analysis, options) {
            Ok(result) => Ok(Recognized::Revolved(result)),
            Err(error) if error.code == "recognition.unsupported" => {
                let cylindrical = Options {
                    reconstruction: ReconstructionMode::Cylinders,
                    ..options.clone()
                };
                recognize_cylinders(analysis, &cylindrical)
            }
            Err(error) => Err(error),
        },
        ReconstructionMode::Surfaces => match recognize_rounded(analysis, options) {
            Ok(result) => Ok(Recognized::Rounded(result)),
            Err(error) if error.code == "recognition.unsupported" => {
                let primitives = Options {
                    reconstruction: ReconstructionMode::Primitives,
                    ..options.clone()
                };
                match recognize_surfaces(analysis, &primitives) {
                    Err(error) if error.code == "recognition.unsupported" => {
                        // A two-level cylinder can also have cospherical vertices.
                        // Preserve every historical success and blocking refusal
                        // before considering entire boundary-free surfaces.
                        recognize_terminal_surface(analysis, options)
                    }
                    result => result,
                }
            }
            Err(error) => Err(error),
        },
    }
}
fn recognize_terminal_surface(
    analysis: &Analysis,
    options: &Options,
) -> Result<Recognized, FabError> {
    match recognize_rounded_box(analysis, options) {
        Err(error) if error.code == "recognition.unsupported" => {}
        result => return result.map(Recognized::RoundedBox),
    }
    match recognize_complete_torus(analysis, options) {
        Err(error) if error.code == "recognition.unsupported" => {
            recognize_complete_sphere(analysis, options).map(Recognized::Rounded)
        }
        result => result.map(Recognized::Rounded),
    }
}

pub fn recognize_cylinders(analysis: &Analysis, options: &Options) -> Result<Recognized, FabError> {
    match recognize_extrusion(analysis, options) {
        Ok(r) => match recognize_mixed(analysis, options) {
            Ok(mixed) if !mixed.model.partial_cylinders.is_empty() => Ok(Recognized::Mixed(mixed)),
            Ok(_) => Ok(Recognized::Extrusion(r)),
            Err(e) if e.code == "recognition.unsupported" => Ok(Recognized::Extrusion(r)),
            Err(e) => Err(e),
        },
        Err(e) if e.code == "recognition.unsupported" => {
            recognize_mixed(analysis, options).map(Recognized::Mixed)
        }
        Err(e) => Err(e),
    }
}

/// La réussite requiert un solide réimporté et des mesures cohérentes avec le maillage.
pub fn convert_stl(bytes: &[u8], options: &Options) -> Result<Conversion, FabError> {
    let mut conversion = convert_analysis(analyze_stl(bytes, options)?, options)?;
    if let Some(diagnostic) = precision_diagnostic(options) {
        conversion
            .report
            .analysis
            .report
            .diagnostics
            .push(diagnostic);
    }
    Ok(conversion)
}

fn convert_analysis(mut analysis: Analysis, options: &Options) -> Result<Conversion, FabError> {
    let recognized = match options.reconstruction {
        ReconstructionMode::Planar => None,
        ReconstructionMode::Cylinders
        | ReconstructionMode::Primitives
        | ReconstructionMode::Surfaces => Some(recognize_surfaces(&analysis, options)?),
    };
    // Contrôler les triangles exacts dans TOUS les modes avant la construction.
    // Même le regroupement planaire peut effacer un relief auto-intersectant
    // dont les écarts restent sous la tolérance de segmentation. La validité
    // du seul modèle regroupé ne prouverait donc pas celle de la source.
    // Aucun STEP source intermédiaire n'est sérialisé.
    validate_source(&triangle_model(&analysis.mesh))?;
    let result = if let Some(recognized) = &recognized {
        match recognized {
            Recognized::Extrusion(r) => build_extrusion_step(&r.extrusion)?,
            Recognized::Mixed(r) => build_mixed_step(&r.model)?,
            Recognized::Revolved(r) => build_revolved_step(&r.model)?,
            Recognized::Rounded(r) => build_rounded_step(&r.model)?,
            Recognized::RoundedBox(r) => build_rounded_box_step(&r.model)?,
        }
    } else {
        build_step(&analysis.model)?
    };
    let bounds = analysis.report.bounds_mm;
    let extent: [f64; 3] = std::array::from_fn(|i| bounds[1][i] - bounds[0][i]);
    let tolerance = options.weld_tolerance_mm.max(options.plane_tolerance_mm);
    let recognition_report = recognized.as_ref().map(Recognized::report);
    let geometric_budget = recognition_report.map_or(tolerance * 8.0, |r| r.max_deviation_bound_mm);
    let rounding_budget = extent.into_iter().fold(0.0_f64, f64::max) * 1e-9 + 1e-7;
    let length_budget = geometric_budget + rounding_budget;
    if !geometric_budget.is_finite() || geometric_budget < 0.0 || !length_budget.is_finite() {
        return Err(FabError::new(
            "validation.numeric_range",
            "Le budget d'écart géométrique est invalide.",
        ));
    }
    if let Some(report) = recognition_report {
        let expected = report.expected_volume_mm3;
        if geometric_budget > options.max_deviation_mm
            || !expected.is_finite()
            || expected <= 0.0
            || (result.report.volume_mm3 - expected).abs() > (expected * 1e-8).max(1e-8)
            || result.report.cylindrical_faces != report.cylinders
            || result.report.conical_faces != report.cones
            || result.report.spherical_faces != report.spheres
            || result.report.toroidal_faces != report.tori
        {
            return Err(FabError::new("validation.analytic_mismatch", "Le solide réimporté ne correspond pas aux primitives reconnues ou dépasse l'écart demandé."));
        }
    }
    for (actual, expected) in result
        .report
        .bounds_mm
        .iter()
        .flatten()
        .zip(bounds.iter().flatten())
    {
        if !actual.is_finite() || (actual - expected).abs() > length_budget {
            return Err(FabError::new(
                "validation.bounds_mismatch",
                "Les dimensions du solide diffèrent du maillage au-delà de la tolérance.",
            ));
        }
    }
    let area: f64 = analysis
        .mesh
        .triangles
        .iter()
        .map(|t| {
            let p = t.map(|i| analysis.mesh.vertices[i]);
            let a: [f64; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
            let b: [f64; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
            let c = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            c[0].hypot(c[1]).hypot(c[2]) * 0.5
        })
        .sum();
    // Garde de cohérence de volume, distincte de la borne géométrique reconnue.
    let volume_tolerance = if recognized.is_some() {
        geometric_budget
    } else {
        tolerance
    };
    let volume_budget = (area * volume_tolerance * 8.0).max(analysis.report.volume_mm3 * 1e-8);
    if !length_budget.is_finite() || !volume_budget.is_finite() {
        return Err(FabError::new(
            "validation.numeric_range",
            "Les dimensions dépassent la précision numérique prise en charge.",
        ));
    }
    if result.report.solids != 1
        || !result.report.volume_mm3.is_finite()
        || result.report.volume_mm3 <= 0.0
        || (result.report.volume_mm3 - analysis.report.volume_mm3).abs() > volume_budget
    {
        return Err(FabError::new(
            "validation.volume_mismatch",
            "Le volume du solide diffère du maillage au-delà de la tolérance.",
        ));
    }
    analysis
        .report
        .diagnostics
        .retain(|d| d.code != "validation.native_required");
    if let Some(report) = recognition_report {
        analysis
            .report
            .diagnostics
            .extend(report.diagnostics.iter().cloned());
    }
    let fidelity = if recognized.is_some() {
        let deflection = (options.max_deviation_mm * 0.1).min(0.01);
        if deflection < 1e-6 {
            return Err(FabError::new("validation.distance_resolution", "Le budget demandé nécessite une tessellation plus fine que la résolution prise en charge."));
        }
        let target = tessellate_step(&result.bytes, deflection)?;
        let report = compare_meshes(&analysis.mesh, &target, deflection)?;
        // Audit échantillonné de deux maillages : le paramètre de tessellation
        // est explicite. Aucun maximum mesuré n’est présenté comme une preuve
        // de distance globale entre les surfaces continues.
        let limit = options.max_deviation_mm + deflection + rounding_budget;
        if report.source_to_result_mm.max(report.result_to_source_mm) > limit {
            return Err(FabError::new("validation.distance_exceeded", format!("L’écart échantillonné source/résultat dépasse le budget de {limit:.6} mm (tessellation incluse).")));
        }
        Some(report)
    } else {
        None
    };
    let (
        recognition,
        mixed_recognition,
        revolved_recognition,
        rounded_recognition,
        rounded_box_recognition,
    ) = match recognized {
        Some(Recognized::Extrusion(r)) => (Some(r), None, None, None, None),
        Some(Recognized::Mixed(r)) => (None, Some(r), None, None, None),
        Some(Recognized::Revolved(r)) => (None, None, Some(r), None, None),
        Some(Recognized::Rounded(r)) => (None, None, None, Some(r), None),
        Some(Recognized::RoundedBox(r)) => (None, None, None, None, Some(r)),
        None => (None, None, None, None, None),
    };
    Ok(Conversion {
        report: ConversionReport {
            settings: ProcessingSettings::from(options),
            analysis,
            cad: result.report,
            recognition,
            mixed_recognition,
            revolved_recognition,
            rounded_recognition,
            rounded_box_recognition,
            fidelity,
        },
        step_bytes: result.bytes,
    })
}

fn triangle_model(mesh: &Mesh) -> PlanarModel {
    let faces = mesh
        .triangles
        .iter()
        .map(|triangle| {
            let p = triangle.map(|i| mesh.vertices[i]);
            let a: [f64; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
            let b: [f64; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
            let normal = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let length = normal[0].hypot(normal[1]).hypot(normal[2]);
            PlanarFace {
                loops: vec![triangle.to_vec()],
                normal: normal.map(|value| value / length),
            }
        })
        .collect();
    PlanarModel {
        vertices: mesh.vertices.clone(),
        faces,
    }
}
