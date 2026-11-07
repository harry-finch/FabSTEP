//! Adaptateur natif Open CASCADE. Le STEP est écrit puis relu en mémoire.

use fabstep_core::{
    AnalyticExtrusion, FabError, Mesh, MixedCone, MixedLoop, MixedModel, MixedSegment, MixedSphere,
    MixedTorus, PlanarModel, ProfileLoop, RevolvedModel, RoundedBoxModel, RoundedModel,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::ffi::{c_char, CStr, CString};
use std::slice;

const MAX_STEP_BYTES: usize = 128 * 1024 * 1024;
const MAX_STEP_BODIES: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CadReport {
    pub solids: usize,
    pub faces: usize,
    pub planar_faces: usize,
    pub cylindrical_faces: usize,
    #[serde(default)]
    pub conical_faces: usize,
    #[serde(default)]
    pub spherical_faces: usize,
    #[serde(default)]
    pub toroidal_faces: usize,
    pub volume_mm3: f64,
    pub bounds_mm: [[f64; 3]; 2],
    pub kernel: String,
}

#[derive(Debug)]
pub struct ValidatedStep {
    pub bytes: Vec<u8>,
    pub report: CadReport,
}

pub struct StepPart<'a> {
    pub bytes: &'a [u8],
    pub name: &'a str,
}

#[derive(Debug)]
pub struct CombinedStep {
    pub bytes: Vec<u8>,
    pub report: CadReport,
    /// Mesures de chaque racine STEP, dans l'ordre des corps fournis.
    pub bodies: Vec<CadReport>,
}

#[repr(C)]
struct NativeStepPart {
    bytes: *const u8,
    size: usize,
    name: *const c_char,
}

#[repr(C)]
struct NativePlanarPart {
    points: *const f64,
    point_count: usize,
    normals: *const f64,
    face_count: usize,
    face_loops: *const usize,
    loop_vertices: *const usize,
    vertex_ids: *const usize,
}

#[repr(C)]
#[derive(Default)]
struct Metrics {
    solids: usize,
    faces: usize,
    planar_faces: usize,
    cylindrical_faces: usize,
    conical_faces: usize,
    spherical_faces: usize,
    toroidal_faces: usize,
    volume: f64,
    bounds: [[f64; 3]; 2],
}

#[repr(C)]
#[derive(Default)]
struct NativeStep {
    data: *mut u8,
    size: usize,
    metrics: Metrics,
}

#[repr(C)]
struct NativeProfileLoop {
    point_start: usize,
    point_count: usize,
    center: [f64; 2],
    radius: f64,
}

#[repr(C)]
#[derive(Default)]
struct NativeMesh {
    points: *mut [f64; 3],
    point_count: usize,
    triangles: *mut [usize; 3],
    triangle_count: usize,
}

#[repr(C)]
struct NativeMixedRing {
    center: [f64; 3],
    axis: [f64; 3],
    radius: f64,
}

#[repr(C)]
struct NativeMixedCylinder {
    rings: [usize; 2],
    inner: i32,
}

#[repr(C)]
struct NativeSphere {
    rings: [usize; 2],
    center: [f64; 3],
    radius: f64,
    inner: i32,
}

#[repr(C)]
struct NativeTorus {
    rings: [usize; 2],
    center: [f64; 3],
    major_radius: f64,
    minor_radius: f64,
    meridian: [f64; 2],
    inner: i32,
}

#[repr(C)]
struct NativeMixedLoop {
    start: usize,
    count: usize,
}

#[repr(C)]
struct NativeMixedArc {
    center: [f64; 3],
    axis: [f64; 3],
    radius: f64,
    sweep: f64,
    vertices: [usize; 2],
}

#[repr(C)]
struct NativeMixedSegment {
    vertices: [usize; 2],
    arc: usize,
}

impl Drop for NativeMesh {
    fn drop(&mut self) {
        // Les deux tableaux appartiennent à l'allocateur C++.
        unsafe { fabstep_free_mesh(self.points.cast(), self.triangles.cast()) };
    }
}

impl Drop for NativeStep {
    fn drop(&mut self) {
        // Le tampon appartient à l'allocateur C++, qui accepte aussi NULL.
        unsafe { fabstep_free_step(self.data) };
    }
}

unsafe extern "C" {
    fn fabstep_build_step(
        points: *const f64,
        point_count: usize,
        normals: *const f64,
        face_count: usize,
        face_loops: *const usize,
        loop_vertices: *const usize,
        vertex_ids: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_validate_source(
        points: *const f64,
        point_count: usize,
        normals: *const f64,
        face_count: usize,
        face_loops: *const usize,
        loop_vertices: *const usize,
        vertex_ids: *const usize,
        output: *mut Metrics,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_extrusion_step(
        origin: *const f64,
        axis: *const f64,
        x_axis: *const f64,
        height: f64,
        points: *const f64,
        loops: *const NativeProfileLoop,
        loop_count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_mixed_step(
        points: *const f64,
        point_count: usize,
        rings: *const NativeMixedRing,
        ring_count: usize,
        normals: *const f64,
        plane_count: usize,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        cylinders: *const NativeMixedCylinder,
        cylinder_count: usize,
        arcs: *const NativeMixedArc,
        arc_count: usize,
        partial: *const NativeMixedCylinder,
        partial_count: usize,
        cones: *const NativeMixedCylinder,
        cone_count: usize,
        spheres: *const NativeSphere,
        sphere_count: usize,
        tori: *const NativeTorus,
        torus_count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_complete_sphere_step(
        center: *const f64,
        radius: f64,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_complete_torus_step(
        center: *const f64,
        axis: *const f64,
        major_radius: f64,
        minor_radius: f64,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_rounded_box_step(
        center: *const f64,
        axes: *const f64,
        core_half_extents: *const f64,
        radius: f64,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_free_step(bytes: *mut u8);
    fn fabstep_combine_steps(
        parts: *const NativeStepPart,
        count: usize,
        bodies: *mut Metrics,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_validate_sources_separated(
        parts: *const NativePlanarPart,
        count: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_tessellate_step(
        bytes: *const u8,
        size: usize,
        deflection: f64,
        output: *mut NativeMesh,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_free_mesh(points: *mut f64, triangles: *mut usize);
    fn fabstep_cad_version() -> *const c_char;
}

/// Regroupe des solides STEP séparés, un par racine, dans l'ordre fourni.
///
/// Chaque entrée doit contenir un seul solide FabSTEP en coordonnées monde.
/// Les corps en contact, intersectés, imbriqués ou coïncidents sont refusés.
/// Le volume du document est la somme des volumes mesurés au réimport.
/// Un nom est refusé si le lecteur STEP ne retrouve pas son libellé.
pub fn combine_steps(parts: &[StepPart<'_>]) -> Result<CombinedStep, FabError> {
    if !(1..=MAX_STEP_BODIES).contains(&parts.len()) {
        return Err(FabError::new(
            "cad.invalid_model",
            "Un document STEP FabSTEP doit contenir de 1 à 32 corps.",
        ));
    }
    let mut total = 0_usize;
    let mut names = Vec::with_capacity(parts.len());
    for part in parts {
        if part.bytes.is_empty() {
            return Err(FabError::new("cad.read_failed", "Un corps STEP est vide."));
        }
        total = total.saturating_add(part.bytes.len());
        if total > MAX_STEP_BYTES {
            return Err(FabError::new(
                "cad.step_document_too_large",
                "La somme des corps STEP dépasse 128 Mio.",
            ));
        }
        let invalid_name = || {
            FabError::new(
            "cad.invalid_model",
            "Le nom d'un corps doit être non vide, sans caractères de contrôle et limité à 128 octets.",
        )
        };
        if part.name.trim().is_empty()
            || part.name.len() > 128
            || part.name.chars().any(char::is_control)
        {
            return Err(invalid_name());
        }
        names.push(CString::new(part.name).map_err(|_| invalid_name())?);
    }

    let inputs: Vec<_> = parts
        .iter()
        .zip(&names)
        .map(|(part, name)| NativeStepPart {
            bytes: part.bytes.as_ptr(),
            size: part.bytes.len(),
            name: name.as_ptr(),
        })
        .collect();
    let mut bodies: Vec<Metrics> = (0..parts.len()).map(|_| Metrics::default()).collect();
    // Tous les octets, noms et tampons de mesures restent valides pendant l'appel.
    let combined = native_step(|output, error, capacity| unsafe {
        fabstep_combine_steps(
            inputs.as_ptr(),
            inputs.len(),
            bodies.as_mut_ptr(),
            output,
            error,
            capacity,
        )
    })?;
    Ok(CombinedStep {
        bytes: combined.bytes,
        report: combined.report,
        bodies: bodies.iter().map(cad_report).collect(),
    })
}

/// Contrôle la séparation des corps triangulés source sans écrire de STEP.
///
/// Chaque face doit avoir un seul contour à trois sommets. Le document est
/// limité à 32 corps, 100 000 triangles et 300 000 sommets cumulés.
pub fn validate_sources_separated(models: &[PlanarModel]) -> Result<(), FabError> {
    if !(1..=MAX_STEP_BODIES).contains(&models.len()) {
        return Err(FabError::new(
            "cad.invalid_model",
            "Un document source doit contenir de 1 à 32 corps.",
        ));
    }
    let mut triangles = 0_usize;
    let mut vertices = 0_usize;
    for model in models {
        triangles = triangles.saturating_add(model.faces.len());
        vertices = vertices.saturating_add(model.vertices.len());
        if triangles > 100_000 || vertices > 300_000 {
            return Err(FabError::new(
                "cad.source_document_too_large",
                "Le document source dépasse 100 000 triangles ou 300 000 sommets.",
            ));
        }
        if model
            .faces
            .iter()
            .any(|face| face.loops.len() != 1 || face.loops[0].len() != 3)
        {
            return Err(FabError::new(
                "cad.invalid_model",
                "La vérification de séparation source attend des faces triangulaires.",
            ));
        }
    }
    let prepared: Vec<_> = models
        .iter()
        .map(prepare_planar)
        .collect::<Result<_, _>>()?;
    let inputs: Vec<_> = models
        .iter()
        .zip(&prepared)
        .map(|(model, input)| NativePlanarPart {
            points: model.vertices.as_ptr().cast(),
            point_count: model.vertices.len(),
            normals: input.normals.as_ptr().cast(),
            face_count: input.normals.len(),
            face_loops: input.face_loops.as_ptr(),
            loop_vertices: input.loop_vertices.as_ptr(),
            vertex_ids: input.vertex_ids.as_ptr(),
        })
        .collect();
    let mut error = [0; 512];
    // Les modèles et leurs tableaux d'indices vivent jusqu'au retour natif.
    let status = unsafe {
        fabstep_validate_sources_separated(
            inputs.as_ptr(),
            inputs.len(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    Ok(())
}

/// Construit un seul solide planaire et valide les octets STEP par réimport.
///
/// Après réimport, le nombre de faces doit être identique. L'écart de volume
/// admis est max(10⁻⁹ mm³, 10⁻⁸ × volume) et l'écart par borne est de 10⁻⁶ mm
/// plus une marge pour l'arrondi en virgule flottante aux grandes coordonnées.
pub fn build_step(model: &PlanarModel) -> Result<ValidatedStep, FabError> {
    let input = prepare_planar(model)?;
    // Les tableaux à plat et les offsets sont valides pendant l'appel.
    native_step(|output, error, capacity| unsafe {
        fabstep_build_step(
            model.vertices.as_ptr().cast(),
            model.vertices.len(),
            input.normals.as_ptr().cast(),
            input.normals.len(),
            input.face_loops.as_ptr(),
            input.loop_vertices.as_ptr(),
            input.vertex_ids.as_ptr(),
            output,
            error,
            capacity,
        )
    })
}

/// Construit et contrôle la B-Rep planaire source sans passer par un fichier STEP.
/// Utiliser des contours triangulaires pour préserver exactement un STL soudé.
pub fn validate_source(model: &PlanarModel) -> Result<CadReport, FabError> {
    let input = prepare_planar(model)?;
    let mut metrics = Metrics::default();
    let mut error = [0; 512];
    // Les indices et offsets ont été vérifiés et les tableaux restent valides.
    let status = unsafe {
        fabstep_validate_source(
            model.vertices.as_ptr().cast(),
            model.vertices.len(),
            input.normals.as_ptr().cast(),
            input.normals.len(),
            input.face_loops.as_ptr(),
            input.loop_vertices.as_ptr(),
            input.vertex_ids.as_ptr(),
            &mut metrics,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    Ok(cad_report(&metrics))
}

struct PlanarInput {
    normals: Vec<[f64; 3]>,
    face_loops: Vec<usize>,
    loop_vertices: Vec<usize>,
    vertex_ids: Vec<usize>,
}

fn prepare_planar(model: &PlanarModel) -> Result<PlanarInput, FabError> {
    if model.faces.len() < 4 || model.vertices.len() < 4 {
        return Err(FabError::new(
            "cad.invalid_model",
            "Au moins quatre faces et quatre sommets sont nécessaires pour former un solide.",
        ));
    }
    if model
        .vertices
        .iter()
        .flatten()
        .any(|coord| !coord.is_finite())
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Un sommet contient une coordonnée non finie.",
        ));
    }

    let mut normals = Vec::with_capacity(model.faces.len());
    let mut face_loops = vec![0];
    let mut loop_vertices = vec![0];
    let mut vertex_ids = Vec::new();
    for face in &model.faces {
        let length_squared: f64 = face.normal.iter().map(|value| value * value).sum();
        if !length_squared.is_finite()
            || length_squared <= f64::MIN_POSITIVE
            || face.loops.is_empty()
        {
            return Err(FabError::new(
                "cad.invalid_model",
                "Une face est sans normale valide ou sans contour.",
            ));
        }
        normals.push(face.normal);
        for boundary in &face.loops {
            if boundary.len() < 3
                || boundary.iter().any(|id| *id >= model.vertices.len())
                || boundary.windows(2).any(|pair| pair[0] == pair[1])
                || boundary.first() == boundary.last()
            {
                return Err(FabError::new(
                    "cad.invalid_model",
                    "Un contour comporte trop peu de sommets ou référence une arête invalide.",
                ));
            }
            vertex_ids.extend_from_slice(boundary);
            loop_vertices.push(vertex_ids.len());
        }
        face_loops.push(loop_vertices.len() - 1);
    }

    Ok(PlanarInput {
        normals,
        face_loops,
        loop_vertices,
        vertex_ids,
    })
}

/// Extrude un profil plan et réimporte le solide STEP avant de rendre ses octets.
/// Les cercles et les parois cylindriques restent analytiques après réimport.
pub fn build_extrusion_step(model: &AnalyticExtrusion) -> Result<ValidatedStep, FabError> {
    let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    if model
        .origin
        .iter()
        .chain(&model.axis)
        .chain(&model.x_axis)
        .any(|v| !v.is_finite())
        || (dot(model.axis, model.axis) - 1.0).abs() > 1e-8
        || (dot(model.x_axis, model.x_axis) - 1.0).abs() > 1e-8
        || dot(model.axis, model.x_axis).abs() > 1e-8
        || !model.height_mm.is_finite()
        || model.height_mm <= 0.0
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le repère d'extrusion doit être orthonormé et la hauteur strictement positive.",
        ));
    }

    let mut points = Vec::<[f64; 2]>::new();
    let mut loops = Vec::with_capacity(1 + model.holes.len());
    for (i, boundary) in std::iter::once(&model.outer)
        .chain(&model.holes)
        .enumerate()
    {
        let native = match boundary {
            ProfileLoop::Circle { center, radius_mm } => {
                if center.iter().any(|c| !c.is_finite())
                    || !radius_mm.is_finite()
                    || *radius_mm <= 0.0
                {
                    return Err(FabError::new(
                        "cad.invalid_model",
                        "Un cercle du profil a un centre invalide ou un rayon non positif.",
                    ));
                }
                NativeProfileLoop {
                    point_start: 0,
                    point_count: 0,
                    center: *center,
                    radius: *radius_mm,
                }
            }
            ProfileLoop::Polygon { points: polygon } => {
                if polygon.len() < 3
                    || polygon.iter().flatten().any(|v| !v.is_finite())
                    || polygon.windows(2).any(|pair| pair[0] == pair[1])
                    || polygon.first() == polygon.last()
                {
                    return Err(FabError::new(
                        "cad.invalid_model",
                        "Un contour d'extrusion comporte des sommets invalides ou répétés.",
                    ));
                }
                let [x, y] = polygon[0];
                let area = polygon[1..]
                    .windows(2)
                    .map(|pair| {
                        (pair[0][0] - x) * (pair[1][1] - y) - (pair[0][1] - y) * (pair[1][0] - x)
                    })
                    .sum::<f64>();
                if !area.is_finite() || area == 0.0 {
                    return Err(FabError::new(
                        "cad.invalid_model",
                        "Un contour d'extrusion a une aire nulle ou invalide.",
                    ));
                }
                let point_start = points.len();
                let clockwise = area < 0.0;
                let is_hole = i != 0;
                if clockwise != is_hole {
                    points.extend(polygon.iter().rev().copied());
                } else {
                    points.extend_from_slice(polygon);
                }
                NativeProfileLoop {
                    point_start,
                    point_count: polygon.len(),
                    center: [0.0; 2],
                    radius: 0.0,
                }
            }
        };
        loops.push(native);
    }

    // Les contours et leur mémoire restent valides pendant l'appel natif.
    native_step(|output, error, capacity| unsafe {
        fabstep_build_extrusion_step(
            model.origin.as_ptr(),
            model.axis.as_ptr(),
            model.x_axis.as_ptr(),
            model.height_mm,
            points.as_ptr().cast(),
            loops.as_ptr(),
            loops.len(),
            output,
            error,
            capacity,
        )
    })
}

/// Construit un solide de faces planes et de cylindres locaux qui partagent
/// leurs arêtes circulaires, puis valide le STEP réimporté.
pub fn build_mixed_step(model: &MixedModel) -> Result<ValidatedStep, FabError> {
    build_surface_step(model, &[], &[], &[])
}

/// Construit un solide comportant des parois coniques et valide son réimport STEP.
pub fn build_revolved_step(model: &RevolvedModel) -> Result<ValidatedStep, FabError> {
    build_surface_step(&model.mixed, &model.cones, &[], &[])
}

fn has_no_boundary_surfaces(model: &RevolvedModel) -> bool {
    let mixed = &model.mixed;
    mixed.vertices.iter().flatten().all(|v| v.is_finite())
        && mixed.rings.is_empty()
        && mixed.planes.is_empty()
        && mixed.cylinders.is_empty()
        && mixed.arcs.is_empty()
        && mixed.partial_cylinders.is_empty()
        && model.cones.is_empty()
}

fn unit_direction(direction: &[f64; 3]) -> bool {
    direction.iter().all(|v| v.is_finite())
        && (direction.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() <= 1e-8
}

/// Construit un boîtier à congés de rayon constant et valide ses 26 faces analytiques.
pub fn build_rounded_box_step(model: &RoundedBoxModel) -> Result<ValidatedStep, FabError> {
    if model
        .vertices
        .iter()
        .flatten()
        .chain(&model.center)
        .any(|v| !v.is_finite())
        || !model.axes.iter().all(unit_direction)
        || model
            .core_half_extents
            .iter()
            .any(|h| !h.is_finite() || *h <= 0.0)
        || !model.radius_mm.is_finite()
        || model.radius_mm <= 0.0
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le boîtier arrondi contient une coordonnée, une direction ou une dimension invalide.",
        ));
    }
    native_step(|output, error, capacity| unsafe {
        fabstep_build_rounded_box_step(
            model.center.as_ptr(),
            model.axes.as_ptr().cast(),
            model.core_half_extents.as_ptr(),
            model.radius_mm,
            output,
            error,
            capacity,
        )
    })
}

/// Construit les surfaces de révolution reconnues, puis valide le STEP réimporté.
pub fn build_rounded_step(model: &RoundedModel) -> Result<ValidatedStep, FabError> {
    if model.spheres.iter().any(|sphere| sphere.rings.is_none()) {
        let invalid = || {
            FabError::new(
                "cad.invalid_model",
                "Une sphère complète doit former seule un solide extérieur sans anneaux.",
            )
        };
        let [sphere] = model.spheres.as_slice() else {
            return Err(invalid());
        };
        if sphere.inner
            || !sphere.radius_mm.is_finite()
            || sphere.radius_mm <= 0.0
            || sphere.center.iter().any(|v| !v.is_finite())
            || !has_no_boundary_surfaces(&model.revolved)
            || !model.tori.is_empty()
        {
            return Err(invalid());
        }
        // The native seam and poles are parametric, with no source-ring indices.
        return native_step(|output, error, capacity| unsafe {
            fabstep_build_complete_sphere_step(
                sphere.center.as_ptr(),
                sphere.radius_mm,
                output,
                error,
                capacity,
            )
        });
    }
    if model.tori.iter().any(|torus| torus.rings.is_none()) {
        let invalid = || {
            FabError::new(
                "cad.invalid_model",
                "Un tore complet doit former seul un solide extérieur sans anneaux.",
            )
        };
        let [torus] = model.tori.as_slice() else {
            return Err(invalid());
        };
        let axis = torus.axis.filter(unit_direction).ok_or_else(invalid)?;
        if torus.inner
            || torus.meridian_rad != [0.0, TAU]
            || torus.center.iter().any(|v| !v.is_finite())
            || !torus.major_radius_mm.is_finite()
            || !torus.minor_radius_mm.is_finite()
            || torus.minor_radius_mm <= 0.0
            || torus.major_radius_mm <= torus.minor_radius_mm
            || !has_no_boundary_surfaces(&model.revolved)
            || !model.spheres.is_empty()
        {
            return Err(invalid());
        }
        return native_step(|output, error, capacity| unsafe {
            fabstep_build_complete_torus_step(
                torus.center.as_ptr(),
                axis.as_ptr(),
                torus.major_radius_mm,
                torus.minor_radius_mm,
                output,
                error,
                capacity,
            )
        });
    }
    build_surface_step(
        &model.revolved.mixed,
        &model.revolved.cones,
        &model.spheres,
        &model.tori,
    )
}

fn build_surface_step(
    model: &MixedModel,
    cones: &[MixedCone],
    spheres: &[MixedSphere],
    tori: &[MixedTorus],
) -> Result<ValidatedStep, FabError> {
    let invalid = |message| FabError::new("cad.invalid_model", message);
    let legacy = cones.is_empty() && spheres.is_empty() && tori.is_empty();
    if model.planes.is_empty()
        || (model.cylinders.is_empty() && model.partial_cylinders.is_empty() && legacy)
        || model
            .vertices
            .iter()
            .flatten()
            .any(|coord| !coord.is_finite())
    {
        return Err(invalid(
            "Le modèle mixte manque de faces ou contient une coordonnée invalide.",
        ));
    }

    let mut rings = Vec::with_capacity(model.rings.len());
    for ring in &model.rings {
        if ring.center.iter().any(|v| !v.is_finite())
            || !unit_direction(&ring.axis)
            || !ring.radius_mm.is_finite()
            || ring.radius_mm < 0.0
            || (ring.radius_mm == 0.0 && cones.is_empty() && spheres.is_empty())
        {
            return Err(invalid(
                "Un anneau circulaire a un centre, un axe ou un rayon invalide.",
            ));
        }
        rings.push(NativeMixedRing {
            center: ring.center,
            axis: ring.axis,
            radius: ring.radius_mm,
        });
    }
    let mut ring_walls = vec![0u8; rings.len()];
    let mut cylinders = Vec::with_capacity(model.cylinders.len());
    for cylinder in &model.cylinders {
        for ring in cylinder.rings {
            let count = ring_walls
                .get_mut(ring)
                .ok_or_else(|| invalid("Un cylindre référence un anneau absent."))?;
            if rings[ring].radius == 0.0 || *count == 2 || (legacy && *count != 0) {
                return Err(invalid(
                    "Un cylindre utilise une section dégénérée ou un anneau déjà occupé.",
                ));
            }
            *count += 1;
        }
        cylinders.push(NativeMixedCylinder {
            rings: cylinder.rings,
            inner: i32::from(cylinder.inner),
        });
    }
    let mut register_sections = |sections: [usize; 2], allows_pole: bool| {
        if sections[0] == sections[1] {
            return Err(invalid(
                "Une paroi doit référencer deux sections distinctes.",
            ));
        }
        for ring in sections {
            let count = ring_walls
                .get_mut(ring)
                .ok_or_else(|| invalid("Une paroi référence une section absente."))?;
            if *count == 2 || (!allows_pole && rings[ring].radius == 0.0) {
                return Err(invalid(
                    "Un anneau est dégénéré ou appartient à trop de parois.",
                ));
            }
            *count += 1;
        }
        Ok(())
    };
    let mut native_cones = Vec::with_capacity(cones.len());
    for cone in cones {
        register_sections(cone.rings, true)?;
        native_cones.push(NativeMixedCylinder {
            rings: cone.rings,
            inner: i32::from(cone.inner),
        });
    }
    let mut native_spheres = Vec::with_capacity(spheres.len());
    for sphere in spheres {
        let sections = sphere
            .rings
            .ok_or_else(|| invalid("Une calotte doit référencer ses sections source."))?;
        register_sections(sections, true)?;
        if sphere.center.iter().any(|v| !v.is_finite())
            || !sphere.radius_mm.is_finite()
            || sphere.radius_mm <= 0.0
        {
            return Err(invalid(
                "Une sphère présente un centre ou un rayon invalide.",
            ));
        }
        native_spheres.push(NativeSphere {
            rings: sections,
            center: sphere.center,
            radius: sphere.radius_mm,
            inner: i32::from(sphere.inner),
        });
    }
    let mut native_tori = Vec::with_capacity(tori.len());
    for torus in tori {
        let sections = torus
            .rings
            .ok_or_else(|| invalid("Une portion torique doit référencer ses anneaux source."))?;
        register_sections(sections, false)?;
        let [start, end] = torus.meridian_rad;
        if torus.axis.is_some()
            || torus.center.iter().any(|v| !v.is_finite())
            || !torus.major_radius_mm.is_finite()
            || !torus.minor_radius_mm.is_finite()
            || torus.minor_radius_mm <= 0.0
            || torus.major_radius_mm <= torus.minor_radius_mm
            || !start.is_finite()
            || !end.is_finite()
            || !(-TAU..=TAU).contains(&start)
            || end <= start
            || end - start >= TAU
        {
            return Err(invalid(
                "Un tore présente un centre, des rayons ou un méridien invalide.",
            ));
        }
        native_tori.push(NativeTorus {
            rings: sections,
            center: torus.center,
            major_radius: torus.major_radius_mm,
            minor_radius: torus.minor_radius_mm,
            meridian: torus.meridian_rad,
            inner: i32::from(torus.inner),
        });
    }

    let mut arcs = Vec::with_capacity(model.arcs.len());
    for arc in &model.arcs {
        if arc.center.iter().any(|v| !v.is_finite())
            || !unit_direction(&arc.axis)
            || !arc.radius_mm.is_finite()
            || arc.radius_mm <= 0.0
            || !arc.sweep_rad.is_finite()
            || arc.sweep_rad <= 0.0
            || arc.sweep_rad >= TAU - 1e-9
            || arc.vertices[0] == arc.vertices[1]
            || arc.vertices.iter().any(|&id| id >= model.vertices.len())
        {
            return Err(invalid(
                "Un arc présente un rayon, un axe ou des extrémités invalides.",
            ));
        }
        arcs.push(NativeMixedArc {
            center: arc.center,
            axis: arc.axis,
            radius: arc.radius_mm,
            sweep: arc.sweep_rad,
            vertices: arc.vertices,
        });
    }
    let mut arc_cylinders = vec![false; arcs.len()];
    let mut rails = BTreeMap::new();
    let mut partial = Vec::with_capacity(model.partial_cylinders.len());
    for cylinder in &model.partial_cylinders {
        for arc in cylinder.arcs {
            let used = arc_cylinders
                .get_mut(arc)
                .ok_or_else(|| invalid("Un cylindre partiel référence un arc absent."))?;
            if *used {
                return Err(invalid(
                    "Un arc appartient à plusieurs parois cylindriques.",
                ));
            }
            *used = true;
        }
        let [lower, upper] = cylinder.arcs;
        for (from, to) in arcs[lower].vertices.into_iter().zip(arcs[upper].vertices) {
            if from == to || rails.insert(edge_key([from, to]), false).is_some() {
                return Err(invalid(
                    "Une génératrice du cylindre partiel est invalide ou partagée entre cylindres.",
                ));
            }
        }
        partial.push(NativeMixedCylinder {
            rings: cylinder.arcs,
            inner: i32::from(cylinder.inner),
        });
    }

    let mut ring_planes = vec![false; rings.len()];
    let mut arc_planes = vec![false; arcs.len()];
    let mut normals = Vec::with_capacity(model.planes.len());
    let mut plane_loops = vec![0];
    let mut loops = Vec::new();
    let mut segments = Vec::new();
    for plane in &model.planes {
        if !unit_direction(&plane.normal) || plane.loops.is_empty() {
            return Err(invalid(
                "Une face mixte est sans normale unitaire ou sans contour.",
            ));
        }
        normals.push(plane.normal);
        for (index, boundary) in plane.loops.iter().enumerate() {
            let start = segments.len();
            let mut area_adjustment = 0.0;
            match boundary {
                MixedLoop::Polygon { vertices } => {
                    if vertices.len() < 3
                        || vertices.iter().any(|id| *id >= model.vertices.len())
                        || vertices.windows(2).any(|pair| pair[0] == pair[1])
                        || vertices.first() == vertices.last()
                    {
                        return Err(invalid(
                            "Un contour polygonal référence des sommets invalides.",
                        ));
                    }
                    for i in 0..vertices.len() {
                        segments.push(NativeMixedSegment {
                            vertices: [vertices[i], vertices[(i + 1) % vertices.len()]],
                            arc: usize::MAX,
                        });
                    }
                }
                MixedLoop::Circle { ring } => {
                    let used = ring_planes
                        .get_mut(*ring)
                        .ok_or_else(|| invalid("Une face plane référence un anneau absent."))?;
                    if *used || rings[*ring].radius == 0.0 {
                        return Err(invalid(
                            "Une face plane utilise une pointe ou un anneau déjà occupé.",
                        ));
                    }
                    *used = true;
                    loops.push(NativeMixedLoop {
                        start: *ring,
                        count: 0,
                    });
                    continue;
                }
                MixedLoop::Segments { segments: path } => {
                    for segment in path {
                        match *segment {
                            MixedSegment::Line { vertices } => {
                                if vertices[0] == vertices[1]
                                    || vertices.iter().any(|&id| id >= model.vertices.len())
                                {
                                    return Err(invalid(
                                        "Un segment référence des sommets invalides.",
                                    ));
                                }
                                segments.push(NativeMixedSegment {
                                    vertices,
                                    arc: usize::MAX,
                                });
                            }
                            MixedSegment::Arc { arc, reversed } => {
                                let used = arc_planes.get_mut(arc).ok_or_else(|| {
                                    invalid("Une face plane référence un arc absent.")
                                })?;
                                if *used {
                                    return Err(invalid(
                                        "Un arc appartient à plusieurs faces planes.",
                                    ));
                                }
                                *used = true;
                                let curve = &arcs[arc];
                                let normal_dot: f64 = curve
                                    .axis
                                    .into_iter()
                                    .zip(plane.normal)
                                    .map(|(a, b)| a * b)
                                    .sum();
                                let sense = if reversed { -1.0 } else { 1.0 };
                                // Aire double entre la corde et l'arc orienté.
                                area_adjustment += sense
                                    * normal_dot
                                    * curve.radius.powi(2)
                                    * (curve.sweep - curve.sweep.sin());
                                let [a, b] = curve.vertices;
                                let vertices = if reversed { [b, a] } else { [a, b] };
                                segments.push(NativeMixedSegment { vertices, arc });
                            }
                        }
                    }
                }
            }
            let path = &segments[start..];
            if path.len() < 2
                || path
                    .iter()
                    .zip(path.iter().cycle().skip(1))
                    .any(|(a, b)| a.vertices[1] != b.vertices[0])
            {
                return Err(invalid("La chaîne de segments et d'arcs n'est pas fermée."));
            }
            let ids: Vec<_> = path.iter().map(|s| s.vertices[0]).collect();
            let area = signed_area(&ids, &model.vertices, plane.normal) + area_adjustment;
            if !area.is_finite() || area == 0.0 || (area > 0.0) != (index == 0) {
                return Err(invalid("Un contour plan a une aire ou un sens invalide."));
            }
            for segment in path.iter().filter(|segment| segment.arc == usize::MAX) {
                if let Some(used) = rails.get_mut(&edge_key(segment.vertices)) {
                    if *used {
                        return Err(invalid(
                            "Une génératrice appartient à plusieurs contours plans.",
                        ));
                    }
                    *used = true;
                }
            }
            loops.push(NativeMixedLoop {
                start,
                count: path.len(),
            });
        }
        plane_loops.push(loops.len());
    }
    if rings
        .iter()
        .zip(ring_walls)
        .zip(ring_planes)
        .any(|((ring, walls), plane)| {
            let required = if ring.radius == 0.0 { 1 } else { 2 };
            walls == 0 || walls + u8::from(plane) != required
        })
    {
        return Err(invalid(
            "Un anneau doit relier deux faces dont une paroi de révolution, et un pôle une seule paroi.",
        ));
    }
    if !arc_cylinders.into_iter().all(|used| used)
        || !arc_planes.into_iter().all(|used| used)
        || !rails.into_values().all(|used| used)
    {
        return Err(invalid(
            "Chaque arc et chaque génératrice doivent relier une face plane à un cylindre partiel.",
        ));
    }

    // La validation des indices précède l'appel. Aucun pointeur de ces
    // tableaux à plat ne survit à l'appel natif.
    native_step(|output, error, capacity| unsafe {
        fabstep_build_mixed_step(
            model.vertices.as_ptr().cast(),
            model.vertices.len(),
            rings.as_ptr(),
            rings.len(),
            normals.as_ptr().cast(),
            normals.len(),
            plane_loops.as_ptr(),
            loops.as_ptr(),
            segments.as_ptr(),
            cylinders.as_ptr(),
            cylinders.len(),
            arcs.as_ptr(),
            arcs.len(),
            partial.as_ptr(),
            partial.len(),
            native_cones.as_ptr(),
            native_cones.len(),
            native_spheres.as_ptr(),
            native_spheres.len(),
            native_tori.as_ptr(),
            native_tori.len(),
            output,
            error,
            capacity,
        )
    })
}

fn edge_key([a, b]: [usize; 2]) -> (usize, usize) {
    (a.min(b), a.max(b))
}

fn signed_area(ids: &[usize], points: &[[f64; 3]], normal: [f64; 3]) -> f64 {
    let origin = points[ids[0]];
    let delta = |index: usize| std::array::from_fn::<_, 3, _>(|i| points[index][i] - origin[i]);
    ids[1..]
        .windows(2)
        .map(|pair| {
            let [x, y, z] = delta(pair[0]);
            let [u, v, w] = delta(pair[1]);
            (y * w - z * v) * normal[0] + (z * u - x * w) * normal[1] + (x * v - y * u) * normal[2]
        })
        .sum()
}

fn native_step(
    invoke: impl FnOnce(*mut NativeStep, *mut c_char, usize) -> i32,
) -> Result<ValidatedStep, FabError> {
    let mut output = NativeStep::default();
    let mut error = [0; 512];
    let status = invoke(&mut output, error.as_mut_ptr(), error.len());
    if status != 0 {
        return Err(native_error(status, &error));
    }
    if output.data.is_null() || output.size == 0 || output.size > isize::MAX as usize {
        return Err(FabError::new(
            "cad.write_failed",
            "Le noyau a renvoyé un fichier STEP vide ou inaccessible.",
        ));
    }

    // Le pont initialise exactement size octets et garde le tampon en vie
    // jusqu'à la destruction de NativeStep.
    let bytes = unsafe { slice::from_raw_parts(output.data, output.size) }.to_vec();
    Ok(ValidatedStep {
        bytes,
        report: cad_report(&output.metrics),
    })
}

fn cad_report(metrics: &Metrics) -> CadReport {
    // La version est une chaîne C statique, fournie par le noyau compilé.
    let kernel = unsafe { CStr::from_ptr(fabstep_cad_version()) }
        .to_string_lossy()
        .into_owned();
    CadReport {
        solids: metrics.solids,
        faces: metrics.faces,
        planar_faces: metrics.planar_faces,
        cylindrical_faces: metrics.cylindrical_faces,
        conical_faces: metrics.conical_faces,
        spherical_faces: metrics.spherical_faces,
        toroidal_faces: metrics.toroidal_faces,
        volume_mm3: metrics.volume,
        bounds_mm: metrics.bounds,
        kernel,
    }
}

fn native_error(status: i32, error: &[c_char; 512]) -> FabError {
    let code = match status {
        1 => "cad.invalid_model",
        2 => "cad.invalid_solid",
        3 => "cad.write_failed",
        4 => "cad.read_failed",
        5 => "cad.roundtrip_mismatch",
        7 => "cad.tessellation_failed",
        8 => "cad.tessellation_too_complex",
        9 => "cad.bodies_not_separated",
        10 => "cad.step_document_too_large",
        11 => "cad.bodies_separation_budget",
        _ => "cad.kernel_failure",
    };
    // C++ termine toujours le tampon d'erreur par un octet nul.
    let message = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
    FabError::new(code, message)
}

/// Relit un STEP FabSTEP et discrétise sa surface pour un audit échantillonné.
///
/// La déflexion est un réglage du mailleur OCCT, sans garantie de distance
/// globale. Les coordonnées renvoyées sont en millimètres.
pub fn tessellate_step(bytes: &[u8], deflection_mm: f64) -> Result<Mesh, FabError> {
    const MAX_TRIANGLES: usize = 100_000;
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err(FabError::new(
            "cad.read_failed",
            "Le STEP à tesseller doit être non vide et ne pas dépasser 128 Mio.",
        ));
    }
    if !deflection_mm.is_finite() || deflection_mm < 1e-6 {
        return Err(FabError::new(
            "cad.tessellation_failed",
            "La déflexion doit être finie et valoir au moins 0,000001 mm.",
        ));
    }

    let mut mesh = NativeMesh::default();
    let mut error = [0; 512];
    // Le STEP et les tampons restent valides pendant l'appel natif.
    let status = unsafe {
        fabstep_tessellate_step(
            bytes.as_ptr(),
            bytes.len(),
            deflection_mm,
            &mut mesh,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    if mesh.points.is_null()
        || mesh.triangles.is_null()
        || !(1..=3 * MAX_TRIANGLES).contains(&mesh.point_count)
        || !(1..=MAX_TRIANGLES).contains(&mesh.triangle_count)
    {
        return Err(FabError::new(
            "cad.tessellation_failed",
            "Le noyau a renvoyé une tessellation vide ou inaccessible.",
        ));
    }
    // Les tableaux C++ de scalaires et les tableaux Rust ont le même
    // alignement et les mêmes triplets contigus.
    let vertices = unsafe { slice::from_raw_parts(mesh.points, mesh.point_count) }.to_vec();
    let triangles = unsafe { slice::from_raw_parts(mesh.triangles, mesh.triangle_count) }.to_vec();
    Ok(Mesh {
        vertices,
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabstep_core::{
        CircularArc, CircularRing, MixedCylinder, MixedPartialCylinder, MixedPlane, PlanarFace,
    };

    fn face(normal: [f64; 3], boundary: &[usize]) -> PlanarFace {
        PlanarFace {
            loops: vec![boundary.to_vec()],
            normal,
        }
    }

    fn box_model() -> PlanarModel {
        PlanarModel {
            vertices: vec![
                [0., 0., 0.],
                [10., 0., 0.],
                [10., 20., 0.],
                [0., 20., 0.],
                [0., 0., 30.],
                [10., 0., 30.],
                [10., 20., 30.],
                [0., 20., 30.],
            ],
            faces: vec![
                face([0., 0., -1.], &[0, 3, 2, 1]),
                face([0., 0., 1.], &[4, 5, 6, 7]),
                face([0., -1., 0.], &[0, 1, 5, 4]),
                face([1., 0., 0.], &[1, 2, 6, 5]),
                face([0., 1., 0.], &[2, 3, 7, 6]),
                face([-1., 0., 0.], &[3, 0, 4, 7]),
            ],
        }
    }

    fn placed_box(scale: f64, translation: [f64; 3]) -> ValidatedStep {
        let mut model = box_model();
        for p in &mut model.vertices {
            for (v, delta) in p.iter_mut().zip(translation) {
                *v = *v * scale + delta;
            }
        }
        build_step(&model).unwrap()
    }

    #[test]
    fn combines_distinct_named_roots_at_repeated_placements() {
        let first = placed_box(1., [40., 0., 0.]);
        let second = placed_box(1., [0., 0., 0.]);
        let third = placed_box(1., [20., 0., 0.]);
        let result = combine_steps(&[
            StepPart {
                bytes: &first.bytes,
                name: "Étage d'été",
            },
            StepPart {
                bytes: &second.bytes,
                name: "Bloc",
            },
            StepPart {
                bytes: &third.bytes,
                name: "Bloc",
            },
        ])
        .unwrap();
        assert_eq!(result.report.solids, 3);
        assert_eq!(result.report.planar_faces, 18);
        assert_eq!(result.report.bounds_mm, [[0., 0., 0.], [50., 20., 30.]]);
        assert!((result.report.volume_mm3 - 18_000.).abs() < 1e-7);
        assert_eq!(
            result
                .bodies
                .iter()
                .map(|b| b.bounds_mm[0][0])
                .collect::<Vec<_>>(),
            [40., 0., 20.]
        );
        let step = std::str::from_utf8(&result.bytes).unwrap();
        assert!(step.contains("Étage d''été"));
        assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 3);
        assert_eq!(
            combine_steps(&[StepPart {
                bytes: &result.bytes,
                name: "Racines multiples"
            }])
            .unwrap_err()
            .code,
            "cad.read_failed"
        );
    }

    #[test]
    fn rejects_contact_overlap_containment_and_coincidence() {
        let base = placed_box(1., [0., 0., 0.]);
        for (name, step) in [
            ("Contact", placed_box(1., [10., 0., 0.])),
            ("Recouvrement", placed_box(1., [9., 0., 0.])),
            ("Intérieur", placed_box(0.5, [1., 1., 1.])),
            ("Coïncident", placed_box(1., [0., 0., 0.])),
        ] {
            let error = combine_steps(&[
                StepPart {
                    bytes: &base.bytes,
                    name: "Base",
                },
                StepPart {
                    bytes: &step.bytes,
                    name,
                },
            ])
            .unwrap_err();
            assert_eq!(error.code, "cad.bodies_not_separated");
            assert!(error.message.contains("Corps 1 « Base »"));
            assert!(error.message.contains(&format!("corps 2 « {name} »")));
        }
    }

    #[test]
    fn checks_separation_of_original_triangles_without_writing_step() {
        let source = |dx| {
            let mut model = box_model();
            for p in &mut model.vertices {
                p[0] += dx;
            }
            model.faces = model
                .faces
                .into_iter()
                .flat_map(|plane| {
                    let p = &plane.loops[0];
                    [
                        face(plane.normal, &[p[0], p[1], p[2]]),
                        face(plane.normal, &[p[0], p[2], p[3]]),
                    ]
                })
                .collect();
            model
        };
        validate_sources_separated(&[source(0.), source(10.01)]).unwrap();
        let error = validate_sources_separated(&[source(0.), source(9.99)]).unwrap_err();
        assert_eq!(error.code, "cad.bodies_not_separated");
        assert!(error.message.contains("Corps 1 et corps 2"));
        assert_eq!(
            validate_sources_separated(&[box_model()]).unwrap_err().code,
            "cad.invalid_model"
        );
    }

    #[test]
    fn allows_disjoint_cylinders_with_overlapping_bounding_boxes() {
        let first = build_extrusion_step(&cylinder()).unwrap();
        let moved = AnalyticExtrusion {
            origin: [3.2, 3.2, 0.],
            ..cylinder()
        };
        let second = build_extrusion_step(&moved).unwrap();
        let result = combine_steps(&[
            StepPart {
                bytes: &first.bytes,
                name: "Cylindre A",
            },
            StepPart {
                bytes: &second.bytes,
                name: "Cylindre B",
            },
        ])
        .unwrap();
        assert_eq!(result.report.solids, 2);
        assert_eq!(result.report.planar_faces, 4);
        assert_eq!(result.report.cylindrical_faces, 2);
        assert!((result.report.volume_mm3 - 40. * std::f64::consts::PI).abs() < 1e-7);
    }

    #[test]
    fn validates_single_root_and_rejects_document_limits_before_native_call() {
        let base = placed_box(1., [0., 0., 0.]);
        let result = combine_steps(&[StepPart {
            bytes: &base.bytes,
            name: "Seul",
        }])
        .unwrap();
        assert_eq!(result.report.solids, 1);
        assert_eq!(result.bodies[0].faces, 6);
        assert_eq!(combine_steps(&[]).unwrap_err().code, "cad.invalid_model");
        let too_many: Vec<_> = (0..33)
            .map(|_| StepPart {
                bytes: &base.bytes,
                name: "Seul",
            })
            .collect();
        assert_eq!(
            combine_steps(&too_many).unwrap_err().code,
            "cad.invalid_model"
        );
        let invalid = StepPart {
            bytes: &base.bytes,
            name: "Seul\nCorps",
        };
        assert_eq!(
            combine_steps(&[invalid]).unwrap_err().code,
            "cad.invalid_model"
        );
        let buffer = vec![0; 5 * 1024 * 1024];
        let too_big: Vec<_> = (0..32)
            .map(|_| StepPart {
                bytes: &buffer,
                name: "Seul",
            })
            .collect();
        assert_eq!(
            combine_steps(&too_big).unwrap_err().code,
            "cad.step_document_too_large"
        );
    }

    #[test]
    fn exports_and_reimports_a_closed_box() {
        let result = build_step(&box_model()).unwrap();
        assert_eq!(result.report.solids, 1);
        assert_eq!(result.report.faces, 6);
        assert_eq!(result.report.planar_faces, 6);
        assert_eq!(result.report.cylindrical_faces, 0);
        assert!((result.report.volume_mm3 - 6000.).abs() < 1e-6);
        assert_eq!(result.report.bounds_mm, [[0., 0., 0.], [10., 20., 30.]]);
        let step = std::str::from_utf8(&result.bytes).unwrap();
        assert!(step.starts_with("ISO-10303-21;"));
        assert!(step.contains("MANIFOLD_SOLID_BREP"));
        assert!(!step.contains("FACETED_BREP"));
    }

    #[test]
    fn preserves_a_rectangular_through_hole() {
        let mut model = box_model();
        model.vertices.extend_from_slice(&[
            [2., 4., 0.],
            [8., 4., 0.],
            [8., 16., 0.],
            [2., 16., 0.],
            [2., 4., 30.],
            [8., 4., 30.],
            [8., 16., 30.],
            [2., 16., 30.],
        ]);
        model.faces[0].loops.push(vec![8, 9, 10, 11]);
        model.faces[1].loops.push(vec![12, 15, 14, 13]);
        model.faces.extend([
            face([0., 1., 0.], &[9, 8, 12, 13]),
            face([-1., 0., 0.], &[10, 9, 13, 14]),
            face([0., -1., 0.], &[11, 10, 14, 15]),
            face([1., 0., 0.], &[8, 11, 15, 12]),
        ]);
        let result = build_step(&model).unwrap();
        assert_eq!(result.report.solids, 1);
        assert_eq!(result.report.faces, 10);
        assert!((result.report.volume_mm3 - 3840.).abs() < 1e-6);
    }

    #[test]
    fn rejects_an_open_shell_and_an_out_of_range_vertex() {
        let mut model = box_model();
        model.faces.pop();
        assert_eq!(build_step(&model).unwrap_err().code, "cad.invalid_solid");
        model.faces[0].loops[0][0] = usize::MAX;
        assert_eq!(build_step(&model).unwrap_err().code, "cad.invalid_model");
    }

    #[test]
    fn validates_exact_source_faces_without_export() {
        let mut model = box_model();
        let report = validate_source(&model).unwrap();
        assert_eq!(report.faces, 6);
        assert!((report.volume_mm3 - 6000.).abs() < 1e-6);
        model.faces = model
            .faces
            .into_iter()
            .flat_map(|plane| {
                let polygon = &plane.loops[0];
                [
                    face(plane.normal, &[polygon[0], polygon[1], polygon[2]]),
                    face(plane.normal, &[polygon[0], polygon[2], polygon[3]]),
                ]
            })
            .collect();
        let report = validate_source(&model).unwrap();
        assert_eq!(report.faces, 12);
        for (actual, expected) in report
            .bounds_mm
            .into_iter()
            .flatten()
            .zip([0., 0., 0., 10., 20., 30.])
        {
            assert!((actual - expected).abs() < 1e-9);
        }
        model.faces.pop();
        assert_eq!(
            validate_source(&model).unwrap_err().code,
            "cad.invalid_solid"
        );
    }

    fn circle(center: [f64; 2], radius_mm: f64) -> ProfileLoop {
        ProfileLoop::Circle { center, radius_mm }
    }

    fn cylinder() -> AnalyticExtrusion {
        AnalyticExtrusion {
            origin: [0., 0., 0.],
            axis: [0., 0., 1.],
            x_axis: [1., 0., 0.],
            height_mm: 5.,
            outer: circle([0., 0.], 2.),
            holes: vec![],
        }
    }

    fn plate() -> AnalyticExtrusion {
        AnalyticExtrusion {
            outer: ProfileLoop::Polygon {
                points: vec![[-5., -4.], [5., -4.], [5., 4.], [-5., 4.]],
            },
            holes: vec![circle([1., -1.], 1.)],
            ..cylinder()
        }
    }

    fn check_extrusion(
        extrusion: &AnalyticExtrusion,
        planar: usize,
        cylindrical: usize,
        volume: f64,
    ) -> CadReport {
        let result = build_extrusion_step(extrusion).unwrap();
        assert_eq!(result.report.solids, 1);
        assert_eq!(result.report.faces, planar + cylindrical);
        assert_eq!(result.report.planar_faces, planar);
        assert_eq!(result.report.cylindrical_faces, cylindrical);
        assert!((result.report.volume_mm3 - volume).abs() < 1e-7);
        let step = std::str::from_utf8(&result.bytes).unwrap();
        assert!(step.contains("MANIFOLD_SOLID_BREP"));
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), cylindrical);
        result.report
    }

    #[test]
    fn exports_analytic_cylinder_and_periodic_seam() {
        let report = check_extrusion(&cylinder(), 2, 1, 20. * std::f64::consts::PI);
        assert_eq!(report.bounds_mm, [[-2., -2., 0.], [2., 2., 5.]]);
    }

    #[test]
    fn tessellates_step_in_millimeters_and_rejects_invalid_input() {
        let step = build_step(&box_model()).unwrap();
        let mesh = tessellate_step(&step.bytes, 0.001).unwrap();
        assert_eq!(mesh.triangles.len(), 12);
        assert!(mesh.vertices.contains(&[10., 20., 30.]));

        let step = build_extrusion_step(&cylinder()).unwrap();
        let coarse = tessellate_step(&step.bytes, 0.1).unwrap();
        let fine = tessellate_step(&step.bytes, 0.001).unwrap();
        assert!(fine.triangles.len() > coarse.triangles.len());
        assert_eq!(
            tessellate_step(&step.bytes, 1e-9).unwrap_err().code,
            "cad.tessellation_failed"
        );
        assert_eq!(
            tessellate_step(b"invalid STEP", 0.001).unwrap_err().code,
            "cad.read_failed"
        );
        let large = AnalyticExtrusion {
            outer: circle([0., 0.], 10_000.),
            ..cylinder()
        };
        let step = build_extrusion_step(&large).unwrap();
        assert_eq!(
            tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
    }

    #[test]
    fn exports_tube_with_two_cylinders() {
        let mut tube = cylinder();
        tube.holes.push(circle([0., 0.], 1.));
        check_extrusion(&tube, 2, 2, 15. * std::f64::consts::PI);
    }

    #[test]
    fn exports_plate_with_cylindrical_through_hole() {
        check_extrusion(&plate(), 6, 1, 5. * (80. - std::f64::consts::PI));
    }

    #[test]
    fn preserves_tilted_translated_cylinder_and_bounds() {
        let mut tilted = cylinder();
        tilted.origin = [3., -7., 12.];
        tilted.axis = [0.6, 0., 0.8];
        tilted.x_axis = [0., 1., 0.];
        tilted.outer = circle([1., 2.], 2.);
        let report = check_extrusion(&tilted, 2, 1, 20. * std::f64::consts::PI);
        let expected = [[-0.2, -8., 12.], [6., -4., 18.4]];
        for (actual, expected) in report
            .bounds_mm
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert!(
                (actual - expected).abs() < 1e-6,
                "{actual} versus {expected}"
            );
        }
    }

    #[test]
    fn rejects_invalid_extrusion_frame_circle_and_contours() {
        let mut model = cylinder();
        model.x_axis = [1., 0., 1.];
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        model.x_axis = [f64::NAN, 0., 0.];
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        model.x_axis = cylinder().x_axis;
        model.outer = circle([0., 0.], 0.);
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );

        let mut model = plate();
        model.holes = vec![circle([10., 0.], 1.)];
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        model.holes = vec![circle([4.5, 0.], 1.)];
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        model.holes = vec![circle([4., 0.], 1.)];
        assert!(build_extrusion_step(&model).is_err());
        model.holes = vec![circle([-1., 0.], 1.), circle([1., 0.], 1.)];
        assert!(build_extrusion_step(&model).is_err());
        model.holes.clear();
        model.outer = ProfileLoop::Polygon {
            points: vec![[0., 0.], [2., 0.], [2., 0.], [0., 2.]],
        };
        assert_eq!(
            build_extrusion_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
    }

    fn mixed_box() -> MixedModel {
        let base = box_model();
        MixedModel {
            vertices: base.vertices,
            rings: vec![],
            planes: base
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
            cylinders: vec![],
            arcs: vec![],
            partial_cylinders: vec![],
        }
    }

    fn add_wall(
        model: &mut MixedModel,
        centers: [[f64; 3]; 2],
        axis: [f64; 3],
        radius_mm: f64,
        inner: bool,
    ) -> [usize; 2] {
        let rings = [model.rings.len(), model.rings.len() + 1];
        model.rings.extend(centers.map(|center| CircularRing {
            center,
            axis,
            radius_mm,
        }));
        model.cylinders.push(MixedCylinder { rings, inner });
        rings
    }

    fn blind_box() -> MixedModel {
        let mut model = mixed_box();
        let [floor, opening] = add_wall(
            &mut model,
            [[5., 7., 24.], [5., 7., 30.]],
            [0., 0., 1.],
            2.,
            true,
        );
        model.planes[1]
            .loops
            .push(MixedLoop::Circle { ring: opening });
        model.planes.push(MixedPlane {
            normal: [0., 0., 1.],
            loops: vec![MixedLoop::Circle { ring: floor }],
        });
        model
    }

    fn check_mixed(model: &MixedModel, planar: usize, cylindrical: usize, volume: f64) {
        let step = build_mixed_step(model).unwrap();
        assert_eq!(step.report.solids, 1);
        assert_eq!(step.report.planar_faces, planar);
        assert_eq!(step.report.cylindrical_faces, cylindrical);
        assert!((step.report.volume_mm3 - volume).abs() < 1e-7);
        let step = std::str::from_utf8(&step.bytes).unwrap();
        assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), cylindrical);
    }

    fn cone_frustum(lower: f64, upper: f64) -> RevolvedModel {
        let rings = [(lower, 0.), (upper, 8.)];
        let planes = rings
            .into_iter()
            .enumerate()
            .filter(|(_, (radius, _))| *radius > 0.)
            .map(|(ring, _)| MixedPlane {
                normal: [0., 0., if ring == 0 { -1. } else { 1. }],
                loops: vec![MixedLoop::Circle { ring }],
            })
            .collect();
        RevolvedModel {
            mixed: MixedModel {
                vertices: vec![],
                rings: rings
                    .map(|(radius_mm, z)| CircularRing {
                        center: [0., 0., z],
                        axis: [0., 0., 1.],
                        radius_mm,
                    })
                    .to_vec(),
                planes,
                cylinders: vec![],
                arcs: vec![],
                partial_cylinders: vec![],
            },
            cones: vec![MixedCone {
                rings: [0, 1],
                inner: false,
            }],
        }
    }

    fn spherical_cap_model(height: f64) -> RoundedModel {
        let radius = 6.;
        let rim_radius = (height * (2. * radius - height)).sqrt();
        let mut revolved = cone_frustum(rim_radius, 0.);
        revolved.cones.clear();
        revolved.mixed.rings[1].center[2] = height;
        RoundedModel {
            revolved,
            spheres: vec![MixedSphere {
                rings: Some([0, 1]),
                center: [0., 0., height - radius],
                radius_mm: radius,
                inner: false,
            }],
            tori: vec![],
        }
    }

    fn hemisphere_model() -> RoundedModel {
        spherical_cap_model(6.)
    }

    fn complete_sphere_model(center: [f64; 3], radius_mm: f64) -> RoundedModel {
        RoundedModel {
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices: vec![],
                    rings: vec![],
                    planes: vec![],
                    cylinders: vec![],
                    arcs: vec![],
                    partial_cylinders: vec![],
                },
                cones: vec![],
            },
            spheres: vec![MixedSphere {
                rings: None,
                center,
                radius_mm,
                inner: false,
            }],
            tori: vec![],
        }
    }

    #[test]
    fn roundtrips_complete_sphere_and_tessellates_two_unique_poles() {
        let center = [127., -39., 51.];
        let radius = 6.;
        let step = build_rounded_step(&complete_sphere_model(center, radius)).unwrap();
        assert_eq!(step.report.solids, 1);
        assert_eq!(step.report.faces, 1);
        assert_eq!(step.report.spherical_faces, 1);
        assert_eq!(step.report.planar_faces, 0);
        assert!(
            (step.report.volume_mm3 - 4. * std::f64::consts::PI * radius.powi(3) / 3.).abs() < 1e-7
        );
        for (index, sign) in [-1., 1.].into_iter().enumerate() {
            for (coordinate, origin) in step.report.bounds_mm[index].into_iter().zip(center) {
                assert!((coordinate - (origin + sign * radius)).abs() < 1e-7);
            }
        }
        let step_text = std::str::from_utf8(&step.bytes).unwrap();
        assert_eq!(step_text.matches("SPHERICAL_SURFACE(").count(), 1);
        let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
        for z in [center[2] - radius, center[2] + radius] {
            assert_eq!(
                mesh.vertices
                    .iter()
                    .filter(|p| {
                        (p[2] - z).abs() < 1e-7 && (p[0] - center[0]).hypot(p[1] - center[1]) < 1e-7
                    })
                    .count(),
                1
            );
        }
        assert!(mesh
            .triangles
            .iter()
            .all(|[a, b, c]| a != b && a != c && b != c));
        assert_eq!(
            tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
    }

    #[test]
    fn complete_sphere_rejects_nonfinite_data_other_faces_and_inner_orientation() {
        let make = || complete_sphere_model([0.; 3], 6.);
        let mut inner = make();
        inner.spheres[0].inner = true;
        let mut duplicate = make();
        duplicate.spheres.push(make().spheres.remove(0));
        let mut mixed = make();
        mixed.revolved.mixed = mixed_box();
        let mut orphan_rings = make();
        orphan_rings.revolved.mixed.rings = hemisphere_model().revolved.mixed.rings;
        for model in [
            inner,
            duplicate,
            mixed,
            orphan_rings,
            complete_sphere_model([f64::NAN, 0., 0.], 6.),
            complete_sphere_model([0.; 3], f64::INFINITY),
            complete_sphere_model([0.; 3], 0.),
            complete_sphere_model([0.; 3], 1e-9),
        ] {
            assert_eq!(
                build_rounded_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
    }

    #[test]
    fn complete_spheres_keep_their_step_roots_and_reject_point_contact() {
        let first = build_rounded_step(&complete_sphere_model([0.; 3], 6.)).unwrap();
        let second = build_rounded_step(&complete_sphere_model([20., 2., 1.], 6.)).unwrap();
        let touching = build_rounded_step(&complete_sphere_model([12., 0., 0.], 6.)).unwrap();
        let body = |step| StepPart {
            bytes: step,
            name: "Sphère",
        };
        let combined = combine_steps(&[body(&first.bytes), body(&second.bytes)]).unwrap();
        assert_eq!(combined.report.solids, 2);
        assert_eq!(combined.report.spherical_faces, 2);
        assert!(combined.bodies.iter().all(|s| s.spherical_faces == 1));
        assert_eq!(
            combine_steps(&[body(&first.bytes), body(&touching.bytes)])
                .unwrap_err()
                .code,
            "cad.bodies_not_separated"
        );
    }

    fn complete_torus_model(center: [f64; 3], axis: [f64; 3]) -> RoundedModel {
        let mut model = complete_sphere_model(center, 6.);
        model.spheres.clear();
        model.tori.push(MixedTorus {
            rings: None,
            axis: Some(axis),
            center,
            major_radius_mm: 6.,
            minor_radius_mm: 2.,
            meridian_rad: [0., TAU],
            inner: false,
        });
        model
    }

    #[test]
    fn complete_rotated_torus_roundtrips_and_tessellates_a_closed_surface() {
        let center = [12., -3., 4.];
        let axis = [2. / 7., 3. / 7., 6. / 7.];
        let step = build_rounded_step(&complete_torus_model(center, axis)).unwrap();
        assert_eq!(step.report.solids, 1);
        assert_eq!(step.report.faces, 1);
        assert_eq!(step.report.toroidal_faces, 1);
        assert_eq!(step.report.planar_faces, 0);
        assert!((step.report.volume_mm3 - 12. * TAU.powi(2)).abs() < 1e-7);
        for (i, axis_coordinate) in axis.into_iter().enumerate() {
            let extent = 2. + 6. * (1. - axis_coordinate.powi(2)).sqrt();
            assert!((step.report.bounds_mm[0][i] - center[i] + extent).abs() < 3e-7);
            assert!((step.report.bounds_mm[1][i] - center[i] - extent).abs() < 3e-7);
        }
        let text = std::str::from_utf8(&step.bytes).unwrap();
        assert_eq!(text.matches("TOROIDAL_SURFACE(").count(), 1);
        let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
        let mut edges = BTreeMap::new();
        for &[a, b, c] in &mesh.triangles {
            for (from, to) in [(a, b), (b, c), (c, a)] {
                assert_ne!(from, to);
                let uses = edges.entry((from.min(to), from.max(to))).or_insert([0, 0]);
                uses[usize::from(from < to)] += 1;
            }
        }
        assert!(edges.values().all(|uses| *uses == [1, 1]));
        assert_eq!(mesh.vertices.len() + mesh.triangles.len(), edges.len());
        let distinct = mesh
            .vertices
            .iter()
            .map(|p| p.map(|v| (v * 1e8).round() as i64))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(distinct.len(), mesh.vertices.len());
        assert_eq!(
            tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
    }

    #[test]
    fn complete_torus_rejects_bad_parameters_and_other_surfaces() {
        let make = || complete_torus_model([0.; 3], [0., 0., 1.]);
        let mut bad_axis = make();
        bad_axis.tori[0].axis = Some([0., 0., 2.]);
        let mut nonfinite_axis = make();
        nonfinite_axis.tori[0].axis = Some([f64::NAN, 0., 1.]);
        let mut no_axis = make();
        no_axis.tori[0].axis = None;
        let mut bad_center = make();
        bad_center.tori[0].center[0] = f64::INFINITY;
        let mut bad_radius = make();
        bad_radius.tori[0].major_radius_mm = f64::INFINITY;
        let mut horn = make();
        horn.tori[0].major_radius_mm = 2.;
        let mut near_horn = make();
        near_horn.tori[0].major_radius_mm = 2. + 1e-9;
        let mut spindle = make();
        spindle.tori[0].major_radius_mm = 1.;
        let mut inner = make();
        inner.tori[0].inner = true;
        let mut partial = make();
        partial.tori[0].meridian_rad = [0., TAU - 1e-6];
        let mut mixed = make();
        mixed.revolved.mixed = mixed_box();
        let mut extra = make();
        extra.spheres = complete_sphere_model([0.; 3], 6.).spheres;
        let mut duplicate = make();
        duplicate.tori.extend(make().tori);
        let mut patch_with_axis = toroidal_fillet_model();
        patch_with_axis.tori[0].axis = Some([0., 0., 1.]);
        for model in [
            bad_axis,
            nonfinite_axis,
            no_axis,
            bad_center,
            bad_radius,
            horn,
            near_horn,
            spindle,
            inner,
            partial,
            mixed,
            extra,
            duplicate,
            patch_with_axis,
        ] {
            assert_eq!(
                build_rounded_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
    }

    #[test]
    fn complete_tori_preserve_step_roots_and_reject_contact() {
        let axis = [0., 0., 1.];
        let first = build_rounded_step(&complete_torus_model([0.; 3], axis)).unwrap();
        let second = build_rounded_step(&complete_torus_model([20., 1., 0.], axis)).unwrap();
        let touching = build_rounded_step(&complete_torus_model([16., 0., 0.], axis)).unwrap();
        let body = |bytes| StepPart {
            bytes,
            name: "Tore",
        };
        let combined = combine_steps(&[body(&first.bytes), body(&second.bytes)]).unwrap();
        assert_eq!(combined.report.solids, 2);
        assert_eq!(combined.report.toroidal_faces, 2);
        assert!(combined
            .bodies
            .iter()
            .all(|report| report.toroidal_faces == 1));
        assert_eq!(
            combine_steps(&[body(&first.bytes), body(&touching.bytes)])
                .unwrap_err()
                .code,
            "cad.bodies_not_separated"
        );
    }

    fn rounded_box_model(center: [f64; 3]) -> RoundedBoxModel {
        RoundedBoxModel {
            vertices: vec![],
            center,
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            core_half_extents: [8., 13., 18.],
            radius_mm: 2.,
        }
    }

    #[test]
    fn rounded_box_reimports_its_26_analytic_faces_and_tessellates() {
        let mut model = rounded_box_model([60., 0., 1.]);
        let q = 13_f64.sqrt();
        model.axes = [
            [-3. / q, 2. / q, 0.],
            [-12. / (7. * q), -18. / (7. * q), q / 7.],
            [2. / 7., 3. / 7., 6. / 7.],
        ];
        let step = build_rounded_box_step(&model).unwrap();
        let report = &step.report;
        assert_eq!(report.solids, 1);
        assert_eq!(report.faces, 26);
        assert_eq!(report.planar_faces, 6);
        assert_eq!(report.cylindrical_faces, 12);
        assert_eq!(report.spherical_faces, 8);
        assert_eq!(report.toroidal_faces + report.conical_faces, 0);
        let pi = std::f64::consts::PI;
        let nominal = 16. * 26. * 36.
            + 4. * (16. * 26. + 16. * 36. + 26. * 36.)
            + 4. * pi * (16. + 26. + 36.)
            + 32. * pi / 3.;
        assert!((report.volume_mm3 - nominal).abs() < 1e-7);
        for i in 0..3 {
            let extent = model.radius_mm
                + model
                    .axes
                    .iter()
                    .zip(model.core_half_extents)
                    .map(|(axis, half)| axis[i].abs() * half)
                    .sum::<f64>();
            assert!((report.bounds_mm[0][i] - model.center[i] + extent).abs() < 3e-7);
            assert!((report.bounds_mm[1][i] - model.center[i] - extent).abs() < 3e-7);
        }
        let text = std::str::from_utf8(&step.bytes).unwrap();
        assert_eq!(text.matches("CYLINDRICAL_SURFACE(").count(), 12);
        assert_eq!(text.matches("SPHERICAL_SURFACE(").count(), 8);
        let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
        assert!(!mesh.triangles.is_empty());
        assert!(mesh
            .triangles
            .iter()
            .all(|[a, b, c]| a != b && b != c && a != c));
        assert_eq!(
            tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
    }

    #[test]
    fn rounded_box_requires_a_resolved_direct_frame() {
        let make = || rounded_box_model([0.; 3]);
        let mut left = make();
        left.axes[2] = [0., 0., -1.];
        let mut repeated_axis = make();
        repeated_axis.axes[2] = repeated_axis.axes[1];
        let mut nonunit = make();
        nonunit.axes[0][0] = 1.5;
        let mut unresolved_core = make();
        unresolved_core.core_half_extents[1] = 1e-9;
        let mut unresolved_radius = make();
        unresolved_radius.radius_mm = 1e-9;
        let mut nonfinite = make();
        nonfinite.center[0] = f64::INFINITY;
        for model in [
            left,
            repeated_axis,
            nonunit,
            unresolved_core,
            unresolved_radius,
            nonfinite,
        ] {
            assert_eq!(
                build_rounded_box_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
    }

    #[test]
    fn rounded_boxes_revalidate_separate_roots_and_reject_contact() {
        let make = |x| build_rounded_box_step(&rounded_box_model([x, 0., 0.])).unwrap();
        let a = make(0.);
        let separated = make(30.);
        let touching = make(20.);
        let body = |bytes| StepPart {
            bytes,
            name: "Boîtier arrondi",
        };
        let result = combine_steps(&[body(&a.bytes), body(&separated.bytes)]).unwrap();
        assert_eq!(result.report.solids, 2);
        assert_eq!(result.report.planar_faces, 12);
        assert_eq!(result.report.cylindrical_faces, 24);
        assert_eq!(result.report.spherical_faces, 16);
        assert_eq!(
            combine_steps(&[body(&a.bytes), body(&touching.bytes)])
                .unwrap_err()
                .code,
            "cad.bodies_not_separated"
        );
    }

    fn toroidal_fillet_model() -> RoundedModel {
        let mut revolved = cone_frustum(8., 6.);
        revolved.cones.clear();
        revolved.mixed.rings.push(CircularRing {
            center: [0., 0., 6.],
            axis: [0., 0., 1.],
            radius_mm: 8.,
        });
        revolved.mixed.cylinders.push(MixedCylinder {
            rings: [0, 2],
            inner: false,
        });
        RoundedModel {
            revolved,
            spheres: vec![],
            tori: vec![MixedTorus {
                rings: Some([2, 1]),
                axis: None,
                center: [0., 0., 6.],
                major_radius_mm: 6.,
                minor_radius_mm: 2.,
                meridian_rad: [0., std::f64::consts::FRAC_PI_2],
                inner: false,
            }],
        }
    }

    #[test]
    fn reimports_a_hemisphere_and_tessellates_the_pole_with_bounded_work() {
        let step = build_rounded_step(&hemisphere_model()).unwrap();
        assert_eq!(step.report.faces, 2);
        assert_eq!(step.report.spherical_faces, 1);
        assert!((step.report.volume_mm3 - 144. * std::f64::consts::PI).abs() < 1e-7);
        assert!(std::str::from_utf8(&step.bytes)
            .unwrap()
            .contains("SPHERICAL_SURFACE("));
        let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
        assert!(mesh
            .vertices
            .iter()
            .any(|p| (p[2] - 6.).abs() < 1e-7 && p[0].hypot(p[1]) < 1e-7));
        assert!(mesh
            .triangles
            .iter()
            .all(|[a, b, c]| a != b && a != c && b != c));
        assert_eq!(
            tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
    }

    #[test]
    fn roundtrips_spherical_caps_on_both_sides_of_the_equator() {
        for (height, width) in [(2., 2. * 20_f64.sqrt()), (10., 12.)] {
            let step = build_rounded_step(&spherical_cap_model(height)).unwrap();
            let volume = std::f64::consts::PI * height * height * (6. - height / 3.);
            assert_eq!(step.report.solids, 1);
            assert_eq!(step.report.planar_faces, 1);
            assert_eq!(step.report.spherical_faces, 1);
            assert!((step.report.volume_mm3 - volume).abs() < 1e-7);
            let [lo, hi] = step.report.bounds_mm;
            assert!(lo[2].abs() < 1e-7);
            assert!((hi[2] - height).abs() < 1e-7);
            for axis in 0..2 {
                assert!((hi[axis] - lo[axis] - width).abs() < 1e-7);
            }
            let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
            let poles = mesh
                .vertices
                .iter()
                .filter(|p| (p[2] - height).abs() < 1e-7 && p[0].hypot(p[1]) < 1e-7)
                .count();
            assert_eq!(poles, 1);
        }
    }

    #[test]
    fn preserves_a_torus_shared_with_a_cylinder_and_a_plane_across_step_roots() {
        let step = build_rounded_step(&toroidal_fillet_model()).unwrap();
        assert_eq!(step.report.faces, 4);
        assert_eq!(step.report.cylindrical_faces, 1);
        assert_eq!(step.report.toroidal_faces, 1);
        let pi = std::f64::consts::PI;
        let volume = pi * (384. + 72. + 12. * pi + 16. / 3.);
        assert!((step.report.volume_mm3 - volume).abs() < 1e-7);
        assert!(std::str::from_utf8(&step.bytes)
            .unwrap()
            .contains("TOROIDAL_SURFACE("));
        let target = tessellate_step(&step.bytes, 0.005).unwrap();
        assert!(!target.triangles.is_empty());
        let mut model = hemisphere_model();
        model.spheres[0].center[0] = 30.;
        for ring in &mut model.revolved.mixed.rings {
            ring.center[0] += 30.;
        }
        let sphere = build_rounded_step(&model).unwrap();
        let combined = combine_steps(&[
            StepPart {
                bytes: &step.bytes,
                name: "Congé",
            },
            StepPart {
                bytes: &sphere.bytes,
                name: "Sphère",
            },
        ])
        .unwrap();
        assert_eq!(combined.report.solids, 2);
        assert_eq!(combined.report.spherical_faces, 1);
        assert_eq!(combined.report.toroidal_faces, 1);
        assert_eq!(combined.bodies[0].toroidal_faces, 1);
        assert_eq!(combined.bodies[1].spherical_faces, 1);
    }

    #[test]
    fn rejects_inconsistent_spherical_and_toroidal_boundaries() {
        let mut off_axis = hemisphere_model();
        off_axis.spheres[0].center[0] = 0.01;
        let mut short_pole = hemisphere_model();
        short_pole.revolved.mixed.rings[1].center[2] -= 0.01;
        let mut wrong_axis = hemisphere_model();
        wrong_axis.revolved.mixed.rings[1].axis = [0., 1., 0.];
        let mut wrong_meridian = toroidal_fillet_model();
        wrong_meridian.tori[0].meridian_rad[1] -= 0.1;
        let mut spindle = toroidal_fillet_model();
        spindle.tori[0].minor_radius_mm = 9.;
        for model in [off_axis, short_pole, wrong_axis, wrong_meridian, spindle] {
            assert_eq!(
                build_rounded_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
    }

    #[test]
    fn roundtrips_truncated_and_pointed_cones_and_tessellates_the_apex() {
        for (lower, upper) in [(7., 3.), (7., 0.), (0., 3.)] {
            let model = cone_frustum(lower, upper);
            let step = build_revolved_step(&model).unwrap();
            let volume =
                std::f64::consts::PI * 8. * (lower * lower + lower * upper + upper * upper) / 3.;
            assert_eq!(step.report.solids, 1);
            assert_eq!(step.report.conical_faces, 1);
            assert_eq!(step.report.planar_faces, model.mixed.planes.len());
            assert!((step.report.volume_mm3 - volume).abs() < 1e-7);
            assert_eq!(
                std::str::from_utf8(&step.bytes)
                    .unwrap()
                    .matches("CONICAL_SURFACE(")
                    .count(),
                1
            );
            assert!(!tessellate_step(&step.bytes, 0.005)
                .unwrap()
                .triangles
                .is_empty());
        }
    }

    fn pointed_hole() -> RevolvedModel {
        let mut mixed = mixed_box();
        let [bottom, opening] = add_wall(
            &mut mixed,
            [[5., 7., 24.], [5., 7., 30.]],
            [0., 0., 1.],
            2.,
            true,
        );
        let tip = mixed.rings.len();
        mixed.rings.push(CircularRing {
            center: [5., 7., 22.],
            axis: [0., 0., 1.],
            radius_mm: 0.,
        });
        mixed.planes[1]
            .loops
            .push(MixedLoop::Circle { ring: opening });
        RevolvedModel {
            mixed,
            cones: vec![MixedCone {
                rings: [tip, bottom],
                inner: true,
            }],
        }
    }

    #[test]
    fn shares_cone_cylinder_rim_and_preserves_cone_counts_across_step_roots() {
        let drilled = build_revolved_step(&pointed_hole()).unwrap();
        let cone_volume = 8. * std::f64::consts::PI / 3.;
        assert!(
            (drilled.report.volume_mm3 - (6000. - 24. * std::f64::consts::PI - cone_volume)).abs()
                < 1e-7
        );
        assert_eq!(drilled.report.faces, 8);
        let cone = build_revolved_step(&cone_frustum(7., 3.)).unwrap();
        let separated = placed_box(1., [30., 0., 0.]);
        let document = combine_steps(&[
            StepPart {
                bytes: &drilled.bytes,
                name: "Foret",
            },
            StepPart {
                bytes: &separated.bytes,
                name: "Cube",
            },
        ])
        .unwrap();
        assert_eq!(document.report.solids, 2);
        assert_eq!(document.report.conical_faces, 1);
        assert_eq!(document.bodies[0].conical_faces, 1);
        assert_eq!(document.bodies[1].conical_faces, 0);
        assert_eq!(
            combine_steps(&[
                StepPart {
                    bytes: &drilled.bytes,
                    name: "Foret"
                },
                StepPart {
                    bytes: &cone.bytes,
                    name: "En contact"
                },
            ])
            .unwrap_err()
            .code,
            "cad.bodies_not_separated"
        );
    }

    #[test]
    fn rejects_a_capped_apex_an_open_rim_and_noncoaxial_sections() {
        let mut capped = cone_frustum(7., 0.);
        capped.mixed.planes.push(MixedPlane {
            normal: [0., 0., 1.],
            loops: vec![MixedLoop::Circle { ring: 1 }],
        });
        let mut open = cone_frustum(7., 3.);
        open.mixed.planes.pop();
        let mut off_axis = cone_frustum(7., 0.);
        off_axis.mixed.rings[1].center[0] = 0.1;
        for model in [capped, open, off_axis] {
            assert_eq!(
                build_revolved_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
    }

    #[test]
    fn exports_a_blind_hole_with_a_shared_circular_floor() {
        check_mixed(&blind_box(), 7, 1, 6000. - 24. * std::f64::consts::PI);
    }

    #[test]
    fn exports_a_cylindrical_boss_joined_to_a_plane() {
        let mut model = mixed_box();
        let [base, cap] = add_wall(
            &mut model,
            [[5., 7., 30.], [5., 7., 35.]],
            [0., 0., 1.],
            2.,
            false,
        );
        model.planes[1].loops.push(MixedLoop::Circle { ring: base });
        model.planes.push(MixedPlane {
            normal: [0., 0., 1.],
            loops: vec![MixedLoop::Circle { ring: cap }],
        });
        check_mixed(&model, 7, 1, 6000. + 20. * std::f64::consts::PI);
    }

    #[test]
    fn exports_independent_holes_on_distinct_axes() {
        let mut model = blind_box();
        let [left, right] = add_wall(
            &mut model,
            [[0., 13., 15.], [10., 13., 15.]],
            [1., 0., 0.],
            1.,
            true,
        );
        model.planes[5].loops.push(MixedLoop::Circle { ring: left });
        model.planes[3]
            .loops
            .push(MixedLoop::Circle { ring: right });
        check_mixed(&model, 7, 2, 6000. - 34. * std::f64::consts::PI);
    }

    #[test]
    fn rejects_intersecting_holes_without_shared_intersection_edges() {
        let mut model = blind_box();
        let [left, right] = add_wall(
            &mut model,
            [[0., 7., 26.], [10., 7., 26.]],
            [1., 0., 0.],
            1.,
            true,
        );
        model.planes[5].loops.push(MixedLoop::Circle { ring: left });
        model.planes[3]
            .loops
            .push(MixedLoop::Circle { ring: right });
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_solid"
        );
    }

    #[test]
    fn rejects_invalid_mixed_ring_incidence_and_geometry() {
        let mut model = blind_box();
        model.cylinders[0].rings = [0, 0];
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut model = blind_box();
        model.rings[1].axis = [0., 1., 0.];
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut model = blind_box();
        model.rings[0].center[2] += 0.01;
        model.rings[1].center[2] += 0.01;
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut model = blind_box();
        model.planes[1].normal = [0., 0., -1.];
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
    }

    fn notched_box() -> MixedModel {
        let profile = [
            [-5., 0.],
            [-2., 0.],
            [2., 0.],
            [5., 0.],
            [5., 8.],
            [-5., 8.],
        ];
        let vertices = [0., 10.]
            .into_iter()
            .flat_map(|z| profile.map(|[x, y]| [x, y, z]))
            .collect();
        let line = |a, b| MixedSegment::Line { vertices: [a, b] };
        let arc = |arc, reversed| MixedSegment::Arc { arc, reversed };
        let mut planes = vec![
            MixedPlane {
                normal: [0., 0., -1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        line(0, 5),
                        line(5, 4),
                        line(4, 3),
                        line(3, 2),
                        arc(0, false),
                        line(1, 0),
                    ],
                }],
            },
            MixedPlane {
                normal: [0., 0., 1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        line(6, 7),
                        arc(1, true),
                        line(8, 9),
                        line(9, 10),
                        line(10, 11),
                        line(11, 6),
                    ],
                }],
            },
        ];
        for (a, b, normal) in [
            (0, 1, [0., -1., 0.]),
            (2, 3, [0., -1., 0.]),
            (3, 4, [1., 0., 0.]),
            (4, 5, [0., 1., 0.]),
            (5, 0, [-1., 0., 0.]),
        ] {
            planes.push(MixedPlane {
                normal,
                loops: vec![MixedLoop::Polygon {
                    vertices: vec![a, b, b + 6, a + 6],
                }],
            });
        }
        MixedModel {
            vertices,
            rings: vec![],
            planes,
            cylinders: vec![],
            arcs: [(0., 0), (10., 6)]
                .into_iter()
                .map(|(z, offset)| CircularArc {
                    center: [0., 0., z],
                    axis: [0., 0., 1.],
                    radius_mm: 2.,
                    sweep_rad: std::f64::consts::PI,
                    vertices: [2 + offset, 1 + offset],
                })
                .collect(),
            partial_cylinders: vec![MixedPartialCylinder {
                arcs: [0, 1],
                inner: true,
            }],
        }
    }

    #[test]
    fn exports_partial_cylinder_sharing_arcs_and_generators_with_planes() {
        let model = notched_box();
        check_mixed(&model, 7, 1, 800. - 20. * std::f64::consts::PI);
        let step = build_mixed_step(&model).unwrap();
        assert!(!tessellate_step(&step.bytes, 0.005)
            .unwrap()
            .triangles
            .is_empty());
    }

    #[test]
    fn exports_partial_and_complete_cylinders_in_one_solid() {
        let mut model = notched_box();
        let [floor, opening] = add_wall(
            &mut model,
            [[0., 5., 6.], [0., 5., 10.]],
            [0., 0., 1.],
            1.,
            true,
        );
        model.planes[1]
            .loops
            .push(MixedLoop::Circle { ring: opening });
        model.planes.push(MixedPlane {
            normal: [0., 0., 1.],
            loops: vec![MixedLoop::Circle { ring: floor }],
        });
        check_mixed(&model, 8, 2, 800. - 24. * std::f64::consts::PI);
    }

    #[test]
    fn rejects_partial_cylinder_with_invalid_chains_or_unshared_generator() {
        let mut model = notched_box();
        let MixedLoop::Segments { segments } = &mut model.planes[0].loops[0] else {
            unreachable!()
        };
        segments.pop();
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );

        let mut model = notched_box();
        model.vertices.push([-2., 0., 5.]);
        model.planes[2].loops[0] = MixedLoop::Polygon {
            vertices: vec![0, 1, 12, 7, 6],
        };
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );

        let mut model = notched_box();
        model.arcs[1].sweep_rad -= 0.01;
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut model = notched_box();
        model.vertices[8][0] += 0.01;
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
    }
}
