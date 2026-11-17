//! Local, complete cylindrical patches and explicitly guided conical walls.
//! No feature is inferred without exact source-triangle coverage and two real
//! planar boundary rings.
use crate::math::*;
use crate::recognition::{
    distance_segment, fit_circle, match_loops, signed_area, validate_analysis, Work,
};
use crate::{
    Analysis, CircularRing, Diagnostic, FabError, GuidanceFamily, MixedCone, MixedCylinder,
    MixedFaceRef, MixedLoop, MixedModel, MixedPlane, MixedRecognition, Options, RecognitionReport,
    ReconstructionMode, RevolvedFaceRef, RevolvedModel, RevolvedRecognition, Severity,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::PI;

type P2 = [f64; 2];
type Edge = (usize, usize);
fn key(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn unsupported() -> FabError {
    FabError::new("recognition.unsupported",
    "Une paroi circulaire complète entre deux contours plans réels n'a pas pu être prouvée. Fond manquant, cône, intersection ou détail intermédiaire : utiliser le mode planaire.")
}

struct Rim {
    region: usize,
    loop_index: usize,
    vertices: Vec<usize>,
    center: V,
    normal: V,
    radius: f64,
}
struct Patch {
    rims: [usize; 2],
    triangles: BTreeSet<usize>,
    rings: [CircularRing; 2],
    inner: bool,
    radial: f64,
    chord: f64,
    wall: f64,
    conic: bool,
}
struct Frame {
    origin: V,
    axis: V,
    x: V,
    y: V,
}
impl Frame {
    fn new(origin: V, axis: V) -> Self {
        let least = (0..3)
            .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
            .unwrap();
        let mut helper = [0.; 3];
        helper[least] = 1.;
        let x = cross(helper, axis);
        let x = scale(x, 1. / norm(x));
        Self {
            origin,
            axis,
            x,
            y: cross(axis, x),
        }
    }
    fn project(&self, p: V) -> (P2, f64) {
        let p = sub(p, self.origin);
        ([dot(p, self.x), dot(p, self.y)], dot(p, self.axis))
    }
    fn lift(&self, p: P2) -> V {
        std::array::from_fn(|i| self.origin[i] + self.x[i] * p[0] + self.y[i] * p[1])
    }
}

/// Replaces only proven complete cylindrical patches. Every detected circular
/// rim must have exactly one valid partner; all other source faces remain planar.
/// Closed-source and final BRep validation by the CAD backend is mandatory.
pub fn recognize_mixed(
    analysis: &Analysis,
    options: &Options,
) -> Result<MixedRecognition, FabError> {
    if options.reconstruction != ReconstructionMode::Cylinders {
        return Err(FabError::new(
            "recognition.disabled",
            "Activer explicitement le mode cylindres.",
        ));
    }
    recognize_scoped(analysis, options, None).map(only_cylinders)
}

/// Reconstructs one complete cylinder containing every selected source triangle.
/// The selection may seed only part of the wall; the same proof as automatic
/// recognition must cover the whole wall and both real planar boundaries.
/// Every other region remains planar/faceted, including other circular walls.
/// As in automatic mixed recognition, segmentation uses max(plane, fit).
/// Retained planes can regroup facets; their residual is charged to the bound.
/// Indices belong to `analysis.mesh`, never directly to an imported preview.
/// This explicit API authorizes the hypothesis independently of the global mode.
/// Native source, result and complete-document validation remain mandatory.
pub fn recognize_guided_cylinder(
    analysis: &Analysis,
    options: &Options,
    triangle_ids: &[usize],
) -> Result<MixedRecognition, FabError> {
    if triangle_ids.is_empty()
        || triangle_ids.len() > analysis.mesh.triangles.len()
        || triangle_ids.len() > crate::stl::MAX_TRIANGLES
    {
        return Err(invalid_selection());
    }
    recognize_guided_cylinders(analysis, options, &[triangle_ids.to_vec()])
}

/// Bounded independent indications, proved in one shared recognition pass.
pub fn recognize_guided_cylinders(
    analysis: &Analysis,
    options: &Options,
    triangle_sets: &[Vec<usize>],
) -> Result<MixedRecognition, FabError> {
    if triangle_sets.is_empty() || triangle_sets.len() > 16 {
        return Err(FabError::new(
            "guidance.limit",
            "Au plus 16 indications cylindriques sont admises.",
        ));
    }
    let count = analysis.mesh.triangles.len();
    let mut total = 0usize;
    let mut selections = Vec::with_capacity(triangle_sets.len());
    for triangle_ids in triangle_sets {
        total = total.saturating_add(triangle_ids.len());
        if triangle_ids.is_empty() || total > count || total > crate::stl::MAX_TRIANGLES {
            return Err(invalid_selection());
        }
        let mut selected = BTreeSet::new();
        for &triangle in triangle_ids {
            if triangle >= count || !selected.insert(triangle) {
                return Err(invalid_selection());
            }
        }
        selections.push((GuidanceFamily::Cylinder, selected));
    }
    let mut result = recognize_scoped(analysis, options, Some(&selections)).map(only_cylinders).map_err(|error| {
        if error.code == "recognition.unsupported" {
            FabError::new("guidance.cylinder_unproven", "Chaque indication doit appartenir à une même paroi cylindrique complète prouvée entre deux contours plans source.")
        } else { error }
    })?;
    result.report.diagnostics.push(Diagnostic {
        code: "guidance.complete_cylinder_verified".into(),
        severity: Severity::Info,
        message: "Chaque indication désigne une paroi cylindrique complète vérifiée ; les autres zones restent facettisées, avec leur résidu inclus dans la borne.".into(),
        count: selections.len(),
    });
    Ok(result)
}

#[derive(Clone)]
enum ScopedFaceRef {
    Mixed(MixedFaceRef),
    Cone { index: usize },
}
struct ScopedRecognition {
    model: RevolvedModel,
    report: RecognitionReport,
    triangle_faces: Vec<ScopedFaceRef>,
}
fn only_cylinders(result: ScopedRecognition) -> MixedRecognition {
    MixedRecognition {
        model: result.model.mixed,
        report: result.report,
        triangle_faces: result
            .triangle_faces
            .into_iter()
            .map(|owner| match owner {
                ScopedFaceRef::Mixed(owner) => owner,
                ScopedFaceRef::Cone { .. } => unreachable!("cylinder-only scoped recognition"),
            })
            .collect(),
    }
}
fn as_revolved(result: ScopedRecognition) -> RevolvedRecognition {
    RevolvedRecognition {
        model: result.model,
        report: result.report,
        triangle_faces: result
            .triangle_faces
            .into_iter()
            .map(|owner| match owner {
                ScopedFaceRef::Mixed(MixedFaceRef::Plane { index }) => {
                    RevolvedFaceRef::Plane { index }
                }
                ScopedFaceRef::Mixed(MixedFaceRef::Cylinder { index }) => {
                    RevolvedFaceRef::Cylinder { index }
                }
                ScopedFaceRef::Cone { index } => RevolvedFaceRef::Cone { index },
                ScopedFaceRef::Mixed(MixedFaceRef::PartialCylinder { .. }) => {
                    unreachable!("guided walls are complete")
                }
            })
            .collect(),
    }
}

/// Proves each explicit family on one complete, independently planar-bounded
/// wall. Only designated walls are reconstructed. All indications share one
/// segmentation and work budget; native document validation remains mandatory.
/// Cone tips and direct curved-surface junctions are deliberately excluded.
pub fn recognize_guided_surfaces(
    analysis: &Analysis,
    options: &Options,
    triangle_sets: &[(GuidanceFamily, Vec<usize>)],
) -> Result<RevolvedRecognition, FabError> {
    if triangle_sets
        .iter()
        .any(|(family, _)| *family == GuidanceFamily::Plane)
    {
        return crate::guided_plane::recognize(analysis, options, triangle_sets);
    }
    if triangle_sets.is_empty() || triangle_sets.len() > 16 {
        return Err(FabError::new(
            "guidance.limit",
            "Au plus 16 indications sont admises.",
        ));
    }
    let count = analysis.mesh.triangles.len();
    let mut total = 0usize;
    let mut selections = Vec::with_capacity(triangle_sets.len());
    for (family, triangle_ids) in triangle_sets {
        total = total.saturating_add(triangle_ids.len());
        if triangle_ids.is_empty() || total > count || total > crate::stl::MAX_TRIANGLES {
            return Err(invalid_selection());
        }
        let mut selected = BTreeSet::new();
        for &triangle in triangle_ids {
            if triangle >= count || !selected.insert(triangle) {
                return Err(invalid_selection());
            }
        }
        selections.push((*family, selected));
    }
    let has_cones = selections
        .iter()
        .any(|(family, _)| *family == GuidanceFamily::Cone);
    let mut result = recognize_scoped(analysis, options, Some(&selections)).map_err(|error| {
        if error.code == "recognition.unsupported" {
            if has_cones { FabError::new("guidance.cone_unproven", "Chaque indication doit appartenir à une seule paroi de la famille demandée, prouvée entre deux contours plans source. Les pointes et raccords courbes directs ne sont pas pris en charge.") }
            else { FabError::new("guidance.cylinder_unproven", "Chaque indication doit appartenir à une même paroi cylindrique complète prouvée entre deux contours plans source.") }
        } else { error }
    })?;
    for family in [GuidanceFamily::Cylinder, GuidanceFamily::Cone] {
        let count = selections.iter().filter(|(f, _)| *f == family).count();
        if count > 0 {
            result.report.diagnostics.push(Diagnostic {
            code: if family == GuidanceFamily::Cylinder { "guidance.complete_cylinder_verified" } else { "guidance.complete_cone_verified" }.into(),
            severity: Severity::Info,
            message: if family == GuidanceFamily::Cylinder { "Chaque indication désigne une paroi cylindrique complète vérifiée ; les autres zones restent facettisées, avec leur résidu inclus dans la borne." }
                else { "Chaque indication conique désigne un tronc de cône complet entre deux contours plans source vérifiés ; les autres zones restent facettisées." }.into(),
            count,
        });
        }
    }
    Ok(as_revolved(result))
}

fn invalid_selection() -> FabError {
    FabError::new(
        "guidance.invalid_selection",
        "La sélection doit contenir des triangles source distincts, présents dans le corps analysé.",
    )
}

fn recognize_scoped(
    analysis: &Analysis,
    options: &Options,
    selected: Option<&[(GuidanceFamily, BTreeSet<usize>)]>,
) -> Result<ScopedRecognition, FabError> {
    for t in [
        options.weld_tolerance_mm,
        options.plane_tolerance_mm,
        options.cylinder_fit_tolerance_mm,
        options.max_deviation_mm,
    ] {
        if !t.is_finite() || t <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    validate_analysis(analysis)?;
    let mesh = &analysis.mesh;
    let adjacency = crate::pipeline::validate_topology(mesh)?;
    let tolerance = options
        .plane_tolerance_mm
        .max(options.cylinder_fit_tolerance_mm);
    let segmentation = crate::planar::build_with_provenance(mesh, &adjacency, tolerance)?;
    let mut work = Work::default();
    let mut rims = Vec::new();
    for (region, face) in segmentation.model.faces.iter().enumerate() {
        let normal = scale(face.normal, 1. / norm(face.normal));
        let frame = Frame::new(mesh.vertices[face.loops[0][0]], normal);
        for (loop_index, vertices) in face.loops.iter().enumerate() {
            work.spend(vertices.len())?;
            let points: Vec<P2> = vertices
                .iter()
                .map(|&v| frame.project(mesh.vertices[v]).0)
                .collect();
            if let Some(fit) = fit_circle(&points, options.cylinder_fit_tolerance_mm) {
                rims.push(Rim {
                    region,
                    loop_index,
                    vertices: vertices.clone(),
                    center: frame.lift(fit.center),
                    normal,
                    radius: fit.radius,
                });
            }
        }
    }
    if rims.len() == 1 {
        return Err(unsupported());
    }
    let mut edge_triangles: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    for (i, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        for (a, b) in [(a, b), (b, c), (c, a)] {
            edge_triangles.entry(key(a, b)).or_default().push(i);
        }
    }
    let mut patches = Vec::new();
    let mut matched = vec![0_usize; rims.len()];
    let mut selection_matches =
        vec![0usize; selected.map_or(0, <[(GuidanceFamily, BTreeSet<usize>)]>::len)];
    let cylinder_needed = selected.is_none_or(|selections| {
        selections
            .iter()
            .any(|(family, _)| *family == GuidanceFamily::Cylinder)
    });
    let cone_needed = selected.is_some_and(|selections| {
        selections
            .iter()
            .any(|(family, _)| *family == GuidanceFamily::Cone)
    });
    for i in 0..rims.len() {
        for j in i + 1..rims.len() {
            work.spend(1)?;
            let patch = if cylinder_needed {
                prove_patch(
                    analysis,
                    &segmentation,
                    &adjacency,
                    &edge_triangles,
                    &rims,
                    [i, j],
                    options,
                    &mut work,
                )?
            } else {
                None
            };
            let patch = if patch.is_none() && cone_needed {
                prove_cone_patch(
                    analysis,
                    &segmentation,
                    &adjacency,
                    &edge_triangles,
                    &rims,
                    [i, j],
                    options,
                    &mut work,
                )?
            } else {
                patch
            };
            if let Some(patch) = patch {
                if let Some(selections) = selected {
                    let mut matching = Vec::new();
                    for (index, (family, selection)) in selections.iter().enumerate() {
                        work.spend(selection.len())?;
                        if (*family == GuidanceFamily::Cone) == patch.conic
                            && selection.is_subset(&patch.triangles)
                        {
                            matching.push(index);
                        }
                    }
                    if matching.is_empty() {
                        continue;
                    }
                    if matching.len() > 1 {
                        return Err(FabError::new(
                            "guidance.overlapping_indications",
                            "Plusieurs indications désignent la même paroi.",
                        ));
                    }
                    selection_matches[matching[0]] += 1;
                    if selection_matches[matching[0]] > 1 {
                        return Err(FabError::new(
                            if patch.conic {
                                "guidance.ambiguous_cone"
                            } else {
                                "guidance.ambiguous_cylinder"
                            },
                            "Plusieurs parois prouvées contiennent la même indication.",
                        ));
                    }
                }
                matched[i] += 1;
                matched[j] += 1;
                patches.push(patch);
            }
        }
    }
    // Each indication requires exactly one match; every accepted wall is complete.
    if selected.is_some() && selection_matches.contains(&0)
        || selected.is_none() && matched.iter().any(|&count| count != 1)
    {
        return Err(unsupported());
    }
    let mut owners: Vec<Option<ScopedFaceRef>> = vec![None; mesh.triangles.len()];
    let mut consumed_regions = BTreeSet::new();
    let mut model = MixedModel {
        vertices: mesh.vertices.clone(),
        rings: Vec::new(),
        planes: Vec::new(),
        cylinders: Vec::new(),
        arcs: Vec::new(),
        partial_cylinders: Vec::new(),
    };
    let mut cones = Vec::new();
    let mut replacements = BTreeMap::new();
    let (mut radial, mut chord, mut wall) = (0_f64, 0_f64, 0_f64);
    for patch in patches {
        let first_ring = model.rings.len();
        model.rings.extend(patch.rings);
        let owner = if patch.conic {
            let index = cones.len();
            cones.push(MixedCone {
                rings: [first_ring, first_ring + 1],
                inner: patch.inner,
            });
            ScopedFaceRef::Cone { index }
        } else {
            let index = model.cylinders.len();
            model.cylinders.push(MixedCylinder {
                rings: [first_ring, first_ring + 1],
                inner: patch.inner,
            });
            ScopedFaceRef::Mixed(MixedFaceRef::Cylinder { index })
        };
        for (end, &rim) in patch.rims.iter().enumerate() {
            if replacements
                .insert((rims[rim].region, rims[rim].loop_index), first_ring + end)
                .is_some()
            {
                return Err(unsupported());
            }
        }
        for &triangle in &patch.triangles {
            if owners[triangle].is_some() {
                return Err(if selected.is_some() {
                    FabError::new(
                        "guidance.overlapping_indications",
                        "Les parois des indications se recouvrent.",
                    )
                } else {
                    unsupported()
                });
            }
            owners[triangle] = Some(owner.clone());
            consumed_regions.insert(segmentation.triangle_regions[triangle]);
        }
        radial = radial.max(patch.radial);
        chord = chord.max(patch.chord);
        wall = wall.max(patch.wall);
    }
    let mut plane_mapping = BTreeMap::new();
    for (region, face) in segmentation.model.faces.iter().enumerate() {
        if consumed_regions.contains(&region) {
            continue;
        }
        let index = model.planes.len();
        plane_mapping.insert(region, index);
        model.planes.push(MixedPlane {
            normal: face.normal,
            loops: face
                .loops
                .iter()
                .enumerate()
                .map(|(lp, vertices)| {
                    if let Some(&ring) = replacements.get(&(region, lp)) {
                        MixedLoop::Circle { ring }
                    } else {
                        MixedLoop::Polygon {
                            vertices: vertices.clone(),
                        }
                    }
                })
                .collect(),
        });
    }
    for (triangle, owner) in owners.iter_mut().enumerate() {
        if owner.is_none() {
            let Some(&index) = plane_mapping.get(&segmentation.triangle_regions[triangle]) else {
                return Err(unsupported());
            };
            *owner = Some(ScopedFaceRef::Mixed(MixedFaceRef::Plane { index }));
        }
    }

    if selected.is_none() {
        let mut mixed_owners = owners
            .into_iter()
            .map(|owner| {
                owner.map(|owner| match owner {
                    ScopedFaceRef::Mixed(owner) => owner,
                    ScopedFaceRef::Cone { .. } => unreachable!(),
                })
            })
            .collect::<Vec<_>>();
        let partial =
            crate::partial::augment(analysis, &mut model, &mut mixed_owners, options, &mut work)?;
        owners = mixed_owners
            .into_iter()
            .map(|owner| owner.map(ScopedFaceRef::Mixed))
            .collect();
        radial = radial.max(partial.radial);
        chord = chord.max(partial.chord);
        wall = wall.max(partial.wall);
    }
    if model.cylinders.is_empty() && model.partial_cylinders.is_empty() && cones.is_empty() {
        return Err(unsupported());
    }
    // Retained planes may represent an approximately coplanar source region.
    // Include every source vertex's distance to the actual output plane, even
    // when OCCT's own tighter acceptance tolerance will later reject it.
    let mut planar_residual: f64 = 0.;
    for (triangle, owner) in owners.iter().enumerate() {
        if let Some(ScopedFaceRef::Mixed(MixedFaceRef::Plane { index })) = owner {
            let plane = &model.planes[*index];
            let normal = scale(plane.normal, 1. / norm(plane.normal));
            let origin = crate::partial::loop_origin(&model, &plane.loops[0]);
            for &v in &mesh.triangles[triangle] {
                planar_residual =
                    planar_residual.max(dot(sub(mesh.vertices[v], origin), normal).abs());
            }
        }
    }
    wall += planar_residual;
    // Every guided ring belongs to one real plane and one complete wall.
    let mut ring_uses = vec![0; model.rings.len()];
    for plane in &model.planes {
        for lp in &plane.loops {
            if let MixedLoop::Circle { ring } = lp {
                ring_uses[*ring] += 1;
            }
        }
    }
    if ring_uses.iter().any(|&uses| uses != 1) {
        return Err(unsupported());
    }
    let revolved_model = RevolvedModel {
        mixed: model,
        cones,
    };
    let expected_volume = if revolved_model.cones.is_empty() {
        volume(&revolved_model.mixed)?
    } else {
        crate::revolved::volume(&revolved_model)?
    };
    let model = &revolved_model.mixed;
    let deviation = options.weld_tolerance_mm + wall + radial + chord;
    if !deviation.is_finite() || deviation > options.max_deviation_mm {
        return Err(FabError::new("recognition.deviation_exceeded",format!(
        "Borne d'écart {:.6} mm supérieure au budget {:.6} mm. Augmenter explicitement le budget ou utiliser le mode planaire.",deviation,options.max_deviation_mm)));
    }
    let cylinders = model.cylinders.len() + model.partial_cylinders.len();
    let planes = model.planes.len();
    let cones = revolved_model.cones.len();
    Ok(ScopedRecognition{model:revolved_model,triangle_faces:owners.into_iter().map(Option::unwrap).collect(),
        report:RecognitionReport{cylinders,cones,spheres:0,tori:0,toroidal_fillets:0,max_radial_residual_mm:radial,max_chord_error_mm:chord,max_wall_residual_mm:wall,
            max_deviation_bound_mm:deviation,expected_volume_mm3:expected_volume,diagnostics:vec![
                Diagnostic{code:if cones == 0 { "recognition.circular_intent_assumed" } else { "recognition.conic_intent_assumed" }.into(),severity:Severity::Warning,count:cylinders + cones,
                    message:"Les profils polygonaux circulaires sont interprétés comme cercles dans le mode explicitement activé.".into()},
                Diagnostic{code:"recognition.mixed_coverage".into(),severity:Severity::Info,count:analysis.mesh.triangles.len(),
                    message:if cones == 0 { format!("Tous les triangles sont attribués à une face : {cylinders} parois cylindriques prouvées et {planes} faces planes conservées. Aucun fond ajouté.") } else { format!("Tous les triangles sont attribués : {cones} cônes, {cylinders} cylindres et {planes} faces planes conservées. Aucun fond ajouté.") }},
                Diagnostic{code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,
                    message:"Borne locale composée de soudage, ajustement, correspondance et flèche des cordes ; pas une mesure Hausdorff globale. Validation native source et résultat requise.".into()}]}})
}

#[allow(clippy::too_many_arguments)]
fn prove_patch(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adjacency: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    rims: &[Rim],
    pair: [usize; 2],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Patch>, FabError> {
    let (lower, upper) = (&rims[pair[0]], &rims[pair[1]]);
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    if lower.region == upper.region
        || (lower.radius - upper.radius).abs() > o.cylinder_fit_tolerance_mm
        || dot(lower.normal, upper.normal).abs() < 1. - 1e-10
    {
        return Ok(None);
    }
    let delta = sub(upper.center, lower.center);
    let signed_height = dot(delta, lower.normal);
    if signed_height.abs() <= tol * 4. {
        return Ok(None);
    }
    let axis = scale(lower.normal, signed_height.signum());
    let height = signed_height.abs();
    let lateral_offset = norm(sub(delta, scale(axis, height)));
    if lateral_offset > tol {
        return Ok(None);
    }
    let radius = (lower.radius + upper.radius) * 0.5;
    let frame = Frame::new(lower.center, axis);
    work.spend(a.mesh.vertices.len())?;
    let projected: Vec<(P2, f64)> = a.mesh.vertices.iter().map(|&p| frame.project(p)).collect();
    let mut base: Vec<P2> = lower.vertices.iter().map(|&v| projected[v].0).collect();
    let mut other: Vec<P2> = upper.vertices.iter().map(|&v| projected[v].0).collect();
    if signed_area(&base) < 0. {
        base.reverse();
    }
    if signed_area(&other) < 0. {
        other.reverse();
    }
    let Some(correspondence) = match_loops(
        std::slice::from_ref(&base),
        std::slice::from_ref(&other),
        tol,
        work,
    )?
    else {
        return Ok(None);
    };
    let mut boundaries = BTreeSet::new();
    let mut starts = Vec::new();
    for (which, rim) in [lower, upper].into_iter().enumerate() {
        for i in 0..rim.vertices.len() {
            let edge = key(rim.vertices[i], rim.vertices[(i + 1) % rim.vertices.len()]);
            boundaries.insert(edge);
            let incident = &edges[&edge];
            let candidates: Vec<_> = incident
                .iter()
                .copied()
                .filter(|&t| s.triangle_regions[t] != rim.region)
                .collect();
            if candidates.len() != 1 {
                return Ok(None);
            }
            if which == 0 {
                starts.push(candidates[0]);
            }
        }
    }
    let first = starts[0];
    let triangle = a.mesh.triangles[first];
    let [p, q, r] = triangle.map(|v| a.mesh.vertices[v]);
    let tri_normal = cross(sub(q, p), sub(r, p));
    let mid = std::array::from_fn(|i| {
        (p[i] - lower.center[i] + q[i] - lower.center[i] + r[i] - lower.center[i]) / 3.
    });
    let radial_mid = sub(mid, scale(axis, dot(mid, axis)));
    let sense = dot(tri_normal, radial_mid);
    if !sense.is_finite() || sense == 0. {
        return Ok(None);
    }
    let inner = sense < 0.;
    let mut selected = BTreeSet::new();
    let mut examined = BTreeSet::new();
    let mut queue = VecDeque::from([first]);
    let mut wall: f64 = 0.;
    while let Some(t) = queue.pop_front() {
        if !examined.insert(t) {
            continue;
        }
        if s.triangle_regions[t] == lower.region || s.triangle_regions[t] == upper.region {
            continue;
        }
        let ids = a.mesh.triangles[t];
        let positions = ids.map(|v| projected[v]);
        if positions
            .iter()
            .any(|(_, z)| *z < -tol || *z > height + tol)
        {
            continue;
        }
        let [p, q, r] = ids.map(|v| a.mesh.vertices[v]);
        let normal = cross(sub(q, p), sub(r, p));
        let mut best = f64::INFINITY;
        for i in 0..base.len() {
            work.spend(1)?;
            let start = base[i];
            let end = base[(i + 1) % base.len()];
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let outward: V = std::array::from_fn(|j| {
                (frame.x[j] * dy - frame.y[j] * dx) * if inner { -1. } else { 1. }
            });
            if dot(normal, outward) <= 0. {
                continue;
            }
            let error = positions
                .iter()
                .map(|(point, z)| {
                    distance_segment(*point, start, end).hypot((-*z).max(*z - height).max(0.))
                })
                .fold(0., f64::max);
            best = best.min(error);
        }
        if best > tol {
            continue;
        }
        wall = wall.max(best);
        selected.insert(t);
        queue.extend(adjacency[t].iter().copied());
    }
    if selected.is_empty() || starts.iter().any(|t| !selected.contains(t)) {
        return Ok(None);
    }
    let mut actual_boundaries = BTreeSet::new();
    for &t in &selected {
        let [a, b, c] = a.mesh.triangles[t];
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let edge = key(u, v);
            if edges[&edge].iter().any(|other| !selected.contains(other)) {
                actual_boundaries.insert(edge);
            }
        }
    }
    if actual_boundaries != boundaries {
        return Ok(None);
    }
    // A retained planar region must never lose only some of its triangles.
    let consumed: BTreeSet<usize> = selected.iter().map(|&t| s.triangle_regions[t]).collect();
    for (t, &region) in s.triangle_regions.iter().enumerate() {
        work.spend(1)?;
        if consumed.contains(&region) && !selected.contains(&t) {
            return Ok(None);
        }
    }
    let mut radial: f64 = 0.;
    let mut cap: f64 = 0.;
    for (rim, end) in [(lower, 0.), (upper, height)] {
        for &v in &rim.vertices {
            let (p, z) = projected[v];
            radial = radial.max((p[0].hypot(p[1]) - radius).abs());
            cap = cap.max((z - end).abs());
        }
    }
    if radial > o.cylinder_fit_tolerance_mm || cap > tol {
        return Ok(None);
    }
    let mut max_angle: f64 = 0.;
    for i in 0..base.len() {
        let p = base[i];
        let q = base[(i + 1) % base.len()];
        let angle = (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
        if angle <= 0. || angle > PI / 4. + 1e-12 {
            return Ok(None);
        }
        max_angle = max_angle.max(angle);
    }
    let chord = 2. * radius * (max_angle / 4.).sin().powi(2);
    // Circle/plane alignment error, including a slight relative plane tilt.
    let tilt = norm(cross(axis, upper.normal)) * radius;
    wall += cap + correspondence + lateral_offset + tilt;
    let end = std::array::from_fn(|i| lower.center[i] + axis[i] * height);
    Ok(Some(Patch {
        rims: pair,
        conic: false,
        triangles: selected,
        inner,
        radial,
        chord,
        wall,
        rings: [
            CircularRing {
                center: lower.center,
                axis,
                radius_mm: radius,
            },
            CircularRing {
                center: end,
                axis,
                radius_mm: radius,
            },
        ],
    }))
}

#[allow(clippy::too_many_arguments)]
fn prove_cone_patch(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adjacency: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    rims: &[Rim],
    pair: [usize; 2],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Patch>, FabError> {
    let (lower, upper) = (&rims[pair[0]], &rims[pair[1]]);
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let delta = sub(upper.center, lower.center);
    let h = dot(delta, lower.normal);
    if lower.region == upper.region
        || h.abs() <= 4. * tol
        || (lower.radius - upper.radius).abs() <= 2. * o.cylinder_fit_tolerance_mm
        || dot(lower.normal, upper.normal).abs() < 1. - 1e-10
        || norm(sub(delta, scale(lower.normal, h))) > tol
    {
        return Ok(None);
    }
    let mut boundaries = BTreeSet::new();
    let mut starts = Vec::new();
    for rim in [lower, upper] {
        for i in 0..rim.vertices.len() {
            work.spend(1)?;
            let edge = key(rim.vertices[i], rim.vertices[(i + 1) % rim.vertices.len()]);
            boundaries.insert(edge);
            let other = edges[&edge]
                .iter()
                .copied()
                .filter(|&t| s.triangle_regions[t] != rim.region)
                .collect::<Vec<_>>();
            if other.len() != 1 {
                return Ok(None);
            }
            starts.push(other[0]);
        }
    }
    let mut triangles = BTreeSet::new();
    let mut pending = VecDeque::from([starts[0]]);
    while let Some(t) = pending.pop_front() {
        work.spend(1)?;
        if s.triangle_regions[t] == lower.region
            || s.triangle_regions[t] == upper.region
            || !triangles.insert(t)
        {
            continue;
        }
        pending.extend(adjacency[t].iter().copied());
    }
    if starts.iter().any(|t| !triangles.contains(t)) {
        return Ok(None);
    }
    let mut actual = BTreeSet::new();
    for &t in &triangles {
        work.spend(3)?;
        let [a, b, c] = a.mesh.triangles[t];
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let edge = key(u, v);
            if edges[&edge].iter().any(|other| !triangles.contains(other)) {
                actual.insert(edge);
            }
        }
    }
    if actual != boundaries {
        return Ok(None);
    }
    let consumed = triangles
        .iter()
        .map(|&t| s.triangle_regions[t])
        .collect::<BTreeSet<_>>();
    for (t, region) in s.triangle_regions.iter().enumerate() {
        work.spend(1)?;
        if consumed.contains(region) && !triangles.contains(&t) {
            return Ok(None);
        }
    }
    let Some(proof) = crate::revolved::prove_guided_cone(
        a,
        &triangles,
        [&lower.vertices, &upper.vertices],
        lower.center,
        lower.normal,
        edges,
        o,
        work,
    )?
    else {
        return Ok(None);
    };
    let rims = if proof.first_boundary == 0 {
        pair
    } else {
        [pair[1], pair[0]]
    };
    Ok(Some(Patch {
        rims,
        triangles,
        rings: proof.rings,
        inner: proof.inner,
        radial: proof.radial,
        chord: proof.chord,
        wall: proof.wall,
        conic: true,
    }))
}

/// Divergence theorem evaluated on the actual analytic boundary, not a mesh
/// volume plus a guessed cylindrical correction. A local origin limits loss
/// of precision under translation.
pub(crate) fn volume(model: &MixedModel) -> Result<f64, FabError> {
    let bbox = bounds(&model.vertices);
    let reference = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let mut sum = Sum::default();
    for plane in &model.planes {
        let normal = scale(plane.normal, 1. / norm(plane.normal));
        let origin = crate::partial::loop_origin(model, &plane.loops[0]);
        let frame = Frame::new(origin, normal);
        let mut area = Sum::default();
        for (index, lp) in plane.loops.iter().enumerate() {
            let a = match lp {
                MixedLoop::Polygon { vertices } => signed_area(
                    &vertices
                        .iter()
                        .map(|&v| frame.project(model.vertices[v]).0)
                        .collect::<Vec<_>>(),
                )
                .abs(),
                MixedLoop::Circle { ring } => PI * model.rings[*ring].radius_mm.powi(2),
                MixedLoop::Segments { segments } => {
                    crate::partial::segmented_area(model, segments, normal).abs()
                }
            };
            area.add(a * if index == 0 { 1. } else { -1. });
        }
        if area.get() <= 0. || !area.get().is_finite() {
            return Err(unsupported());
        }
        sum.add(dot(sub(origin, reference), normal) * area.get() / 3.);
    }
    for cylinder in &model.cylinders {
        let a = &model.rings[cylinder.rings[0]];
        let b = &model.rings[cylinder.rings[1]];
        let height = dot(sub(b.center, a.center), a.axis);
        sum.add(
            (if cylinder.inner { -1. } else { 1. }) * 2. * PI * a.radius_mm.powi(2) * height / 3.,
        );
    }
    for cylinder in &model.partial_cylinders {
        sum.add(crate::partial::cylinder_flux(model, cylinder, reference));
    }
    let value = sum.get();
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(unsupported())
    }
}

#[cfg(test)]
mod precision_regression {
    use super::*;
    #[test]
    fn quantized_mixed_caps_require_explicit_plane_tolerance() {
        let options = Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        };
        let source = crate::analyze_stl(
            include_bytes!("../../../fixtures/m2b_boss_blind_rotated.stl"),
            &options,
        )
        .unwrap();
        let mut bytes = vec![0; 80];
        bytes.extend_from_slice(&(source.mesh.triangles.len() as u32).to_le_bytes());
        for tri in &source.mesh.triangles {
            bytes.extend_from_slice(&[0; 12]);
            for &v in tri {
                for c in source.mesh.vertices[v] {
                    bytes.extend_from_slice(&(c as f32).to_le_bytes());
                }
            }
            bytes.extend_from_slice(&[0; 2]);
        }
        let a = crate::analyze_stl(&bytes, &options).unwrap();
        assert_eq!(
            recognize_mixed(&a, &options).unwrap_err().code,
            "recognition.unsupported"
        );
        // The larger plane allowance is explicitly chosen; radial tolerance is
        // unchanged. Recognition is not equivalent to successful native export.
        let explicit = Options {
            plane_tolerance_mm: 2e-5,
            ..options
        };
        let recognized = recognize_mixed(&a, &explicit).unwrap();
        assert_eq!(recognized.report.cylinders, 2);
        assert!(recognized.report.max_radial_residual_mm <= explicit.cylinder_fit_tolerance_mm);
        assert!(recognized.report.max_wall_residual_mm > 1e-7);
        assert!(recognized.report.max_deviation_bound_mm < 0.011);
    }
}
