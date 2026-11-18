//! Adaptateur natif Open CASCADE. Le STEP est écrit puis relu en mémoire.

use fabstep_core::{
    AnalyticExtrusion, FabError, Mesh, MixedCone, MixedLoop, MixedModel, MixedPartialCone,
    MixedSegment, MixedSphere, MixedSphericalOctant, MixedTorus, PlanarFaceProfile, PlanarModel,
    PlanarProfileLoop, ProfileFrame, ProfileLoop, ProfileSegment, RevolvedModel, RoundedBoxModel,
    RoundedModel,
};

const MAX_RESULT_FACES: usize = 100_000;
const MAX_SOURCE_TRIANGLES: usize = fabstep_core::SOURCE_MAX_TRIANGLES;
const MAX_SOURCE_VERTICES: usize = fabstep_core::SOURCE_MAX_VERTICES;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::ffi::{c_char, CStr, CString};
use std::slice;

mod section;
pub use section::{planar_face_frame, section_profile};

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
    #[serde(default)]
    pub revolution_faces: usize,
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
    revolution_faces: usize,
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
struct NativeResultFace {
    solid_id: usize,
    family: usize,
    triangle_start: usize,
    triangle_count: usize,
    neighbor_start: usize,
    neighbor_count: usize,
    source_face_token: usize,
}

#[repr(C)]
#[derive(Default)]
struct NativeResultFaces {
    mesh: NativeMesh,
    faces: *mut NativeResultFace,
    face_count: usize,
    neighbors: *mut usize,
    neighbor_count: usize,
}

impl Drop for NativeResultFaces {
    fn drop(&mut self) {
        // NativeMesh separately releases its owned buffers.
        unsafe { fabstep_free_result_faces(self.faces, self.neighbors) };
    }
}

#[repr(C)]
struct NativeProfileSegment {
    kind: usize,
    start: [f64; 2],
    end: [f64; 2],
    center: [f64; 2],
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    major_axis: [f64; 2],
    ratio: f64,
}

#[repr(C)]
struct NativePlanarProfileLoop {
    start: usize,
    count: usize,
    is_outer: usize,
}

#[repr(C)]
#[derive(Default)]
struct NativePlanarProfile {
    solid_id: usize,
    origin: [f64; 3],
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    normal: [f64; 3],
    loops: *mut NativePlanarProfileLoop,
    loop_count: usize,
    segments: *mut NativeProfileSegment,
    segment_count: usize,
}

impl Drop for NativePlanarProfile {
    fn drop(&mut self) {
        // Only this ABI allocator owns these scalar buffers.
        unsafe { fabstep_free_planar_profile(self.loops, self.segments) };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultFaceFamily {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Torus,
    Other,
    Revolution,
}

/// IDs are zero-based and scoped to this read of the final STEP, not the source.
#[derive(Debug, Serialize)]
pub struct ResultFace {
    pub id: usize,
    pub solid_id: usize,
    pub family: ResultFaceFamily,
    pub triangle_start: usize,
    pub triangle_count: usize,
    pub adjacent_face_ids: Vec<usize>,
    /// Exact transfer-table label, not source identity by itself. Only a
    /// trusted sidecar from the same conversion may give it source meaning.
    /// The entire inspection is None if any STEP face label is ambiguous.
    pub source_face_token: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ResultFaces {
    pub mesh: Mesh,
    pub faces: Vec<ResultFace>,
    pub deflection_mm: f64,
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
struct NativeEllipse {
    center: [f64; 3],
    axis: [f64; 3],
    major_direction: [f64; 3],
    major: f64,
    minor: f64,
    start: f64,
    sweep: f64,
    vertices: [usize; 2],
}

#[repr(C)]
struct NativeSmoothRevolution {
    origin: [f64; 3],
    axis: [f64; 3],
    height: f64,
    radii: [f64; 4],
    knot: f64,
    next_radii: [f64; 3],
}

#[repr(C)]
struct NativeCrossDrill {
    small_offset: [f64; 2],
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    half: [f64; 3],
    radii: [f64; 2],
}

#[repr(C)]
struct NativeLinearBlend {
    apex: [f64; 3],
    axis: [f64; 3],
    angle: f64,
    arcs: [usize; 2],
    rails: [[usize; 2]; 2],
}

#[repr(C)]
struct NativeOctant {
    center: [f64; 3],
    radius: f64,
    directions: [[f64; 3]; 3],
    arcs: [usize; 3],
    reversed: [i32; 3],
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
    fn fabstep_build_step_with_provenance(
        points: *const f64,
        point_count: usize,
        normals: *const f64,
        face_count: usize,
        face_loops: *const usize,
        loop_vertices: *const usize,
        vertex_ids: *const usize,
        tokens: *const usize,
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
    fn fabstep_build_torus_sector_step(
        domain: *const torus_sector::NativeTorusSector,
        tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_build_smooth_revolution_step(
        domain: *const NativeSmoothRevolution,
        caps: *const usize,
        wall: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_build_cross_drill_step(
        domain: *const NativeCrossDrill,
        planes: *const usize,
        walls: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_build_linear_blend_step(
        points: *const f64,
        point_count: usize,
        normals: *const f64,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        ellipses: *const NativeEllipse,
        blend: *const NativeLinearBlend,
        plane_tokens: *const usize,
        blend_token: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_trihedral_step(
        points: *const f64,
        point_count: usize,
        normals: *const f64,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        arcs: *const NativeMixedArc,
        partial: *const NativeMixedCylinder,
        octant: *const NativeOctant,
        plane_tokens: *const usize,
        partial_tokens: *const usize,
        octant_tokens: *const usize,
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
        partial_cones: *const NativeMixedCylinder,
        partial_cone_count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_rounded_step_with_provenance(
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
        plane_tokens: *const usize,
        cylinder_tokens: *const usize,
        partial_tokens: *const usize,
        cone_tokens: *const usize,
        sphere_tokens: *const usize,
        torus_tokens: *const usize,
        partial_cones: *const NativeMixedCylinder,
        partial_cone_count: usize,
        partial_cone_tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_mixed_step_with_provenance(
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
        plane_tokens: *const usize,
        cylinder_tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_build_revolved_step_with_provenance(
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
        cones: *const NativeMixedCylinder,
        cone_count: usize,
        plane_tokens: *const usize,
        cylinder_tokens: *const usize,
        cone_tokens: *const usize,
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
    fn fabstep_build_complete_sphere_step_with_provenance(
        center: *const f64,
        radius: f64,
        token: usize,
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
    fn fabstep_build_complete_torus_step_with_provenance(
        center: *const f64,
        axis: *const f64,
        major_radius: f64,
        minor_radius: f64,
        token: usize,
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
    fn fabstep_combine_steps_with_provenance(
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
    fn fabstep_result_faces(
        bytes: *const u8,
        size: usize,
        deflection: f64,
        output: *mut NativeResultFaces,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_free_result_faces(faces: *mut NativeResultFace, neighbors: *mut usize);
    fn fabstep_planar_face_profile(
        bytes: *const u8,
        size: usize,
        face_id: usize,
        output: *mut NativePlanarProfile,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn fabstep_free_planar_profile(
        loops: *mut NativePlanarProfileLoop,
        segments: *mut NativeProfileSegment,
    );
    fn fabstep_cad_version() -> *const c_char;
}

/// Regroupe des solides STEP séparés, un par racine, dans l'ordre fourni.
///
/// Chaque entrée doit contenir un seul solide FabSTEP en coordonnées monde.
/// Les corps en contact, intersectés, imbriqués ou coïncidents sont refusés.
/// Le volume du document est la somme des volumes mesurés au réimport.
/// Un nom est refusé si le lecteur STEP ne retrouve pas son libellé.
pub fn combine_steps(parts: &[StepPart<'_>]) -> Result<CombinedStep, FabError> {
    combine_steps_impl(parts, false)
}

/// Combines valid geometry while preserving exact transfer labels when every
/// face of every root has a distinct label. Metadata loss never changes the
/// geometry's export eligibility; inspection then exposes no source tokens.
pub fn combine_steps_with_provenance(parts: &[StepPart<'_>]) -> Result<CombinedStep, FabError> {
    combine_steps_impl(parts, true)
}

fn combine_steps_impl(parts: &[StepPart<'_>], provenance: bool) -> Result<CombinedStep, FabError> {
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
        let combine = if provenance {
            fabstep_combine_steps_with_provenance
        } else {
            fabstep_combine_steps
        };
        combine(
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
        if triangles > MAX_SOURCE_TRIANGLES || vertices > MAX_SOURCE_VERTICES {
            return Err(FabError::new(
                "cad.source_document_too_large",
                format!("Le document source dépasse {MAX_SOURCE_TRIANGLES} triangles ou {MAX_SOURCE_VERTICES} sommets."),
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
        .map(|model| prepare_planar_for(model, PlanarRole::Source))
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

/// Labels each input planar face at construction and tracks its exact identity
/// through sewing and STEP transfer. Labels must be globally unique within a
/// document, in 1..=100_000. They have no source meaning without a trusted
/// caller-owned sidecar. Any incomplete native history omits all labels.
pub fn build_step_with_provenance(
    model: &PlanarModel,
    face_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    if face_tokens.len() != model.faces.len() || face_tokens.len() > 100_000 {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants de provenance des faces doivent être exhaustifs et bornés.",
        ));
    }
    let mut unique = face_tokens.to_vec();
    unique.sort_unstable();
    if unique.iter().any(|&token| !(1..=100_000).contains(&token))
        || unique.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants de provenance des faces doivent être exhaustifs, uniques et bornés.",
        ));
    }
    let input = prepare_planar(model)?;
    native_step(|output, error, capacity| unsafe {
        fabstep_build_step_with_provenance(
            model.vertices.as_ptr().cast(),
            model.vertices.len(),
            input.normals.as_ptr().cast(),
            input.normals.len(),
            input.face_loops.as_ptr(),
            input.loop_vertices.as_ptr(),
            input.vertex_ids.as_ptr(),
            face_tokens.as_ptr(),
            output,
            error,
            capacity,
        )
    })
}

/// Construit et contrôle la B-Rep planaire source sans passer par un fichier STEP.
/// Utiliser des contours triangulaires pour préserver exactement un STL soudé.
pub fn validate_source(model: &PlanarModel) -> Result<CadReport, FabError> {
    let input = prepare_planar_for(model, PlanarRole::Source)?;
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

#[derive(Clone, Copy)]
enum PlanarRole {
    Source,
    Result,
}

fn prepare_planar(model: &PlanarModel) -> Result<PlanarInput, FabError> {
    prepare_planar_for(model, PlanarRole::Result)
}

fn prepare_planar_for(model: &PlanarModel, role: PlanarRole) -> Result<PlanarInput, FabError> {
    let max_faces = match role {
        PlanarRole::Source => MAX_SOURCE_TRIANGLES,
        PlanarRole::Result => MAX_RESULT_FACES,
    };
    if model.faces.len() > max_faces || model.vertices.len() > MAX_SOURCE_VERTICES {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le modèle dépasse le plafond de faces ou de coordonnées de son profil de traitement.",
        ));
    }
    // Preserve the historical <=100k planar-source API (including quads).
    // The newly admitted large profile only validates exact source triangles.
    if matches!(role, PlanarRole::Source)
        && model.faces.len() > MAX_RESULT_FACES
        && model
            .faces
            .iter()
            .any(|face| face.loops.len() != 1 || face.loops[0].len() != 3)
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le profil source étendu exige un unique contour triangulaire par face.",
        ));
    }
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
            if vertex_ids.len().saturating_add(boundary.len()) > 6 * MAX_SOURCE_TRIANGLES
                || loop_vertices.len() > 2 * MAX_SOURCE_TRIANGLES
            {
                return Err(FabError::new(
                    "cad.invalid_model",
                    "Les contours dépassent le budget du modèle planaire.",
                ));
            }
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
    let result_faces =
        std::iter::once(&model.outer)
            .chain(&model.holes)
            .try_fold(2usize, |count, boundary| {
                let walls = match boundary {
                    ProfileLoop::Circle { .. } => 1,
                    ProfileLoop::Polygon { points } => points.len(),
                };
                count.checked_add(walls)
            });
    if result_faces.is_none_or(|count| count > MAX_RESULT_FACES) {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le résultat extrudé dépasse 100 000 faces.",
        ));
    }
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

/// Tracks exact construction identities for planes and complete cylinders.
/// Each token slice indexes the corresponding `MixedModel` array; tokens must
/// be unique across both slices, in 1..=100_000, and require a trusted sidecar
/// to have source meaning. Incomplete native history omits all labels. Models
/// with partial cylinders retain their validated geometry without provenance.
pub fn build_mixed_step_with_provenance(
    model: &MixedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    if plane_tokens.len() != model.planes.len()
        || cylinder_tokens.len() != model.cylinders.len()
        || plane_tokens.len().saturating_add(cylinder_tokens.len()) > 100_000
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants des plans et cylindres doivent être exhaustifs et bornés.",
        ));
    }
    let mut unique: Vec<_> = plane_tokens
        .iter()
        .chain(cylinder_tokens)
        .copied()
        .collect();
    unique.sort_unstable();
    if unique.iter().any(|token| !(1..=100_000).contains(token))
        || unique.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants des plans et cylindres doivent être uniques et bornés.",
        ));
    }
    if !model.partial_cylinders.is_empty() {
        return build_mixed_step(model);
    }
    build_surface_step_with_tokens(
        model,
        &[],
        &[],
        &[],
        &[],
        &[],
        Some(SurfaceTokens {
            planes: plane_tokens,
            cylinders: cylinder_tokens,
            ..SurfaceTokens::default()
        }),
    )
}

/// Construit un solide comportant des parois coniques et valide son réimport STEP.
pub fn build_revolved_step(model: &RevolvedModel) -> Result<ValidatedStep, FabError> {
    build_surface_step(&model.mixed, &model.cones, &[], &[])
}

/// Tracks exact construction identities for planes, complete cylinders and cones.
/// Each slice indexes its corresponding model array; all tokens must be unique
/// in 1..=100_000 and have source meaning only with a trusted same-result sidecar.
/// Incomplete native history omits all labels without weakening solid validation.
/// Partial-cylinder models remain validated but carry no provenance.
pub fn build_revolved_step_with_provenance(
    model: &RevolvedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    cone_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    let count = plane_tokens
        .len()
        .saturating_add(cylinder_tokens.len())
        .saturating_add(cone_tokens.len());
    if plane_tokens.len() != model.mixed.planes.len()
        || cylinder_tokens.len() != model.mixed.cylinders.len()
        || cone_tokens.len() != model.cones.len()
        || count > 100_000
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants des plans, cylindres et cônes doivent être exhaustifs et bornés.",
        ));
    }
    let mut unique: Vec<_> = plane_tokens
        .iter()
        .chain(cylinder_tokens)
        .chain(cone_tokens)
        .copied()
        .collect();
    unique.sort_unstable();
    if unique.iter().any(|token| !(1..=100_000).contains(token))
        || unique.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Les identifiants des plans, cylindres et cônes doivent être uniques et bornés.",
        ));
    }
    if !model.mixed.partial_cylinders.is_empty() {
        return build_revolved_step(model);
    }
    build_surface_step_with_tokens(
        &model.mixed,
        &model.cones,
        &[],
        &[],
        &[],
        &[],
        Some(SurfaceTokens {
            planes: plane_tokens,
            cylinders: cylinder_tokens,
            cones: cone_tokens,
            ..SurfaceTokens::default()
        }),
    )
}

fn has_no_boundary_surfaces(model: &RevolvedModel) -> bool {
    let mixed = &model.mixed;
    mixed.vertices.iter().flatten().all(|v| v.is_finite())
        && mixed.rings.is_empty()
        && mixed.planes.is_empty()
        && mixed.cylinders.is_empty()
        && mixed.arcs.is_empty()
        && mixed.elliptic_arcs.is_empty()
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
    build_rounded_step_impl(model, None)
}

#[derive(Clone, Copy, Default)]
struct SurfaceTokens<'a> {
    planes: &'a [usize],
    cylinders: &'a [usize],
    partial: &'a [usize],
    cones: &'a [usize],
    spheres: &'a [usize],
    tori: &'a [usize],
    partial_cones: &'a [usize],
    octants: &'a [usize],
    linear_blends: &'a [usize],
    cross_drill: &'a [usize],
    smooth_revolution: &'a [usize],
    torus_sectors: &'a [usize],
}

/// Tracks construction identities for all complete rounded surface families.
/// Models containing partial cylinders require `build_rounded_step_with_all_provenance`.
pub fn build_rounded_step_with_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_all_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        &[],
        cone_tokens,
        sphere_tokens,
        torus_tokens,
    )
}

/// Each slice indexes the corresponding model array. All tokens are exhaustive,
/// globally unique and in 1..=100_000. Their source meaning requires a trusted
/// sidecar belonging to this exact result; failed native history omits all labels.
pub fn build_rounded_step_with_all_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_chain_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        partial_cylinder_tokens,
        cone_tokens,
        sphere_tokens,
        torus_tokens,
        &[],
    )
}

/// Tracks all rounded families, including partial conical transitions.
#[allow(clippy::too_many_arguments)]
pub fn build_rounded_step_with_chain_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
    partial_cone_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_octant_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        partial_cylinder_tokens,
        cone_tokens,
        sphere_tokens,
        torus_tokens,
        partial_cone_tokens,
        &[],
    )
}

/// The last slice labels the bounded spherical octants, in model order.
#[allow(clippy::too_many_arguments)]
pub fn build_rounded_step_with_octant_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
    partial_cone_tokens: &[usize],
    octant_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_linear_blend_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        partial_cylinder_tokens,
        cone_tokens,
        sphere_tokens,
        torus_tokens,
        partial_cone_tokens,
        octant_tokens,
        &[],
    )
}

/// Includes the inclined cone of a bounded linear-radius blend as the ninth group.
#[allow(clippy::too_many_arguments)]
pub fn build_rounded_step_with_linear_blend_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
    partial_cone_tokens: &[usize],
    octant_tokens: &[usize],
    linear_blend_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_cross_drill_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        partial_cylinder_tokens,
        cone_tokens,
        sphere_tokens,
        torus_tokens,
        partial_cone_tokens,
        octant_tokens,
        linear_blend_tokens,
        &[],
    )
}

/// The tenth group labels the lower small wall, upper small wall, and large wall of two intersecting bores.
#[allow(clippy::too_many_arguments)]
pub fn build_rounded_step_with_cross_drill_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
    partial_cone_tokens: &[usize],
    octant_tokens: &[usize],
    linear_blend_tokens: &[usize],
    cross_drill_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_smooth_revolution_provenance(
        model,
        plane_tokens,
        cylinder_tokens,
        partial_cylinder_tokens,
        cone_tokens,
        sphere_tokens,
        torus_tokens,
        partial_cone_tokens,
        octant_tokens,
        linear_blend_tokens,
        cross_drill_tokens,
        &[],
    )
}

/// The eleventh group labels the single certified cubic surface of revolution.
#[allow(clippy::too_many_arguments)]
pub fn build_rounded_step_with_smooth_revolution_provenance(
    model: &RoundedModel,
    plane_tokens: &[usize],
    cylinder_tokens: &[usize],
    partial_cylinder_tokens: &[usize],
    cone_tokens: &[usize],
    sphere_tokens: &[usize],
    torus_tokens: &[usize],
    partial_cone_tokens: &[usize],
    octant_tokens: &[usize],
    linear_blend_tokens: &[usize],
    cross_drill_tokens: &[usize],
    smooth_revolution_tokens: &[usize],
) -> Result<ValidatedStep, FabError> {
    build_rounded_step_with_token_groups(
        model,
        [
            plane_tokens,
            cylinder_tokens,
            partial_cylinder_tokens,
            cone_tokens,
            sphere_tokens,
            torus_tokens,
            partial_cone_tokens,
            octant_tokens,
            linear_blend_tokens,
            cross_drill_tokens,
            smooth_revolution_tokens,
            &[],
        ],
    )
}

/// Groups follow the source owner family order; the last group labels torus sectors.
pub fn build_rounded_step_with_token_groups(
    model: &RoundedModel,
    arrays: [&[usize]; 12],
) -> Result<ValidatedStep, FabError> {
    let [plane_tokens, cylinder_tokens, partial_cylinder_tokens, cone_tokens, sphere_tokens, torus_tokens, partial_cone_tokens, octant_tokens, linear_blend_tokens, cross_drill_tokens, smooth_revolution_tokens, sector_tokens] =
        arrays;
    let counts = [
        model.revolved.mixed.planes.len(),
        model.revolved.mixed.cylinders.len(),
        model.partial_cylinder_count(),
        model.revolved.cones.len(),
        model.spheres.len(),
        model.tori.len(),
        model.partial_cones.len(),
        model.octants.len(),
        usize::from(model.linear_blend.is_some()),
        if model.cross_drill.is_some() { 3 } else { 0 },
        usize::from(model.smooth_revolution.is_some()),
        model.torus_sectors.len(),
    ];
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les identifiants de surfaces doivent être exhaustifs, uniques et bornés.",
        )
    };
    let mut total = 0usize;
    for (tokens, count) in arrays.iter().zip(counts) {
        total = total.saturating_add(tokens.len());
        if tokens.len() != count || total > 100_000 {
            return Err(invalid());
        }
    }
    let mut unique: Vec<_> = arrays.into_iter().flatten().copied().collect();
    unique.sort_unstable();
    if unique.iter().any(|token| !(1..=100_000).contains(token))
        || unique.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(invalid());
    }
    build_rounded_step_impl(
        model,
        Some(SurfaceTokens {
            planes: plane_tokens,
            cylinders: cylinder_tokens,
            partial: partial_cylinder_tokens,
            cones: cone_tokens,
            spheres: sphere_tokens,
            tori: torus_tokens,
            partial_cones: partial_cone_tokens,
            octants: octant_tokens,
            linear_blends: linear_blend_tokens,
            cross_drill: cross_drill_tokens,
            smooth_revolution: smooth_revolution_tokens,
            torus_sectors: sector_tokens,
        }),
    )
}

mod adjacent_pocket;
mod oblique_pocket;
mod torus_sector;
mod unequal_pocket;

fn build_rounded_step_impl(
    model: &RoundedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    if let Some(pocket) = &model.unequal_pocket {
        return unequal_pocket::build(model, pocket, tokens);
    }
    if model
        .revolved
        .mixed
        .partial_cylinders
        .iter()
        .any(|wall| wall.elliptic_end)
        && (!model.torus_sectors.is_empty()
            || model.smooth_revolution.is_some()
            || model.cross_drill.is_some()
            || model.linear_blend.is_some())
    {
        return Err(oblique_pocket::invalid());
    }
    if !model.torus_sectors.is_empty() {
        return torus_sector::build(model, tokens);
    }
    if let Some(f) = &model.smooth_revolution {
        return build_smooth_revolution(model, f, tokens);
    }
    if let Some(f) = &model.cross_drill {
        return build_cross_drill(model, f, tokens);
    }
    if let Some(blend) = &model.linear_blend {
        return build_linear_blend(model, blend, tokens);
    }
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
            || !model.octants.is_empty()
            || !model.partial_cones.is_empty()
            || !has_no_boundary_surfaces(&model.revolved)
            || !model.tori.is_empty()
        {
            return Err(invalid());
        }
        // The native seam and poles are parametric, with no source-ring indices.
        return native_step(|output, error, capacity| unsafe {
            if let Some(tokens) = tokens {
                return fabstep_build_complete_sphere_step_with_provenance(
                    sphere.center.as_ptr(),
                    sphere.radius_mm,
                    tokens.spheres[0],
                    output,
                    error,
                    capacity,
                );
            }
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
            || !model.octants.is_empty()
            || !model.partial_cones.is_empty()
            || !has_no_boundary_surfaces(&model.revolved)
            || !model.spheres.is_empty()
        {
            return Err(invalid());
        }
        return native_step(|output, error, capacity| unsafe {
            if let Some(tokens) = tokens {
                return fabstep_build_complete_torus_step_with_provenance(
                    torus.center.as_ptr(),
                    axis.as_ptr(),
                    torus.major_radius_mm,
                    torus.minor_radius_mm,
                    tokens.tori[0],
                    output,
                    error,
                    capacity,
                );
            }
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
    build_surface_step_with_tokens(
        &model.revolved.mixed,
        &model.revolved.cones,
        &model.spheres,
        &model.tori,
        &model.partial_cones,
        &model.octants,
        tokens,
    )
}

/// Validate the additive scalar domain; source coverage remains a core proof.
fn build_smooth_revolution(
    model: &RoundedModel,
    f: &fabstep_core::SmoothRevolution,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les supports et les limites de la révolution cubique sont incohérents.",
        )
    };
    let m = &model.revolved.mixed;
    let finite = |v: &[f64; 3]| v.iter().all(|x| x.is_finite());
    if m.planes.len() != 2
        || m.rings.len() != 2
        || m.vertices.len() > MAX_SOURCE_VERTICES
        || !m.vertices.iter().all(finite)
        || !m.cylinders.is_empty()
        || !m.partial_cylinders.is_empty()
        || !m.arcs.is_empty()
        || !m.elliptic_arcs.is_empty()
        || !model.revolved.cones.is_empty()
        || !model.spheres.is_empty()
        || !model.tori.is_empty()
        || !model.partial_cones.is_empty()
        || !model.octants.is_empty()
        || model.linear_blend.is_some()
        || model.cross_drill.is_some()
        || !finite(&f.origin)
        || !unit_direction(&f.axis)
        || !f.height_mm.is_finite()
        || f.height_mm <= 0.
        || f.radii_mm.iter().any(|r| !r.is_finite() || *r <= 0.)
        || f.rings[0] >= 2
        || f.rings[1] >= 2
        || f.rings[0] == f.rings[1]
        || !f.witness_tolerance_mm.is_finite()
        || f.witness_tolerance_mm < 0.
    {
        return Err(invalid());
    }
    let (knot, next_radii) = match &f.second_span {
        None => (0., [0.; 3]),
        Some(span) => {
            if !(0.125..=0.875).contains(&span.start_fraction)
                || span.next_radii_mm.iter().any(|r| !r.is_finite() || *r <= 0.)
            {
                return Err(invalid());
            }
            (span.start_fraction, span.next_radii_mm)
        }
    };
    let radii = || f.radii_mm.iter().copied().chain(
        f.second_span.iter().flat_map(|span| span.next_radii_mm.iter().copied()),
    );
    let scale = f.origin.into_iter().chain(radii()).chain([f.height_mm])
        .map(f64::abs).fold(1., f64::max);
    let roundoff = 512. * f64::EPSILON * scale;
    let shortest_span = if f.second_span.is_some() { knot.min(1. - knot) } else { 1. };
    let min_feature = radii().fold(f.height_mm * shortest_span, f64::min);
    if roundoff > 5e-8 || f.witness_tolerance_mm >= min_feature / 2. {
        return Err(invalid());
    }
    // Corrected controls must already define a C1 spline. The absolute cap
    // also bounds any displacement when the native split is evaluated.
    let support_tolerance = (roundoff * 2.).clamp(1e-9, 5e-8);
    if f.second_span.is_some()
        && ((1. - knot) * f.radii_mm[2] + knot * next_radii[0] - f.radii_mm[3]).abs() > support_tolerance
    {
        return Err(invalid());
    }
    let feature_extent = radii().fold(f.height_mm, f64::max).max(1.);
    let axis_norm = f.axis.iter().map(|v| v * v).sum::<f64>().sqrt();
    if (axis_norm - 1.).abs() * feature_extent > 5e-8 {
        return Err(invalid());
    }
    let direction_tolerance = 5e-8 / feature_extent;
    for end in 0..2 {
        let ring = &m.rings[f.rings[end]];
        let plane = &m.planes[end];
        let expected_center: [f64; 3] =
            std::array::from_fn(|i| f.origin[i] + end as f64 * f.height_mm * f.axis[i]);
        let expected_radius = if end == 0 { f.radii_mm[0] } else { f.second_span.as_ref().map_or(f.radii_mm[3], |s| s.next_radii_mm[2]) };
        let expected_sign = if end == 0 { -1. } else { 1. };
        if !finite(&ring.center)
            || !unit_direction(&ring.axis)
            || !ring.radius_mm.is_finite()
            || (ring.radius_mm - expected_radius).abs() > support_tolerance
            || (0..3).any(|i| {
                (ring.center[i] - expected_center[i]).abs() > support_tolerance
                    || (ring.axis[i] - f.axis[i]).abs() > direction_tolerance
                    || (plane.normal[i] - expected_sign * f.axis[i]).abs() > direction_tolerance
            })
            || !unit_direction(&plane.normal)
            || !matches!(plane.loops.as_slice(), [MixedLoop::Circle {ring}] if *ring == f.rings[end])
        {
            return Err(invalid());
        }
    }
    let domain = NativeSmoothRevolution {
        origin: f.origin,
        axis: f.axis,
        height: f.height_mm,
        radii: f.radii_mm,
        knot,
        next_radii,
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_smooth_revolution_step(
            &domain,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.smooth_revolution.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}

/// Validate the six stock planes, four mouths and two witnessed branches
/// before constructing the three coupled cylindrical domains.
fn build_cross_drill(
    model: &RoundedModel,
    f: &fabstep_core::CrossDrill,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les limites et les supports des perçages sécants sont incohérents.",
        )
    };
    let m = &model.revolved.mixed;
    let [r, big] = f.radii;
    let [hx, hy, hz] = f.half_lengths;
    let [dx, dy] = f.small_offset;
    if m.planes.len() != 6
        || m.rings.len() != 4
        || m.vertices.len() > MAX_SOURCE_VERTICES
        || !m.cylinders.is_empty()
        || !m.partial_cylinders.is_empty()
        || !m.arcs.is_empty()
        || !m.elliptic_arcs.is_empty()
        || !model.revolved.cones.is_empty()
        || !model.partial_cones.is_empty()
        || !model.octants.is_empty()
        || model.linear_blend.is_some()
        || !model.spheres.is_empty()
        || !model.tori.is_empty()
        || !f.axes.iter().all(unit_direction)
        || !f
            .center
            .iter()
            .chain(&f.half_lengths)
            .chain(&f.small_offset)
            .chain(&f.radii)
            .all(|x| x.is_finite())
        || !(r > 0. && hx > r && hy > big && hz > big)
    {
        return Err(invalid());
    }
    let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let norm = |a: [f64; 3]| dot(a, a).sqrt();
    let sub = |a: [f64; 3], b: [f64; 3]| std::array::from_fn(|j| a[j] - b[j]);
    let [x, y, z] = f.axes;
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    if dot(x, z).abs() > 1e-10
        || norm(sub(cross(z, x), y)) > 1e-10
        || !m.vertices.iter().flatten().all(|p| p.is_finite())
    {
        return Err(invalid());
    }
    let local = |id: usize| {
        m.vertices
            .get(id)
            .map(|&p| f.axes.map(|u| dot(sub(p, f.center), u)))
            .ok_or_else(invalid)
    };
    let numerical = 128.
        * f64::EPSILON
        * f.center
            .into_iter()
            .map(f64::abs)
            .chain(f.half_lengths)
            .fold(1., f64::max);
    if numerical > 5e-8 || !f.witness_tolerance_mm.is_finite() || f.witness_tolerance_mm < 0. {
        return Err(invalid());
    }
    let witness = f.witness_tolerance_mm + numerical;
    // Both radius errors are at most w; moving both axes charges 2w to their separation.
    // The centered path retains its historical margin.
    let extent = dy.abs() + r;
    let margin = if dy == 0. { 2.5 } else { 5.5 };
    if big - 1.5 * extent <= margin * witness.max(1e-7)
        || dy.abs() > r
        || witness >= r.min(hx - dx.abs() - r).min(hy - big).min(hz - big) / 2.
    {
        return Err(invalid());
    }
    let mut used = [false; 4];
    let mut corners = [None; 8];
    for (face, p) in m.planes.iter().enumerate() {
        let axis = face % 3;
        let sign = if face < 3 { -1. } else { 1. };
        let normal = f.axes[axis].map(|v| sign * v);
        if norm(sub(p.normal, normal)) > 1e-10 || !unit_direction(&p.normal) {
            return Err(invalid());
        }
        let Some(MixedLoop::Polygon { vertices }) = p.loops.first() else {
            return Err(invalid());
        };
        if vertices.len() != 4
            || vertices.iter().any(|&i| i >= m.vertices.len())
            || signed_area(vertices, &m.vertices, p.normal) <= 0.
        {
            return Err(invalid());
        }
        let mut mask = 0u8;
        for &id in vertices {
            let at = local(id)?;
            let bits: usize = (0..3).map(|k| usize::from(at[k] > 0.) << k).sum();
            let goal = std::array::from_fn(|k| {
                if bits & (1 << k) != 0 {
                    f.half_lengths[k]
                } else {
                    -f.half_lengths[k]
                }
            });
            if at[axis] * sign <= 0.
                || norm(sub(at, goal)) > witness
                || mask & (1 << bits) != 0
                || corners[bits].is_some_and(|previous| previous != id)
            {
                return Err(invalid());
            }
            corners[bits] = Some(id);
            mask |= 1 << bits;
        }
        let rim_slot = match face {
            2 => Some(0),
            5 => Some(1),
            0 => Some(2),
            3 => Some(3),
            _ => None,
        };
        let Some(slot) = rim_slot else {
            if p.loops.len() != 1 {
                return Err(invalid());
            }
            continue;
        };
        let ring = f.rims[slot];
        let Some(curve) = m.rings.get(ring) else {
            return Err(invalid());
        };
        if !matches!(p.loops.as_slice(),[_,MixedLoop::Circle{ring:i}] if *i==ring) || used[ring] {
            return Err(invalid());
        }
        used[ring] = true;
        let center = std::array::from_fn(|k| {
            let offset = if slot < 2 { dx * x[k] + dy * y[k] } else { 0. };
            f.center[k] + normal[k] * f.half_lengths[axis] + offset
        });
        if !curve.radius_mm.is_finite()
            || !curve.center.iter().all(|x| x.is_finite())
            || !unit_direction(&curve.axis)
            || norm(sub(curve.center, center)) > numerical
            || norm(sub(curve.axis, f.axes[axis])) > 1e-10
            || (curve.radius_mm - if slot < 2 { r } else { big }).abs() > numerical
        {
            return Err(invalid());
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for (branch, cycle) in f.branches.iter().enumerate() {
        if !(12..=4096).contains(&cycle.len()) {
            return Err(invalid());
        }
        let mut phases = Vec::with_capacity(cycle.len());
        for &id in cycle {
            if !ids.insert(id) {
                return Err(invalid());
            }
            let [x, y, z] = local(id)?;
            let t = (y - dy).atan2(x - dx);
            let ideal_y = dy + r * t.sin();
            let stop = (big * big - ideal_y * ideal_y).sqrt() * if branch == 0 { -1. } else { 1. };
            if !stop.is_finite()
                || (branch == 0) != (z < 0.)
                || norm([x - dx - r * t.cos(), y - ideal_y, z - stop]) > witness
            {
                return Err(invalid());
            }
            phases.push(t);
        }
        let mut wrap = 0;
        for (&a, &b) in phases.iter().zip(phases.iter().cycle().skip(1)) {
            let delta = (b - a).rem_euclid(TAU);
            if !(delta > 0. && delta <= std::f64::consts::FRAC_PI_4 + 1e-10) {
                return Err(invalid());
            }
            wrap += usize::from(b < a);
        }
        if wrap != 1 {
            return Err(invalid());
        }
    }
    let domain = NativeCrossDrill {
        small_offset: f.small_offset,
        center: f.center,
        axes: f.axes,
        half: f.half_lengths,
        radii: f.radii,
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_cross_drill_step(
            &domain,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.cross_drill.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}

fn build_linear_blend(
    model: &RoundedModel,
    blend: &fabstep_core::MixedLinearBlend,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let mixed = &model.revolved.mixed;
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les supports et les limites du raccord linéaire sont incohérents.",
        )
    };
    let finite = |p: &[f64; 3]| p.iter().all(|v| v.is_finite());
    let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(v, w)| v * w).sum::<f64>();
    if mixed.planes.len() != 6
        || mixed.elliptic_arcs.len() != 2
        || mixed.vertices.len() > MAX_SOURCE_VERTICES
        || !mixed.vertices.iter().all(finite)
        || !mixed.rings.is_empty()
        || !mixed.arcs.is_empty()
        || !mixed.cylinders.is_empty()
        || !mixed.partial_cylinders.is_empty()
        || !model.revolved.cones.is_empty()
        || !model.octants.is_empty()
        || !model.partial_cones.is_empty()
        || !model.spheres.is_empty()
        || !model.tori.is_empty()
        || !finite(&blend.apex)
        || !unit_direction(&blend.axis)
        || !(0.0 < blend.semi_angle_rad && blend.semi_angle_rad < std::f64::consts::FRAC_PI_2)
        || !matches!(blend.arcs, [0, 1] | [1, 0])
    {
        return Err(invalid());
    }
    let mut ellipses = Vec::with_capacity(2);
    for arc in &mixed.elliptic_arcs {
        if !(finite(&arc.center)
            && unit_direction(&arc.axis)
            && unit_direction(&arc.major_direction)
            && dot(arc.axis, arc.major_direction).abs() <= 1e-8
            && arc.major_mm.is_finite()
            && arc.major_mm > arc.minor_mm
            && arc.minor_mm > 0.)
            || !arc.start_rad.is_finite()
            || !(0. < arc.sweep_rad && arc.sweep_rad < std::f64::consts::PI)
            || arc.vertices[0] == arc.vertices[1]
            || arc.vertices.iter().any(|&v| v >= mixed.vertices.len())
        {
            return Err(invalid());
        }
        ellipses.push(NativeEllipse {
            center: arc.center,
            axis: arc.axis,
            major_direction: arc.major_direction,
            major: arc.major_mm,
            minor: arc.minor_mm,
            start: arc.start_rad,
            sweep: arc.sweep_rad,
            vertices: arc.vertices,
        });
    }
    let [bottom, top] = blend.arcs.map(|i| &ellipses[i]);
    let [lo_y, lo_x] = bottom.vertices;
    let [hi_y, hi_x] = top.vertices;
    let expected_rails = [[lo_x, hi_x], [hi_y, lo_y]];
    if bottom.vertices.iter().any(|v| top.vertices.contains(v))
        || !(blend.rails == expected_rails || blend.rails == [expected_rails[1], expected_rails[0]])
    {
        return Err(invalid());
    }
    let native = NativeLinearBlend {
        apex: blend.apex,
        axis: blend.axis,
        angle: blend.semi_angle_rad,
        arcs: blend.arcs,
        rails: blend.rails,
    };
    let mut used = [false; 2];
    let mut lines = BTreeMap::<(usize, usize), (usize, usize)>::new();
    for &[from, to] in &blend.rails {
        let entry = lines.entry(edge_key([from, to])).or_default();
        if from < to {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }
    let mut segments = Vec::new();
    let mut normals = Vec::with_capacity(6);
    let mut loops = Vec::with_capacity(6);
    for plane in &mixed.planes {
        let [path] = plane.loops.as_slice() else {
            return Err(invalid());
        };
        if !unit_direction(&plane.normal) {
            return Err(invalid());
        }
        let count = match path {
            MixedLoop::Polygon { vertices } => vertices.len(),
            MixedLoop::Segments { segments } => segments.len(),
            MixedLoop::Circle { .. } => return Err(invalid()),
        };
        let start = segments.len();
        if count < 3 || count > MAX_SOURCE_VERTICES - start {
            return Err(invalid());
        }
        let mut area_adjustment = 0.;
        let mut contains_ellipse = false;
        for i in 0..count {
            let segment = match path {
                MixedLoop::Polygon { vertices } => MixedSegment::Line {
                    vertices: [vertices[i], vertices[(i + 1) % count]],
                },
                MixedLoop::Segments { segments } => segments[i].clone(),
                MixedLoop::Circle { .. } => unreachable!(),
            };
            match segment {
                MixedSegment::Line { vertices } => {
                    if vertices[0] == vertices[1]
                        || vertices.iter().any(|&id| id >= mixed.vertices.len())
                    {
                        return Err(invalid());
                    }
                    let entry = lines.entry(edge_key(vertices)).or_default();
                    let uses = if vertices[0] < vertices[1] {
                        &mut entry.0
                    } else {
                        &mut entry.1
                    };
                    if *uses != 0 {
                        return Err(invalid());
                    }
                    *uses = 1;
                    segments.push(NativeMixedSegment {
                        vertices,
                        arc: usize::MAX,
                    });
                }
                MixedSegment::EllipticArc { arc, reversed } => {
                    let curve = ellipses.get(arc).ok_or_else(invalid)?;
                    if used[arc] || contains_ellipse || reversed != (arc == blend.arcs[0]) {
                        return Err(invalid());
                    }
                    used[arc] = true;
                    contains_ellipse = true;
                    let [a, b] = curve.vertices;
                    let sense = if reversed { -1. } else { 1. };
                    area_adjustment += sense
                        * dot(plane.normal, curve.axis)
                        * curve.major
                        * curve.minor
                        * (curve.sweep - curve.sweep.sin());
                    segments.push(NativeMixedSegment {
                        vertices: if reversed { [b, a] } else { [a, b] },
                        arc,
                    });
                }
                MixedSegment::Arc { .. } => return Err(invalid()),
            }
        }
        let path = &segments[start..];
        let ids: Vec<_> = path.iter().map(|s| s.vertices[0]).collect();
        let area = signed_area(&ids, &mixed.vertices, plane.normal) + area_adjustment;
        if !area.is_finite()
            || area <= 0.
            || path
                .iter()
                .zip(path.iter().cycle().skip(1))
                .any(|(a, b)| a.vertices[1] != b.vertices[0])
        {
            return Err(invalid());
        }
        normals.push(plane.normal);
        loops.push(NativeMixedLoop { start, count });
    }
    if used != [true, true] || lines.values().any(|&uses| uses != (1, 1)) {
        return Err(invalid());
    }
    let plane_loops = [0usize, 1, 2, 3, 4, 5, 6];
    native_step(|output, error, capacity| unsafe {
        fabstep_build_linear_blend_step(
            mixed.vertices.as_ptr().cast(),
            mixed.vertices.len(),
            normals.as_ptr().cast(),
            plane_loops.as_ptr(),
            loops.as_ptr(),
            segments.as_ptr(),
            ellipses.as_ptr(),
            &native,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.linear_blends.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}

fn build_surface_step(
    model: &MixedModel,
    cones: &[MixedCone],
    spheres: &[MixedSphere],
    tori: &[MixedTorus],
) -> Result<ValidatedStep, FabError> {
    build_surface_step_with_tokens(model, cones, spheres, tori, &[], &[], None)
}

fn build_surface_step_with_tokens(
    model: &MixedModel,
    cones: &[MixedCone],
    spheres: &[MixedSphere],
    tori: &[MixedTorus],
    partial_cones: &[MixedPartialCone],
    octants: &[MixedSphericalOctant],
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    if model.partial_cylinders.iter().any(|wall| wall.elliptic_end) {
        if !cones.is_empty()
            || !spheres.is_empty()
            || !tori.is_empty()
            || !partial_cones.is_empty()
            || !octants.is_empty()
        {
            return Err(oblique_pocket::invalid());
        }
        if model.partial_cylinders.len() == 2 {
            return adjacent_pocket::build(model, tokens);
        }
        return oblique_pocket::build(model, tokens);
    }
    let counts = [
        model.planes.len(),
        model.cylinders.len(),
        model.partial_cylinders.len(),
        cones.len(),
        spheres.len(),
        tori.len(),
        partial_cones.len(),
        octants.len(),
    ];
    if counts
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .is_none_or(|count| count > MAX_RESULT_FACES)
        || model.vertices.len() > MAX_SOURCE_VERTICES
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Le résultat mixte dépasse 100 000 faces ou le plafond de coordonnées source.",
        ));
    }
    let invalid = |message| FabError::new("cad.invalid_model", message);
    let legacy =
        cones.is_empty() && spheres.is_empty() && tori.is_empty() && partial_cones.is_empty();
    if !model.elliptic_arcs.is_empty()
        || model.planes.is_empty()
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
    let mut arc_walls = vec![0u8; arcs.len()];
    let mut rails: BTreeMap<_, (u8, u8)> = BTreeMap::new();
    let mut partial = Vec::with_capacity(model.partial_cylinders.len());
    let mut native_partial_cones = Vec::with_capacity(partial_cones.len());
    for (arc_ids, inner, conical) in model
        .partial_cylinders
        .iter()
        .map(|c| (c.arcs, c.inner, false))
        .chain(partial_cones.iter().map(|c| (c.arcs, c.inner, true)))
    {
        for arc in arc_ids {
            let used = arc_walls
                .get_mut(arc)
                .ok_or_else(|| invalid("Une paroi partielle référence un arc absent."))?;
            if *used == 2 {
                return Err(invalid("Un arc appartient à plus de deux parois."));
            }
            *used += 1;
        }
        let [lower, upper] = arc_ids;
        if edge_key([arcs[lower].vertices[0], arcs[upper].vertices[0]])
            == edge_key([arcs[lower].vertices[1], arcs[upper].vertices[1]])
        {
            return Err(invalid(
                "Les deux génératrices d'une paroi partielle sont confondues.",
            ));
        }
        for (from, to) in arcs[lower].vertices.into_iter().zip(arcs[upper].vertices) {
            let incidence = rails.entry(edge_key([from, to])).or_default();
            incidence.0 += 1;
            if from == to || incidence.0 > 2 {
                return Err(invalid(
                    "Une génératrice doit border au plus deux parois partielles distinctes.",
                ));
            }
        }
        let data = NativeMixedCylinder {
            rings: arc_ids,
            inner: i32::from(inner),
        };
        if conical {
            native_partial_cones.push(data);
        } else {
            partial.push(data);
        }
    }

    // The first local-octant contract is one full orthogonal trihedron.
    // Each of its three curves already belongs to exactly one different
    // exterior partial cylinder. The other arc of that cylinder must then
    // have a planar owner, checked by the ordinary incidence accounting.
    let native_octant = match octants {
        [] => None,
        [o] if model.planes.len() == 6
            && partial.len() == 3
            && arcs.len() == 6
            && rings.is_empty()
            && cylinders.is_empty()
            && cones.is_empty()
            && spheres.is_empty()
            && tori.is_empty()
            && partial_cones.is_empty()
            && !o.inner
            && partial.iter().all(|c| c.inner == 0) =>
        {
            if !o.radius_mm.is_finite()
                || o.radius_mm <= 0.
                || o.center.iter().any(|c| !c.is_finite())
                || !o.directions.iter().all(unit_direction)
            {
                return Err(invalid(
                    "Le centre, le rayon ou les directions de l’octant sont invalides.",
                ));
            }
            let ids = o.arc_uses.map(|(id, _)| id);
            for &id in &ids {
                if arc_walls.get(id) != Some(&1) {
                    return Err(invalid(
                        "Chaque arc de l’octant doit appartenir à un seul cylindre.",
                    ));
                }
                arc_walls[id] += 1;
            }
            if partial
                .iter()
                .any(|c| c.rings.iter().filter(|i| ids.contains(i)).count() != 1)
                || model.planes.iter().any(|p| p.loops.len() != 1)
            {
                return Err(invalid(
                    "L’octant doit rejoindre trois cylindres distincts du même coin.",
                ));
            }
            Some(NativeOctant {
                center: o.center,
                radius: o.radius_mm,
                directions: o.directions,
                arcs: ids,
                reversed: o.arc_uses.map(|(_, reversed)| i32::from(reversed)),
            })
        }
        _ => {
            return Err(invalid(
                "L’octant exige trois congés cylindriques extérieurs et six plans.",
            ))
        }
    };

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
                            MixedSegment::EllipticArc { .. } => {
                                return Err(invalid(
                                    "Un arc elliptique exige son raccord conique.",
                                ));
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
                if let Some(incidence) = rails.get_mut(&edge_key(segment.vertices)) {
                    incidence.1 += 1;
                    if incidence.0 + incidence.1 > 2 {
                        return Err(invalid("Une génératrice appartient à plus de deux faces."));
                    }
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
    if arc_walls
        .into_iter()
        .zip(arc_planes)
        .any(|(walls, plane)| walls == 0 || walls + u8::from(plane) != 2)
        || !rails
            .into_values()
            .all(|(walls, planes)| matches!((walls, planes), (1, 1) | (2, 0)))
    {
        return Err(invalid(
            "Chaque arc et génératrice relient exactement deux faces, dont au moins une paroi partielle.",
        ));
    }

    // La validation des indices précède l'appel. Aucun pointeur de ces
    // tableaux à plat ne survit à l'appel natif.
    native_step(|output, error, capacity| unsafe {
        if let Some(o) = &native_octant {
            return fabstep_build_trihedral_step(
                model.vertices.as_ptr().cast(),
                model.vertices.len(),
                normals.as_ptr().cast(),
                plane_loops.as_ptr(),
                loops.as_ptr(),
                segments.as_ptr(),
                arcs.as_ptr(),
                partial.as_ptr(),
                o,
                tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
                tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
                tokens.map_or(std::ptr::null(), |t| t.octants.as_ptr()),
                output,
                error,
                capacity,
            );
        }
        if let Some(tokens) = tokens {
            if spheres.is_empty()
                && tori.is_empty()
                && model.partial_cylinders.is_empty()
                && partial_cones.is_empty()
            {
                if !cones.is_empty() {
                    return fabstep_build_revolved_step_with_provenance(
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
                        native_cones.as_ptr(),
                        native_cones.len(),
                        tokens.planes.as_ptr(),
                        tokens.cylinders.as_ptr(),
                        tokens.cones.as_ptr(),
                        output,
                        error,
                        capacity,
                    );
                }
                return fabstep_build_mixed_step_with_provenance(
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
                    tokens.planes.as_ptr(),
                    tokens.cylinders.as_ptr(),
                    output,
                    error,
                    capacity,
                );
            }
            return fabstep_build_rounded_step_with_provenance(
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
                tokens.planes.as_ptr(),
                tokens.cylinders.as_ptr(),
                tokens.partial.as_ptr(),
                tokens.cones.as_ptr(),
                tokens.spheres.as_ptr(),
                tokens.tori.as_ptr(),
                native_partial_cones.as_ptr(),
                native_partial_cones.len(),
                tokens.partial_cones.as_ptr(),
                output,
                error,
                capacity,
            );
        }
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
            native_partial_cones.as_ptr(),
            native_partial_cones.len(),
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
        revolution_faces: metrics.revolution_faces,
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
        12 => "dxf.face_not_found",
        13 => "dxf.face_not_planar",
        14 => "dxf.unsupported_curve",
        15 => "dxf.invalid_profile",
        16 => "dxf.too_complex",
        17 => "dxf.section_coplanar",
        18 => "dxf.section_tangent",
        19 => "dxf.section_open",
        20 => "dxf.section_invalid_plane",
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

/// Exact boundary of one planar face in the same face order as `result_faces`.
/// IDs are scoped to these immutable STEP bytes. No display mesh is used.
pub fn planar_face_profile(bytes: &[u8], face_id: usize) -> Result<PlanarFaceProfile, FabError> {
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err(FabError::new(
            "cad.read_failed",
            "Le STEP à inspecter doit être non vide et ne pas dépasser 128 Mio.",
        ));
    }
    let invalid = || {
        FabError::new(
            "dxf.invalid_profile",
            "Le noyau a renvoyé un profil incohérent.",
        )
    };
    let mut native = NativePlanarProfile::default();
    let mut error = [0; 512];
    let status = unsafe {
        fabstep_planar_face_profile(
            bytes.as_ptr(),
            bytes.len(),
            face_id,
            &mut native,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    if native.solid_id >= MAX_STEP_BODIES
        || native.loops.is_null()
        || native.segments.is_null()
        || !(1..=4096).contains(&native.loop_count)
        || !(1..=100_000).contains(&native.segment_count)
    {
        return Err(invalid());
    }
    // Native-owned contiguous buffers are bounded before any Rust slice exists.
    let native_loops = unsafe { slice::from_raw_parts(native.loops, native.loop_count) };
    let native_segments = unsafe { slice::from_raw_parts(native.segments, native.segment_count) };
    let mut loops = Vec::with_capacity(native.loop_count);
    let mut end = 0;
    for row in native_loops {
        if row.start != end
            || row.count == 0
            || row.count > native.segment_count - end
            || row.is_outer > 1
        {
            return Err(invalid());
        }
        end += row.count;
        let mut segments = Vec::with_capacity(row.count);
        for segment in &native_segments[row.start..end] {
            segments.push(match segment.kind {
                0 => ProfileSegment::Line {
                    start_mm: segment.start,
                    end_mm: segment.end,
                },
                1 => ProfileSegment::Arc {
                    center_mm: segment.center,
                    radius_mm: segment.radius,
                    start_angle_rad: segment.start_angle,
                    sweep_angle_rad: segment.sweep_angle,
                },
                2 => ProfileSegment::Circle {
                    center_mm: segment.center,
                    radius_mm: segment.radius,
                },
                3 => ProfileSegment::Ellipse {
                    center_mm: segment.center,
                    major_axis_mm: segment.major_axis,
                    ratio: segment.ratio,
                    start_param_rad: segment.start_angle,
                    sweep_param_rad: segment.sweep_angle,
                },
                _ => return Err(invalid()),
            });
        }
        loops.push(PlanarProfileLoop {
            is_outer: row.is_outer == 1,
            segments,
        });
    }
    if end != native.segment_count {
        return Err(invalid());
    }
    let profile = PlanarFaceProfile {
        face_id,
        solid_id: native.solid_id,
        frame: ProfileFrame {
            origin_mm: native.origin,
            x_axis: native.x_axis,
            y_axis: native.y_axis,
            normal: native.normal,
        },
        loops,
    };
    fabstep_core::validate_planar_profile(&profile)?;
    Ok(profile)
}

/// Read-only face inspection of a previously validated FabSTEP document.
/// The fixed display deflection is not a geometric correction or an audit bound.
/// No topology identifier here denotes a source triangle or recognition region.
pub fn result_faces(bytes: &[u8]) -> Result<ResultFaces, FabError> {
    const MAX_TRIANGLES: usize = 100_000;
    const MAX_INCIDENCES: usize = 600_000;
    const DEFLECTION_MM: f64 = 0.01;
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err(FabError::new(
            "cad.read_failed",
            "Le STEP à inspecter doit être non vide et ne pas dépasser 128 Mio.",
        ));
    }
    let invalid = || {
        FabError::new(
            "cad.tessellation_failed",
            "Le noyau a renvoyé une inspection de faces STEP incohérente.",
        )
    };
    let mut native = NativeResultFaces::default();
    let mut error = [0; 512];
    let status = unsafe {
        fabstep_result_faces(
            bytes.as_ptr(),
            bytes.len(),
            DEFLECTION_MM,
            &mut native,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    let mesh = &native.mesh;
    if mesh.points.is_null()
        || mesh.triangles.is_null()
        || !(1..=3 * MAX_TRIANGLES).contains(&mesh.point_count)
        || !(1..=MAX_TRIANGLES).contains(&mesh.triangle_count)
        || native.faces.is_null()
        || !(1..=MAX_TRIANGLES).contains(&native.face_count)
        || native.neighbor_count > MAX_INCIDENCES
        || (native.neighbor_count > 0 && native.neighbors.is_null())
    {
        return Err(invalid());
    }
    // Only bounded, native-owned scalar arrays cross the ABI.
    let vertices = unsafe { slice::from_raw_parts(mesh.points, mesh.point_count) };
    let triangles = unsafe { slice::from_raw_parts(mesh.triangles, mesh.triangle_count) };
    if vertices.iter().flatten().any(|v| !v.is_finite())
        || triangles.iter().flatten().any(|&v| v >= vertices.len())
    {
        return Err(invalid());
    }
    let metadata = unsafe { slice::from_raw_parts(native.faces, native.face_count) };
    let neighbors = if native.neighbor_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(native.neighbors, native.neighbor_count) }
    };
    let mut faces = Vec::with_capacity(metadata.len());
    let mut triangle_end = 0;
    let mut neighbor_end = 0;
    let mut largest_solid = 0;
    for (id, face) in metadata.iter().enumerate() {
        if face.solid_id >= MAX_STEP_BODIES
            || face.solid_id > largest_solid + usize::from(id != 0)
            || (id > 0 && face.solid_id < metadata[id - 1].solid_id)
            || face.triangle_start != triangle_end
            || face.triangle_count == 0
            || face.triangle_count > triangles.len() - triangle_end
            || face.neighbor_start != neighbor_end
            || face.neighbor_count > neighbors.len() - neighbor_end
        {
            return Err(invalid());
        }
        largest_solid = face.solid_id;
        triangle_end += face.triangle_count;
        neighbor_end += face.neighbor_count;
        let adjacent = &neighbors[face.neighbor_start..neighbor_end];
        if adjacent.windows(2).any(|pair| pair[0] >= pair[1])
            || adjacent.iter().any(|&other| {
                other >= metadata.len() || other == id || metadata[other].solid_id != face.solid_id
            })
        {
            return Err(invalid());
        }
        let family = match face.family {
            0 => ResultFaceFamily::Plane,
            1 => ResultFaceFamily::Cylinder,
            2 => ResultFaceFamily::Cone,
            3 => ResultFaceFamily::Sphere,
            4 => ResultFaceFamily::Torus,
            5 => ResultFaceFamily::Other,
            6 => ResultFaceFamily::Revolution,
            _ => return Err(invalid()),
        };
        faces.push(ResultFace {
            id,
            solid_id: face.solid_id,
            family,
            triangle_start: face.triangle_start,
            triangle_count: face.triangle_count,
            adjacent_face_ids: adjacent.to_vec(),
            source_face_token: (face.source_face_token != 0).then_some(face.source_face_token),
        });
    }
    if triangle_end != triangles.len()
        || neighbor_end != neighbors.len()
        || faces.iter().any(|face| {
            face.adjacent_face_ids.iter().any(|&other| {
                faces[other]
                    .adjacent_face_ids
                    .binary_search(&face.id)
                    .is_err()
            })
        })
    {
        return Err(invalid());
    }
    Ok(ResultFaces {
        mesh: Mesh {
            vertices: vertices.to_vec(),
            triangles: triangles.to_vec(),
        },
        faces,
        deflection_mm: DEFLECTION_MM,
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

    fn assert_label_supports(inspection: &ResultFaces, model: &PlanarModel, tokens: &[usize]) {
        let found: std::collections::BTreeSet<_> = inspection
            .faces
            .iter()
            .map(|face| face.source_face_token.expect("exact transfer label"))
            .collect();
        assert_eq!(found, tokens.iter().copied().collect());
        for face in &inspection.faces {
            let token = face.source_face_token.unwrap();
            let source = &model.faces[tokens.iter().position(|&t| t == token).unwrap()];
            let origin = model.vertices[source.loops[0][0]];
            for triangle in &inspection.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    let point = inspection.mesh.vertices[vertex];
                    let distance: f64 = (0..3)
                        .map(|axis| (point[axis] - origin[axis]) * source.normal[axis])
                        .sum();
                    assert!(
                        distance.abs() < 1e-8,
                        "label {token} attached to wrong plane"
                    );
                }
            }
        }
    }

    #[test]
    fn provenance_tracks_constructed_faces_through_sewing_and_step_transfer() {
        let mut model = box_model();
        model.faces.rotate_left(3);
        model.faces.swap(1, 4);
        // Separate vertex/edge topology for each input face forces sewing to
        // replace subshapes: input handles cannot just bypass sewing history.
        let original = model.vertices.clone();
        model.vertices.clear();
        for face in &mut model.faces {
            for boundary in &mut face.loops {
                for vertex in boundary {
                    let point = original[*vertex];
                    *vertex = model.vertices.len();
                    model.vertices.push(point);
                }
            }
        }
        let tokens = [91, 7, 54, 100_000, 3, 12];
        let step = build_step_with_provenance(&model, &tokens).unwrap();
        let inspected = result_faces(&step.bytes).unwrap();
        assert_label_supports(&inspected, &model, &tokens);
        assert!(result_faces(&build_step(&model).unwrap().bytes)
            .unwrap()
            .faces
            .iter()
            .all(|face| face.source_face_token.is_none()));
    }

    #[test]
    fn provenance_is_independent_of_step_entity_numbers() {
        let model = box_model();
        let tokens = [43, 71, 18, 6, 99, 21];
        let step = build_step_with_provenance(&model, &tokens).unwrap();
        let text = String::from_utf8(step.bytes).unwrap();
        // Reverse every entity number and its references. This fixture has no
        // '#' in strings; source attribution must not depend on entity rank.
        let largest = text
            .split('#')
            .skip(1)
            .map(|tail| {
                tail.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse::<usize>()
                    .unwrap()
            })
            .max()
            .unwrap();
        let mut rewritten = String::new();
        let mut parts = text.split('#');
        rewritten.push_str(parts.next().unwrap());
        for tail in parts {
            let digits = tail.chars().take_while(char::is_ascii_digit).count();
            let number: usize = tail[..digits].parse().unwrap();
            rewritten.push('#');
            rewritten.push_str(&(largest + 1 - number).to_string());
            rewritten.push_str(&tail[digits..]);
        }
        let inspected = result_faces(rewritten.as_bytes()).unwrap();
        assert_label_supports(&inspected, &model, &tokens);
    }

    #[test]
    fn provenance_missing_duplicate_and_malformed_labels_preserve_display_without_mapping() {
        let step = build_step_with_provenance(&box_model(), &[1, 2, 3, 4, 5, 6]).unwrap();
        let text = String::from_utf8(step.bytes).unwrap();
        for replacement in [
            "",
            "FABSTEP_SOURCE_FACE_V1_2",
            "FABSTEP_SOURCE_FACE_V1_0",
            "FABSTEP_SOURCE_FACE_V1_100001",
            "FABSTEP_SOURCE_FACE_V1_01",
            "unrelated",
        ] {
            let corrupted =
                text.replacen("FABSTEP_SOURCE_FACE_V1_1'", &format!("{replacement}'"), 1);
            assert_ne!(corrupted, text);
            let inspected = result_faces(corrupted.as_bytes()).unwrap();
            assert_eq!(inspected.faces.len(), 6);
            assert!(inspected
                .faces
                .iter()
                .all(|face| face.source_face_token.is_none()));
        }
    }

    #[test]
    fn provenance_combination_preserves_labels_across_reversed_roots_and_repeated_names() {
        let mut first_model = box_model();
        first_model.faces.reverse();
        let first_tokens = [81, 14, 73, 22, 19, 55];
        let first = build_step_with_provenance(&first_model, &first_tokens).unwrap();
        let mut second_model = box_model();
        for point in &mut second_model.vertices {
            point[0] += 40.;
        }
        let second_tokens = [29, 61, 32, 88, 9, 100];
        let second = build_step_with_provenance(&second_model, &second_tokens).unwrap();
        let combined = combine_steps_with_provenance(&[
            StepPart {
                bytes: &second.bytes,
                name: "same",
            },
            StepPart {
                bytes: &first.bytes,
                name: "same",
            },
        ])
        .unwrap();
        let inspected = result_faces(&combined.bytes).unwrap();
        let mut model = first_model;
        // Keep source coordinates disjoint while collecting expected face supports.
        let offset = model.vertices.len();
        model.vertices.extend(second_model.vertices);
        for mut face in second_model.faces {
            for boundary in &mut face.loops {
                for vertex in boundary {
                    *vertex += offset;
                }
            }
            model.faces.push(face);
        }
        let tokens: Vec<_> = first_tokens.into_iter().chain(second_tokens).collect();
        assert_label_supports(&inspected, &model, &tokens);
        for face in inspected.faces {
            assert_eq!(
                face.solid_id,
                usize::from(first_tokens.contains(&face.source_face_token.unwrap()))
            );
        }
    }

    #[test]
    fn provenance_combination_drops_incomplete_or_cross_root_duplicate_metadata_only() {
        let first = build_step_with_provenance(&box_model(), &[1, 2, 3, 4, 5, 6]).unwrap();
        let mut model = box_model();
        for point in &mut model.vertices {
            point[0] += 40.;
        }
        let duplicate = build_step_with_provenance(&model, &[1, 2, 3, 4, 5, 6]).unwrap();
        let absent = build_step(&model).unwrap();
        for second in [&duplicate, &absent] {
            let combined = combine_steps_with_provenance(&[
                StepPart {
                    bytes: &first.bytes,
                    name: "a",
                },
                StepPart {
                    bytes: &second.bytes,
                    name: "b",
                },
            ])
            .unwrap();
            assert_eq!(combined.report.solids, 2);
            assert_eq!(combined.report.volume_mm3, 12_000.);
            assert!(result_faces(&combined.bytes)
                .unwrap()
                .faces
                .iter()
                .all(|face| face.source_face_token.is_none()));
        }
    }

    #[test]
    fn provenance_rejects_invalid_requested_tokens() {
        for tokens in [
            &[1, 2, 3, 4, 5][..],
            &[1, 2, 3, 4, 5, 5],
            &[0, 2, 3, 4, 5, 6],
            &[100_001, 2, 3, 4, 5, 6],
        ] {
            assert_eq!(
                build_step_with_provenance(&box_model(), tokens)
                    .unwrap_err()
                    .code,
                "cad.invalid_model"
            );
        }
    }

    fn section_request(z: f64, solid_ids: Vec<usize>) -> fabstep_core::SectionRequest {
        fabstep_core::SectionRequest {
            frame: ProfileFrame {
                origin_mm: [0., 0., z],
                x_axis: [1., 0., 0.],
                y_axis: [0., 1., 0.],
                normal: [0., 0., 1.],
            },
            solid_ids,
        }
    }

    #[test]
    fn section_profile_cuts_box_and_preserves_empty_body_choices() {
        let first = build_step(&box_model()).unwrap();
        let second = placed_box(1., [40., 0., 50.]);
        let combined = combine_steps(&[
            StepPart {
                bytes: &first.bytes,
                name: "first",
            },
            StepPart {
                bytes: &second.bytes,
                name: "second",
            },
        ])
        .unwrap();
        let section = section_profile(&combined.bytes, &section_request(15., vec![0, 1])).unwrap();
        assert_eq!(section.bodies.len(), 2);
        assert_eq!(section.bodies[0].loops.len(), 1);
        assert_eq!(section.bodies[0].loops[0].segments.len(), 4);
        assert!(section.bodies[0].loops[0].is_outer);
        assert!(section.bodies[1].loops.is_empty());
        let empty = section_profile(&combined.bytes, &section_request(100., vec![0, 1])).unwrap();
        assert!(empty.bodies.iter().all(|b| b.loops.is_empty()));
        assert_eq!(
            fabstep_core::export_section_dxf(&empty).unwrap_err().code,
            "dxf.section_empty"
        );
        fabstep_core::export_section_dxf(&section).unwrap();
    }

    #[test]
    fn section_profile_preserves_circular_and_polygonal_holes() {
        let mut polygon = plate();
        polygon.holes = vec![ProfileLoop::Polygon {
            points: vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
        }];
        for model in [plate(), polygon] {
            let step = build_extrusion_step(&model).unwrap();
            let section = section_profile(&step.bytes, &section_request(2.5, vec![0])).unwrap();
            let loops = &section.bodies[0].loops;
            assert_eq!(loops.len(), 2);
            assert_eq!(loops.iter().filter(|l| l.is_outer).count(), 1);
            assert_eq!(loops[0].component_id, loops[1].component_id);
            let hole = loops.iter().find(|l| !l.is_outer).unwrap();
            match model.holes[0] {
                ProfileLoop::Circle { .. } => assert!(
                    matches!(hole.segments.as_slice(),[ProfileSegment::Circle{radius_mm,..}] if (*radius_mm-1.).abs()<1e-10)
                ),
                _ => assert_eq!(hole.segments.len(), 4),
            }
            fabstep_core::export_section_dxf(&section).unwrap();
        }
    }

    #[test]
    fn section_profile_torus_has_annulus_or_two_disconnected_components() {
        let step = build_rounded_step(&complete_torus_model([0.; 3], [0., 0., 1.])).unwrap();
        let annulus = section_profile(&step.bytes, &section_request(1., vec![0])).unwrap();
        assert_eq!(annulus.bodies[0].loops.len(), 2);
        assert_eq!(
            annulus.bodies[0]
                .loops
                .iter()
                .filter(|l| l.is_outer)
                .count(),
            1
        );
        for boundary in &annulus.bodies[0].loops {
            let [ProfileSegment::Circle { radius_mm, .. }] = boundary.segments.as_slice() else {
                panic!("native torus section must remain circular")
            };
            let expected = 6.
                + if boundary.is_outer {
                    3_f64.sqrt()
                } else {
                    -3_f64.sqrt()
                };
            assert!((radius_mm - expected).abs() < 1e-7);
        }
        let request = fabstep_core::SectionRequest {
            frame: ProfileFrame {
                origin_mm: [0.; 3],
                x_axis: [0., 1., 0.],
                y_axis: [0., 0., 1.],
                normal: [1., 0., 0.],
            },
            solid_ids: vec![0],
        };
        let split = section_profile(&step.bytes, &request).unwrap();
        assert_eq!(split.bodies[0].loops.len(), 2);
        assert!(split.bodies[0].loops.iter().all(|l| l.is_outer));
        assert_ne!(
            split.bodies[0].loops[0].component_id,
            split.bodies[0].loops[1].component_id
        );
        for boundary in &split.bodies[0].loops {
            let [ProfileSegment::Circle { radius_mm, .. }] = boundary.segments.as_slice() else {
                panic!("native torus meridian must remain circular")
            };
            assert!((radius_mm - 2.).abs() < 1e-7);
        }
        fabstep_core::export_section_dxf(&annulus).unwrap();
        fabstep_core::export_section_dxf(&split).unwrap();
    }

    #[test]
    fn section_profile_rejects_coplanar_tangent_and_native_common_edges() {
        let cube = build_step(&box_model()).unwrap();
        assert_eq!(
            section_profile(&cube.bytes, &section_request(0., vec![0]))
                .unwrap_err()
                .code,
            "dxf.section_coplanar"
        );
        let torus = build_rounded_step(&complete_torus_model([0.; 3], [0., 0., 1.])).unwrap();
        assert_eq!(
            section_profile(&torus.bytes, &section_request(2., vec![0]))
                .unwrap_err()
                .code,
            "dxf.section_tangent"
        );
        let sphere = build_rounded_step(&complete_sphere_model([0.; 3], 6.)).unwrap();
        assert_eq!(
            section_profile(&sphere.bytes, &section_request(6., vec![0]))
                .unwrap_err()
                .code,
            "dxf.section_tangent"
        );
    }

    #[test]
    fn section_profile_keeps_oblique_mirrored_placements_and_large_native_circle() {
        let mut model = plate();
        model.origin = [40., -31., 12.];
        model.axis = [0.6, 0., 0.8];
        model.x_axis = [-0.8, 0., 0.6];
        let step = build_extrusion_step(&model).unwrap();
        let request = fabstep_core::SectionRequest {
            frame: ProfileFrame {
                origin_mm: [41.5, -31., 14.],
                x_axis: [-0.8, 0., 0.6],
                y_axis: [0., -1., 0.],
                normal: [0.6, 0., 0.8],
            },
            solid_ids: vec![0],
        };
        let section = section_profile(&step.bytes, &request).unwrap();
        assert_eq!(section.bodies[0].loops.len(), 2);
        let hole = section.bodies[0]
            .loops
            .iter()
            .find(|l| !l.is_outer)
            .unwrap();
        let [ProfileSegment::Circle {
            center_mm,
            radius_mm,
        }] = hole.segments.as_slice()
        else {
            panic!("hole must stay circular")
        };
        assert!(
            (center_mm[0] - 1.).abs() < 1e-7
                && (center_mm[1] + 1.).abs() < 1e-7
                && (radius_mm - 1.).abs() < 1e-7
        );
        let sphere = build_rounded_step(&complete_sphere_model([0.; 3], 1000.)).unwrap();
        let circle = section_profile(&sphere.bytes, &section_request(0., vec![0])).unwrap();
        let [ProfileSegment::Circle { radius_mm, .. }] =
            circle.bodies[0].loops[0].segments.as_slice()
        else {
            panic!("large sphere section must remain exact circle")
        };
        assert!((radius_mm - 1000.).abs() < 1e-7);
    }

    #[test]
    fn section_profile_validates_selection_frame_and_surface_only_face_frame() {
        let step = build_extrusion_step(&cylinder()).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        for face in inspection
            .faces
            .iter()
            .filter(|f| f.family == ResultFaceFamily::Plane)
        {
            let frame = planar_face_frame(&step.bytes, face.id).unwrap();
            let profile = planar_face_profile(&step.bytes, face.id).unwrap();
            assert_eq!(frame.origin_mm, profile.frame.origin_mm);
            assert_eq!(frame.normal, profile.frame.normal);
        }
        let mut request = section_request(2.5, vec![0]);
        request.frame.normal = [0.; 3];
        assert_eq!(
            section_profile(&step.bytes, &request).unwrap_err().code,
            "dxf.section_invalid_plane"
        );
        assert!(section_profile(&step.bytes, &section_request(2.5, vec![1])).is_err());
        assert!(section_profile(&step.bytes, &section_request(2.5, vec![0, 0])).is_err());
    }

    #[test]
    fn section_profile_refuses_tangent_point_away_from_seam_after_valid_body() {
        let regular = build_rounded_step(&complete_sphere_model([20., 0., 0.], 8.)).unwrap();
        let tangent = build_rounded_step(&complete_sphere_model([0.; 3], 6.)).unwrap();
        let combined = combine_steps(&[
            StepPart {
                bytes: &regular.bytes,
                name: "regular",
            },
            StepPart {
                bytes: &tangent.bytes,
                name: "tangent",
            },
        ])
        .unwrap();
        let mut request = fabstep_core::SectionRequest {
            frame: ProfileFrame {
                origin_mm: [3.6, 4.8, 0.],
                x_axis: [-0.8, 0.6, 0.],
                y_axis: [0., 0., 1.],
                normal: [0.6, 0.8, 0.],
            },
            solid_ids: vec![0],
        };
        let section = section_profile(&combined.bytes, &request).unwrap();
        assert_eq!(section.bodies[0].loops.len(), 1);
        request.solid_ids = vec![0, 1];
        assert_eq!(
            section_profile(&combined.bytes, &request).unwrap_err().code,
            "dxf.section_tangent"
        );
    }

    #[test]
    fn section_profile_refuses_unsupported_torus_planes_and_trimmed_support_levels() {
        let torus = build_rounded_step(&complete_torus_model([0.; 3], [0., 0., 1.])).unwrap();
        let mut request = section_request(0., vec![0]);
        request.frame.normal = [0.6, 0., 0.8];
        request.frame.x_axis = [0.8, 0., -0.6];
        assert_eq!(
            section_profile(&torus.bytes, &request).unwrap_err().code,
            "dxf.unsupported_curve"
        );
        // A conservative native body box proves this trimmed hemisphere absent
        // from the plane; its distant spherical support cannot veto emptiness.
        let hemisphere = build_rounded_step(&hemisphere_model()).unwrap();
        assert!(
            section_profile(&hemisphere.bytes, &section_request(-6., vec![0]))
                .unwrap()
                .bodies[0]
                .loops
                .is_empty()
        );
        let frustum = build_revolved_step(&cone_frustum(4., 2.)).unwrap();
        assert!(
            section_profile(&frustum.bytes, &section_request(16., vec![0]))
                .unwrap()
                .bodies[0]
                .loops
                .is_empty()
        );
        let cone = build_revolved_step(&cone_frustum(4., 0.)).unwrap();
        assert_eq!(
            section_profile(&cone.bytes, &section_request(8., vec![0]))
                .unwrap_err()
                .code,
            "dxf.section_tangent"
        );
    }

    #[test]
    fn section_profile_accepts_oblique_ellipses_and_partial_elliptic_arcs() {
        let cylinder = build_extrusion_step(&cylinder()).unwrap();
        for degrees in [1_f64, 5., 30., 60., 89.] {
            let (s, c) = degrees.to_radians().sin_cos();
            let mut request = section_request(2.5, vec![0]);
            request.frame.normal = [s, 0., c];
            request.frame.x_axis = [c, 0., -s];
            for reverse in [false, true] {
                if reverse {
                    request.frame.normal = request.frame.normal.map(|v| -v);
                    request.frame.y_axis = request.frame.y_axis.map(|v| -v);
                }
                let profile = section_profile(&cylinder.bytes, &request).unwrap();
                let ellipses: Vec<_> = profile.bodies[0].loops[0]
                    .segments
                    .iter()
                    .filter_map(|segment| {
                        if let ProfileSegment::Ellipse {
                            major_axis_mm,
                            ratio,
                            sweep_param_rad,
                            ..
                        } = segment
                        {
                            Some((
                                major_axis_mm[0].hypot(major_axis_mm[1]),
                                *ratio,
                                *sweep_param_rad,
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(!ellipses.is_empty());
                for (major, ratio, sweep) in ellipses {
                    assert!((major - 2. / c).abs() < 1e-7);
                    assert!((ratio - c).abs() < 1e-10);
                    assert_eq!(sweep.abs() == TAU, degrees <= 30.);
                }
                fabstep_core::export_section_dxf(&profile).unwrap();
            }
        }
        let cone = build_revolved_step(&cone_frustum(4., 2.)).unwrap();
        let (s, c) = 5_f64.to_radians().sin_cos();
        let mut request = section_request(4., vec![0]);
        request.frame.normal = [s, 0., c];
        request.frame.x_axis = [c, 0., -s];
        let profile = section_profile(&cone.bytes, &request).unwrap();
        assert!(profile.bodies[0].loops[0]
            .segments
            .iter()
            .all(|segment| matches!(segment, ProfileSegment::Ellipse { .. })));
        fabstep_core::export_section_dxf(&profile).unwrap();
    }

    #[test]
    fn section_profile_rejects_hyperbola_without_partial_body_output() {
        let cone = build_revolved_step(&cone_frustum(4., 2.)).unwrap();
        let cube = placed_box(1., [20., 0., 0.]);
        let combined = combine_steps(&[
            StepPart {
                bytes: &cube.bytes,
                name: "box",
            },
            StepPart {
                bytes: &cone.bytes,
                name: "hyperbola",
            },
        ])
        .unwrap();
        let mut request = section_request(0., vec![0]);
        request.frame.origin_mm = [0., 1., 0.];
        request.frame.normal = [0., -1., 0.];
        request.frame.y_axis = [0., 0., 1.];
        assert_eq!(
            section_profile(&cone.bytes, &request).unwrap_err().code,
            "dxf.unsupported_curve"
        );
        assert_eq!(
            section_profile(&combined.bytes, &request).unwrap().bodies[0]
                .loops
                .len(),
            1
        );
        request.solid_ids = vec![0, 1];
        assert_eq!(
            section_profile(&combined.bytes, &request).unwrap_err().code,
            "dxf.unsupported_curve"
        );
    }

    #[test]
    fn section_profile_crosses_exact_periodic_seams_without_joining_faces() {
        let cylinder = build_extrusion_step(&cylinder()).unwrap();
        let sphere = build_rounded_step(&complete_sphere_model([0.; 3], 6.)).unwrap();
        let hemisphere = build_rounded_step(&hemisphere_model()).unwrap();
        let mut meridian = section_request(0., vec![0]);
        meridian.frame.normal = [0., -1., 0.];
        meridian.frame.y_axis = [0., 0., 1.];
        for (step, segments) in [(&cylinder, 4), (&sphere, 2), (&hemisphere, 3)] {
            let profile = section_profile(&step.bytes, &meridian).unwrap();
            assert_eq!(profile.bodies[0].loops.len(), 1);
            assert_eq!(profile.bodies[0].loops[0].segments.len(), segments);
            fabstep_core::export_section_dxf(&profile).unwrap();
        }
        let torus = build_rounded_step(&complete_torus_model([0.; 3], [0., 0., 1.])).unwrap();
        let profile = section_profile(&torus.bytes, &section_request(0., vec![0])).unwrap();
        assert_eq!(profile.bodies[0].loops.len(), 2);
        for boundary in &profile.bodies[0].loops {
            let [ProfileSegment::Circle { radius_mm, .. }] = boundary.segments.as_slice() else {
                panic!("exact equatorial circles")
            };
            assert!((radius_mm - if boundary.is_outer { 8. } else { 4. }).abs() < 1e-7);
        }
        let rounded = build_rounded_box_step(&rounded_box_model([0.; 3])).unwrap();
        assert_eq!(
            section_profile(&rounded.bytes, &section_request(18., vec![0]))
                .unwrap_err()
                .code,
            "dxf.section_tangent"
        );
    }

    #[test]
    fn planar_profile_keeps_exact_native_ellipse_holes_and_oriented_frames() {
        // Native elliptical annular prism, validated after STEP roundtrip:
        // major/minor axes 4/2 mm and 1/.5 mm; height 5 mm; volume 37.5*pi.
        let bytes = include_bytes!("../tests/fixtures/elliptical_ring.step");
        let mut caps = 0;
        for face_id in 0..4 {
            let Ok(frame) = planar_face_frame(bytes, face_id) else {
                continue;
            };
            let profile = planar_face_profile(bytes, face_id).unwrap();
            caps += 1;
            assert_eq!(profile.loops.len(), 2);
            assert!(frame.normal[2].abs() > 1. - 1e-12);
            for boundary in &profile.loops {
                let [ProfileSegment::Ellipse {
                    major_axis_mm,
                    ratio,
                    sweep_param_rad,
                    ..
                }] = boundary.segments.as_slice()
                else {
                    panic!("native ellipse preserved")
                };
                assert!(
                    (major_axis_mm[0].hypot(major_axis_mm[1])
                        - if boundary.is_outer { 4. } else { 1. })
                    .abs()
                        < 1e-7
                );
                assert!((ratio - 0.5).abs() < 1e-12);
                assert_eq!(sweep_param_rad.abs(), TAU);
            }
            fabstep_core::export_planar_face_dxf(&profile).unwrap();
        }
        assert_eq!(caps, 2);
    }

    fn profile_world(profile: &PlanarFaceProfile, p: [f64; 2]) -> [f64; 3] {
        std::array::from_fn(|i| {
            profile.frame.origin_mm[i]
                + p[0] * profile.frame.x_axis[i]
                + p[1] * profile.frame.y_axis[i]
        })
    }

    #[test]
    fn planar_profile_keeps_display_face_identity_and_placed_mirrored_oblique_geometry() {
        let first = placed_box(1., [80., 0., 0.]);
        let mut model = box_model();
        // Reflection X, followed by a proper rotation around Y and translation.
        let transform = |[x, y, z]: [f64; 3]| [-0.8 * x + 0.6 * z, y, 0.6 * x + 0.8 * z];
        for p in &mut model.vertices {
            *p = transform(*p);
            for (value, offset) in p.iter_mut().zip([17., -31., 12.]) {
                *value += offset;
            }
        }
        for face in &mut model.faces {
            face.normal = transform(face.normal);
            for boundary in &mut face.loops {
                boundary.reverse();
            }
        }
        let second = build_step(&model).unwrap();
        let combined = combine_steps(&[
            StepPart {
                bytes: &first.bytes,
                name: "first",
            },
            StepPart {
                bytes: &second.bytes,
                name: "mirrored",
            },
        ])
        .unwrap();
        let inspection = result_faces(&combined.bytes).unwrap();
        for face in &inspection.faces {
            let profile = planar_face_profile(&combined.bytes, face.id).unwrap();
            assert_eq!(
                (profile.face_id, profile.solid_id),
                (face.id, face.solid_id)
            );
            assert_eq!(profile.loops.len(), 1);
            assert!(profile.loops[0].is_outer);
            assert_eq!(profile.loops[0].segments.len(), 4);
            let face_vertices: Vec<_> = inspection.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
                .iter()
                .flatten()
                .map(|&id| inspection.mesh.vertices[id])
                .collect();
            for segment in &profile.loops[0].segments {
                let ProfileSegment::Line { start_mm, end_mm } = segment else {
                    panic!("box must retain native lines")
                };
                for point in [start_mm, end_mm] {
                    let world = profile_world(&profile, *point);
                    assert!(face_vertices
                        .iter()
                        .any(|p| p.iter().zip(world).all(|(a, b)| (a - b).abs() < 1e-7)));
                }
            }
            fabstep_core::export_planar_face_dxf(&profile).unwrap();
        }
    }

    #[test]
    fn planar_profile_retains_long_arc_sweep_without_shortening() {
        let model = MixedModel {
            elliptic_arcs: vec![],
            vertices: vec![[2., 0., 0.], [0., -2., 0.], [2., 0., 5.], [0., -2., 5.]],
            rings: vec![],
            cylinders: vec![],
            arcs: vec![
                CircularArc {
                    center: [0., 0., 0.],
                    axis: [0., 0., 1.],
                    radius_mm: 2.,
                    sweep_rad: 1.5 * std::f64::consts::PI,
                    vertices: [0, 1],
                },
                CircularArc {
                    center: [0., 0., 5.],
                    axis: [0., 0., 1.],
                    radius_mm: 2.,
                    sweep_rad: 1.5 * std::f64::consts::PI,
                    vertices: [2, 3],
                },
            ],
            partial_cylinders: vec![MixedPartialCylinder {
                elliptic_end: false,
                arcs: [0, 1],
                inner: false,
            }],
            planes: vec![
                MixedPlane {
                    normal: [0., 0., -1.],
                    loops: vec![MixedLoop::Segments {
                        segments: vec![
                            MixedSegment::Line { vertices: [0, 1] },
                            MixedSegment::Arc {
                                arc: 0,
                                reversed: true,
                            },
                        ],
                    }],
                },
                MixedPlane {
                    normal: [0., 0., 1.],
                    loops: vec![MixedLoop::Segments {
                        segments: vec![
                            MixedSegment::Arc {
                                arc: 1,
                                reversed: false,
                            },
                            MixedSegment::Line { vertices: [3, 2] },
                        ],
                    }],
                },
                MixedPlane {
                    normal: [
                        std::f64::consts::FRAC_1_SQRT_2,
                        -std::f64::consts::FRAC_1_SQRT_2,
                        0.,
                    ],
                    loops: vec![MixedLoop::Polygon {
                        vertices: vec![1, 0, 2, 3],
                    }],
                },
            ],
        };
        let step = build_mixed_step(&model).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        let mut count = 0;
        for face in inspection
            .faces
            .iter()
            .filter(|f| f.family == ResultFaceFamily::Plane)
        {
            let profile = planar_face_profile(&step.bytes, face.id).unwrap();
            for segment in profile.loops.iter().flat_map(|l| &l.segments) {
                if let ProfileSegment::Arc {
                    sweep_angle_rad, ..
                } = segment
                {
                    count += 1;
                    assert!((sweep_angle_rad.abs() - 1.5 * std::f64::consts::PI).abs() < 1e-10);
                }
            }
            fabstep_core::export_planar_face_dxf(&profile).unwrap();
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn planar_profile_keeps_circle_caps_and_circular_holes() {
        for model in [cylinder(), plate()] {
            let step = build_extrusion_step(&model).unwrap();
            let inspection = result_faces(&step.bytes).unwrap();
            let mut cap_count = 0;
            for face in &inspection.faces {
                if face.family != ResultFaceFamily::Plane {
                    continue;
                }
                let profile = planar_face_profile(&step.bytes, face.id).unwrap();
                let circles: Vec<_> = profile
                    .loops
                    .iter()
                    .filter(|l| matches!(l.segments.as_slice(), [ProfileSegment::Circle { .. }]))
                    .collect();
                if circles.is_empty() {
                    continue;
                }
                cap_count += 1;
                assert_eq!(circles.len(), 1);
                assert_eq!(
                    profile.loops.len(),
                    if model.holes.is_empty() { 1 } else { 2 }
                );
                assert_eq!(circles[0].is_outer, model.holes.is_empty());
                let ProfileSegment::Circle { radius_mm, .. } = circles[0].segments[0] else {
                    unreachable!()
                };
                assert!((radius_mm - if model.holes.is_empty() { 2. } else { 1. }).abs() < 1e-10);
                fabstep_core::export_planar_face_dxf(&profile).unwrap();
            }
            assert_eq!(cap_count, 2);
        }
    }

    #[test]
    fn planar_profile_preserves_native_arcs_and_reversed_face_frames() {
        let step = build_mixed_step(&notched_box()).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        let mut arcs = Vec::new();
        for face in &inspection.faces {
            if face.family != ResultFaceFamily::Plane {
                continue;
            }
            let profile = planar_face_profile(&step.bytes, face.id).unwrap();
            for segment in profile.loops.iter().flat_map(|l| &l.segments) {
                if let ProfileSegment::Arc {
                    radius_mm,
                    sweep_angle_rad,
                    ..
                } = segment
                {
                    assert!((radius_mm - 2.).abs() < 1e-10);
                    assert!((sweep_angle_rad.abs() - std::f64::consts::PI).abs() < 1e-10);
                    arcs.push(profile.frame.normal);
                }
            }
            fabstep_core::export_planar_face_dxf(&profile).unwrap();
        }
        assert_eq!(arcs.len(), 2);
        assert!((arcs[0][2] + arcs[1][2]).abs() < 1e-10);
    }

    #[test]
    fn planar_profile_keeps_polygonal_holes_as_lines() {
        let mut model = plate();
        model.holes = vec![ProfileLoop::Polygon {
            points: vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
        }];
        let step = build_extrusion_step(&model).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        let mut holes = 0;
        for face in &inspection.faces {
            let profile = planar_face_profile(&step.bytes, face.id).unwrap();
            if profile.loops.len() == 2 {
                holes += 1;
                assert_eq!(profile.loops.iter().filter(|l| l.is_outer).count(), 1);
                assert!(profile.loops.iter().all(|l| l.segments.len() == 4
                    && l.segments
                        .iter()
                        .all(|s| matches!(s, ProfileSegment::Line { .. }))));
            }
        }
        assert_eq!(holes, 2);
    }

    #[test]
    fn planar_profile_refuses_missing_nonplanar_and_invalid_step() {
        let step = build_extrusion_step(&cylinder()).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        let side = inspection
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::Cylinder)
            .unwrap();
        assert_eq!(
            planar_face_profile(&step.bytes, side.id).unwrap_err().code,
            "dxf.face_not_planar"
        );
        assert_eq!(
            planar_face_profile(&step.bytes, usize::MAX)
                .unwrap_err()
                .code,
            "dxf.face_not_found"
        );
        for bytes in [b"".as_slice(), b"not STEP"] {
            assert_eq!(
                planar_face_profile(bytes, 0).unwrap_err().code,
                "cad.read_failed"
            );
        }
    }

    #[test]
    fn planar_profile_rejects_incomplete_native_boundary() {
        let step = build_step(&box_model()).unwrap();
        let text = String::from_utf8(step.bytes).unwrap();
        let mut changed = false;
        let text = text
            .lines()
            .map(|line| {
                if !changed && line.contains(" = EDGE_LOOP(") {
                    let comma = line.rfind(',').unwrap();
                    changed = true;
                    format!("{}));", &line[..comma])
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(changed);
        let error = planar_face_profile(text.as_bytes(), 0).unwrap_err();
        // The STEP reader already refuses the now-open solid, before extraction.
        assert_eq!(error.code, "cad.read_failed");
    }

    #[test]
    fn planar_profile_native_byte_limit_precedes_reading_buffer() {
        let mut native = NativePlanarProfile::default();
        let mut error = [0; 512];
        // This one-byte input is safe because oversized inputs must be refused
        // before StepInputBuffer or the STEP reader accesses their contents.
        let status = unsafe {
            fabstep_planar_face_profile(
                b"x".as_ptr(),
                MAX_STEP_BYTES + 1,
                0,
                &mut native,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_eq!(native_error(status, &error).code, "cad.read_failed");
        assert!(native.loops.is_null() && native.segments.is_null());
    }

    #[test]
    fn result_faces_exposes_six_actual_cube_faces_and_shared_edge_neighbors() {
        let step = build_step(&box_model()).unwrap();
        let inspection = result_faces(&step.bytes).unwrap();
        assert_eq!(inspection.faces.len(), 6);
        assert_eq!(inspection.mesh.triangles.len(), 12);
        let mut supports = Vec::new();
        for face in &inspection.faces {
            assert_eq!(face.family, ResultFaceFamily::Plane);
            assert_eq!(face.solid_id, 0);
            assert_eq!(face.triangle_count, 2);
            assert_eq!(face.adjacent_face_ids.len(), 4);
            let points = inspection.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
                .iter()
                .flatten()
                .map(|&i| inspection.mesh.vertices[i])
                .collect::<Vec<_>>();
            let axis = (0..3)
                .find(|&axis| points.iter().all(|p| p[axis] == points[0][axis]))
                .expect("every triangle in a STEP planar face lies on its exact support");
            supports.push((axis, points[0][axis]));
        }
        supports.sort_by(|a, b| a.partial_cmp(b).unwrap());
        supports.dedup();
        assert_eq!(supports.len(), 6);
    }

    #[test]
    fn result_faces_keeps_multiroot_placements_and_adjacency_separate() {
        let first = placed_box(1., [40., 0., 0.]);
        let second = placed_box(1., [0., 0., 0.]);
        let combined = combine_steps(&[
            StepPart {
                bytes: &first.bytes,
                name: "same",
            },
            StepPart {
                bytes: &second.bytes,
                name: "same",
            },
        ])
        .unwrap();
        let inspected = result_faces(&combined.bytes).unwrap();
        assert_eq!(inspected.faces.len(), 12);
        for face in &inspected.faces {
            assert_eq!(face.solid_id, usize::from(face.id >= 6));
            for &other in &face.adjacent_face_ids {
                assert_eq!(inspected.faces[other].solid_id, face.solid_id);
            }
            for triangle in &inspected.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    let x = inspected.mesh.vertices[vertex][0];
                    assert!(if face.solid_id == 0 {
                        (40.0..=50.0).contains(&x)
                    } else {
                        (0.0..=10.0).contains(&x)
                    });
                }
            }
        }
    }

    #[test]
    fn result_faces_cylinder_ignores_periodic_self_seam() {
        let step = build_extrusion_step(&cylinder()).unwrap();
        let inspected = result_faces(&step.bytes).unwrap();
        assert_eq!(inspected.faces.len(), 3);
        let side = inspected
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::Cylinder)
            .unwrap();
        assert_eq!(side.adjacent_face_ids.len(), 2);
        for face in &inspected.faces {
            assert!(!face.adjacent_face_ids.contains(&face.id));
            if face.family == ResultFaceFamily::Plane {
                assert_eq!(face.adjacent_face_ids, vec![side.id]);
            }
        }
    }

    #[test]
    fn result_faces_rejects_invalid_step_and_excessive_display_resolution() {
        for bytes in [b"".as_slice(), b"not STEP"] {
            assert_eq!(result_faces(bytes).unwrap_err().code, "cad.read_failed");
        }
        let step = build_rounded_step(&complete_sphere_model([0.; 3], 1_000_000.)).unwrap();
        assert_eq!(
            result_faces(&step.bytes).unwrap_err().code,
            "cad.tessellation_too_complex"
        );
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
            elliptic_arcs: vec![],
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

    fn assert_mixed_label_supports(
        inspected: &ResultFaces,
        model: &MixedModel,
        plane_tokens: &[usize],
        cylinder_tokens: &[usize],
    ) {
        let mut seen = std::collections::BTreeSet::new();
        for face in &inspected.faces {
            let token = face.source_face_token.expect("complete native provenance");
            let plane = plane_tokens.iter().position(|&value| value == token);
            let cylinder = cylinder_tokens.iter().position(|&value| value == token);
            if plane.is_none() && cylinder.is_none() {
                continue; // Other explicitly labelled roots may share this inspection.
            }
            assert!(seen.insert(token));
            for triangle in &inspected.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    let point = inspected.mesh.vertices[vertex];
                    if let Some(index) = plane {
                        assert_eq!(face.family, ResultFaceFamily::Plane);
                        let source = &model.planes[index];
                        let origin = match &source.loops[0] {
                            MixedLoop::Polygon { vertices } => model.vertices[vertices[0]],
                            MixedLoop::Circle { ring } => model.rings[*ring].center,
                            _ => panic!("test fixture expects polygons or circles"),
                        };
                        let distance: f64 = (0..3)
                            .map(|axis| (point[axis] - origin[axis]) * source.normal[axis])
                            .sum();
                        assert!(distance.abs() < 1e-8, "wrong plane for token {token}");
                    } else {
                        assert_eq!(face.family, ResultFaceFamily::Cylinder);
                        let rings = model.cylinders[cylinder.unwrap()].rings;
                        let first = &model.rings[rings[0]];
                        let second = &model.rings[rings[1]];
                        let offset: [f64; 3] =
                            std::array::from_fn(|axis| point[axis] - first.center[axis]);
                        let axial: f64 = (0..3).map(|i| offset[i] * first.axis[i]).sum();
                        let height: f64 = (0..3)
                            .map(|i| (second.center[i] - first.center[i]) * first.axis[i])
                            .sum();
                        let radial: f64 = (0..3)
                            .map(|i| (offset[i] - axial * first.axis[i]).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        assert!(
                            (radial - first.radius_mm).abs() < 1e-8,
                            "wrong cylinder for token {token}"
                        );
                        assert!(axial >= height.min(0.) - 1e-8);
                        assert!(axial <= height.max(0.) + 1e-8);
                    }
                }
            }
        }
        assert_eq!(seen.len(), plane_tokens.len() + cylinder_tokens.len());
    }

    #[test]
    fn mixed_provenance_tracks_permuted_planes_and_distinct_cylinders_through_sewing() {
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
        model.planes.rotate_left(3);
        model.cylinders.reverse();
        // Distinct input edge topology exercises exact sewing history as well
        // as circle/plane construction sharing; token order is arbitrary.
        let original = model.vertices.clone();
        model.vertices.clear();
        for plane in &mut model.planes {
            for boundary in &mut plane.loops {
                if let MixedLoop::Polygon { vertices } = boundary {
                    for vertex in vertices {
                        let point = original[*vertex];
                        *vertex = model.vertices.len();
                        model.vertices.push(point);
                    }
                }
            }
        }
        let planes = [71, 4, 91, 23, 8, 100_000, 16];
        let cylinders = [43, 2];
        let step = build_mixed_step_with_provenance(&model, &planes, &cylinders).unwrap();
        assert_eq!(step.report.faces, 9);
        assert!((step.report.volume_mm3 - (6000. - 34. * std::f64::consts::PI)).abs() < 1e-7);
        assert_mixed_label_supports(
            &result_faces(&step.bytes).unwrap(),
            &model,
            &planes,
            &cylinders,
        );
        assert!(result_faces(&build_mixed_step(&model).unwrap().bytes)
            .unwrap()
            .faces
            .iter()
            .all(|face| face.source_face_token.is_none()));
    }

    #[test]
    fn mixed_provenance_survives_reversed_mixed_and_planar_roots() {
        let model = blind_box();
        let planes = [31, 19, 81, 41, 12, 6, 58];
        let cylinders = [97];
        let mixed = build_mixed_step_with_provenance(&model, &planes, &cylinders).unwrap();
        let mut planar_model = box_model();
        for point in &mut planar_model.vertices {
            point[0] += 40.;
        }
        let planar_tokens = [72, 35, 90, 24, 18, 44];
        let planar = build_step_with_provenance(&planar_model, &planar_tokens).unwrap();
        for reverse in [false, true] {
            let mut parts = [
                StepPart {
                    bytes: &mixed.bytes,
                    name: "same",
                },
                StepPart {
                    bytes: &planar.bytes,
                    name: "same",
                },
            ];
            if reverse {
                parts.reverse();
            }
            let combined = combine_steps_with_provenance(&parts).unwrap();
            assert_eq!(combined.report.solids, 2);
            let inspected = result_faces(&combined.bytes).unwrap();
            assert_eq!(inspected.faces.len(), 14);
            assert_mixed_label_supports(&inspected, &model, &planes, &cylinders);
            let mut planar_inspection = inspected;
            planar_inspection.faces.retain(|face| {
                let is_planar_root = planar_tokens.contains(&face.source_face_token.unwrap());
                assert_eq!(face.solid_id, usize::from(is_planar_root != reverse));
                is_planar_root
            });
            assert_label_supports(&planar_inspection, &planar_model, &planar_tokens);
        }
    }

    #[test]
    fn mixed_provenance_corruption_drops_metadata_without_losing_geometry() {
        let step =
            build_mixed_step_with_provenance(&blind_box(), &[1, 2, 3, 4, 5, 6, 7], &[8]).unwrap();
        let text = String::from_utf8(step.bytes).unwrap();
        for replacement in [
            "",
            "FABSTEP_SOURCE_FACE_V1_1",
            "FABSTEP_SOURCE_FACE_V1_100001",
        ] {
            let corrupted =
                text.replacen("FABSTEP_SOURCE_FACE_V1_8'", &format!("{replacement}'"), 1);
            assert_ne!(corrupted, text);
            let inspected = result_faces(corrupted.as_bytes()).unwrap();
            assert_eq!(inspected.faces.len(), 8);
            assert_eq!(
                inspected
                    .faces
                    .iter()
                    .filter(|face| face.family == ResultFaceFamily::Cylinder)
                    .count(),
                1
            );
            assert!(inspected
                .faces
                .iter()
                .all(|face| face.source_face_token.is_none()));
        }
    }

    #[test]
    fn mixed_provenance_validates_tokens_and_leaves_partial_cylinders_unlabelled() {
        let model = blind_box();
        for (planes, cylinders) in [
            (&[1, 2, 3, 4, 5, 6][..], &[8][..]),
            (&[1, 2, 3, 4, 5, 6, 7][..], &[][..]),
            (&[1, 2, 3, 4, 5, 6, 7][..], &[7][..]),
            (&[1, 2, 3, 4, 5, 6, 7][..], &[0][..]),
            (&[1, 2, 3, 4, 5, 6, 7][..], &[100_001][..]),
        ] {
            assert_eq!(
                build_mixed_step_with_provenance(&model, planes, cylinders)
                    .unwrap_err()
                    .code,
                "cad.invalid_model"
            );
        }
        let partial = notched_box();
        let planes: Vec<_> = (1..=partial.planes.len()).collect();
        let step = build_mixed_step_with_provenance(&partial, &planes, &[]).unwrap();
        assert_eq!(step.report.cylindrical_faces, 1);
        assert!(result_faces(&step.bytes)
            .unwrap()
            .faces
            .iter()
            .all(|face| face.source_face_token.is_none()));
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
                elliptic_arcs: vec![],
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

    fn labelled_revolved_fixture() -> RevolvedModel {
        let mut model = cone_frustum(4., 2.);
        model
            .mixed
            .rings
            .extend([(2., 12.), (3., 16.)].map(|(radius_mm, z)| CircularRing {
                center: [0., 0., z],
                axis: [0., 0., 1.],
                radius_mm,
            }));
        model.mixed.planes[1].loops = vec![MixedLoop::Circle { ring: 3 }];
        model.mixed.cylinders.push(MixedCylinder {
            rings: [1, 2],
            inner: false,
        });
        model.cones.push(MixedCone {
            rings: [2, 3],
            inner: false,
        });
        model.mixed.planes.reverse();
        model.cones.reverse();
        model
    }

    fn assert_revolved_label_supports(
        inspected: &ResultFaces,
        model: &RevolvedModel,
        plane_tokens: &[usize],
        cylinder_tokens: &[usize],
        cone_tokens: &[usize],
    ) {
        // The existing helper verifies plane/cylinder supports, ignoring other roots.
        assert_mixed_label_supports(inspected, &model.mixed, plane_tokens, cylinder_tokens);
        let mut seen = std::collections::BTreeSet::new();
        for face in &inspected.faces {
            let token = face.source_face_token.expect("complete native provenance");
            let Some(index) = cone_tokens.iter().position(|&value| value == token) else {
                continue;
            };
            assert!(seen.insert(token));
            assert_eq!(face.family, ResultFaceFamily::Cone);
            let [first, second] = model.cones[index]
                .rings
                .map(|ring| &model.mixed.rings[ring]);
            let height: f64 = (0..3)
                .map(|axis| (second.center[axis] - first.center[axis]) * first.axis[axis])
                .sum();
            for triangle in &inspected.mesh.triangles
                [face.triangle_start..face.triangle_start + face.triangle_count]
            {
                for &vertex in triangle {
                    let offset: [f64; 3] = std::array::from_fn(|axis| {
                        inspected.mesh.vertices[vertex][axis] - first.center[axis]
                    });
                    let axial: f64 = (0..3).map(|axis| offset[axis] * first.axis[axis]).sum();
                    let radial = (0..3)
                        .map(|axis| (offset[axis] - axial * first.axis[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    let radius =
                        first.radius_mm + (second.radius_mm - first.radius_mm) * axial / height;
                    assert!(
                        (radial - radius).abs() < 1e-8,
                        "wrong cone for token {token}"
                    );
                    assert!(axial >= height.min(0.) - 1e-8 && axial <= height.max(0.) + 1e-8);
                }
            }
        }
        assert_eq!(seen.len(), cone_tokens.len());
    }

    #[test]
    fn revolved_provenance_tracks_permuted_surfaces_and_combined_roots() {
        let model = labelled_revolved_fixture();
        let planes = [91, 7];
        let cylinders = [63];
        let cones = [100_000, 2];
        let step =
            build_revolved_step_with_provenance(&model, &planes, &cylinders, &cones).unwrap();
        assert_eq!(step.report.faces, 5);
        assert_eq!(step.report.conical_faces, 2);
        assert_revolved_label_supports(
            &result_faces(&step.bytes).unwrap(),
            &model,
            &planes,
            &cylinders,
            &cones,
        );
        let mut planar = box_model();
        for point in &mut planar.vertices {
            point[0] += 40.;
        }
        let planar_tokens = [72, 35, 90, 24, 18, 44];
        let planar_step = build_step_with_provenance(&planar, &planar_tokens).unwrap();
        for reverse in [false, true] {
            let mut parts = [
                StepPart {
                    bytes: &step.bytes,
                    name: "same",
                },
                StepPart {
                    bytes: &planar_step.bytes,
                    name: "same",
                },
            ];
            if reverse {
                parts.reverse();
            }
            let combined = combine_steps_with_provenance(&parts).unwrap();
            assert_eq!(combined.report.solids, 2);
            assert_eq!(combined.report.conical_faces, 2);
            let inspected = result_faces(&combined.bytes).unwrap();
            assert_revolved_label_supports(&inspected, &model, &planes, &cylinders, &cones);
            let mut planar_inspection = inspected;
            planar_inspection.faces.retain(|face| {
                let is_planar_root = planar_tokens.contains(&face.source_face_token.unwrap());
                assert_eq!(face.solid_id, usize::from(is_planar_root != reverse));
                is_planar_root
            });
            assert_label_supports(&planar_inspection, &planar, &planar_tokens);
        }
    }

    #[test]
    fn revolved_provenance_rejects_invalid_tokens_and_drops_corrupt_metadata() {
        let model = labelled_revolved_fixture();
        for (planes, cylinders, cones) in [
            (&[1][..], &[3][..], &[4, 5][..]),
            (&[1, 2][..], &[][..], &[4, 5][..]),
            (&[1, 2][..], &[3][..], &[4][..]),
            (&[1, 2][..], &[3][..], &[4, 3][..]),
            (&[1, 2][..], &[3][..], &[4, 2][..]),
            (&[1, 2][..], &[3][..], &[4, 0][..]),
            (&[1, 2][..], &[3][..], &[4, 100_001][..]),
        ] {
            assert_eq!(
                build_revolved_step_with_provenance(&model, planes, cylinders, cones)
                    .unwrap_err()
                    .code,
                "cad.invalid_model"
            );
        }
        let step = build_revolved_step_with_provenance(&model, &[1, 2], &[3], &[4, 5]).unwrap();
        let text = String::from_utf8(step.bytes).unwrap();
        for replacement in [
            "",
            "FABSTEP_SOURCE_FACE_V1_1",
            "FABSTEP_SOURCE_FACE_V1_100001",
        ] {
            let corrupted =
                text.replacen("FABSTEP_SOURCE_FACE_V1_5'", &format!("{replacement}'"), 1);
            assert_ne!(corrupted, text);
            let inspected = result_faces(corrupted.as_bytes()).unwrap();
            assert_eq!(inspected.faces.len(), 5);
            assert_eq!(
                inspected
                    .faces
                    .iter()
                    .filter(|face| face.family == ResultFaceFamily::Cone)
                    .count(),
                2
            );
            assert!(inspected
                .faces
                .iter()
                .all(|face| face.source_face_token.is_none()));
        }
    }

    fn spherical_cap_model(height: f64) -> RoundedModel {
        let radius = 6.;
        let rim_radius = (height * (2. * radius - height)).sqrt();
        let mut revolved = cone_frustum(rim_radius, 0.);
        revolved.cones.clear();
        revolved.mixed.rings[1].center[2] = height;
        RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
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
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
            revolved: RevolvedModel {
                mixed: MixedModel {
                    elliptic_arcs: vec![],
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

    #[test]
    fn rounded_provenance_tracks_all_surface_families_and_partial_cylinders() {
        let partial = RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
            revolved: RevolvedModel {
                mixed: notched_box(),
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
        };
        for model in [
            toroidal_fillet_model(),
            hemisphere_model(),
            complete_sphere_model([1., 2., 3.], 4.),
            complete_torus_model([1., 2., 3.], [0., 0., 1.]),
            partial,
        ] {
            let mixed = &model.revolved.mixed;
            let counts = [
                mixed.planes.len(),
                mixed.cylinders.len(),
                mixed.partial_cylinders.len(),
                model.revolved.cones.len(),
                model.spheres.len(),
                model.tori.len(),
            ];
            let total: usize = counts.iter().sum();
            let all: Vec<_> = (1..=total).rev().map(|i| i * 7).collect();
            let mut at = 0;
            let tokens: Vec<_> = counts
                .iter()
                .map(|&n| {
                    let slice = &all[at..at + n];
                    at += n;
                    slice
                })
                .collect();
            let step = build_rounded_step_with_all_provenance(
                &model, tokens[0], tokens[1], tokens[2], tokens[3], tokens[4], tokens[5],
            )
            .unwrap();
            let historical = build_rounded_step(&model).unwrap();
            assert!((step.report.volume_mm3 - historical.report.volume_mm3).abs() < 1e-8);
            let inspected = result_faces(&step.bytes).unwrap();
            assert_eq!(inspected.faces.len(), total);
            let families = [
                ResultFaceFamily::Plane,
                ResultFaceFamily::Cylinder,
                ResultFaceFamily::Cylinder,
                ResultFaceFamily::Cone,
                ResultFaceFamily::Sphere,
                ResultFaceFamily::Torus,
            ];
            for face in &inspected.faces {
                let token = face.source_face_token.expect("complete tracked history");
                let group = tokens.iter().position(|row| row.contains(&token)).unwrap();
                assert_eq!(face.family, families[group]);
            }
            let seen: std::collections::BTreeSet<_> = inspected
                .faces
                .iter()
                .map(|f| f.source_face_token.unwrap())
                .collect();
            assert_eq!(seen, all.iter().copied().collect());
        }
    }

    #[test]
    fn rounded_provenance_requires_exhaustive_unique_bounded_tokens() {
        let model = toroidal_fillet_model();
        let mixed = &model.revolved.mixed;
        let planes: Vec<_> = (1..=mixed.planes.len()).collect();
        let cylinders: Vec<_> = (planes.len() + 1..=planes.len() + mixed.cylinders.len()).collect();
        let torus = planes.len() + cylinders.len() + 1;
        assert!(build_rounded_step_with_provenance(
            &model,
            &planes,
            &cylinders,
            &[],
            &[],
            &[torus]
        )
        .is_ok());
        for bad in [vec![], vec![0], vec![100_001], vec![planes[0]]] {
            assert_eq!(
                build_rounded_step_with_provenance(&model, &planes, &cylinders, &[], &[], &bad)
                    .unwrap_err()
                    .code,
                "cad.invalid_model"
            );
        }
        let partial = RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
            revolved: RevolvedModel {
                mixed: notched_box(),
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
        };
        let planes: Vec<_> = (1..=partial.revolved.mixed.planes.len()).collect();
        assert_eq!(
            build_rounded_step_with_provenance(&partial, &planes, &[], &[], &[], &[])
                .unwrap_err()
                .code,
            "cad.invalid_model"
        );
        let sphere = complete_sphere_model([0.; 3], 2.);
        assert_eq!(
            build_rounded_step_with_provenance(&sphere, &[], &[], &[], &[0], &[])
                .unwrap_err()
                .code,
            "cad.invalid_model"
        );
    }

    #[test]
    fn inner_meridian_torus_cone_cylinder_reimports_with_shared_rings() {
        let pi = std::f64::consts::PI;
        let alpha = (1_f64 / 15.).atan();
        let major = 10.;
        let minor = 0.5;
        let first_angle = -pi / 2.;
        let tangent_angle = -pi + alpha;
        let tangent_z = minor * (1. + tangent_angle.sin());
        let tangent_radius = major + minor * tangent_angle.cos();
        let cone_height = 2.;
        let cone_z = tangent_z + cone_height;
        let land_radius = tangent_radius - cone_height * alpha.tan();
        let length = 5.;
        let mut revolved = cone_frustum(major, land_radius);
        revolved.mixed.rings[1].center[2] = length;
        revolved
            .mixed
            .rings
            .extend(
                [(tangent_radius, tangent_z), (land_radius, cone_z)].map(|(radius_mm, z)| {
                    CircularRing {
                        center: [0., 0., z],
                        axis: [0., 0., 1.],
                        radius_mm,
                    }
                }),
            );
        revolved.cones[0].rings = [2, 3];
        revolved.mixed.cylinders.push(MixedCylinder {
            rings: [3, 1],
            inner: false,
        });
        let mut model = RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            revolved,
            spheres: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
            tori: vec![MixedTorus {
                rings: Some([2, 0]),
                axis: None,
                center: [0., 0., minor],
                major_radius_mm: major,
                minor_radius_mm: minor,
                meridian_rad: [tangent_angle, first_angle],
                inner: true,
            }],
        };

        // Independent volume by integration of horizontal circular sections.
        // The angular change of variable removes the endpoint singularity in dρ/dz.
        let n = 4096;
        let angular_step = (pi / 2. - alpha) / f64::from(n);
        let arc_volume = (0..=n)
            .map(|i| {
                let theta = angular_step * f64::from(i);
                let coefficient = match i {
                    i if i == 0 || i == n => 1.,
                    i if i % 2 == 0 => 2.,
                    _ => 4.,
                };
                coefficient * (major - minor * theta.sin()).powi(2) * minor * theta.sin()
            })
            .sum::<f64>()
            * pi
            * angular_step
            / 3.;
        let cone_volume = pi
            * cone_height
            * (tangent_radius.powi(2) + tangent_radius * land_radius + land_radius.powi(2))
            / 3.;
        let expected_volume =
            arc_volume + cone_volume + pi * land_radius.powi(2) * (length - cone_z);

        for mirrored in [false, true] {
            if mirrored {
                for ring in &mut model.revolved.mixed.rings {
                    ring.center[2] = length - ring.center[2];
                }
                for plane in &mut model.revolved.mixed.planes {
                    plane.normal[2] = -plane.normal[2];
                }
                model.revolved.cones[0].rings.reverse();
                model.revolved.mixed.cylinders[0].rings.reverse();
                model.tori[0].rings = Some([0, 2]);
                model.tori[0].center[2] = length - minor;
                model.tori[0].meridian_rad = [-first_angle, -tangent_angle];
            }
            let step =
                build_rounded_step_with_provenance(&model, &[1, 2], &[3], &[4], &[], &[5]).unwrap();
            let report = &step.report;
            assert_eq!(report.solids, 1);
            assert_eq!(report.faces, 5);
            assert_eq!(report.planar_faces, 2);
            assert_eq!(report.cylindrical_faces, 1);
            assert_eq!(report.conical_faces, 1);
            assert_eq!(report.toroidal_faces, 1);
            assert!((report.volume_mm3 - expected_volume).abs() < 1e-7);
            for (measured, expected) in report
                .bounds_mm
                .iter()
                .flatten()
                .zip([-major, -major, 0., major, major, length])
            {
                assert!((measured - expected).abs() < 2e-6);
            }
            let mut observed: Vec<_> = result_faces(&step.bytes)
                .unwrap()
                .faces
                .into_iter()
                .map(|face| (face.source_face_token.unwrap(), face.family))
                .collect();
            observed.sort_by_key(|entry| entry.0);
            assert_eq!(
                observed,
                vec![
                    (1, ResultFaceFamily::Plane),
                    (2, ResultFaceFamily::Plane),
                    (3, ResultFaceFamily::Cylinder),
                    (4, ResultFaceFamily::Cone),
                    (5, ResultFaceFamily::Torus),
                ]
            );
        }
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
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
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
            elliptic_arcs: vec![],
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
                elliptic_end: false,
                arcs: [0, 1],
                inner: true,
            }],
        }
    }

    fn shared_rail_lens() -> MixedModel {
        let y = 5_f64.sqrt();
        let sweep = 2. * (2_f64 / 3.).acos();
        let arcs = [
            (-2., 0., [0, 1]),
            (-2., 7., [2, 3]),
            (2., 0., [1, 0]),
            (2., 7., [3, 2]),
        ]
        .into_iter()
        .map(|(x, z, vertices)| CircularArc {
            center: [x, 0., z],
            axis: [0., 0., 1.],
            radius_mm: 3.,
            sweep_rad: sweep,
            vertices,
        })
        .collect();
        MixedModel {
            elliptic_arcs: vec![],
            vertices: vec![[0., -y, 0.], [0., y, 0.], [0., -y, 7.], [0., y, 7.]],
            rings: vec![],
            cylinders: vec![],
            arcs,
            partial_cylinders: vec![
                MixedPartialCylinder {
                    elliptic_end: false,
                    arcs: [0, 1],
                    inner: false,
                },
                MixedPartialCylinder {
                    elliptic_end: false,
                    arcs: [2, 3],
                    inner: false,
                },
            ],
            planes: vec![
                MixedPlane {
                    normal: [0., 0., -1.],
                    loops: vec![MixedLoop::Segments {
                        segments: vec![
                            MixedSegment::Arc {
                                arc: 0,
                                reversed: true,
                            },
                            MixedSegment::Arc {
                                arc: 2,
                                reversed: true,
                            },
                        ],
                    }],
                },
                MixedPlane {
                    normal: [0., 0., 1.],
                    loops: vec![MixedLoop::Segments {
                        segments: vec![
                            MixedSegment::Arc {
                                arc: 1,
                                reversed: false,
                            },
                            MixedSegment::Arc {
                                arc: 3,
                                reversed: false,
                            },
                        ],
                    }],
                },
            ],
        }
    }

    fn reverse_partial_axis(model: &mut MixedModel, index: usize) {
        let ids = model.partial_cylinders[index].arcs;
        model.partial_cylinders[index].arcs.reverse();
        for id in ids {
            model.arcs[id].axis = model.arcs[id].axis.map(|x| -x);
            model.arcs[id].vertices.reverse();
        }
        for plane in &mut model.planes {
            for lp in &mut plane.loops {
                if let MixedLoop::Segments { segments } = lp {
                    for segment in segments {
                        if let MixedSegment::Arc { arc, reversed } = segment {
                            if ids.contains(arc) {
                                *reversed = !*reversed;
                            }
                        }
                    }
                }
            }
        }
    }

    fn split_notched_box() -> MixedModel {
        let mut model = notched_box();
        model.vertices.extend([[0., 2., 0.], [0., 2., 10.]]);
        for (arc, midpoint) in [(0, 12), (1, 13)] {
            let mut second = model.arcs[arc].clone();
            model.arcs[arc].vertices[1] = midpoint;
            model.arcs[arc].sweep_rad *= 0.5;
            second.vertices[0] = midpoint;
            second.sweep_rad *= 0.5;
            model.arcs.push(second);
        }
        model.partial_cylinders.push(MixedPartialCylinder {
            elliptic_end: false,
            arcs: [2, 3],
            inner: true,
        });
        for plane in &mut model.planes {
            for lp in &mut plane.loops {
                if let MixedLoop::Segments { segments } = lp {
                    let mut expanded = vec![];
                    for segment in std::mem::take(segments) {
                        if let MixedSegment::Arc { arc, reversed } = segment {
                            let pair = if reversed {
                                [arc + 2, arc]
                            } else {
                                [arc, arc + 2]
                            };
                            expanded.extend(pair.map(|arc| MixedSegment::Arc { arc, reversed }));
                        } else {
                            expanded.push(segment);
                        }
                    }
                    *segments = expanded;
                }
            }
        }
        model
    }

    #[test]
    fn exports_partial_cylinders_sharing_source_rails() {
        for reversed_axis in [false, true] {
            for permuted in [false, true] {
                let mut lens = shared_rail_lens();
                let mut notch = split_notched_box();
                if reversed_axis {
                    reverse_partial_axis(&mut lens, 1);
                    reverse_partial_axis(&mut notch, 1);
                }
                if permuted {
                    lens.partial_cylinders.reverse();
                    notch.partial_cylinders.reverse();
                }
                check_mixed(
                    &lens,
                    2,
                    2,
                    7. * (18. * (2_f64 / 3.).acos() - 4. * 5_f64.sqrt()),
                );
                check_mixed(&notch, 7, 2, 800. - 20. * std::f64::consts::PI);
            }
        }
    }

    #[test]
    fn rejects_invalid_shared_partial_rail_incidence() {
        let mut duplicate = shared_rail_lens();
        duplicate
            .partial_cylinders
            .push(duplicate.partial_cylinders[0].clone());
        assert_eq!(
            build_mixed_step(&duplicate).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut third = shared_rail_lens();
        third
            .arcs
            .extend([third.arcs[0].clone(), third.arcs[1].clone()]);
        third.partial_cylinders.push(MixedPartialCylinder {
            elliptic_end: false,
            arcs: [4, 5],
            inner: false,
        });
        assert_eq!(
            build_mixed_step(&third).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut planar_third = shared_rail_lens();
        planar_third.planes.push(MixedPlane {
            normal: [-1., 0., 0.],
            loops: vec![MixedLoop::Polygon {
                vertices: vec![0, 2, 3, 1],
            }],
        });
        assert_eq!(
            build_mixed_step(&planar_third).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut endpoints = shared_rail_lens();
        endpoints.arcs[3].vertices.reverse();
        assert_eq!(
            build_mixed_step(&endpoints).unwrap_err().code,
            "cad.invalid_model"
        );
        let mut orientation = shared_rail_lens();
        orientation.partial_cylinders[1].inner = true;
        assert_eq!(
            build_mixed_step(&orientation).unwrap_err().code,
            "cad.invalid_model"
        );
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
    fn partial_cone_chain() -> RoundedModel {
        let levels = [(0., 2.), (1., 2.), (2., 3.), (3., 3.)];
        let vertices = levels
            .iter()
            .flat_map(|&(z, r)| [[r, 0., z], [-r, 0., z]])
            .collect();
        let arcs = levels
            .iter()
            .enumerate()
            .map(|(i, &(z, r))| CircularArc {
                center: [0., 0., z],
                axis: [0., 0., 1.],
                radius_mm: r,
                sweep_rad: std::f64::consts::PI,
                vertices: [2 * i, 2 * i + 1],
            })
            .collect();
        let line = |a, b| MixedSegment::Line { vertices: [a, b] };
        let planes = vec![
            MixedPlane {
                normal: [0., 0., -1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        MixedSegment::Arc {
                            arc: 0,
                            reversed: true,
                        },
                        line(0, 1),
                    ],
                }],
            },
            MixedPlane {
                normal: [0., 0., 1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        MixedSegment::Arc {
                            arc: 3,
                            reversed: false,
                        },
                        line(7, 6),
                    ],
                }],
            },
            MixedPlane {
                normal: [0., -1., 0.],
                loops: vec![MixedLoop::Polygon {
                    vertices: vec![1, 0, 2, 4, 6, 7, 5, 3],
                }],
            },
        ];
        RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            revolved: RevolvedModel {
                mixed: MixedModel {
                    elliptic_arcs: vec![],
                    vertices,
                    rings: vec![],
                    planes,
                    cylinders: vec![],
                    arcs,
                    partial_cylinders: vec![
                        MixedPartialCylinder {
                            elliptic_end: false,
                            arcs: [0, 1],
                            inner: false,
                        },
                        MixedPartialCylinder {
                            elliptic_end: false,
                            arcs: [2, 3],
                            inner: false,
                        },
                    ],
                },
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![MixedPartialCone {
                arcs: [1, 2],
                inner: false,
            }],
        }
    }

    // Exterior rays are the positive coordinate axes. On each cylinder
    // the lower arc closes on the remote plane and the upper one on the
    // spherical octant. Vertex ids are shared by every facial boundary.
    fn trihedral_model() -> RoundedModel {
        let directions = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let far = [18., 14., 8.];
        let radius = 2.;
        let mut vertices = directions.map(|ray| ray.map(|c| c * radius)).to_vec();
        let mut arcs = Vec::new();
        let mut partial_cylinders = Vec::new();
        let remote = |axis: usize, ray: usize| 3 + 2 * axis + usize::from(ray == (axis + 2) % 3);
        for axis in 0..3 {
            let j = (axis + 1) % 3;
            let k = (axis + 2) % 3;
            let mut center = [0.; 3];
            center[axis] = -far[axis];
            for ray in [j, k] {
                let mut p = vertices[ray];
                p[axis] = -far[axis];
                vertices.push(p);
            }
            for (center, ends) in [
                (center, [remote(axis, j), remote(axis, k)]),
                ([0.; 3], [j, k]),
            ] {
                arcs.push(CircularArc {
                    center,
                    axis: directions[axis],
                    radius_mm: radius,
                    sweep_rad: std::f64::consts::FRAC_PI_2,
                    vertices: ends,
                });
            }
            partial_cylinders.push(MixedPartialCylinder {
                elliptic_end: false,
                arcs: [2 * axis, 2 * axis + 1],
                inner: false,
            });
        }
        let back = far.map(|v| -v);
        for axis in 0..3 {
            let mut p = back;
            p[axis] = radius;
            vertices.push(p);
        }
        vertices.push(back);
        let mut planes = Vec::new();
        for (i, dir) in directions.iter().enumerate() {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            planes.push(MixedPlane {
                normal: *dir,
                loops: vec![MixedLoop::Polygon {
                    vertices: vec![i, remote(j, i), 9 + i, remote(k, i)],
                }],
            });
            let path = [remote(i, j), 9 + j, 12, 9 + k, remote(i, k)];
            let mut segments = vec![MixedSegment::Arc {
                arc: 2 * i,
                reversed: true,
            }];
            segments.extend(path.windows(2).map(|v| MixedSegment::Line {
                vertices: [v[0], v[1]],
            }));
            planes.push(MixedPlane {
                normal: dir.map(|v| -v),
                loops: vec![MixedLoop::Segments { segments }],
            });
        }
        RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            revolved: RevolvedModel {
                mixed: MixedModel {
                    elliptic_arcs: vec![],
                    vertices,
                    rings: vec![],
                    planes,
                    cylinders: vec![],
                    arcs,
                    partial_cylinders,
                },
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
            partial_cones: vec![],
            cross_drill: None,
            linear_blend: None,
            octants: vec![MixedSphericalOctant {
                center: [0.; 3],
                radius_mm: radius,
                directions,
                arc_uses: [(5, false), (1, false), (3, false)],
                inner: false,
            }],
        }
    }

    // Independently construct this scalar block; the mesh recognizer is not
    // involved in testing the data admitted by the native boundary.
    fn linear_blend_model(bottom: f64, slope: f64) -> RoundedModel {
        let denominator = 1. - slope * slope;
        let theta = (denominator / 2.).sqrt().asin();
        let mut vertices = Vec::new();
        let mut arcs = Vec::new();
        for (k, z) in [0., 10.].into_iter().enumerate() {
            let r = bottom + slope * z;
            vertices.extend([
                [0., r, z],
                [r, 0., z],
                [20., 0., z],
                [20., 16., z],
                [0., 16., z],
            ]);
            arcs.push(fabstep_core::EllipticArc {
                center: [r / denominator, r / denominator, z],
                axis: [0., 0., 1.],
                major_direction: [-0.5_f64.sqrt(), -0.5_f64.sqrt(), 0.],
                major_mm: r * (1. + slope * slope).sqrt() / denominator,
                minor_mm: r / denominator.sqrt(),
                start_rad: -theta,
                sweep_rad: 2. * theta,
                vertices: [5 * k, 5 * k + 1],
            });
        }
        let faces: [(&[usize], [f64; 3]); 6] = [
            (&[1, 0, 4, 3, 2], [0., 0., -1.]),
            (&[5, 6, 7, 8, 9], [0., 0., 1.]),
            (&[1, 2, 7, 6], [0., -1., 0.]),
            (&[2, 3, 8, 7], [1., 0., 0.]),
            (&[3, 4, 9, 8], [0., 1., 0.]),
            (&[4, 0, 5, 9], [-1., 0., 0.]),
        ];
        let planes = faces
            .into_iter()
            .enumerate()
            .map(|(face, (ids, normal))| {
                let boundary = if face < 2 {
                    let segments = ids
                        .iter()
                        .enumerate()
                        .map(|(j, &from)| {
                            if j == 0 {
                                MixedSegment::EllipticArc {
                                    arc: face,
                                    reversed: face == 0,
                                }
                            } else {
                                MixedSegment::Line {
                                    vertices: [from, ids[(j + 1) % ids.len()]],
                                }
                            }
                        })
                        .collect();
                    MixedLoop::Segments { segments }
                } else {
                    MixedLoop::Polygon {
                        vertices: ids.to_vec(),
                    }
                };
                MixedPlane {
                    normal,
                    loops: vec![boundary],
                }
            })
            .collect();
        let q = (1. + 2. * slope * slope).sqrt();
        RoundedModel {
            smooth_revolution: None,
            unequal_pocket: None,
            torus_sectors: vec![],
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices,
                    planes,
                    elliptic_arcs: arcs,
                    rings: vec![],
                    arcs: vec![],
                    cylinders: vec![],
                    partial_cylinders: vec![],
                },
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
            partial_cones: vec![],
            octants: vec![],
            cross_drill: None,
            linear_blend: Some(Box::new(fabstep_core::MixedLinearBlend {
                apex: [0., 0., -bottom / slope],
                axis: [slope.abs() / q, slope.abs() / q, slope.signum() / q],
                semi_angle_rad: (slope.abs() / q).asin(),
                arcs: [0, 1],
                rails: [[1, 6], [5, 0]],
            })),
        }
    }

    #[test]
    fn linear_blend_reimports_each_trim_and_label_for_both_slopes() {
        let labels = [12, 31, 7, 64, 17, 91];
        for (bottom, slope) in [(3., 0.2), (5., -0.2), (3.5, 0.12), (9., -0.67)] {
            let model = linear_blend_model(bottom, slope);
            let step = build_rounded_step_with_linear_blend_provenance(
                &model,
                &labels,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[93],
            )
            .unwrap();
            assert_eq!(
                (step.report.planar_faces, step.report.conical_faces),
                (6, 1)
            );
            let result = result_faces(&step.bytes).unwrap();
            assert_eq!(result.faces.len(), 7);
            let mut ids = std::collections::BTreeSet::new();
            for f in result.faces {
                let label = f.source_face_token.unwrap();
                assert!(ids.insert(label));
                if label == 93 {
                    assert_eq!(f.family, ResultFaceFamily::Cone);
                    assert_eq!(f.adjacent_face_ids.len(), 4);
                    continue;
                }
                let idx = labels.iter().position(|&t| t == label).unwrap();
                let p = planar_face_profile(&step.bytes, f.id).unwrap();
                for (a, b) in p
                    .frame
                    .normal
                    .into_iter()
                    .zip(model.revolved.mixed.planes[idx].normal)
                {
                    assert!((a - b).abs() < 1e-8);
                }
                let arcs = p.loops[0]
                    .segments
                    .iter()
                    .filter(|s| matches!(s, ProfileSegment::Ellipse { .. }))
                    .count();
                assert_eq!(arcs, usize::from(idx < 2));
                fabstep_core::export_planar_face_dxf(&p).unwrap();
            }
            assert_eq!(ids.len(), 7);
            let plain = build_rounded_step(&model).unwrap();
            assert!((plain.report.volume_mm3 - step.report.volume_mm3).abs() < 1e-8);
        }
    }

    #[test]
    fn linear_blend_validates_scalar_references_and_exclusive_ownership() {
        let model = linear_blend_model(3., 0.2);
        for corrupt in [
            (|m: &mut RoundedModel| m.linear_blend.as_mut().unwrap().arcs = [1, 1])
                as fn(&mut RoundedModel),
            |m| m.linear_blend.as_mut().unwrap().rails[0].reverse(),
            |m| m.linear_blend.as_mut().unwrap().axis[0] = f64::NAN,
            |m| m.linear_blend.as_mut().unwrap().apex[0] += 0.02,
            |m| m.revolved.mixed.elliptic_arcs[0].minor_mm += 0.1,
            |m| m.revolved.mixed.elliptic_arcs[1].start_rad += 0.02,
            |m| m.revolved.mixed.elliptic_arcs[0].vertices[0] = usize::MAX,
        ] {
            let mut bad = model.clone();
            corrupt(&mut bad);
            assert!(build_rounded_step(&bad).is_err());
        }
        let mut bad = model.clone();
        let MixedLoop::Segments { segments } = &mut bad.revolved.mixed.planes[0].loops[0] else {
            unreachable!()
        };
        segments[0] = MixedSegment::EllipticArc {
            arc: 1,
            reversed: true,
        };
        assert_eq!(
            build_rounded_step(&bad).unwrap_err().code,
            "cad.invalid_model"
        );
        assert!(build_mixed_step(&model.revolved.mixed).is_err());
        assert!(build_rounded_step_with_octant_provenance(
            &model,
            &[1, 2, 3, 4, 5, 6],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[]
        )
        .is_err());
        for tokens in [&[][..], &[4][..], &[100_001][..]] {
            assert!(build_rounded_step_with_linear_blend_provenance(
                &model,
                &[1, 2, 3, 4, 5, 6],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                tokens
            )
            .is_err());
        }
    }

    #[test]
    fn trihedral_octant_reimports_with_all_ten_source_tokens() {
        let mut model = trihedral_model();
        for reverse in [false, true] {
            if reverse {
                // Same cylinder with its axis and arc parameters reversed.
                let mixed = &mut model.revolved.mixed;
                mixed.partial_cylinders[0].arcs.reverse();
                for arc in &mut mixed.arcs[..2] {
                    arc.vertices.reverse();
                    arc.axis = arc.axis.map(|v| -v);
                }
                let MixedLoop::Segments { segments } = &mut mixed.planes[1].loops[0] else {
                    unreachable!()
                };
                segments[0] = MixedSegment::Arc {
                    arc: 0,
                    reversed: false,
                };
                model.octants[0].arc_uses[1].1 = true;
            }
            let step = build_rounded_step_with_octant_provenance(
                &model,
                &[1, 2, 3, 4, 5, 6],
                &[],
                &[7, 8, 9],
                &[],
                &[],
                &[],
                &[],
                &[10],
            )
            .unwrap();
            assert_eq!(step.report.faces, 10);
            assert!((step.report.volume_mm3 - 3161.8524963483783).abs() < 1e-7);
            let result = result_faces(&step.bytes).unwrap();
            let labels: std::collections::BTreeSet<_> = result
                .faces
                .iter()
                .map(|f| f.source_face_token.unwrap())
                .collect();
            assert_eq!(labels, (1..=10).collect());
            assert!(result
                .faces
                .iter()
                .any(|f| f.source_face_token == Some(10) && f.family == ResultFaceFamily::Sphere));
            assert!(
                (build_rounded_step(&model).unwrap().report.volume_mm3 - step.report.volume_mm3)
                    .abs()
                    < 1e-8
            );
        }
    }

    #[test]
    fn trihedral_octant_rejects_bad_owners_or_geometry_before_export() {
        let base = trihedral_model();
        for mutate in [
            (|s: &mut MixedSphericalOctant| s.arc_uses[0] = s.arc_uses[1])
                as fn(&mut MixedSphericalOctant),
            |s| s.arc_uses[0] = (0, false),
            |s| s.arc_uses[0].1 = true,
            |s| s.arc_uses[0].0 = usize::MAX,
            |s| s.directions.swap(0, 1),
            |s| s.directions[1] = s.directions[0],
            |s| s.center[0] += 0.01,
            |s| s.radius_mm += 0.01,
            |s| s.inner = true,
        ] {
            let mut wrong = base.clone();
            mutate(&mut wrong.octants[0]);
            assert_eq!(
                build_rounded_step(&wrong).unwrap_err().code,
                "cad.invalid_model"
            );
        }
        assert!(build_rounded_step_with_chain_provenance(
            &base,
            &[1, 2, 3, 4, 5, 6],
            &[],
            &[7, 8, 9],
            &[],
            &[],
            &[],
            &[]
        )
        .is_err());
    }

    #[test]
    fn partial_cone_chain_reimports_with_exact_shared_arcs_and_provenance() {
        let model = partial_cone_chain();
        let step = build_rounded_step_with_chain_provenance(
            &model,
            &[31, 9, 17],
            &[],
            &[42, 7],
            &[],
            &[],
            &[],
            &[93],
        )
        .unwrap();
        assert_eq!(step.report.solids, 1);
        assert_eq!(step.report.faces, 6);
        assert_eq!(step.report.cylindrical_faces, 2);
        assert_eq!(step.report.conical_faces, 1);
        assert!((step.report.volume_mm3 - std::f64::consts::PI * 29. / 3.).abs() < 1e-7);
        let inspected = result_faces(&step.bytes).unwrap();
        let mut tokens = inspected
            .faces
            .iter()
            .map(|f| f.source_face_token.unwrap())
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        assert_eq!(tokens, vec![7, 9, 17, 31, 42, 93]);
        let plain = build_rounded_step(&model).unwrap();
        assert!((plain.report.volume_mm3 - step.report.volume_mm3).abs() < 1e-9);
    }

    #[test]
    fn partial_cone_chain_rejects_missing_overincident_or_incompatible_boundaries() {
        let mut absent = partial_cone_chain();
        absent.partial_cones.clear();
        let mut duplicate = partial_cone_chain();
        duplicate
            .partial_cones
            .push(duplicate.partial_cones[0].clone());
        let mut shifted = partial_cone_chain();
        shifted.revolved.mixed.arcs[2].center[0] += 0.01;
        let mut endpoint = partial_cone_chain();
        endpoint.revolved.mixed.vertices[4][0] += 0.01;
        let mut missing_cap = partial_cone_chain();
        missing_cap.revolved.mixed.planes.remove(0);
        let mut axis = partial_cone_chain();
        axis.revolved.mixed.arcs[2].axis = [0., 1., 0.];
        let mut sweep = partial_cone_chain();
        sweep.revolved.mixed.arcs[2].sweep_rad -= 0.1;
        for model in [
            absent,
            duplicate,
            shifted,
            endpoint,
            missing_cap,
            axis,
            sweep,
        ] {
            assert_eq!(
                build_rounded_step(&model).unwrap_err().code,
                "cad.invalid_model"
            );
        }
        let model = partial_cone_chain();
        assert_eq!(
            build_rounded_step_with_all_provenance(&model, &[1, 2, 3], &[], &[4, 5], &[], &[], &[])
                .unwrap_err()
                .code,
            "cad.invalid_model"
        );
        assert_eq!(
            build_rounded_step_with_chain_provenance(
                &model,
                &[1, 2, 3],
                &[],
                &[4, 5],
                &[],
                &[],
                &[],
                &[5]
            )
            .unwrap_err()
            .code,
            "cad.invalid_model"
        );
    }
    #[test]
    fn partial_cone_chain_accepts_decreasing_radius_rotation_and_concave_notch() {
        let expected = std::f64::consts::PI * 29. / 3.;
        let mut decreasing = partial_cone_chain();
        for (i, arc) in decreasing.revolved.mixed.arcs.iter_mut().enumerate() {
            arc.radius_mm = if i < 2 { 3. } else { 2. };
            decreasing.revolved.mixed.vertices[2 * i][0] = arc.radius_mm;
            decreasing.revolved.mixed.vertices[2 * i + 1][0] = -arc.radius_mm;
        }
        let mut rotated = partial_cone_chain();
        let rotate = |p: [f64; 3]| [p[2], p[0], p[1]];
        for p in &mut rotated.revolved.mixed.vertices {
            *p = rotate(*p);
        }
        for arc in &mut rotated.revolved.mixed.arcs {
            arc.center = rotate(arc.center);
            arc.axis = rotate(arc.axis);
        }
        for plane in &mut rotated.revolved.mixed.planes {
            plane.normal = rotate(plane.normal);
        }
        let mut inner = partial_cone_chain();
        let mixed = &mut inner.revolved.mixed;
        mixed.vertices = (0..4)
            .flat_map(|i| {
                let r = if i < 2 { 2. } else { 3. };
                [
                    [-5., 0., i as f64],
                    [-r, 0., i as f64],
                    [r, 0., i as f64],
                    [5., 0., i as f64],
                    [5., 8., i as f64],
                    [-5., 8., i as f64],
                ]
            })
            .collect();
        for (i, arc) in mixed.arcs.iter_mut().enumerate() {
            arc.vertices = [i * 6 + 2, i * 6 + 1];
        }
        for cylinder in &mut mixed.partial_cylinders {
            cylinder.inner = true;
        }
        inner.partial_cones[0].inner = true;
        let line = |a, b| MixedSegment::Line { vertices: [a, b] };
        mixed.planes = vec![
            MixedPlane {
                normal: [0., 0., -1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        line(0, 5),
                        line(5, 4),
                        line(4, 3),
                        line(3, 2),
                        MixedSegment::Arc {
                            arc: 0,
                            reversed: false,
                        },
                        line(1, 0),
                    ],
                }],
            },
            MixedPlane {
                normal: [0., 0., 1.],
                loops: vec![MixedLoop::Segments {
                    segments: vec![
                        line(18, 19),
                        MixedSegment::Arc {
                            arc: 3,
                            reversed: true,
                        },
                        line(20, 21),
                        line(21, 22),
                        line(22, 23),
                        line(23, 18),
                    ],
                }],
            },
        ];
        for level in 0..3 {
            for (a, b, normal) in [
                (0, 1, [0., -1., 0.]),
                (2, 3, [0., -1., 0.]),
                (3, 4, [1., 0., 0.]),
                (4, 5, [0., 1., 0.]),
                (5, 0, [-1., 0., 0.]),
            ] {
                let offset = level * 6;
                mixed.planes.push(MixedPlane {
                    normal,
                    loops: vec![MixedLoop::Polygon {
                        vertices: vec![offset + a, offset + b, offset + b + 6, offset + a + 6],
                    }],
                });
            }
        }
        for (model, volume) in [
            (decreasing, expected),
            (rotated, expected),
            (inner, 240. - expected),
        ] {
            let step = build_rounded_step(&model).unwrap();
            assert_eq!(step.report.solids, 1);
            assert_eq!(step.report.cylindrical_faces, 2);
            assert_eq!(step.report.conical_faces, 1);
            assert!((step.report.volume_mm3 - volume).abs() < 1e-7);
        }
    }
}

#[cfg(test)]
mod size_profile_tests {
    use super::*;
    use fabstep_core::PlanarFace;

    fn triangles(count: usize) -> PlanarModel {
        PlanarModel {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            faces: vec![
                PlanarFace {
                    normal: [0., 0., 1.],
                    loops: vec![vec![0, 1, 2]]
                };
                count
            ],
        }
    }

    #[test]
    fn source_profile_does_not_raise_result_faces_or_lose_historical_quads() {
        let mut model = triangles(MAX_RESULT_FACES + 1);
        assert_eq!(build_step(&model).unwrap_err().code, "cad.invalid_model");
        let source = prepare_planar_for(&model, PlanarRole::Source);
        if cfg!(feature = "large-stl") {
            assert_eq!(source.unwrap().normals.len(), MAX_RESULT_FACES + 1);
            model.faces[0].loops[0] = vec![0, 1, 2, 3];
            assert!(prepare_planar_for(&model, PlanarRole::Source).is_err());
        } else {
            assert!(source.is_err());
        }
        model.faces.truncate(6);
        model.faces[0].loops[0] = vec![0, 1, 2, 3];
        assert!(prepare_planar_for(&model, PlanarRole::Source).is_ok());
    }

    #[test]
    fn source_face_cap_is_checked_before_native_construction() {
        let model = triangles(MAX_SOURCE_TRIANGLES + 1);
        assert_eq!(
            validate_source(&model).unwrap_err().code,
            "cad.invalid_model"
        );
        assert_eq!(
            validate_sources_separated(&[model]).unwrap_err().code,
            "cad.source_document_too_large"
        );
    }
}
