//! Reconstruction of complete open circular arc walls, without invented caps.
use crate::math::*;
use crate::recognition::{distance_segment, Work};
use crate::{
    Analysis, CircularArc, FabError, MixedFaceRef, MixedLoop, MixedModel, MixedPartialCylinder,
    MixedSegment, Options,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::{PI, TAU};
type P2 = [f64; 2];
type Edge = (usize, usize);
const NATIVE_TOL: f64 = 1e-7;
const MIN_POINTS: usize = 8;
fn edge(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn unproven() -> FabError {
    FabError::new("recognition.partial_unproven","Un arc complet a été détecté, mais sa paroi, ses extrémités partagées ou ses deux génératrices ne sont pas entièrement prouvées. Utiliser le mode planaire.")
}

struct Frame {
    origin: V,
    axis: V,
    x: V,
    y: V,
}
impl Frame {
    fn new(origin: V, axis: V) -> Self {
        let k = (0..3)
            .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
            .unwrap();
        let mut h = [0.; 3];
        h[k] = 1.;
        let x = cross(h, axis);
        let x = scale(x, 1. / norm(x));
        Self {
            origin,
            axis,
            x,
            y: cross(axis, x),
        }
    }
    fn project(&self, p: V) -> (P2, f64) {
        let d = sub(p, self.origin);
        ([dot(d, self.x), dot(d, self.y)], dot(d, self.axis))
    }
    fn lift(&self, p: P2) -> V {
        std::array::from_fn(|i| self.origin[i] + self.x[i] * p[0] + self.y[i] * p[1])
    }
}
struct Fit {
    center: P2,
    radius: f64,
    sweep: f64,
    radial: f64,
    chord: f64,
}
struct Candidate {
    plane: usize,
    loop_index: usize,
    chain: Vec<usize>,
    center: V,
    normal: V,
    radius: f64,
}
struct Proven {
    pair: [usize; 2],
    arcs: [CircularArc; 2],
    rails: [Vec<usize>; 2],
    rail_planes: [usize; 2],
    triangles: BTreeSet<usize>,
    inner: bool,
    radial: f64,
    chord: f64,
    wall: f64,
}
pub(crate) struct Residual {
    pub radial: f64,
    pub chord: f64,
    pub wall: f64,
}

/// Circle constrained through both endpoint vertices. Center lies on their
/// perpendicular bisector, so neighboring source faces need no vertex motion.
fn constrained_circle(points: &[P2], tolerance: f64) -> Option<Fit> {
    let a = points[0];
    let b = *points.last()?;
    let chord = (b[0] - a[0]).hypot(b[1] - a[1]);
    if chord <= tolerance * 4. {
        return None;
    }
    let mid = [a[0] + (b[0] - a[0]) * 0.5, a[1] + (b[1] - a[1]) * 0.5];
    let x = [(b[0] - a[0]) / chord, (b[1] - a[1]) / chord];
    let y = [-x[1], x[0]];
    let mut numerator = Sum::default();
    let mut denominator = Sum::default();
    for p in points {
        let dx = (p[0] - mid[0]) / chord;
        let dy = (p[1] - mid[1]) / chord;
        let u = dx * x[0] + dy * x[1];
        let v = dx * y[0] + dy * y[1];
        numerator.add(v * (u * u + v * v - 0.25));
        denominator.add(2. * v * v);
    }
    if denominator.get() < 1e-12 {
        return None;
    }
    let offset = chord * numerator.get() / denominator.get();
    let center = [mid[0] + offset * y[0], mid[1] + offset * y[1]];
    let radius = (chord * 0.5).hypot(offset);
    if !radius.is_finite() {
        return None;
    }
    let radial = points
        .iter()
        .map(|p| ((p[0] - center[0]).hypot(p[1] - center[1]) - radius).abs())
        .fold(0., f64::max);
    if radial > tolerance {
        return None;
    }
    Some(Fit {
        center,
        radius,
        radial,
        sweep: 0.,
        chord: 0.,
    })
}

fn fit(points: &[P2], tolerance: f64) -> Option<Fit> {
    let f = constrained_circle(points, tolerance)?;
    let center = f.center;
    let mut sweep = Sum::default();
    let mut maximum: f64 = 0.;
    let mut sign = 0.;
    for pair in points.windows(2) {
        let p = [pair[0][0] - center[0], pair[0][1] - center[1]];
        let q = [pair[1][0] - center[0], pair[1][1] - center[1]];
        let angle = (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
        if angle.abs() < 1e-10 || angle.abs() > PI / 4. + 1e-12 {
            return None;
        }
        if sign == 0. {
            sign = angle.signum();
        }
        if angle.signum() != sign {
            return None;
        }
        sweep.add(angle);
        maximum = maximum.max(angle.abs());
    }
    Some(Fit {
        center,
        radius: f.radius,
        sweep: sweep.get(),
        radial: f.radial,
        chord: 2. * f.radius * (maximum / 4.).sin().powi(2),
    })
}
fn closed_circle(points: &[P2], tol: f64) -> bool {
    // Detect cocircularity before applying sampling rules: otherwise a sparse
    // complete circle can masquerade as a well-sampled partial cylinder.
    let Some(f) = constrained_circle(points, tol) else {
        return false;
    };
    let mut winding = Sum::default();
    let mut large = 0;
    let mut intermediate = false;
    for i in 0..points.len() {
        let p = points[i];
        let q = points[(i + 1) % points.len()];
        let p = [p[0] - f.center[0], p[1] - f.center[1]];
        let q = [q[0] - f.center[0], q[1] - f.center[1]];
        let angle = (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
        winding.add(angle);
        if angle.abs() >= PI / 2. - 1e-12 {
            large += 1;
        } else if angle.abs() > PI / 4. + 1e-12 {
            intermediate = true;
        }
    }
    // Explicit cylindrical mode permits a D profile: one isolated chord
    // spanning >=90 degrees, all remaining steps <=45 degrees. A loop with
    // two sampling gaps is still a complete circle, never a convenient arc.
    let isolated_chord = large == 1 && !intermediate;
    winding.get().abs() > TAU - 1e-8 && !isolated_chord
}

fn candidates(
    model: &MixedModel,
    o: &Options,
    work: &mut Work,
) -> Result<Vec<Candidate>, FabError> {
    let mut result = Vec::new();
    for (plane_index, plane) in model.planes.iter().enumerate() {
        let normal = scale(plane.normal, 1. / norm(plane.normal));
        for (loop_index, lp) in plane.loops.iter().enumerate() {
            let MixedLoop::Polygon { vertices } = lp else {
                continue;
            };
            if vertices.len() < MIN_POINTS {
                continue;
            }
            let frame = Frame::new(model.vertices[vertices[0]], normal);
            let points: Vec<P2> = vertices
                .iter()
                .map(|&v| frame.project(model.vertices[v]).0)
                .collect();
            work.spend(points.len())?;
            if closed_circle(&points, o.cylinder_fit_tolerance_mm) {
                continue;
            }
            let mut found: Vec<(Vec<usize>, Fit)> = Vec::new();
            for start in 0..vertices.len() {
                let mut best = None;
                for length in MIN_POINTS..=vertices.len() {
                    work.spend(length)?;
                    let chain: Vec<P2> = (0..length)
                        .map(|k| points[(start + k) % points.len()])
                        .collect();
                    let Some(f) = fit(&chain, o.cylinder_fit_tolerance_mm) else {
                        break;
                    };
                    if f.sweep.abs() > 330_f64.to_radians() {
                        break;
                    }
                    if f.sweep.abs() >= 30_f64.to_radians() {
                        best = Some((length, f));
                    }
                }
                if let Some((length, f)) = best {
                    let ids = (0..length)
                        .map(|k| vertices[(start + k) % vertices.len()])
                        .collect();
                    found.push((ids, f));
                }
            }
            // Keep maximal evidence only. A defective wall cannot be retried
            // with a shorter, convenient subset of an already detected arc.
            let sets: Vec<BTreeSet<Edge>> = found
                .iter()
                .map(|(chain, _)| chain.windows(2).map(|p| edge(p[0], p[1])).collect())
                .collect();
            for i in 0..found.len() {
                let mut contained = false;
                for j in 0..found.len() {
                    work.spend(sets[i].len())?;
                    if i != j
                        && sets[i].is_subset(&sets[j])
                        && (sets[i].len() < sets[j].len() || j < i)
                    {
                        contained = true;
                        break;
                    }
                }
                if contained {
                    continue;
                }
                let (chain, f) = &found[i];
                result.push(Candidate {
                    plane: plane_index,
                    loop_index,
                    chain: chain.clone(),
                    center: frame.lift(f.center),
                    normal,
                    radius: f.radius,
                });
            }
        }
    }
    // Overlapping maximal arcs on a source boundary are ambiguous.
    for i in 0..result.len() {
        for j in i + 1..result.len() {
            work.spend(1)?;
            if result[i].plane == result[j].plane && result[i].loop_index == result[j].loop_index {
                work.spend(result[i].chain.len() + result[j].chain.len())?;
                let first: BTreeSet<Edge> = result[i]
                    .chain
                    .windows(2)
                    .map(|p| edge(p[0], p[1]))
                    .collect();
                if result[j]
                    .chain
                    .windows(2)
                    .any(|p| first.contains(&edge(p[0], p[1])))
                {
                    return Err(unproven());
                }
            }
        }
    }
    Ok(result)
}

pub(crate) fn augment(
    a: &Analysis,
    model: &mut MixedModel,
    owners: &mut [Option<MixedFaceRef>],
    o: &Options,
    work: &mut Work,
) -> Result<Residual, FabError> {
    let candidates = candidates(model, o, work)?;
    if candidates.is_empty() {
        return Ok(Residual {
            radial: 0.,
            chord: 0.,
            wall: 0.,
        });
    }
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let mut edges: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    let mut vertex_triangles = vec![Vec::new(); a.mesh.vertices.len()];
    for (t, &[a, b, c]) in a.mesh.triangles.iter().enumerate() {
        for v in [a, b, c] {
            vertex_triangles[v].push(t);
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            edges.entry(edge(u, v)).or_default().push(t);
        }
    }
    let mut matches = vec![0_usize; candidates.len()];
    let mut patches = Vec::new();
    for i in 0..candidates.len() {
        for j in i + 1..candidates.len() {
            work.spend(1)?;
            if let Some(p) = prove(
                a,
                owners,
                &candidates,
                [i, j],
                &adjacency,
                &edges,
                &vertex_triangles,
                o,
                work,
            )? {
                matches[i] += 1;
                matches[j] += 1;
                patches.push(p);
            }
        }
    }
    if matches.iter().any(|&n| n != 1) {
        return Err(unproven());
    }
    let mut replacements: BTreeMap<usize, Vec<Replacement>> = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    let mut residual = Residual {
        radial: 0.,
        chord: 0.,
        wall: 0.,
    };
    for patch in patches {
        let index = model.partial_cylinders.len();
        let arc0 = model.arcs.len();
        for end in 0..2 {
            let candidate = &candidates[patch.pair[end]];
            let arc = &patch.arcs[end];
            let reversed = candidate.chain[0] != arc.vertices[0];
            replacements
                .entry(candidate.plane)
                .or_default()
                .push(Replacement {
                    chain: candidate.chain.clone(),
                    segment: MixedSegment::Arc {
                        arc: arc0 + end,
                        reversed,
                    },
                });
            replacements
                .entry(patch.rail_planes[end])
                .or_default()
                .push(Replacement {
                    chain: patch.rails[end].clone(),
                    segment: MixedSegment::Line {
                        vertices: [patch.rails[end][0], *patch.rails[end].last().unwrap()],
                    },
                });
        }
        model.arcs.extend(patch.arcs);
        model.partial_cylinders.push(MixedPartialCylinder {
            arcs: [arc0, arc0 + 1],
            inner: patch.inner,
        });
        for &triangle in &patch.triangles {
            let Some(MixedFaceRef::Plane { index: plane }) = &owners[triangle] else {
                return Err(unproven());
            };
            deleted.insert(*plane);
            owners[triangle] = Some(MixedFaceRef::PartialCylinder { index });
        }
        residual.radial = residual.radial.max(patch.radial);
        residual.chord = residual.chord.max(patch.chord);
        residual.wall = residual.wall.max(patch.wall);
    }
    for (plane, replacements) in replacements {
        if deleted.contains(&plane) {
            return Err(unproven());
        }
        for lp in &mut model.planes[plane].loops {
            if let MixedLoop::Polygon { vertices } = lp {
                if let Some(segments) = replace_loop(vertices, &replacements, work)? {
                    *lp = MixedLoop::Segments { segments };
                }
            }
        }
    }
    let mut mapping = vec![None; model.planes.len()];
    let mut kept = Vec::new();
    for (old, plane) in std::mem::take(&mut model.planes).into_iter().enumerate() {
        if !deleted.contains(&old) {
            mapping[old] = Some(kept.len());
            kept.push(plane);
        }
    }
    model.planes = kept;
    for owner in owners.iter_mut().flatten() {
        if let MixedFaceRef::Plane { index } = owner {
            *index = mapping[*index].ok_or_else(unproven)?;
        }
    }
    Ok(residual)
}

#[allow(clippy::too_many_arguments)]
fn prove(
    a: &Analysis,
    owners: &[Option<MixedFaceRef>],
    candidates: &[Candidate],
    pair: [usize; 2],
    adjacency: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    vertex_triangles: &[Vec<usize>],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Proven>, FabError> {
    let (lower, upper) = (&candidates[pair[0]], &candidates[pair[1]]);
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    if lower.plane == upper.plane
        || lower.chain.len() != upper.chain.len()
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
    if norm(sub(delta, scale(axis, height))) > tol {
        return Ok(None);
    }
    let frame = Frame::new(lower.center, axis);
    work.spend(a.mesh.vertices.len())?;
    let projected: Vec<(P2, f64)> = a.mesh.vertices.iter().map(|&p| frame.project(p)).collect();
    let mut bottom = lower.chain.clone();
    let mut top = upper.chain.clone();
    let mut points: Vec<P2> = bottom.iter().map(|&v| projected[v].0).collect();
    let initial = fit(&points, o.cylinder_fit_tolerance_mm).ok_or_else(unproven)?;
    if initial.sweep < 0. {
        bottom.reverse();
        points.reverse();
    }
    let points_top: Vec<P2> = top.iter().map(|&v| projected[v].0).collect();
    let Some(top_fit) = fit(&points_top, o.cylinder_fit_tolerance_mm) else {
        return Ok(None);
    };
    if top_fit.sweep < 0. {
        top.reverse();
    }
    let upper_center = std::array::from_fn(|i| lower.center[i] + axis[i] * height);
    // Endpoints must remain the exact shared source vertices. No opportunistic
    // projection is used to force the two analytic arcs to agree.
    let mut correspondence: f64 = 0.;
    let mut cap: f64 = 0.;
    let mut radial: f64 = 0.;
    for i in 0..bottom.len() {
        let (b, bz) = projected[bottom[i]];
        let (t, tz) = projected[top[i]];
        let distance = (b[0] - t[0]).hypot(b[1] - t[1]);
        correspondence = correspondence.max(distance);
        if distance > tol {
            return Ok(None);
        }
        if (i == 0 || i + 1 == bottom.len())
            && (distance > NATIVE_TOL || bz.abs() > NATIVE_TOL || (tz - height).abs() > NATIVE_TOL)
        {
            return Ok(None);
        }
        cap = cap.max(bz.abs()).max((tz - height).abs());
        radial = radial
            .max((b[0].hypot(b[1]) - lower.radius).abs())
            .max((t[0].hypot(t[1]) - lower.radius).abs());
    }
    if radial > o.cylinder_fit_tolerance_mm || cap > tol {
        return Ok(None);
    }
    let mut arc_edges = BTreeSet::new();
    let mut starts = Vec::new();
    for (n, chain) in [&bottom, &top].into_iter().enumerate() {
        let plane = if n == 0 { lower.plane } else { upper.plane };
        for p in chain.windows(2) {
            let k = edge(p[0], p[1]);
            arc_edges.insert(k);
            let opposite: Vec<usize> = edges[&k]
                .iter()
                .copied()
                .filter(|&t| !is_plane(&owners[t], plane))
                .collect();
            if opposite.len() != 1 {
                return Ok(None);
            }
            if n == 0 {
                starts.push(opposite[0]);
            }
        }
    }
    let first = starts[0];
    let [p, q, r] = a.mesh.triangles[first].map(|v| a.mesh.vertices[v]);
    let normal = cross(sub(q, p), sub(r, p));
    let midpoint: V = std::array::from_fn(|i| (p[i] + q[i] + r[i]) / 3. - lower.center[i]);
    let radial_mid = sub(midpoint, scale(axis, dot(midpoint, axis)));
    let sign = dot(normal, radial_mid);
    if sign == 0. {
        return Ok(None);
    }
    let inner = sign < 0.;
    let mut selected = BTreeSet::new();
    let mut examined = BTreeSet::new();
    let mut queue = VecDeque::from([first]);
    let mut wall: f64 = 0.;
    while let Some(t) = queue.pop_front() {
        if !examined.insert(t) {
            continue;
        }
        if !matches!(owners[t], Some(MixedFaceRef::Plane { .. }))
            || is_plane(&owners[t], lower.plane)
            || is_plane(&owners[t], upper.plane)
        {
            continue;
        }
        let tri = a.mesh.triangles[t];
        let positions = tri.map(|v| projected[v]);
        if positions
            .iter()
            .any(|(_, z)| *z < -tol || *z > height + tol)
        {
            continue;
        }
        let [p, q, r] = tri.map(|v| a.mesh.vertices[v]);
        let normal = cross(sub(q, p), sub(r, p));
        let mut best = f64::INFINITY;
        for segment in points.windows(2) {
            work.spend(1)?;
            let dx = segment[1][0] - segment[0][0];
            let dy = segment[1][1] - segment[0][1];
            let outward: V = std::array::from_fn(|i| {
                (frame.x[i] * dy - frame.y[i] * dx) * if inner { -1. } else { 1. }
            });
            if dot(normal, outward) <= 0. {
                continue;
            }
            let error = positions
                .iter()
                .map(|(p, z)| {
                    distance_segment(*p, segment[0], segment[1])
                        .hypot((-*z).max(*z - height).max(0.))
                })
                .fold(0., f64::max);
            best = best.min(error);
        }
        if best > tol {
            continue;
        }
        selected.insert(t);
        wall = wall.max(best);
        queue.extend(adjacency[t].iter().copied());
    }
    if starts.iter().any(|t| !selected.contains(t)) {
        return Ok(None);
    }
    let mut boundary = BTreeSet::new();
    let mut consumed_planes = BTreeSet::new();
    for &t in &selected {
        if let Some(MixedFaceRef::Plane { index }) = owners[t] {
            consumed_planes.insert(index);
        }
        let [a, b, c] = a.mesh.triangles[t];
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let k = edge(u, v);
            if edges[&k].iter().any(|other| !selected.contains(other)) {
                boundary.insert(k);
            }
        }
    }
    if !arc_edges.is_subset(&boundary) {
        return Ok(None);
    }
    for k in &arc_edges {
        boundary.remove(k);
    }
    let Some((rail0, plane0)) = rail(
        bottom[0],
        top[0],
        &mut boundary,
        &selected,
        owners,
        edges,
        vertex_triangles,
        &a.mesh.vertices,
        work,
    )?
    else {
        return Ok(None);
    };
    let Some((rail1, plane1)) = rail(
        *bottom.last().unwrap(),
        *top.last().unwrap(),
        &mut boundary,
        &selected,
        owners,
        edges,
        vertex_triangles,
        &a.mesh.vertices,
        work,
    )?
    else {
        return Ok(None);
    };
    if !boundary.is_empty() {
        return Ok(None);
    }
    for (t, owner) in owners.iter().enumerate() {
        work.spend(1)?;
        if let Some(MixedFaceRef::Plane { index }) = owner {
            if consumed_planes.contains(index) && !selected.contains(&t) {
                return Ok(None);
            }
        }
    }
    let final_fit = fit(&points, o.cylinder_fit_tolerance_mm).ok_or_else(unproven)?;
    // The constrained fit center is identical to the candidate circle up to
    // rounding; reject any material discrepancy instead of moving endpoints.
    if final_fit.center[0].hypot(final_fit.center[1]) > NATIVE_TOL {
        return Ok(None);
    }
    let arcs = [
        CircularArc {
            center: lower.center,
            axis,
            radius_mm: lower.radius,
            sweep_rad: final_fit.sweep,
            vertices: [bottom[0], *bottom.last().unwrap()],
        },
        CircularArc {
            center: upper_center,
            axis,
            radius_mm: lower.radius,
            sweep_rad: final_fit.sweep,
            vertices: [top[0], *top.last().unwrap()],
        },
    ];
    Ok(Some(Proven {
        pair,
        arcs,
        rails: [rail0, rail1],
        rail_planes: [plane0, plane1],
        triangles: selected,
        inner,
        radial: radial.max(final_fit.radial),
        chord: final_fit.chord,
        wall: wall + cap + correspondence,
    }))
}
fn is_plane(owner: &Option<MixedFaceRef>, plane: usize) -> bool {
    matches!(owner,Some(MixedFaceRef::Plane{index}) if *index==plane)
}

#[allow(clippy::too_many_arguments)]
fn rail(
    start: usize,
    end: usize,
    boundary: &mut BTreeSet<Edge>,
    patch: &BTreeSet<usize>,
    owners: &[Option<MixedFaceRef>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    vertex_triangles: &[Vec<usize>],
    vertices: &[V],
    work: &mut Work,
) -> Result<Option<(Vec<usize>, usize)>, FabError> {
    let mut chain = vec![start];
    let mut current = start;
    let mut neighboring_plane = None;
    let mut visited = BTreeSet::new();
    work.spend(boundary.len())?;
    let mut incident: BTreeMap<usize, Vec<Edge>> = BTreeMap::new();
    for &k in boundary.iter() {
        incident.entry(k.0).or_default().push(k);
        incident.entry(k.1).or_default().push(k);
    }
    while current != end {
        let local = incident.get(&current).map(Vec::as_slice).unwrap_or(&[]);
        work.spend(local.len())?;
        let choices: Vec<Edge> = local
            .iter()
            .copied()
            .filter(|k| boundary.contains(k))
            .collect();
        if choices.len() != 1 {
            return Ok(None);
        }
        let k = choices[0];
        boundary.remove(&k);
        let next = if k.0 == current { k.1 } else { k.0 };
        if !visited.insert(next) {
            return Ok(None);
        }
        let outside: Vec<usize> = edges[&k]
            .iter()
            .copied()
            .filter(|t| !patch.contains(t))
            .collect();
        if outside.len() != 1 {
            return Ok(None);
        }
        let Some(MixedFaceRef::Plane { index }) = owners[outside[0]] else {
            return Ok(None);
        };
        if neighboring_plane.is_some_and(|old| old != index) {
            return Ok(None);
        }
        neighboring_plane = Some(index);
        chain.push(next);
        current = next;
    }
    let plane = neighboring_plane.ok_or_else(unproven)?;
    let origin = vertices[start];
    let delta = sub(vertices[end], origin);
    let length = norm(delta);
    if length == 0. {
        return Ok(None);
    }
    let direction = scale(delta, 1. / length);
    let mut previous = f64::NEG_INFINITY;
    for &v in &chain {
        let d = sub(vertices[v], origin);
        let t = dot(d, direction);
        if t <= previous
            || t < -NATIVE_TOL
            || t > length + NATIVE_TOL
            || norm(sub(d, scale(direction, t))) > NATIVE_TOL
        {
            return Ok(None);
        }
        previous = t;
    }
    for &v in &chain[1..chain.len() - 1] {
        work.spend(vertex_triangles[v].len())?;
        if vertex_triangles[v]
            .iter()
            .any(|t| !patch.contains(t) && !is_plane(&owners[*t], plane))
        {
            return Ok(None);
        }
    }
    Ok(Some((chain, plane)))
}

struct Replacement {
    chain: Vec<usize>,
    segment: MixedSegment,
}
fn reversed(segment: &MixedSegment) -> MixedSegment {
    match segment {
        MixedSegment::Line { vertices } => MixedSegment::Line {
            vertices: [vertices[1], vertices[0]],
        },
        MixedSegment::Arc { arc, reversed } => MixedSegment::Arc {
            arc: *arc,
            reversed: !*reversed,
        },
    }
}
fn replace_loop(
    vertices: &[usize],
    replacements: &[Replacement],
    work: &mut Work,
) -> Result<Option<Vec<MixedSegment>>, FabError> {
    for replacement in replacements {
        work.spend(vertices.len().saturating_mul(replacement.chain.len()))?;
    }
    let applicable: Vec<&Replacement> = replacements
        .iter()
        .filter(|r| r.chain.iter().all(|v| vertices.contains(v)))
        .collect();
    if applicable.is_empty() {
        return Ok(None);
    }
    let interior: BTreeSet<usize> = applicable
        .iter()
        .flat_map(|r| r.chain[1..r.chain.len() - 1].iter().copied())
        .collect();
    let start = (0..vertices.len())
        .find(|&i| !interior.contains(&vertices[i]))
        .ok_or_else(unproven)?;
    let mut traversed = 0;
    let mut segments = Vec::new();
    let mut used = 0;
    while traversed < vertices.len() {
        let i = (start + traversed) % vertices.len();
        let mut replacement = None;
        for r in &applicable {
            for reverse in [false, true] {
                work.spend(r.chain.len())?;
                let chain: Vec<usize> = if reverse {
                    r.chain.iter().rev().copied().collect()
                } else {
                    r.chain.clone()
                };
                if chain
                    .iter()
                    .enumerate()
                    .all(|(k, &v)| vertices[(i + k) % vertices.len()] == v)
                {
                    replacement = Some((
                        chain.len() - 1,
                        if reverse {
                            reversed(&r.segment)
                        } else {
                            r.segment.clone()
                        },
                    ));
                    break;
                }
            }
            if replacement.is_some() {
                break;
            }
        }
        if let Some((length, segment)) = replacement {
            segments.push(segment);
            traversed += length;
            used += 1;
        } else {
            segments.push(MixedSegment::Line {
                vertices: [vertices[i], vertices[(i + 1) % vertices.len()]],
            });
            traversed += 1;
        }
    }
    if traversed != vertices.len() || used != applicable.len() {
        return Err(unproven());
    }
    Ok(Some(segments))
}

pub(crate) fn loop_origin(model: &MixedModel, lp: &MixedLoop) -> V {
    match lp {
        MixedLoop::Polygon { vertices } => model.vertices[vertices[0]],
        MixedLoop::Circle { ring } => model.rings[*ring].center,
        MixedLoop::Segments { segments } => match &segments[0] {
            MixedSegment::Line { vertices } => model.vertices[vertices[0]],
            MixedSegment::Arc { arc, reversed } => {
                model.vertices[model.arcs[*arc].vertices[usize::from(*reversed)]]
            }
        },
    }
}
pub(crate) fn segmented_area(model: &MixedModel, segments: &[MixedSegment], normal: V) -> f64 {
    let origin = loop_origin(
        model,
        &MixedLoop::Segments {
            segments: segments.to_vec(),
        },
    );
    let mut sum = Sum::default();
    for segment in segments {
        let (ids, correction) = match segment {
            MixedSegment::Line { vertices } => (*vertices, 0.),
            MixedSegment::Arc { arc, reversed } => {
                let a = &model.arcs[*arc];
                let signed = (if *reversed { -1. } else { 1. }) * dot(a.axis, normal).signum();
                (
                    if *reversed {
                        [a.vertices[1], a.vertices[0]]
                    } else {
                        a.vertices
                    },
                    signed * a.radius_mm.powi(2) * (a.sweep_rad - a.sweep_rad.sin()) / 2.,
                )
            }
        };
        sum.add(
            dot(
                cross(
                    sub(model.vertices[ids[0]], origin),
                    sub(model.vertices[ids[1]], origin),
                ),
                normal,
            ) / 2.
                + correction,
        );
    }
    sum.get()
}
pub(crate) fn cylinder_flux(
    model: &MixedModel,
    cylinder: &MixedPartialCylinder,
    reference: V,
) -> f64 {
    let a = &model.arcs[cylinder.arcs[0]];
    let b = &model.arcs[cylinder.arcs[1]];
    let height = dot(sub(b.center, a.center), a.axis);
    let x = scale(
        sub(model.vertices[a.vertices[0]], a.center),
        1. / a.radius_mm,
    );
    let y = cross(a.axis, x);
    let theta = a.sweep_rad;
    let integral: V = std::array::from_fn(|i| x[i] * theta.sin() + y[i] * (1. - theta.cos()));
    (if cylinder.inner { -1. } else { 1. })
        * a.radius_mm
        * height
        * (a.radius_mm * theta + dot(sub(a.center, reference), integral))
        / 3.
}
