//! Géométrie pure et déterministe de FabSTEP, en millimètres.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabError {
    pub code: String,
    pub message: String,
    /// Structured source context; `message` remains readable by older clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<ErrorBody>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub id: String,
    pub name: String,
    pub detail: String,
}
impl FabError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            body: None,
        }
    }

    pub fn with_body(mut self, id: &str, name: &str) -> Self {
        let detail = self.body.take().map_or(self.message, |body| body.detail);
        self.message = format!("{name} [{id}] : {detail}");
        self.body = Some(ErrorBody {
            id: id.to_owned(),
            name: name.to_owned(),
            detail,
        });
        self
    }
}
impl std::fmt::Display for FabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for FabError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputUnit {
    Mm,
    Inch,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructionMode {
    #[default]
    Planar,
    Cylinders,
    Primitives,
    Surfaces,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub input_unit: InputUnit,
    /// Euclidean distance in mm, independent of the input unit. The earliest
    /// qualifying representative is retained; welding is not transitive.
    pub weld_tolerance_mm: f64,
    /// Maximum distance to an immutable seed plane. Vertices are not projected.
    /// Region growth additionally requires almost parallel triangle normals.
    pub plane_tolerance_mm: f64,
    pub reconstruction: ReconstructionMode,
    pub cylinder_fit_tolerance_mm: f64,
    pub max_deviation_mm: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            input_unit: InputUnit::Mm,
            weld_tolerance_mm: 1e-6,
            plane_tolerance_mm: 1e-6,
            reconstruction: ReconstructionMode::Planar,
            cylinder_fit_tolerance_mm: 1e-5,
            max_deviation_mm: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanarFace {
    /// Contour extérieur en premier, puis trous. Sens par rapport à la normale sortante.
    pub loops: Vec<Vec<usize>>,
    pub normal: [f64; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanarModel {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<PlanarFace>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub input_triangles: usize,
    pub vertices: usize,
    pub planar_faces: usize,
    /// Number of triangle-corner occurrences reusing an existing representative.
    /// Includes exact duplicates inherently present in STL triangle soups.
    pub welded_vertices: usize,
    pub volume_mm3: f64,
    pub bounds_mm: [[f64; 3]; 2],
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub mesh: Mesh,
    pub model: PlanarModel,
    pub report: Report,
}

/// A profile in the orthonormal frame (x_axis, axis × x_axis).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProfileLoop {
    Polygon { points: Vec<[f64; 2]> },
    Circle { center: [f64; 2], radius_mm: f64 },
}

/// Complete straight extrusion; outer is counterclockwise and holes clockwise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticExtrusion {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub x_axis: [f64; 3],
    pub height_mm: f64,
    pub outer: ProfileLoop,
    pub holes: Vec<ProfileLoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionReport {
    pub cylinders: usize,
    #[serde(default)]
    pub cones: usize,
    #[serde(default)]
    pub spheres: usize,
    #[serde(default)]
    pub tori: usize,
    #[serde(default)]
    pub toroidal_fillets: usize,
    pub max_radial_residual_mm: f64,
    pub max_chord_error_mm: f64,
    /// Conservative wall/plane residual. Conical reconstruction measures source
    /// vertices against their associated ideal ruled facets; convex
    /// interpolation bounds the entire triangle by its maximum vertex error.
    pub max_wall_residual_mm: f64,
    /// Conservative sum of welding allowance, verified wall/plane residual,
    /// radial fitting residual and chord sagitta; not a global Hausdorff proof.
    pub max_deviation_bound_mm: f64,
    pub expected_volume_mm3: f64,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recognition {
    pub extrusion: AnalyticExtrusion,
    pub report: RecognitionReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularRing {
    pub center: [f64; 3],
    /// Shared cylinder axis, not the outward normal of an incident plane.
    pub axis: [f64; 3],
    pub radius_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MixedLoop {
    Polygon { vertices: Vec<usize> },
    Circle { ring: usize },
    Segments { segments: Vec<MixedSegment> },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularArc {
    pub center: [f64; 3],
    pub axis: [f64; 3],
    pub radius_mm: f64,
    pub sweep_rad: f64,
    pub vertices: [usize; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MixedSegment {
    Line { vertices: [usize; 2] },
    Arc { arc: usize, reversed: bool },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedPartialCylinder {
    pub arcs: [usize; 2],
    pub inner: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedPlane {
    pub normal: [f64; 3],
    /// Outer boundary first; circle winding follows role and plane normal.
    pub loops: Vec<MixedLoop>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedCylinder {
    /// Centers ordered positively along their common unit axis; equal radii.
    pub rings: [usize; 2],
    pub inner: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedModel {
    pub vertices: Vec<[f64; 3]>,
    pub rings: Vec<CircularRing>,
    pub planes: Vec<MixedPlane>,
    pub cylinders: Vec<MixedCylinder>,
    #[serde(default)]
    pub arcs: Vec<CircularArc>,
    #[serde(default)]
    pub partial_cylinders: Vec<MixedPartialCylinder>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MixedFaceRef {
    Plane { index: usize },
    Cylinder { index: usize },
    PartialCylinder { index: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedRecognition {
    pub model: MixedModel,
    pub report: RecognitionReport,
    /// Exact, exhaustive source triangle attribution; one entry per triangle.
    pub triangle_faces: Vec<MixedFaceRef>,
}

/// Complete conical side between two coaxial source boundaries. Centers are
/// ordered positively along their common unit axis and radii differ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedCone {
    pub rings: [usize; 2],
    pub inner: bool,
}

/// Explicit M4 representation. Positive-radius rings have exactly two face
/// incidences and may join a cylinder directly to a cone. A zero-radius ring is
/// permitted ONLY here: it represents a demonstrated source apex vertex, with
/// one cone incidence and no planar/cylindrical incidence. Legacy MixedModel
/// consumers must never receive this wrapper's mixed member on its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevolvedModel {
    pub mixed: MixedModel,
    pub cones: Vec<MixedCone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RevolvedFaceRef {
    Plane { index: usize },
    Cylinder { index: usize },
    Cone { index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevolvedRecognition {
    pub model: RevolvedModel,
    pub report: RecognitionReport,
    /// Exhaustive source-triangle attribution. No analytic patch may hide an
    /// unproved triangle or invent a closing disk/apex.
    pub triangle_faces: Vec<RevolvedFaceRef>,
}

/// Full-azimuth spherical patch, ordered by increasing latitude on the rings'
/// common axis. A zero-radius end denotes an actual source pole.
/// `None` denotes one complete exterior sphere, exclusively occupying its
/// RoundedModel: no artificial source boundaries, poles, or adjacent surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedSphere {
    pub rings: Option<[usize; 2]>,
    pub center: [f64; 3],
    pub radius_mm: f64,
    pub inner: bool,
}

/// Full-azimuth toroidal patch. Meridian angles are increasing, with
/// rho = major + minor*cos(v), z = minor*sin(v), relative to center.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedTorus {
    /// Existing patches retain their two demonstrated source rings. None is
    /// reserved for a complete exterior torus alone in its RoundedModel.
    pub rings: Option<[usize; 2]>,
    /// Required only for a complete torus. Patch axes remain defined by rings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[f64; 3]>,
    pub center: [f64; 3],
    pub major_radius_mm: f64,
    pub minor_radius_mm: f64,
    pub meridian_rad: [f64; 2],
    pub inner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundedModel {
    pub revolved: RevolvedModel,
    pub spheres: Vec<MixedSphere>,
    pub tori: Vec<MixedTorus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoundedFaceRef {
    Plane { index: usize },
    Cylinder { index: usize },
    PartialCylinder { index: usize },
    Cone { index: usize },
    Sphere { index: usize },
    Torus { index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundedRecognition {
    pub model: RoundedModel,
    pub report: RecognitionReport,
    pub triangle_faces: Vec<RoundedFaceRef>,
}

/// Isolated convex rounded cuboid: the Minkowski sum of an oriented box
/// with positive half extents and a sphere of one demonstrated radius.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundedBoxModel {
    pub vertices: Vec<[f64; 3]>,
    pub center: [f64; 3],
    /// Direct orthonormal basis. Outer half extents are core_half_extents + radius.
    pub axes: [[f64; 3]; 3],
    pub core_half_extents: [f64; 3],
    pub radius_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoundedBoxFaceRef {
    Plane {
        axis: usize,
        positive: bool,
    },
    /// Signs of the two transverse axes, in increasing axis-index order.
    Cylinder {
        axis: usize,
        positive: [bool; 2],
    },
    Sphere {
        positive: [bool; 3],
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundedBoxRecognition {
    pub model: RoundedBoxModel,
    pub report: RecognitionReport,
    pub triangle_faces: Vec<RoundedBoxFaceRef>,
}
mod rounded_box;
pub use rounded_box::recognize_rounded_box;

mod complete_sphere;
mod complete_torus;
mod math;
mod mixed;
mod partial;
mod pipeline;
mod planar;
mod recognition;
mod revolved;
mod rounded;
pub use complete_sphere::recognize_complete_sphere;
pub use complete_torus::recognize_complete_torus;
mod stl;
pub use mixed::recognize_mixed;
pub use pipeline::{
    analyze_mesh, analyze_mesh_bodies, analyze_stl, analyze_stl_bodies, read_stl_mesh,
};
pub use recognition::recognize_extrusion;
pub use revolved::recognize_revolved;
pub use rounded::recognize_rounded;
