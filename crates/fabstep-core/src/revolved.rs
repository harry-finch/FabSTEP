//! Explicit, bounded recognition of full circular ruled bands. Axes originate
//! in real planar circular boundaries; every wall triangle, intermediate ring
//! and apex is proved. Consecutive conical bands first follow ONE affine radius
//! law; a unique source break may prove two witnessed adjacent frusta.
//! Longer chains require unique proved breaks and interior source rings. Tiny
//! radius changes must not manufacture cylindrical separators.
use crate::math::*;
use crate::recognition::{fit_circle, signed_area, validate_analysis, Work};
use crate::{
    Analysis, CircularRing, Diagnostic, FabError, MixedCone, MixedCylinder, MixedLoop, MixedModel,
    MixedPlane, Options, RecognitionReport, ReconstructionMode, RevolvedFaceRef, RevolvedModel,
    RevolvedRecognition, Severity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

pub(crate) mod adjacent_chain;

type Edge = (usize, usize);
fn edge(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn unsupported() -> FabError {
    FabError::new(
        "recognition.unsupported",
        "Aucune paroi conique complète n'a été détectée à partir des contours plans source.",
    )
}
fn unproven(detail: impl std::fmt::Display) -> FabError {
    FabError::new(
        "recognition.conic_unproven",
        format!("Reconstruction conique non prouvée : {detail}. Aucun résultat partiel proposé."),
    )
}
/// Narrow extension of the single-cone rule: an opposed pair separated by a
/// demonstrably distinct cylindrical land. Input points are [axial z, radius]
/// at cone start, land start, land end and cone end, after canonicalization.
/// Existing vertex/UV proofs still establish every surface and its error.
///
/// Each coordinate may vary by epsilon. Thus a cone's absolute slope is at
/// least (|dr|-2e)/(h+2e), whereas a nominal cylinder's slope is at most
/// 2e/(h-2e). Requiring disjoint intervals with a factor-four margin prevents
/// tolerance-scale radial changes from manufacturing a separator. Opposed
/// slopes additionally exclude a monotone meridian split into several cones.
/// This is an admissibility restriction, never a relaxed fitting tolerance.
pub(crate) fn resolved_double_chamfer(profile: [[f64; 2]; 4], epsilon: f64) -> bool {
    resolved_cone_land_pair(profile, epsilon)
        && (profile[1][1] - profile[0][1]).is_sign_positive()
            != (profile[3][1] - profile[2][1]).is_sign_positive()
}

/// A real cylindrical land distinguishes both adjacent affine runs even when
/// their slopes have the same sign. Callers admitting unrestricted signs must
/// additionally require interior source witnesses in every run.
fn resolved_cone_land_pair(profile: [[f64; 2]; 4], epsilon: f64) -> bool {
    if !epsilon.is_finite() || epsilon <= 0. || profile.iter().flatten().any(|x| !x.is_finite()) {
        return false;
    }
    let heights: [f64; 3] = std::array::from_fn(|i| profile[i + 1][0] - profile[i][0]);
    let changes: [f64; 3] = std::array::from_fn(|i| profile[i + 1][1] - profile[i][1]);
    if heights.iter().any(|&h| h <= 2. * epsilon)
        || heights[1] <= 8. * epsilon
        || changes[1].abs() > 2. * epsilon
        || changes[0].abs() <= 2. * epsilon
        || changes[2].abs() <= 2. * epsilon
    {
        return false;
    }
    // Avoid dividing by the land height; all factors are positive above.
    [0, 2].into_iter().all(|i| {
        (changes[i].abs() - 2. * epsilon) * (heights[1] - 2. * epsilon)
            > 8. * epsilon * (heights[i] + 2. * epsilon)
    })
}
/// Interior evidence must come from complete source rings, not interpolated
/// endpoints. Two separated levels in the middle half witness each affine run.
/// The caller still proves their radii and the full triangular wall.
pub(crate) fn resolved_run_samples(
    z0: f64,
    z1: f64,
    samples: impl Iterator<Item = f64>,
    epsilon: f64,
) -> bool {
    let height = z1 - z0;
    let mut lower = None::<f64>;
    let mut upper = None::<f64>;
    for z in samples {
        let t = (z - z0) / height;
        if (0.25..0.5).contains(&t) {
            lower = Some(lower.map_or(z, |old| old.min(z)));
        }
        if t > 0.5 && t <= 0.75 {
            upper = Some(upper.map_or(z, |old| old.max(z)));
        }
    }
    matches!((lower, upper), (Some(a), Some(b)) if b - a > 4. * epsilon)
}

/// Bounded C1 subset: 2..=8 cones separated by resolved cylinders.
/// Every run has interior source evidence; every cone/land transition retains
/// the historical slope-uncertainty margin. Signs may differ or agree, but
/// adjacent cones without a land remain excluded. No new break is inferred.
pub(crate) fn resolved_conical_chain(
    profile: &[[f64; 2]],
    witnessed: &[bool],
    epsilon: f64,
) -> bool {
    let spans = witnessed.len();
    if !(3..=15).contains(&spans)
        || spans % 2 == 0
        || profile.len() != spans + 1
        || witnessed.iter().any(|w| !w)
    {
        return false;
    }
    (0..spans - 2).step_by(2).all(|i| {
        resolved_cone_land_pair(
            [profile[i], profile[i + 1], profile[i + 2], profile[i + 3]],
            epsilon,
        )
    })
}

/// First adjacent-cone subset: exactly two positive-radius affine runs, with
/// two genuine interior rings on each side of a unique complete source ring.
/// A single affine law always wins. Multiple admissible source breakpoints,
/// even within tolerance of one another, are deliberately ambiguous.
pub(crate) fn adjacent_cone_break(
    profile: &[[f64; 2]],
    o: &Options,
    work: &mut Work,
) -> Result<Option<usize>, FabError> {
    let epsilon = o
        .cylinder_fit_tolerance_mm
        .max(o.plane_tolerance_mm)
        .max(o.weld_tolerance_mm);
    work.spend(profile.len().saturating_mul(3))?;
    if profile.len() < 7
        || !epsilon.is_finite()
        || epsilon <= 0.
        || profile
            .iter()
            .any(|p| p.iter().any(|x| !x.is_finite()) || p[1] <= epsilon)
        || profile.windows(2).any(|w| {
            w[1][0] - w[0][0] <= 4. * epsilon
                || (w[1][1] - w[0][1]).abs() <= 2. * o.cylinder_fit_tolerance_mm
        })
    {
        return Ok(None);
    }
    let fits = |points: &[[f64; 2]]| {
        let [z0, r0] = points[0];
        let [z1, r1] = *points.last().unwrap();
        points.iter().all(|&[z, r]| {
            (r - (r0 + (r1 - r0) * (z - z0) / (z1 - z0))).abs() <= o.cylinder_fit_tolerance_mm
        })
    };
    if fits(profile) {
        return Ok(None);
    }
    let slope = |points: &[[f64; 2]]| {
        let a = points[0];
        let b = points.last().unwrap();
        let h = b[0] - a[0];
        let m = (b[1] - a[1]) / h;
        // Each endpoint coordinate may move by epsilon. This bounds the
        // signed slope perturbation for every positive h +/- 2 epsilon.
        let uncertainty = (2. * epsilon + 2. * epsilon * m.abs()) / (h - 2. * epsilon);
        (m, uncertainty)
    };
    let mut found = None;
    for split in 3..profile.len() - 3 {
        work.spend(profile.len().saturating_mul(3))?;
        let sides = [&profile[..=split], &profile[split..]];
        if !sides.iter().all(|side| {
            (side.last().unwrap()[1] - side[0][1]).abs() > 2. * epsilon
                && resolved_run_samples(
                    side[0][0],
                    side.last().unwrap()[0],
                    side.iter().map(|p| p[0]),
                    epsilon,
                )
                && fits(side)
        }) {
            continue;
        }
        let (left, le) = slope(sides[0]);
        let (right, re) = slope(sides[1]);
        if ![left, le, right, re].iter().all(|x| x.is_finite())
            || (left - right).abs() <= 4. * (le + re)
        {
            continue;
        }
        if found.is_some() {
            return Ok(None);
        }
        found = Some(split);
    }
    Ok(found)
}
#[derive(Clone)]
struct Frame {
    origin: V,
    axis: V,
    x: V,
    y: V,
}
impl Frame {
    fn new(origin: V, normal: V) -> Self {
        let mut axis = scale(normal, 1. / norm(normal));
        let major = (0..3)
            .max_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
            .unwrap();
        if axis[major] < 0. {
            axis = scale(axis, -1.);
        }
        let least = (0..3)
            .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
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
    fn project(&self, p: V) -> V {
        let p = sub(p, self.origin);
        [dot(p, self.x), dot(p, self.y), dot(p, self.axis)]
    }
    fn lift(&self, p: V) -> V {
        std::array::from_fn(|i| {
            self.origin[i] + self.x[i] * p[0] + self.y[i] * p[1] + self.axis[i] * p[2]
        })
    }
}
struct Node {
    vertices: Vec<usize>,
    z: f64,
    radius: f64,
    measured_radius: f64,
}
struct AxisData {
    frame: Frame,
    projected: Vec<V>,
    nodes: Vec<Node>,
    vertex_node: Vec<Option<usize>>,
    potential: bool,
}
struct Band {
    nodes: [usize; 2],
    triangles: Vec<usize>,
    inner: bool,
    conic: bool,
}
struct Span {
    nodes: [usize; 2],
    bands: Vec<usize>,
    inner: bool,
    conic: bool,
}
struct AxisResult {
    rings: Vec<CircularRing>,
    ring_vertices: Vec<Vec<usize>>,
    cylinders: Vec<MixedCylinder>,
    cones: Vec<MixedCone>,
    owners: Vec<(usize, RevolvedFaceRef)>,
    radial: f64,
    wall: f64,
    chord: f64,
}

pub fn recognize_revolved(a: &Analysis, o: &Options) -> Result<RevolvedRecognition, FabError> {
    if o.reconstruction != ReconstructionMode::Primitives {
        return Err(FabError::new(
            "recognition.disabled",
            "Activer explicitement le mode cônes et cylindres.",
        ));
    }
    for t in [
        o.weld_tolerance_mm,
        o.plane_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.max_deviation_mm,
    ] {
        if !t.is_finite() || t <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    validate_analysis(a)?;
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let segmentation = crate::planar::build_with_provenance(&a.mesh, &adjacency, tol)?;
    let mut work = Work::default();
    let mut frames: Vec<Frame> = Vec::new();
    for face in &segmentation.model.faces {
        for lp in &face.loops {
            if lp.len() < 12 {
                continue;
            }
            work.spend(lp.len())?;
            let mut frame = Frame::new(a.mesh.vertices[lp[0]], face.normal);
            let points: Vec<_> = lp
                .iter()
                .map(|&v| {
                    let p = frame.project(a.mesh.vertices[v]);
                    [p[0], p[1]]
                })
                .collect();
            let Some(fit) = fit_circle(&points, o.cylinder_fit_tolerance_mm) else {
                continue;
            };
            frame.origin = frame.lift([fit.center[0], fit.center[1], 0.]);
            work.spend(frames.len())?;
            if frames.iter().any(|f| {
                let delta = sub(frame.origin, f.origin);
                dot(frame.axis, f.axis) > 1. - 1e-10
                    && norm(sub(delta, scale(f.axis, dot(delta, f.axis)))) <= tol
            }) {
                continue;
            }
            if frames.len() >= 64 {
                return Err(FabError::new(
                    "recognition.too_complex",
                    "Plus de 64 axes candidats pour les primitives.",
                ));
            }
            frames.push(frame);
        }
    }
    if frames.is_empty() {
        return Err(unsupported());
    }
    let mut edges: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    for (i, &[x, y, z]) in a.mesh.triangles.iter().enumerate() {
        for (u, v) in [(x, y), (y, z), (z, x)] {
            edges.entry(edge(u, v)).or_default().push(i);
        }
    }
    let mut potential = false;
    let mut failure = None;
    let mut results = Vec::new();
    for frame in frames {
        let data = axis_data(a, frame, &edges, o, &mut work)?;
        potential |= data.potential;
        match prove_axis(a, data, &edges, o, &mut work, None) {
            Ok(result) => results.push(result),
            Err(e) if e.code == "recognition.conic_unproven" => {
                if failure.is_none() {
                    failure = Some(e);
                }
            }
            Err(e) => return Err(e),
        }
    }
    // Pure cylinders retain the legacy pipeline and its exact mode semantics.
    if !potential {
        return Err(unsupported());
    }
    if let Some(error) = failure {
        return Err(error);
    }
    let mut model = RevolvedModel {
        mixed: MixedModel {
            vertices: a.mesh.vertices.clone(),
            rings: Vec::new(),
            planes: Vec::new(),
            cylinders: Vec::new(),
            arcs: Vec::new(),
            elliptic_arcs: vec![],
            partial_cylinders: Vec::new(),
        },
        cones: Vec::new(),
    };
    let mut owners = vec![None; a.mesh.triangles.len()];
    let mut ring_edges = BTreeMap::new();
    let (mut radial, mut wall, mut chord) = (0_f64, 0_f64, 0_f64);
    for r in results {
        let ring_offset = model.mixed.rings.len();
        let cylinder_offset = model.mixed.cylinders.len();
        let cone_offset = model.cones.len();
        for (i, vertices) in r.ring_vertices.iter().enumerate() {
            if vertices.len() == 1 {
                continue;
            }
            let keys = loop_edges(vertices);
            if ring_edges.insert(keys, ring_offset + i).is_some() {
                return Err(unproven("anneau attribué à plusieurs axes"));
            }
        }
        model.mixed.rings.extend(r.rings);
        model
            .mixed
            .cylinders
            .extend(r.cylinders.into_iter().map(|c| MixedCylinder {
                rings: c.rings.map(|i| i + ring_offset),
                inner: c.inner,
            }));
        model.cones.extend(r.cones.into_iter().map(|c| MixedCone {
            rings: c.rings.map(|i| i + ring_offset),
            inner: c.inner,
        }));
        for (t, owner) in r.owners {
            let owner = match owner {
                RevolvedFaceRef::Cylinder { index } => RevolvedFaceRef::Cylinder {
                    index: index + cylinder_offset,
                },
                RevolvedFaceRef::Cone { index } => RevolvedFaceRef::Cone {
                    index: index + cone_offset,
                },
                RevolvedFaceRef::Plane { .. } => unreachable!(),
            };
            if owners[t].replace(owner).is_some() {
                return Err(unproven(format!(
                    "triangle {t} partagé entre plusieurs primitives"
                )));
            }
        }
        radial = radial.max(r.radial);
        wall = wall.max(r.wall);
        chord = chord.max(r.chord);
    }
    if model.cones.is_empty() {
        return Err(unproven("aucun cône complet n'a satisfait la preuve"));
    }
    let mut region_counts = vec![(0_usize, 0_usize); segmentation.model.faces.len()];
    for (t, &region) in segmentation.triangle_regions.iter().enumerate() {
        region_counts[region].0 += 1;
        region_counts[region].1 += usize::from(owners[t].is_some());
    }
    let mut plane_mapping = BTreeMap::new();
    for (region, face) in segmentation.model.faces.iter().enumerate() {
        let (total, consumed) = region_counts[region];
        if consumed == total {
            continue;
        }
        if consumed != 0 {
            return Err(unproven(format!(
                "région plane {region} seulement partiellement remplacée"
            )));
        }
        let index = model.mixed.planes.len();
        plane_mapping.insert(region, index);
        let loops = face
            .loops
            .iter()
            .map(|lp| {
                if let Some(&ring) = ring_edges.get(&loop_edges(lp)) {
                    MixedLoop::Circle { ring }
                } else {
                    MixedLoop::Polygon {
                        vertices: lp.clone(),
                    }
                }
            })
            .collect();
        model.mixed.planes.push(MixedPlane {
            normal: face.normal,
            loops,
        });
    }
    for (t, owner) in owners.iter_mut().enumerate() {
        if owner.is_none() {
            let index = plane_mapping[&segmentation.triangle_regions[t]];
            *owner = Some(RevolvedFaceRef::Plane { index });
            let plane = &model.mixed.planes[index];
            let normal = scale(plane.normal, 1. / norm(plane.normal));
            let origin = crate::partial::loop_origin(&model.mixed, &plane.loops[0]);
            for &v in &a.mesh.triangles[t] {
                wall = wall.max(dot(sub(a.mesh.vertices[v], origin), normal).abs());
            }
        }
    }
    let mut incidences = vec![0_usize; model.mixed.rings.len()];
    for p in &model.mixed.planes {
        for lp in &p.loops {
            if let MixedLoop::Circle { ring } = lp {
                if model.mixed.rings[*ring].radius_mm == 0. {
                    return Err(unproven("pointe utilisée comme fermeture plane"));
                }
                incidences[*ring] += 1;
            }
        }
    }
    for c in &model.mixed.cylinders {
        for &ring in &c.rings {
            if model.mixed.rings[ring].radius_mm == 0. {
                return Err(unproven("cylindre avec pointe"));
            }
            incidences[ring] += 1;
        }
    }
    for c in &model.cones {
        for &ring in &c.rings {
            incidences[ring] += 1;
        }
    }
    for (i, &count) in incidences.iter().enumerate() {
        if count
            != if model.mixed.rings[i].radius_mm == 0. {
                1
            } else {
                2
            }
        {
            return Err(unproven(format!("anneau {i} avec {count} incidences")));
        }
    }
    let deviation = o.weld_tolerance_mm + radial + wall + chord;
    if !deviation.is_finite() || deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            format!(
                "La borne composée conique {deviation:.9} mm dépasse le budget {:.9} mm.",
                o.max_deviation_mm
            ),
        ));
    }
    let expected_volume = volume(&model)?;
    let cones = model.cones.len();
    let cylinders = model.mixed.cylinders.len();
    let planes = model.mixed.planes.len();
    Ok(RevolvedRecognition {model,triangle_faces:owners.into_iter().map(Option::unwrap).collect(),report:RecognitionReport {
        cones,cylinders,spheres:0,tori:0,toroidal_fillets:0,max_radial_residual_mm:radial,max_wall_residual_mm:wall,max_chord_error_mm:chord,max_deviation_bound_mm:deviation,expected_volume_mm3:expected_volume,
        diagnostics:vec![
            Diagnostic {code:"recognition.conic_intent_assumed".into(),severity:Severity::Warning,count:cones,message:"Les bandes circulaires à rayon affine sont interprétées comme cônes dans le mode explicitement activé.".into()},
            Diagnostic {code:"recognition.revolved_coverage".into(),severity:Severity::Info,count:a.mesh.triangles.len(),message:format!("Tous les triangles sont attribués : {cones} cônes, {cylinders} cylindres et {planes} plans. Anneaux et pointes issus de la source ; aucune fermeture ajoutée.")},
            Diagnostic {code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,message:"Borne composée de soudage, résidu radial, correspondance des triangles et flèche des cordes ; pas une mesure Hausdorff globale. Validation native source et résultat requise.".into()},
        ]}})
}

fn loop_edges(vertices: &[usize]) -> BTreeSet<Edge> {
    (0..vertices.len())
        .map(|i| edge(vertices[i], vertices[(i + 1) % vertices.len()]))
        .collect()
}
fn axis_data(
    a: &Analysis,
    frame: Frame,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<AxisData, FabError> {
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    work.spend(a.mesh.vertices.len() + a.mesh.triangles.len() + edges.len() * 2)?;
    let projected: Vec<_> = a.mesh.vertices.iter().map(|&p| frame.project(p)).collect();
    let horizontal: Vec<bool> = a
        .mesh
        .triangles
        .iter()
        .map(|t| {
            let z = t.map(|v| projected[v][2]);
            z.into_iter().fold(f64::NEG_INFINITY, f64::max)
                - z.into_iter().fold(f64::INFINITY, f64::min)
                <= tol
        })
        .collect();
    let mut incident: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (&(u, v), triangles) in edges {
        if (projected[u][2] - projected[v][2]).abs() <= tol
            && triangles.iter().any(|&t| !horizontal[t])
        {
            incident.entry(u).or_default().push(v);
            incident.entry(v).or_default().push(u);
        }
    }
    let mut visited = BTreeSet::new();
    let mut nodes = Vec::new();
    let mut vertex_node = vec![None; projected.len()];
    for &start in incident.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if !visited.insert(v) {
                continue;
            }
            component.push(v);
            work.spend(incident[&v].len())?;
            stack.extend(incident[&v].iter().copied());
        }
        if component.len() < 12 || component.iter().any(|v| incident[v].len() != 2) {
            continue;
        }
        let mut vertices = vec![start];
        let mut previous = start;
        let mut current = incident[&start][0];
        while current != start {
            work.spend(1)?;
            vertices.push(current);
            if vertices.len() > component.len() {
                return Err(unproven("cycle de frontière non simple"));
            }
            let next = incident[&current]
                .iter()
                .copied()
                .find(|&v| v != previous)
                .unwrap();
            previous = current;
            current = next;
        }
        if vertices.len() != component.len() {
            continue;
        }
        let points: Vec<_> = vertices
            .iter()
            .map(|&v| [projected[v][0], projected[v][1]])
            .collect();
        let Some(fit) = fit_circle(&points, o.cylinder_fit_tolerance_mm) else {
            continue;
        };
        if fit.center[0].hypot(fit.center[1]) > tol {
            continue;
        }
        let z = vertices.iter().map(|&v| projected[v][2]).sum::<f64>() / vertices.len() as f64;
        if vertices.iter().any(|&v| (projected[v][2] - z).abs() > tol) {
            continue;
        }
        let radius = vertices
            .iter()
            .map(|&v| projected[v][0].hypot(projected[v][1]))
            .sum::<f64>()
            / vertices.len() as f64;
        if vertices.iter().any(|&v| {
            (projected[v][0].hypot(projected[v][1]) - radius).abs() > o.cylinder_fit_tolerance_mm
        }) {
            continue;
        }
        if signed_area(&points) < 0. {
            vertices.reverse();
        }
        let index = nodes.len();
        for &v in &vertices {
            vertex_node[v] = Some(index);
        }
        nodes.push(Node {
            vertices,
            z,
            radius,
            measured_radius: radius,
        });
    }
    let mut potential = false;
    for (t, triangle) in a.mesh.triangles.iter().enumerate() {
        if horizontal[t] || !triangle.iter().any(|&v| vertex_node[v].is_some()) {
            continue;
        }
        let radii = triangle.map(|v| projected[v][0].hypot(projected[v][1]));
        if radii.into_iter().fold(f64::NEG_INFINITY, f64::max)
            - radii.into_iter().fold(f64::INFINITY, f64::min)
            > o.cylinder_fit_tolerance_mm * 2.
        {
            potential = true;
        }
        for &v in triangle {
            if vertex_node[v].is_none() && projected[v][0].hypot(projected[v][1]) <= tol {
                vertex_node[v] = Some(nodes.len());
                nodes.push(Node {
                    vertices: vec![v],
                    z: projected[v][2],
                    radius: 0.,
                    measured_radius: 0.,
                });
            }
        }
    }
    Ok(AxisData {
        frame,
        projected,
        nodes,
        vertex_node,
        potential,
    })
}

fn prove_axis(
    a: &Analysis,
    mut d: AxisData,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
    scope: Option<&BTreeSet<usize>>,
) -> Result<AxisResult, FabError> {
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let mut groups: BTreeMap<[usize; 2], Vec<usize>> = BTreeMap::new();
    for (t, triangle) in a.mesh.triangles.iter().enumerate() {
        work.spend(1)?;
        if scope.is_some_and(|scope| !scope.contains(&t)) {
            continue;
        }
        if !triangle.iter().any(|&v| d.vertex_node[v].is_some()) {
            continue;
        }
        let z = triangle.map(|v| d.projected[v][2]);
        if z.into_iter().fold(f64::NEG_INFINITY, f64::max)
            - z.into_iter().fold(f64::INFINITY, f64::min)
            <= tol
        {
            continue;
        }
        let mut ids = BTreeSet::new();
        for &v in triangle {
            ids.insert(d.vertex_node[v].ok_or_else(|| {
                unproven(format!(
                    "triangle {t} hors des anneaux ou d'une pointe coaxiale source"
                ))
            })?);
        }
        if ids.len() != 2 {
            return Err(unproven(format!(
                "triangle {t} ne relie pas deux frontières source"
            )));
        }
        let ids: Vec<_> = ids.into_iter().collect();
        let mut pair = [ids[0], ids[1]];
        if d.nodes[pair[0]].z > d.nodes[pair[1]].z {
            pair.swap(0, 1);
        }
        if d.nodes[pair[1]].z - d.nodes[pair[0]].z <= tol * 4. {
            return Err(unproven(format!("bande du triangle {t} trop courte")));
        }
        groups.entry(pair).or_default().push(t);
    }
    if groups.is_empty() {
        return Err(unproven("aucune bande complète entre les anneaux"));
    }
    let mut bands = Vec::new();
    for (nodes, triangles) in groups {
        let inner = prove_band(a, &d, nodes, &triangles, edges, o, work)?;
        let conic = (d.nodes[nodes[0]].radius - d.nodes[nodes[1]].radius).abs()
            > o.cylinder_fit_tolerance_mm * 2.;
        bands.push(Band {
            nodes,
            triangles,
            inner,
            conic,
        });
    }
    let mut incident = vec![Vec::new(); d.nodes.len()];
    for (i, band) in bands.iter().enumerate() {
        for &n in &band.nodes {
            incident[n].push(i);
        }
    }
    if incident
        .iter()
        .enumerate()
        .any(|(i, b)| b.is_empty() || b.len() > if d.nodes[i].radius == 0. { 1 } else { 2 })
    {
        return Err(unproven("anneau sans bande ou frontière ramifiée"));
    }
    let mut assigned = vec![false; bands.len()];
    let mut spans = Vec::new();
    for seed in 0..bands.len() {
        if assigned[seed] {
            continue;
        }
        let mut component = BTreeSet::new();
        let mut pending = vec![seed];
        while let Some(b) = pending.pop() {
            work.spend(1)?;
            if !component.insert(b) {
                continue;
            }
            for &n in &bands[b].nodes {
                pending.extend(incident[n].iter().copied());
            }
        }
        let mut chain: Vec<_> = component.into_iter().collect();
        chain.sort_by(|&i, &j| {
            d.nodes[bands[i].nodes[0]]
                .z
                .total_cmp(&d.nodes[bands[j].nodes[0]].z)
        });
        for pair in chain.windows(2) {
            if bands[pair[0]].nodes[1] != bands[pair[1]].nodes[0]
                || bands[pair[0]].inner != bands[pair[1]].inner
            {
                return Err(unproven(
                    "profil axial non monotone ou orientation incohérente",
                ));
            }
        }
        let resolved = match historical_spans(&mut d.nodes, &bands, &chain, o, work) {
            Ok(spans) => spans,
            Err(previous) if previous.code == "recognition.conic_unproven" && chain.len() >= 9 => {
                let Some(spans) = adjacent_spans(&mut d, &bands, &chain, o, work)? else {
                    return Err(previous);
                };
                spans
            }
            Err(error) => return Err(error),
        };
        for &band in &chain {
            assigned[band] = true;
        }
        spans.extend(resolved);
    }
    let mut result = AxisResult {
        rings: Vec::new(),
        ring_vertices: Vec::new(),
        cylinders: Vec::new(),
        cones: Vec::new(),
        owners: Vec::new(),
        radial: 0.,
        wall: 0.,
        chord: 0.,
    };
    let mut mapping = BTreeMap::new();
    for span in &spans {
        for &n in &span.nodes {
            mapping.entry(n).or_insert_with(|| {
                let index = result.rings.len();
                let node = &d.nodes[n];
                result.rings.push(CircularRing {
                    center: d.frame.lift([0., 0., node.z]),
                    axis: d.frame.axis,
                    radius_mm: node.radius,
                });
                result.ring_vertices.push(node.vertices.clone());
                index
            });
        }
    }
    for span in spans {
        let rings = span.nodes.map(|n| mapping[&n]);
        let owner = if span.conic {
            let index = result.cones.len();
            result.cones.push(MixedCone {
                rings,
                inner: span.inner,
            });
            RevolvedFaceRef::Cone { index }
        } else {
            let index = result.cylinders.len();
            result.cylinders.push(MixedCylinder {
                rings,
                inner: span.inner,
            });
            RevolvedFaceRef::Cylinder { index }
        };
        let low = &d.nodes[span.nodes[0]];
        let high = &d.nodes[span.nodes[1]];
        let height = high.z - low.z;
        let mut max_angle: f64 = 0.;
        for b in span.bands {
            for &n in &bands[b].nodes {
                let node = &d.nodes[n];
                if node.radius > 0. {
                    for i in 0..node.vertices.len() {
                        let p = d.projected[node.vertices[i]];
                        let q = d.projected[node.vertices[(i + 1) % node.vertices.len()]];
                        let angle = (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
                        if angle <= 0. || angle > PI / 4. + 1e-12 {
                            return Err(unproven("échantillonnage angulaire incohérent"));
                        }
                        max_angle = max_angle.max(angle);
                    }
                }
                for &v in &node.vertices {
                    work.spend(1)?;
                    let p = d.projected[v];
                    let expected =
                        low.radius + (high.radius - low.radius) * (node.z - low.z) / height;
                    let radial = (p[0].hypot(p[1]) - expected).abs();
                    if radial > o.cylinder_fit_tolerance_mm {
                        return Err(unproven(format!(
                            "sommet {v} : résidu radial {radial:.9} mm supérieur à la tolérance"
                        )));
                    }
                    result.radial = result.radial.max(radial);
                    result.wall = result.wall.max((p[2] - node.z).abs());
                }
            }
            // Recheck actual source-to-corresponding ideal facet vertices after
            // cylinder canonicalization. Convex interpolation bounds each
            // entire triangle by this maximum vertex correspondence residual.
            result.wall = result
                .wall
                .max(correspondence_residual(&d, bands[b].nodes, work)?);
            for &t in &bands[b].triangles {
                result.owners.push((t, owner));
            }
        }
        result.chord = result
            .chord
            .max(2. * low.radius.max(high.radius) * (max_angle / 4.).sin().powi(2));
    }
    Ok(result)
}

fn historical_spans(
    nodes: &mut [Node],
    bands: &[Band],
    chain: &[usize],
    o: &Options,
    work: &mut Work,
) -> Result<Vec<Span>, FabError> {
    let mut start = 0;
    let mut spans = Vec::new();
    let mut adjacent_split = None;
    while start < chain.len() {
        let first = &bands[chain[start]];
        let mut end = start + 1;
        while end < chain.len() && bands[chain[end]].conic == first.conic {
            end += 1;
        }
        if let Some(split) = adjacent_split {
            if start < split {
                end = end.min(split);
            }
        }
        let ends = [first.nodes[0], bands[chain[end - 1]].nodes[1]];
        let low = &nodes[ends[0]];
        let high = &nodes[ends[1]];
        let (z0, z1, r0, r1) = (low.z, high.z, low.radius, high.radius);
        let mut affine = true;
        for &b in &chain[start..end] {
            for &n in &bands[b].nodes {
                work.spend(1)?;
                let expected = if first.conic {
                    r0 + (r1 - r0) * (nodes[n].z - z0) / (z1 - z0)
                } else {
                    (r0 + r1) * 0.5
                };
                if (nodes[n].radius - expected).abs() > o.cylinder_fit_tolerance_mm {
                    affine = false;
                }
            }
        }
        if !affine {
            // Preserve the successful historical path and its work budget.
            // Only a failed single-cone law may search for one new break.
            if first.conic && start == 0 && end == chain.len() && adjacent_split.is_none() {
                work.spend(chain.len() + 1)?;
                let mut profile: Vec<_> = chain
                    .iter()
                    .map(|&b| {
                        let n = &nodes[bands[b].nodes[0]];
                        [n.z, n.radius]
                    })
                    .collect();
                let last = &nodes[bands[*chain.last().unwrap()].nodes[1]];
                profile.push([last.z, last.radius]);
                adjacent_split = adjacent_cone_break(&profile, o, work)?;
                if adjacent_split.is_some() {
                    continue;
                }
            }
            return Err(unproven(
                "rayon non affine : plusieurs cônes ne peuvent remplacer une courbe méridienne",
            ));
        }
        if !first.conic {
            let radius = (r0 + r1) * 0.5;
            for &b in &chain[start..end] {
                for &n in &bands[b].nodes {
                    nodes[n].radius = radius;
                }
            }
        }
        let span_bands = chain[start..end].to_vec();
        spans.push(Span {
            nodes: ends,
            bands: span_bands,
            inner: first.inner,
            conic: first.conic,
        });
        start = end;
    }
    let chain_spans = spans.as_slice();
    work.spend(chain_spans.len())?;
    if chain_spans.iter().filter(|s| s.conic).count() > 1 {
        let epsilon = o
            .cylinder_fit_tolerance_mm
            .max(o.plane_tolerance_mm)
            .max(o.weld_tolerance_mm);
        let cone_count = chain_spans.iter().filter(|s| s.conic).count();
        if cone_count > 8 {
            return Err(FabError::new(
                "recognition.too_complex",
                "Plus de 8 portions coniques dans une chaîne.",
            ));
        }
        let historical = if let [left, land, right] = chain_spans {
            let profile = [left.nodes[0], land.nodes[0], land.nodes[1], right.nodes[1]]
                .map(|n| [nodes[n].z, nodes[n].radius]);
            left.conic
                && !land.conic
                && right.conic
                && resolved_double_chamfer(
                    profile,
                    o.cylinder_fit_tolerance_mm
                        .max(o.plane_tolerance_mm)
                        .max(o.weld_tolerance_mm),
                )
        } else {
            false
        };
        let adjacent = adjacent_split.is_some()
            && chain_spans.len() == 2
            && chain_spans.iter().all(|s| s.conic);
        let extended = if !historical && !adjacent && cone_count >= 2 {
            let start = usize::from(!chain_spans[0].conic);
            let end = chain_spans.len() - usize::from(!chain_spans.last().unwrap().conic);
            let inner_spans = &chain_spans[start..end];
            work.spend(chain_spans.iter().map(|s| 2 * s.bands.len()).sum())?;
            let witnessed: Vec<_> = chain_spans
                .iter()
                .map(|s| {
                    resolved_run_samples(
                        nodes[s.nodes[0]].z,
                        nodes[s.nodes[1]].z,
                        s.bands
                            .iter()
                            .flat_map(|&b| bands[b].nodes)
                            .map(|n| nodes[n].z),
                        epsilon,
                    )
                })
                .collect();
            let mut profile: Vec<_> = inner_spans
                .iter()
                .map(|s| [nodes[s.nodes[0]].z, nodes[s.nodes[0]].radius])
                .collect();
            let last = &nodes[inner_spans.last().unwrap().nodes[1]];
            profile.push([last.z, last.radius]);
            witnessed.iter().all(|&w| w)
                && inner_spans
                    .iter()
                    .enumerate()
                    .all(|(i, s)| s.conic == (i % 2 == 0))
                && resolved_conical_chain(&profile, &witnessed[start..end], epsilon)
        } else {
            false
        };
        if !historical && !extended && !adjacent {
            return Err(unproven("plusieurs plages coniques sans portées résolues et anneaux intérieurs démontrés (hors double chanfrein historique)"));
        }
    }
    Ok(spans)
}

fn adjacent_spans(
    data: &mut AxisData,
    bands: &[Band],
    chain: &[usize],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Vec<Span>>, FabError> {
    work.spend((chain.len() + 1).saturating_mul(2))?;
    let mut indices: Vec<_> = chain.iter().map(|&b| bands[b].nodes[0]).collect();
    indices.push(bands[*chain.last().unwrap()].nodes[1]);
    // Keep the original measurement and summation order even if the failed
    // historical path changed a radius or the vertex cycle was reversed.
    let profile: Vec<_> = indices
        .iter()
        .map(|&index| {
            let node = &data.nodes[index];
            [node.z, node.measured_radius]
        })
        .collect();
    let Some(plan) = adjacent_chain::classify(&profile, o, work)? else {
        return Ok(None);
    };
    work.spend(indices.len().saturating_add(chain.len()))?;
    for (&index, &[_, radius]) in indices.iter().zip(&profile) {
        data.nodes[index].radius = radius;
    }
    let mut spans = Vec::with_capacity(plan.len());
    for piece in plan {
        let [start, end] = piece.ends;
        if !piece.conic {
            let radius = (profile[start][1] + profile[end][1]) * 0.5;
            for &index in &indices[start..=end] {
                data.nodes[index].radius = radius;
            }
        }
        spans.push(Span {
            nodes: [indices[start], indices[end]],
            bands: chain[start..end].to_vec(),
            inner: bands[chain[start]].inner,
            conic: piece.conic,
        });
    }
    Ok(Some(spans))
}

fn correspondence(
    d: &AxisData,
    nodes: [usize; 2],
    work: &mut Work,
) -> Result<(Vec<usize>, Vec<usize>), FabError> {
    let a = &d.nodes[nodes[0]];
    let b = &d.nodes[nodes[1]];
    if a.radius == 0. {
        return Ok((vec![a.vertices[0]; b.vertices.len()], b.vertices.clone()));
    }
    if b.radius == 0. {
        return Ok((a.vertices.clone(), vec![b.vertices[0]; a.vertices.len()]));
    }
    if a.vertices.len() != b.vertices.len() {
        return Err(unproven("anneaux avec des échantillonnages différents"));
    }
    let p = d.projected[a.vertices[0]];
    let plen = p[0].hypot(p[1]);
    work.spend(b.vertices.len())?;
    let offset = (0..b.vertices.len())
        .min_by(|&i, &j| {
            let dist = |k: usize| {
                let q = d.projected[b.vertices[k]];
                let qlen = q[0].hypot(q[1]);
                (p[0] / plen - q[0] / qlen).hypot(p[1] / plen - q[1] / qlen)
            };
            dist(i).total_cmp(&dist(j))
        })
        .unwrap();
    Ok((
        a.vertices.clone(),
        (0..b.vertices.len())
            .map(|i| b.vertices[(i + offset) % b.vertices.len()])
            .collect(),
    ))
}
fn correspondence_residual(
    d: &AxisData,
    nodes: [usize; 2],
    work: &mut Work,
) -> Result<f64, FabError> {
    let (lower, upper) = correspondence(d, nodes, work)?;
    let mut residual: f64 = 0.;
    for (&u, &v) in lower.iter().zip(&upper) {
        work.spend(1)?;
        let reference = if d.nodes[nodes[0]].radius > 0. {
            d.projected[u]
        } else {
            d.projected[v]
        };
        let length = reference[0].hypot(reference[1]);
        for (w, n) in [(u, nodes[0]), (v, nodes[1])] {
            let node = &d.nodes[n];
            let ideal = [
                reference[0] / length * node.radius,
                reference[1] / length * node.radius,
                node.z,
            ];
            residual = residual.max(norm(sub(d.projected[w], ideal)));
        }
    }
    Ok(residual)
}
#[allow(clippy::too_many_arguments)]
fn prove_band(
    a: &Analysis,
    d: &AxisData,
    nodes: [usize; 2],
    triangles: &[usize],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<bool, FabError> {
    let low = &d.nodes[nodes[0]];
    let high = &d.nodes[nodes[1]];
    if low.radius == 0. && high.radius == 0. {
        return Err(unproven("bande sans anneau circulaire"));
    }
    let (lower, upper) = correspondence(d, nodes, work)?;
    let count = lower.len();
    let apex = low.radius == 0. || high.radius == 0.;
    if triangles.len() != count * if apex { 1 } else { 2 } {
        return Err(unproven("couverture triangulaire incomplète de la bande"));
    }
    let residual = correspondence_residual(d, nodes, work)?;
    if residual > o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm) * 2. {
        return Err(unproven(format!(
            "correspondance des anneaux : résidu {residual:.9} mm"
        )));
    }
    let mut expected = BTreeSet::new();
    for i in 0..count {
        let j = (i + 1) % count;
        if apex {
            let mut t = if low.radius == 0. {
                [lower[i], upper[i], upper[j]]
            } else {
                [lower[i], lower[j], upper[i]]
            };
            t.sort();
            expected.insert(t);
        } else {
            // Both diagonals of a ruled quadrilateral are permitted. Coverage
            // is checked geometrically by its four boundary edges below.
            let _ = j;
        }
    }
    let mut angular = BTreeMap::new();
    for (i, &v) in lower.iter().enumerate() {
        if low.radius > 0. {
            angular.insert(v, i);
        }
    }
    for (i, &v) in upper.iter().enumerate() {
        if high.radius > 0. {
            angular.insert(v, i);
        }
    }
    let mut wedges = vec![0_usize; count];
    let selected: BTreeSet<_> = triangles.iter().copied().collect();
    let mut boundary = BTreeSet::new();
    let mut inner = None;
    for &t in triangles {
        work.spend(1)?;
        let ids = a.mesh.triangles[t];
        let mut angles: Vec<_> = ids.iter().filter_map(|v| angular.get(v).copied()).collect();
        angles.sort();
        angles.dedup();
        if angles.len() != 2 {
            return Err(unproven(format!("triangle {t} hors d'une bande angulaire")));
        }
        let wedge = if angles[1] == angles[0] + 1 {
            angles[0]
        } else if angles[0] == 0 && angles[1] == count - 1 {
            count - 1
        } else {
            return Err(unproven(format!(
                "triangle {t} traverse plusieurs secteurs angulaires"
            )));
        };
        wedges[wedge] += 1;
        if apex {
            let mut key = ids;
            key.sort();
            if !expected.remove(&key) {
                return Err(unproven("éventail de pointe incohérent"));
            }
        }
        for (u, v) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            let key = edge(u, v);
            if edges[&key].iter().any(|other| !selected.contains(other)) {
                boundary.insert(key);
            }
        }
        let [p, q, r] = ids.map(|v| d.projected[v]);
        let normal = cross(sub(q, p), sub(r, p));
        let radial = [(p[0] + q[0] + r[0]) / 3., (p[1] + q[1] + r[1]) / 3., 0.];
        let sense = dot(normal, radial);
        if !sense.is_finite() || sense == 0. {
            return Err(unproven("orientation de paroi indéterminée"));
        }
        if inner.is_some_and(|previous| previous != (sense < 0.)) {
            return Err(unproven("orientation de paroi changeante"));
        }
        inner = Some(sense < 0.);
    }
    if wedges.iter().any(|&n| n != if apex { 1 } else { 2 }) {
        return Err(unproven("secteur angulaire incomplet"));
    }
    let mut wanted = BTreeSet::new();
    if low.radius > 0. {
        wanted.extend(loop_edges(&low.vertices));
    }
    if high.radius > 0. {
        wanted.extend(loop_edges(&high.vertices));
    }
    if boundary != wanted {
        return Err(unproven("frontière de bande différente des anneaux source"));
    }
    Ok(inner.unwrap())
}

pub(crate) fn volume(model: &RevolvedModel) -> Result<f64, FabError> {
    let m = &model.mixed;
    let bbox = bounds(&m.vertices);
    let reference = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let mut sum = Sum::default();
    for plane in &m.planes {
        let normal = scale(plane.normal, 1. / norm(plane.normal));
        let origin = crate::partial::loop_origin(m, &plane.loops[0]);
        let frame = Frame::new(origin, normal);
        let mut area = Sum::default();
        for (i, lp) in plane.loops.iter().enumerate() {
            let a = match lp {
                MixedLoop::Polygon { vertices } => signed_area(
                    &vertices
                        .iter()
                        .map(|&v| {
                            let p = frame.project(m.vertices[v]);
                            [p[0], p[1]]
                        })
                        .collect::<Vec<_>>(),
                )
                .abs(),
                MixedLoop::Circle { ring } => PI * m.rings[*ring].radius_mm.powi(2),
                MixedLoop::Segments { .. } => {
                    return Err(unproven(
                        "contour partiel non admis dans la reconstruction conique",
                    ))
                }
            };
            area.add(a * if i == 0 { 1. } else { -1. });
        }
        if area.get() <= 0. || !area.get().is_finite() {
            return Err(unproven("aire plane non positive"));
        }
        sum.add(dot(sub(origin, reference), normal) * area.get() / 3.);
    }
    for (rings, inner) in m
        .cylinders
        .iter()
        .map(|c| (c.rings, c.inner))
        .chain(model.cones.iter().map(|c| (c.rings, c.inner)))
    {
        let a = &m.rings[rings[0]];
        let b = &m.rings[rings[1]];
        let h = dot(sub(b.center, a.center), a.axis);
        let z = dot(sub(a.center, reference), a.axis);
        // Exact divergence flux of a full cone/frustum, subtracting its two
        // cap fluxes from the closed-frustum volume. Works at either apex.
        let flux = PI / 3.
            * (h * (a.radius_mm.powi(2) + a.radius_mm * b.radius_mm)
                + z * (a.radius_mm.powi(2) - b.radius_mm.powi(2)));
        sum.add(flux * if inner { -1. } else { 1. });
    }
    let value = sum.get();
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(unproven("volume analytique non positif ou non fini"))
    }
}

/// Uses exactly the ordinary band, affine-meridian, coverage and residual
/// proofs, restricted to one topologically isolated wall. Unrelated walls do
/// not participate, and complexity failures are never converted to a miss.
pub(crate) struct GuidedConeProof {
    pub rings: [CircularRing; 2],
    pub first_boundary: usize,
    pub inner: bool,
    pub radial: f64,
    pub wall: f64,
    pub chord: f64,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn prove_guided_cone(
    a: &Analysis,
    triangles: &BTreeSet<usize>,
    boundaries: [&[usize]; 2],
    origin: V,
    normal: V,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<GuidedConeProof>, FabError> {
    let mut attempt = || -> Result<GuidedConeProof, FabError> {
        let mut data = axis_data(a, Frame::new(origin, normal), edges, o, work)?;
        work.spend(a.mesh.vertices.len() + triangles.len() * 3)?;
        let used: BTreeSet<_> = triangles
            .iter()
            .flat_map(|&t| a.mesh.triangles[t])
            .collect();
        let mut mapping = BTreeMap::new();
        let mut nodes = Vec::new();
        for (old, node) in data.nodes.into_iter().enumerate() {
            if node.vertices.iter().any(|v| used.contains(v)) {
                mapping.insert(old, nodes.len());
                nodes.push(node);
            }
        }
        for (v, node) in data.vertex_node.iter_mut().enumerate() {
            *node = if used.contains(&v) {
                node.and_then(|n| mapping.get(&n).copied())
            } else {
                None
            };
        }
        data.nodes = nodes;
        let result = prove_axis(a, data, edges, o, work, Some(triangles))?;
        if result.cones.len() != 1
            || !result.cylinders.is_empty()
            || result.rings.len() != 2
            || result.rings.iter().any(|r| r.radius_mm <= 0.)
            || result.owners.len() != triangles.len()
        {
            return Err(unproven("la sélection doit désigner un seul tronc de cône"));
        }
        let boundary_sets = boundaries.map(|vs| vs.iter().copied().collect::<BTreeSet<_>>());
        let first_set = result.ring_vertices[0]
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let second_set = result.ring_vertices[1]
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let first_boundary = if first_set == boundary_sets[0] && second_set == boundary_sets[1] {
            0
        } else if first_set == boundary_sets[1] && second_set == boundary_sets[0] {
            1
        } else {
            return Err(unproven(
                "les frontières ne sont pas les deux contours plans source",
            ));
        };
        Ok(GuidedConeProof {
            rings: [result.rings[0].clone(), result.rings[1].clone()],
            first_boundary,
            inner: result.cones[0].inner,
            radial: result.radial,
            wall: result.wall,
            chord: result.chord,
        })
    };
    match attempt() {
        Ok(proof) => Ok(Some(proof)),
        Err(e) if e.code == "recognition.conic_unproven" => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod guided_budget_tests {
    use super::*;
    #[test]
    fn adjacent_break_charges_its_candidate_search_and_all_coordinate_uncertainty() {
        let profile = [
            [0., 10.],
            [1., 11.],
            [2., 12.],
            [3., 13.],
            [5., 14.],
            [7., 15.],
            [9., 16.],
        ];
        let mut work = Work::default();
        // Finite/spacing/affine scans cost 3n; each candidate costs 3n.
        work.spend(10_000_000 - 6 * profile.len() + 1).unwrap();
        assert_eq!(
            adjacent_cone_break(&profile, &Options::default(), &mut work)
                .unwrap_err()
                .code,
            "recognition.too_complex"
        );
        let profile = [
            [0., 10.],
            [1., 10.05],
            [2., 10.1],
            [3., 10.15],
            [4., 11.15],
            [5., 12.15],
            [6., 13.15],
        ];
        for o in [
            Options {
                plane_tolerance_mm: 0.1,
                ..Options::default()
            },
            Options {
                weld_tolerance_mm: 0.1,
                ..Options::default()
            },
        ] {
            assert_eq!(
                adjacent_cone_break(&profile, &o, &mut Work::default()).unwrap(),
                None
            );
        }
        let affine = [
            [0., 10.],
            [1., 11.],
            [2., 12.],
            [3., 13.],
            [4., 14.],
            [5., 15.],
            [6., 16.],
        ];
        let mut work = Work::default();
        work.spend(10_000_000 - 3 * affine.len()).unwrap();
        assert_eq!(
            adjacent_cone_break(&affine, &Options::default(), &mut work).unwrap(),
            None
        );
    }
    #[test]
    fn same_sign_guard_is_additive_to_the_historical_pair() {
        let profile = [[0., 10.], [3., 13.], [9., 13.], [12., 16.]];
        assert!(!resolved_double_chamfer(profile, 1e-5));
        assert!(resolved_conical_chain(&profile, &[true; 3], 1e-5));
        assert!(!resolved_conical_chain(
            &profile,
            &[true, false, true],
            1e-5
        ));
        assert!(!resolved_conical_chain(&profile, &[true; 3], f64::NAN));
        assert!(!resolved_conical_chain(&profile, &[true; 3], 0.));
        let opposed = [[0., 10.], [3., 13.], [9., 13.], [12., 10.]];
        // Historical pairs do not acquire an interior-witness requirement.
        assert!(resolved_double_chamfer(opposed, 1e-5));
    }
    #[test]
    fn same_sign_axis_cannot_bypass_an_exhausted_work_budget() {
        let o = Options::default();
        let a = crate::analyze_stl(
            include_bytes!("../../../fixtures/same_sign_chains/two_increasing.stl"),
            &o,
        )
        .unwrap();
        let mut edges = BTreeMap::<Edge, Vec<usize>>::new();
        for (t, triangle) in a.mesh.triangles.iter().enumerate() {
            for i in 0..3 {
                edges
                    .entry(edge(triangle[i], triangle[(i + 1) % 3]))
                    .or_default()
                    .push(t);
            }
        }
        let data = axis_data(
            &a,
            Frame::new([0.; 3], [0., 0., 1.]),
            &edges,
            &o,
            &mut Work::default(),
        )
        .unwrap();
        let mut exhausted = Work::default();
        exhausted.spend(10_000_000).unwrap();
        let error = prove_axis(&a, data, &edges, &o, &mut exhausted, None)
            .err()
            .unwrap();
        assert_eq!(error.code, "recognition.too_complex");
    }
    #[test]
    fn guided_cone_proof_preserves_the_callers_exhausted_budget() {
        let a = crate::analyze_stl(
            include_bytes!("../../../fixtures/m4_frustum.stl"),
            &Options::default(),
        )
        .unwrap();
        let mut work = Work::default();
        work.spend(10_000_000).unwrap();
        let error = prove_guided_cone(
            &a,
            &BTreeSet::new(),
            [&[], &[]],
            [0.; 3],
            [0., 0., 1.],
            &BTreeMap::new(),
            &Options::default(),
            &mut work,
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "recognition.too_complex");
    }
}
