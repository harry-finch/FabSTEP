//! Orchestration commune à la CLI et au bureau, sans accès aux fichiers utilisateur.
#[cfg(feature = "large-stl")]
pub use fabstep_cad::prepare_isolated_atlas_checks;
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
use serde::{Deserialize, Serialize};

pub use fabstep_core::{SOURCE_MAX_TRIANGLES, SOURCE_MAX_VERTICES};

mod control;
use control::{controlled, ConversionControl};
pub use control::{ConversionCheckpoint, ConversionPhase};
mod document;
mod product_name;
pub use document::*;
pub mod isolated_report;
mod parameter_search;
pub use parameter_search::{search_parameters, ParameterSearchReport, ParameterTrial};
mod source_section;
pub use source_section::{
    prepare_source_section, prepare_source_section_controlled, PreparedSourceSection,
    SourceSectionBody, SourceSectionMetadata,
};
mod source_proof_cache;
use source_proof_cache::SourceProofCache;

/// Snapshot des réglages géométriques de la requête. L'unité effective est
/// portée séparément par DocumentSource : une préférence STL ne décrit pas un 3MF.
/// Les deux seuils analytiques sont actifs dans tous les modes analytiques.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_patch_recognition: Option<fabstep_core::HeightPatchRecognition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cubic_prism_recognition: Option<fabstep_core::CubicPrismRecognition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oblique_lip_recognition: Option<fabstep_core::oblique_lip::ObliqueLipRecognition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oblique_band_recognition: Option<fabstep_core::oblique_band::ObliqueBandRecognition>,
    pub fidelity: Option<FidelityReport>,
}

mod native_blends;

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
    HeightPatch(fabstep_core::HeightPatchRecognition),
    CubicPrism(fabstep_core::CubicPrismRecognition),
    ObliqueLip(fabstep_core::oblique_lip::ObliqueLipRecognition),
    ObliqueBand(fabstep_core::oblique_band::ObliqueBandRecognition),
}
impl Recognized {
    pub fn report(&self) -> &RecognitionReport {
        match self {
            Self::Extrusion(r) => &r.report,
            Self::Mixed(r) => &r.report,
            Self::Revolved(r) => &r.report,
            Self::Rounded(r) => &r.report,
            Self::RoundedBox(r) => &r.report,
            Self::HeightPatch(r) => &r.report,
            Self::CubicPrism(r) => &r.report,
            Self::ObliqueLip(r) => &r.report,
            Self::ObliqueBand(r) => &r.report,
        }
    }
}

/// Exact whole-body domains that publish one source-owner index per triangle.
fn recognize_partitioned_body(
    analysis: &Analysis,
    options: &Options,
) -> Result<Option<Recognized>, FabError> {
    if let Some(prism) = fabstep_core::recognize_cubic_prism(analysis, options)? {
        return Ok(Some(Recognized::CubicPrism(prism)));
    }
    if let Some(lip) = fabstep_core::oblique_lip::recognize(analysis, options)? {
        return Ok(Some(Recognized::ObliqueLip(lip)));
    }
    fabstep_core::oblique_band::recognize(analysis, options).map(|r| r.map(Recognized::ObliqueBand))
}

/// Chaque mode ajoute une preuve sans modifier les modes précédents. Seule
/// l'absence de candidat autorise leur parcours historique ; une preuve
/// échouée ou un budget dépassé ne sont jamais masqués par un repli.
pub fn recognize_surfaces(analysis: &Analysis, options: &Options) -> Result<Recognized, FabError> {
    match options.reconstruction {
        ReconstructionMode::HeightPatch => {
            fabstep_core::recognize_height_patch(analysis, options).map(Recognized::HeightPatch)
        }
        ReconstructionMode::Auto => {
            if let Some(body) = recognize_partitioned_body(analysis, options)? {
                return Ok(body);
            }
            fabstep_core::recognize_automatic(analysis, options).map(Recognized::Rounded)
        }
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

fn convert_analysis(analysis: Analysis, options: &Options) -> Result<Conversion, FabError> {
    controlled(&mut |_| Ok(()), |control| {
        convert_analysis_with_face_tokens(analysis, options, None, control, 0, None)
    })
}

fn convert_analysis_with_face_tokens(
    mut analysis: Analysis,
    options: &Options,
    face_tokens: Option<&[usize]>,
    control: &mut ConversionControl<'_>,
    body_index: usize,
    mut source_proofs: Option<&mut SourceProofCache>,
) -> Result<Conversion, FabError> {
    if face_tokens.is_none() {
        if let Some(built) = native_blends::try_build(
            &mut analysis,
            options,
            1,
            body_index,
            control,
            source_proofs.as_deref_mut(),
        )? {
            return Ok(built.finish(analysis, options));
        }
    }
    let recognized = match options.reconstruction {
        ReconstructionMode::Planar => None,
        ReconstructionMode::Auto
        | ReconstructionMode::Cylinders
        | ReconstructionMode::Primitives
        | ReconstructionMode::HeightPatch
        | ReconstructionMode::Surfaces => Some(control.run(
            ConversionPhase::Recognition,
            Some(body_index),
            || recognize_surfaces(&analysis, options),
        )?),
    };
    convert_analysis_with_recognized(
        analysis,
        options,
        face_tokens,
        recognized,
        control,
        body_index,
        source_proofs,
    )
}

/// Group order shared by the native face tokens and the source correspondence.
fn rounded_face_counts(model: &fabstep_core::RoundedModel) -> [usize; 12] {
    let m = &model.revolved.mixed;
    [
        m.planes.len(),
        m.cylinders.len() + 2 * usize::from(model.drilled_shaft.is_some()),
        model.partial_cylinder_count(),
        model.revolved.cones.len(),
        model.spheres.len(),
        model.tori.len(),
        model.partial_cones.len(),
        model.octants.len(),
        model.linear_blend.as_ref().map_or(0, |b| b.iter().count()),
        model
            .cross_drill
            .as_ref()
            .map_or(0, |f| 3 + usize::from(f.stock_fillet.is_some())),
        usize::from(model.smooth_revolution.is_some()),
        model.torus_sectors.len(),
    ]
}

fn audit_reimport(
    analysis: &mut Analysis,
    options: &Options,
    result: &fabstep_cad::ValidatedStep,
    bound: Option<f64>,
    control: &mut ConversionControl<'_>,
    body_index: usize,
) -> Result<Option<FidelityReport>, FabError> {
    let bounds = analysis.report.bounds_mm;
    let extent: [f64; 3] = std::array::from_fn(|i| bounds[1][i] - bounds[0][i]);
    let tolerance = options.weld_tolerance_mm.max(options.plane_tolerance_mm);
    let geometric_budget = bound.unwrap_or(tolerance * 8.0);
    let rounding_budget = extent.into_iter().fold(0.0_f64, f64::max) * 1e-9 + 1e-7;
    let length_budget = geometric_budget + rounding_budget;
    if !geometric_budget.is_finite() || geometric_budget < 0.0 || !length_budget.is_finite() {
        return Err(FabError::new(
            "validation.numeric_range",
            "Le budget d'écart géométrique est invalide.",
        ));
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
    let volume_tolerance = if bound.is_some() {
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
    let fidelity = if bound.is_some() {
        let deflection = (options.max_deviation_mm * 0.1).min(0.01);
        if deflection < 1e-6 {
            return Err(FabError::new("validation.distance_resolution", "Le budget demandé nécessite une tessellation plus fine que la résolution prise en charge."));
        }
        let target = control.run(ConversionPhase::AuditTessellation, Some(body_index), || {
            tessellate_step(&result.bytes, deflection)
        })?;
        let report = control.run(ConversionPhase::AuditComparison, Some(body_index), || {
            compare_meshes(&analysis.mesh, &target, deflection)
        })?;
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
    analysis
        .report
        .diagnostics
        .retain(|d| d.code != "validation.native_required");
    Ok(fidelity)
}

fn validate_mesh_source(
    mesh: &Mesh,
    body_index: usize,
    control: &mut ConversionControl<'_>,
    source_proofs: Option<&mut SourceProofCache>,
) -> Result<(), FabError> {
    // Contrôler les triangles exacts dans TOUS les modes avant la construction.
    // Même le regroupement planaire peut effacer un relief auto-intersectant
    // dont les écarts restent sous la tolérance de segmentation. La validité
    // du seul modèle regroupé ne prouverait donc pas celle de la source.
    // Aucun STEP source intermédiaire n'est sérialisé.
    control.run(ConversionPhase::SourceValidation, Some(body_index), || {
        let prove = || validate_source(&triangle_model(mesh)).map(|_| ());
        match source_proofs {
            Some(cache) => cache.validate_source(body_index, mesh, prove),
            None => prove(),
        }
    })
}

fn convert_analysis_with_recognized(
    mut analysis: Analysis,
    options: &Options,
    face_tokens: Option<&[usize]>,
    recognized: Option<Recognized>,
    control: &mut ConversionControl<'_>,
    body_index: usize,
    source_proofs: Option<&mut SourceProofCache>,
) -> Result<Conversion, FabError> {
    validate_mesh_source(&analysis.mesh, body_index, control, source_proofs)?;
    let result = control.run(ConversionPhase::Build, Some(body_index), || {
        if let Some(recognized) = &recognized {
            match recognized {
                Recognized::Extrusion(r) => build_extrusion_step(&r.extrusion),
                Recognized::Mixed(r) => {
                    if let Some(tokens) = face_tokens {
                        let count = r.model.planes.len();
                        if tokens.len() != count + r.model.cylinders.len() {
                            return Err(FabError::new(
                                "step.invalid_source_provenance",
                                "Le nombre de jetons du résultat guidé est incohérent.",
                            ));
                        }
                        fabstep_cad::build_mixed_step_with_provenance(
                            &r.model,
                            &tokens[..count],
                            &tokens[count..],
                        )
                    } else {
                        build_mixed_step(&r.model)
                    }
                }
                Recognized::Revolved(r) => {
                    // Plan-only guidance uses the existing planar native path.
                    // Keep its recognition report for fidelity and provenance.
                    if r.model.cones.is_empty() && r.model.mixed.cylinders.is_empty()
                        && r.model.mixed.partial_cylinders.is_empty()
                    {
                        let model = PlanarModel {
                            vertices: r.model.mixed.vertices.clone(),
                            faces: r.model.mixed.planes.iter().map(|plane| {
                                let loops = plane.loops.iter().map(|lp| match lp {
                                    fabstep_core::MixedLoop::Polygon { vertices } => Ok(vertices.clone()),
                                    _ => Err(FabError::new("guidance.plane_unsupported_boundary", "Contour courbe non pris en charge pour cette correction plane.")),
                                }).collect::<Result<Vec<_>, _>>()?;
                                Ok(PlanarFace { loops, normal: plane.normal })
                            }).collect::<Result<Vec<_>, FabError>>()?,
                        };
                        return if let Some(tokens) = face_tokens {
                            fabstep_cad::build_step_with_provenance(&model, tokens)
                        } else { build_step(&model) };
                    }
                    if let Some(tokens) = face_tokens {
                        let planes = r.model.mixed.planes.len();
                        let cylinders = r.model.mixed.cylinders.len();
                        if tokens.len() != planes + cylinders + r.model.cones.len() {
                            return Err(FabError::new(
                                "step.invalid_source_provenance",
                                "Le nombre de jetons du résultat guidé est incohérent.",
                            ));
                        }
                        fabstep_cad::build_revolved_step_with_provenance(
                            &r.model,
                            &tokens[..planes],
                            &tokens[planes..planes + cylinders],
                            &tokens[planes + cylinders..],
                        )
                    } else {
                        build_revolved_step(&r.model)
                    }
                }
                Recognized::Rounded(r) => {
                    let m = &r.model.revolved.mixed;
                    let counts = rounded_face_counts(&r.model);
                    if counts[1..].iter().all(|&count| count == 0) {
                        let model = PlanarModel { vertices: m.vertices.clone(), faces: m.planes.iter().map(|plane| {
                            let loops = plane.loops.iter().map(|lp| match lp {
                                fabstep_core::MixedLoop::Polygon { vertices } => Ok(vertices.clone()),
                                _ => Err(FabError::new("cad.invalid_model", "Un modèle entièrement plan contient une frontière courbe.")),
                            }).collect::<Result<Vec<_>, _>>()?;
                            Ok(PlanarFace { normal: plane.normal, loops })
                        }).collect::<Result<Vec<_>, FabError>>()? };
                        return match face_tokens { Some(tokens) => fabstep_cad::build_step_with_provenance(&model,tokens), None => build_step(&model) };
                    }
                    if let Some(tokens) = face_tokens {
                        if counts.iter().sum::<usize>() != tokens.len() {
                            return Err(FabError::new("step.invalid_source_provenance", "Nombre de jetons du résultat arrondi incohérent."));
                        }
                        let mut rest = tokens;
                        let groups = counts.map(|count| {
                            let (group, next) = rest.split_at(count);
                            rest = next;
                            group
                        });
                        fabstep_cad::build_rounded_step_with_token_groups(&r.model, groups)
                    } else { build_rounded_step(&r.model) }
                },
                Recognized::HeightPatch(r) => fabstep_cad::build_height_patch(&r.model, face_tokens),
                Recognized::CubicPrism(r) => fabstep_cad::build_cubic_prism(&r.model, face_tokens),
                Recognized::ObliqueLip(r) => fabstep_cad::build_oblique_lip(&r.model, face_tokens),
                Recognized::ObliqueBand(r) => fabstep_cad::build_oblique_band(&r.model,face_tokens),
                Recognized::RoundedBox(r) => build_rounded_box_step(&r.model),
            }
        } else if let Some(tokens) = face_tokens {
            fabstep_cad::build_step_with_provenance(&analysis.model, tokens)
        } else {
            build_step(&analysis.model)
        }
    })?;
    let recognition_report = recognized.as_ref().map(Recognized::report);
    if let Some(report) = recognition_report {
        let expected = report.expected_volume_mm3;
        if report.max_deviation_bound_mm > options.max_deviation_mm
            || !expected.is_finite()
            || expected <= 0.0
            || (result.report.volume_mm3 - expected).abs() > (expected * 1e-8).max(1e-8)
            || result.report.cylindrical_faces != report.cylinders
            || result.report.conical_faces != report.cones
            || result.report.spherical_faces != report.spheres
            || result.report.toroidal_faces != report.tori
            || result.report.revolution_faces != report.revolutions
        {
            return Err(FabError::new("validation.analytic_mismatch", "Le solide réimporté ne correspond pas aux primitives reconnues ou dépasse l'écart demandé."));
        }
    }
    let fidelity = audit_reimport(
        &mut analysis,
        options,
        &result,
        recognition_report.map(|r| r.max_deviation_bound_mm),
        control,
        body_index,
    )?;
    if let Some(report) = recognition_report {
        analysis
            .report
            .diagnostics
            .extend(report.diagnostics.iter().cloned());
    }
    let mut report = ConversionReport {
        settings: ProcessingSettings::from(options),
        analysis,
        cad: result.report,
        fidelity,
        recognition: None,
        mixed_recognition: None,
        revolved_recognition: None,
        rounded_recognition: None,
        rounded_box_recognition: None,
        height_patch_recognition: None,
        cubic_prism_recognition: None,
        oblique_lip_recognition: None,
        oblique_band_recognition: None,
    };
    match recognized {
        Some(Recognized::Extrusion(r)) => report.recognition = Some(r),
        Some(Recognized::Mixed(r)) => report.mixed_recognition = Some(r),
        Some(Recognized::Revolved(r)) => report.revolved_recognition = Some(r),
        Some(Recognized::Rounded(r)) => report.rounded_recognition = Some(r),
        Some(Recognized::RoundedBox(r)) => report.rounded_box_recognition = Some(r),
        Some(Recognized::HeightPatch(r)) => report.height_patch_recognition = Some(r),
        Some(Recognized::CubicPrism(r)) => report.cubic_prism_recognition = Some(r),
        Some(Recognized::ObliqueLip(r)) => report.oblique_lip_recognition = Some(r),
        Some(Recognized::ObliqueBand(r)) => report.oblique_band_recognition = Some(r),
        None => {}
    }
    Ok(Conversion {
        report,
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
