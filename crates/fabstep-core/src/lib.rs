//! Géométrie pure et déterministe de FabSTEP, en millimètres.
use serde::{Deserialize, Serialize};
mod height_patch;
pub use height_patch::{recognize_height_patch, HeightPatch, HeightPatchRecognition};

/// Source-only storage bounds. The larger profile is compiled only into the
/// isolated STL helper; result tessellation, DXF and 3MF keep their own bounds.
#[cfg(not(feature = "large-stl"))]
pub const SOURCE_MAX_TRIANGLES: usize = 100_000;
#[cfg(feature = "large-stl")]
pub const SOURCE_MAX_TRIANGLES: usize = 600_000;
pub const SOURCE_MAX_VERTICES: usize = 3 * SOURCE_MAX_TRIANGLES;

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
    Auto,
    #[default]
    Planar,
    Cylinders,
    Primitives,
    Surfaces,
    HeightPatch,
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

/// Component ordering and source triangle identity shared by analysis and
/// selection. Components are ordered by their first source triangle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceComponentTopology {
    /// Strictly increasing source indices; entry i identifies analysis triangle i.
    pub analysis_to_source_triangles: Vec<usize>,
    /// Row i belongs to analysis_to_source_triangles[i]. Values are sorted
    /// SOURCE triangle indices (not analysis indices), exactly three per row.
    pub source_triangle_neighbors: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenancedAnalysis {
    pub analysis: Analysis,
    pub topology: SourceComponentTopology,
    /// Exact membership from the planar construction pass. Entry i identifies
    /// the constructed planar face containing analysis triangle i.
    pub planar_triangle_faces: Vec<usize>,
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
    #[serde(default)]
    pub revolutions: usize,
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
/// A trimmed planar ellipse, oriented by increasing parameter. Its minor
/// direction is axis × major_direction. Both endpoint ids refer to the mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EllipticArc {
    pub center: [f64; 3],
    pub axis: [f64; 3],
    pub major_direction: [f64; 3],
    pub major_mm: f64,
    pub minor_mm: f64,
    pub start_rad: f64,
    pub sweep_rad: f64,
    pub vertices: [usize; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MixedSegment {
    Line { vertices: [usize; 2] },
    Arc { arc: usize, reversed: bool },
    EllipticArc { arc: usize, reversed: bool },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedPartialCylinder {
    /// The near index addresses an ellipse for a cylinder trimmed by two oblique planes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub elliptic_start: bool,
    /// The far index addresses an
    /// elliptic arc when this flag is set, and a circular arc otherwise.
    /// Two adjacent fillets use the same elliptic index for their common edge.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub elliptic_end: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elliptic_arcs: Vec<EllipticArc>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct MixedPartialCone {
    /// Shared source arcs ordered positively along their common axis.
    pub arcs: [usize; 2],
    pub inner: bool,
}

/// An exterior spherical octant bounded by three positively oriented
/// great-circle arcs. Directions form a direct orthonormal basis; arc i
/// joins endpoint i to endpoint (i+1)%3. True reverses the stored arc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedSphericalOctant {
    pub center: [f64; 3],
    pub radius_mm: f64,
    pub directions: [[f64; 3]; 3],
    pub arc_uses: [(usize, bool); 3],
    pub inner: bool,
}

/// One exterior linear rolling blend. The two planar cap ellipse uses
/// are opposed by the cone, and rails run along its outward boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedLinearBlend {
    pub apex: [f64; 3],
    pub axis: [f64; 3],
    pub semi_angle_rad: f64,
    pub arcs: [usize; 2],
    pub rails: [[usize; 2]; 2],
}

/// One independently trimmed rolling cone, optionally paired with a second
/// one at the opposite corner. Old one-cone documents retain their shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearBlends {
    #[serde(flatten)]
    pub first: MixedLinearBlend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<MixedLinearBlend>,
}

impl LinearBlends {
    pub fn iter(&self) -> impl Iterator<Item = &MixedLinearBlend> {
        std::iter::once(&self.first).chain(self.second.iter())
    }
}

impl std::ops::Deref for LinearBlends {
    type Target = MixedLinearBlend;
    fn deref(&self) -> &Self::Target {
        &self.first
    }
}

impl std::ops::DerefMut for LinearBlends {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.first
    }
}

/// Centered orthogonal through bores. The right-handed frame uses the large
/// bore axis x and the small bore axis z. Radii are [small, large]. Rims are
/// the two small mouths in increasing z, then the two large mouths in x.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDrill {
    /// Small bore axis position along local X/Y, measured from the block center.
    #[serde(default)]
    pub small_offset: [f64; 2],
    /// Achieved source-witness residual, already charged to the deviation bound.
    pub witness_tolerance_mm: f64,
    pub center: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub half_lengths: [f64; 3],
    pub radii: [f64; 2],
    pub rims: [usize; 4],
    /// Each witnessed source cycle (z negative, positive) is ordered by
    /// increasing phase around z, without repeating the first vertex id.
    pub branches: [Vec<usize>; 2],
}

/// An additional positive Bernstein span, joined C1 to the first at a witnessed level.
/// The first control radius is the last radius in SmoothRevolution::radii_mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubicRevolutionSpan {
    pub start_fraction: f64,
    pub next_radii_mm: [f64; 3],
}

/// One full exterior turn of the fitted positive Bernstein radius r(t),
/// with axial distance height_mm*t. The two end circles are witnessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothRevolution {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub height_mm: f64,
    pub radii_mm: [f64; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_span: Option<CubicRevolutionSpan>,
    /// At a witnessed axial tip the first endpoint has no circle. Both ring
    /// indices then refer to the only cap, and this is the welded source vertex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pole_vertex: Option<usize>,
    pub rings: [usize; 2],
    pub witness_tolerance_mm: f64,
}

/// A closed annular wedge with an exterior quarter-round. The axis points from
/// the bottom to the top. Arc uses run around the toroidal rectangle starting
/// at (u=0,v=0), along +u, +v, -u, -v respectively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedTorusSector {
    pub center: [f64; 3],
    pub axis: [f64; 3],
    pub start_direction: [f64; 3],
    pub major_radius_mm: f64,
    pub minor_radius_mm: f64,
    pub height_mm: f64,
    pub inner_radius_mm: f64,
    pub sweep_rad: f64,
    pub arc_uses: [(usize, bool); 4],
    pub inner: bool,
}

/// Rectangular blind pocket with two concave quarter cylinders of distinct
/// radii. All lengths are measured along the local frame, in millimetres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnequalPocket {
    /// Intersection of the untrimmed pocket floor and its two tangent walls.
    pub origin: [f64; 3],
    /// Inward normals of the large and small fillet sides, then the floor.
    /// Both handednesses are valid for this ordered orthonormal frame.
    pub axes: [[f64; 3]; 3],
    pub outer_min: [f64; 3],
    pub outer_max: [f64; 3],
    /// Opposite pocket stops at positive local x and y.
    pub pocket_spans: [f64; 2],
    /// Small then large. Their source owners are PartialCylinder 0 and 1.
    pub radii: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundedModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unequal_pocket: Option<Box<UnequalPocket>>,
    pub revolved: RevolvedModel,
    pub spheres: Vec<MixedSphere>,
    pub tori: Vec<MixedTorus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub torus_sectors: Vec<MixedTorusSector>,
    #[serde(default)]
    pub partial_cones: Vec<MixedPartialCone>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub octants: Vec<MixedSphericalOctant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linear_blend: Option<Box<LinearBlends>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_drill: Option<Box<CrossDrill>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth_revolution: Option<Box<SmoothRevolution>>,
}

impl RoundedModel {
    /// Number of partial cylindrical faces addressable by the source owners.
    pub fn partial_cylinder_count(&self) -> usize {
        if self.unequal_pocket.is_some() {
            2
        } else {
            self.revolved.mixed.partial_cylinders.len()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoundedFaceRef {
    TorusSector {
        index: usize,
    },
    SmoothRevolution {
        index: usize,
    },
    /// 0 and 1 are the lower and upper small walls; 2 is the large wall.
    CrossDrill {
        index: usize,
    },
    LinearBlend {
        index: usize,
    },
    SphericalOctant {
        index: usize,
    },
    PartialCone {
        index: usize,
    },
    Plane {
        index: usize,
    },
    Cylinder {
        index: usize,
    },
    PartialCylinder {
        index: usize,
    },
    Cone {
        index: usize,
    },
    Sphere {
        index: usize,
    },
    Torus {
        index: usize,
    },
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
mod cross_drill;
mod math;
mod mesh_section;
mod oblique_pocket;
pub use mesh_section::{section_meshes, section_meshes_controlled};
mod mixed;
mod partial;
mod pipeline;
mod planar;
mod profile;
pub use profile::{
    export_planar_face_dxf, export_section_dxf, validate_planar_profile, validate_profile_frame,
    validate_section_profile, validate_section_request, PlanarFaceProfile, PlanarProfileLoop,
    ProfileFrame, ProfileSegment, SectionBodyProfile, SectionLoop, SectionProfile, SectionRequest,
};
mod recognition;
mod revolved;
mod rounded;
pub use complete_sphere::recognize_complete_sphere;
pub use complete_torus::recognize_complete_torus;
mod automatic;
mod guided_plane;
pub use automatic::{
    automatic_region, guided_rounded_region, recognize_automatic, recognize_automatic_guided,
};
mod stl;
pub use mixed::{
    recognize_guided_cylinder, recognize_guided_cylinders, recognize_guided_surfaces,
    recognize_mixed,
};

/// Explicit family hypothesis for one selected source wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceFamily {
    Auto,
    Sphere,
    Torus,
    Plane,
    Cylinder,
    Cone,
}
pub use pipeline::{
    analyze_mesh, analyze_mesh_bodies, analyze_mesh_bodies_with_provenance, analyze_stl,
    analyze_stl_bodies, analyze_stl_bodies_with_provenance, prepare_mesh_bodies_for_sections,
    prepare_mesh_bodies_for_sections_controlled, read_stl_mesh, selection_mesh_topology,
    PreparedMesh,
};
pub use recognition::recognize_extrusion;
pub use revolved::recognize_revolved;
pub use rounded::recognize_rounded;
