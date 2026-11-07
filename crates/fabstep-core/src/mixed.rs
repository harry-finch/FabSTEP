//! Local, complete cylindrical patches. No feature is inferred without exact
//! source-triangle coverage and two real planar boundary rings.
use crate::math::*;
use crate::recognition::{
    distance_segment, fit_circle, match_loops, signed_area, validate_analysis, Work,
};
use crate::{
    Analysis, CircularRing, Diagnostic, FabError, MixedCylinder, MixedFaceRef, MixedLoop,
    MixedModel, MixedPlane, MixedRecognition, Options, RecognitionReport, ReconstructionMode,
    Severity,
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
    for i in 0..rims.len() {
        for j in i + 1..rims.len() {
            work.spend(1)?;
            if let Some(patch) = prove_patch(
                analysis,
                &segmentation,
                &adjacency,
                &edge_triangles,
                &rims,
                [i, j],
                options,
                &mut work,
            )? {
                matched[i] += 1;
                matched[j] += 1;
                patches.push(patch);
            }
        }
    }
    // No arbitrary first match, partial recognition or silently flattened cone.
    if matched.iter().any(|&count| count != 1) {
        return Err(unsupported());
    }
    let mut owners: Vec<Option<MixedFaceRef>> = vec![None; mesh.triangles.len()];
    let mut consumed_regions = BTreeSet::new();
    let mut model = MixedModel {
        vertices: mesh.vertices.clone(),
        rings: Vec::new(),
        planes: Vec::new(),
        cylinders: Vec::new(),
        arcs: Vec::new(),
        partial_cylinders: Vec::new(),
    };
    let mut replacements = BTreeMap::new();
    let (mut radial, mut chord, mut wall) = (0_f64, 0_f64, 0_f64);
    for patch in patches {
        let cylinder = model.cylinders.len();
        let first_ring = model.rings.len();
        model.rings.extend(patch.rings);
        model.cylinders.push(MixedCylinder {
            rings: [first_ring, first_ring + 1],
            inner: patch.inner,
        });
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
                return Err(unsupported());
            }
            owners[triangle] = Some(MixedFaceRef::Cylinder { index: cylinder });
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
            *owner = Some(MixedFaceRef::Plane { index });
        }
    }
    let partial = crate::partial::augment(analysis, &mut model, &mut owners, options, &mut work)?;
    radial = radial.max(partial.radial);
    chord = chord.max(partial.chord);
    wall = wall.max(partial.wall);
    if model.cylinders.is_empty() && model.partial_cylinders.is_empty() {
        return Err(unsupported());
    }
    // Retained planes may represent an approximately coplanar source region.
    // Include every source vertex's distance to the actual output plane, even
    // when OCCT's own tighter acceptance tolerance will later reject it.
    let mut planar_residual: f64 = 0.;
    for (triangle, owner) in owners.iter().enumerate() {
        if let Some(MixedFaceRef::Plane { index }) = owner {
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
    // Every ring belongs to one real plane and one complete cylinder.
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
    let expected_volume = volume(&model)?;
    let deviation = options.weld_tolerance_mm + wall + radial + chord;
    if !deviation.is_finite() || deviation > options.max_deviation_mm {
        return Err(FabError::new("recognition.deviation_exceeded",format!(
        "Borne d'écart {:.6} mm supérieure au budget {:.6} mm. Augmenter explicitement le budget ou utiliser le mode planaire.",deviation,options.max_deviation_mm)));
    }
    let cylinders = model.cylinders.len() + model.partial_cylinders.len();
    let planes = model.planes.len();
    Ok(MixedRecognition{model,triangle_faces:owners.into_iter().map(Option::unwrap).collect(),
        report:RecognitionReport{cylinders,cones:0,spheres:0,tori:0,toroidal_fillets:0,max_radial_residual_mm:radial,max_chord_error_mm:chord,max_wall_residual_mm:wall,
            max_deviation_bound_mm:deviation,expected_volume_mm3:expected_volume,diagnostics:vec![
                Diagnostic{code:"recognition.circular_intent_assumed".into(),severity:Severity::Warning,count:cylinders,
                    message:"Les profils polygonaux circulaires sont interprétés comme cercles dans le mode explicitement activé.".into()},
                Diagnostic{code:"recognition.mixed_coverage".into(),severity:Severity::Info,count:analysis.mesh.triangles.len(),
                    message:format!("Tous les triangles sont attribués à une face : {cylinders} parois cylindriques prouvées et {planes} faces planes conservées. Aucun fond ajouté.")},
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

/// Divergence theorem evaluated on the actual analytic boundary, not a mesh
/// volume plus a guessed cylindrical correction. A local origin limits loss
/// of precision under translation.
fn volume(model: &MixedModel) -> Result<f64, FabError> {
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
