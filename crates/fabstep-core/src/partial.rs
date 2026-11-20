//! Reconstruction of complete open circular arc walls, without invented caps.
use crate::math::*;
use crate::recognition::{distance_segment, Work};
use crate::{
    Analysis, CircularArc, FabError, MixedFaceRef, MixedLoop, MixedModel, MixedPartialCylinder,
    MixedSegment, Options,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::{PI, TAU};
pub(crate) mod chain;
mod normal_bands;
mod short;

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
struct Found {
    start: usize,
    chain: Vec<usize>,
    fit: Fit,
}

#[derive(Clone, Copy)]
enum MaximalityMode {
    Adaptive,
    #[cfg(test)]
    Historical,
}

enum MaximalEvidence {
    Cyclic { vertices: usize },
    Historical(Vec<BTreeSet<Edge>>),
}

fn maximal_work(work: &mut Work, count: usize) -> Result<(), FabError> {
    work.spend(count)
}

/// Establish a bijection between source edge positions and undirected edges.
/// A simple cycle with at least three distinct vertices has no repeated edge.
/// Sort a fallibly allocated copy, charging copies, comparisons and exchanges;
/// heap sort bounds both memory and work even for adversarial vertex indices.
fn unique_source_vertices(vertices: &[usize], work: &mut Work) -> Result<bool, FabError> {
    fn sift(values: &mut [usize], mut root: usize, work: &mut Work) -> Result<(), FabError> {
        while let Some(left) = root.checked_mul(2).and_then(|v| v.checked_add(1)) {
            if left >= values.len() {
                break;
            }
            let mut child = left;
            if left + 1 < values.len() {
                maximal_work(work, 1)?;
                if values[left] < values[left + 1] {
                    child = left + 1;
                }
            }
            maximal_work(work, 1)?;
            if values[root] >= values[child] {
                break;
            }
            maximal_work(work, 1)?;
            values.swap(root, child);
            root = child;
        }
        Ok(())
    }
    if vertices.len() < 3 {
        return Ok(false);
    }
    maximal_work(work, vertices.len())?;
    let mut sorted = Vec::new();
    sorted.try_reserve_exact(vertices.len()).map_err(|_| {
        FabError::new(
            "recognition.too_complex",
            "Capacité de maximalité des arcs dépassée.",
        )
    })?;
    sorted.extend_from_slice(vertices);
    for root in (0..sorted.len() / 2).rev() {
        sift(&mut sorted, root, work)?;
    }
    for end in (1..sorted.len()).rev() {
        maximal_work(work, 1)?;
        sorted.swap(0, end);
        sift(&mut sorted[..end], 0, work)?;
    }
    for pair in sorted.windows(2) {
        maximal_work(work, 1)?;
        if pair[0] == pair[1] {
            return Ok(false);
        }
    }
    Ok(true)
}

impl MaximalEvidence {
    fn new(
        vertices: &[usize],
        found: &[Found],
        mode: MaximalityMode,
        work: &mut Work,
    ) -> Result<Self, FabError> {
        if matches!(mode, MaximalityMode::Adaptive)
            && found.len() > 1
            && unique_source_vertices(vertices, work)?
        {
            return Ok(Self::Cyclic {
                vertices: vertices.len(),
            });
        }
        Ok(Self::Historical(
            found
                .iter()
                .map(|f| f.chain.windows(2).map(|p| edge(p[0], p[1])).collect())
                .collect(),
        ))
    }

    fn dominates(
        &self,
        found: &[Found],
        i: usize,
        j: usize,
        work: &mut Work,
    ) -> Result<bool, FabError> {
        match self {
            Self::Cyclic { vertices } => {
                maximal_work(work, 1)?;
                let a = &found[i];
                let b = &found[j];
                let a_count = a.chain.len() - 1;
                let b_count = b.chain.len() - 1;
                if i == j || a_count > b_count || (a_count == b_count && j > i) {
                    return Ok(false);
                }
                // Every chain has <= n vertices, hence < n edges. Its edge
                // set is exactly the half-open cyclic interval [start,start+m).
                let offset = if a.start >= b.start {
                    a.start - b.start
                } else {
                    vertices - (b.start - a.start)
                };
                Ok(offset <= b_count - a_count)
            }
            Self::Historical(sets) => {
                maximal_work(work, sets[i].len())?;
                Ok(i != j
                    && sets[i].is_subset(&sets[j])
                    && (sets[i].len() < sets[j].len() || j < i))
            }
        }
    }
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
#[derive(Default)]
pub(crate) struct Residual {
    pub rejected: usize,
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
fn closed_circle(points: &[P2], tol: f64) -> Option<bool> {
    // Detect cocircularity before applying sampling rules: otherwise a sparse
    // complete circle can masquerade as a well-sampled partial cylinder.
    let f = constrained_circle(points, tol)?;
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
    Some(winding.get().abs() > TAU - 1e-8 && !isolated_chord)
}

fn candidates(
    model: &MixedModel,
    o: &Options,
    work: &mut Work,
) -> Result<Vec<Candidate>, FabError> {
    candidates_with_maximality(model, o, work, MaximalityMode::Adaptive)
}

fn candidates_with_maximality(
    model: &MixedModel,
    o: &Options,
    work: &mut Work,
    maximality: MaximalityMode,
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
            match closed_circle(&points, o.cylinder_fit_tolerance_mm) {
                Some(true) => continue,
                Some(false) => (),
                None => {
                    // Adjacent endpoints amplify quantization on dense circles.
                    // Only when the constrained fit fails, reuse the existing
                    // all-vertex circle proof, with sampling and full winding.
                    // Keep every historical angular/D-profile decision intact.
                    work.spend(points.len())?;
                    if crate::recognition::fit_circle(&points, o.cylinder_fit_tolerance_mm)
                        .is_some()
                    {
                        continue;
                    }
                }
            }
            let mut found = Vec::new();
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
                    found.push(Found {
                        start,
                        chain: ids,
                        fit: f,
                    });
                }
            }
            // Keep maximal evidence only. A defective wall cannot be retried
            // with a shorter, convenient subset of an already detected arc.
            let evidence = MaximalEvidence::new(vertices, &found, maximality, work)?;
            for i in 0..found.len() {
                let mut contained = false;
                for j in 0..found.len() {
                    if evidence.dominates(&found, i, j, work)? {
                        contained = true;
                        break;
                    }
                }
                if contained {
                    continue;
                }
                let Found { chain, fit: f, .. } = &found[i];
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
    // Endpoint-constrained fits are not monotone as the endpoint advances.
    // Recover overlapping fragments only by proving their complete ordered union;
    // never choose a smaller convenient subset of a failed boundary candidate.
    merge_candidates(&mut result, model, o, work)?;
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

/// Bounded recovery after the conservative local candidate search. A failed
/// union remains ambiguous; no tolerance is enlarged and no candidate discarded.
fn merge_candidates(
    candidates: &mut Vec<Candidate>,
    model: &MixedModel,
    o: &Options,
    work: &mut Work,
) -> Result<(), FabError> {
    let mut visited = BTreeSet::new();
    work.spend(candidates.iter().map(|c| c.chain.len()).sum())?;
    let edge_sets: Vec<BTreeSet<Edge>> = candidates
        .iter()
        .map(|candidate| {
            candidate
                .chain
                .windows(2)
                .map(|p| edge(p[0], p[1]))
                .collect()
        })
        .collect();
    let mut removed = BTreeSet::new();
    for seed in 0..candidates.len() {
        if !visited.insert(seed) {
            continue;
        }
        let mut group = vec![seed];
        let mut next = 0;
        while next < group.len() {
            let i = group[next];
            next += 1;
            for j in 0..candidates.len() {
                work.spend(1)?;
                if visited.contains(&j)
                    || candidates[i].plane != candidates[j].plane
                    || candidates[i].loop_index != candidates[j].loop_index
                {
                    continue;
                }
                work.spend(edge_sets[i].len().min(edge_sets[j].len()))?;
                if !edge_sets[i].is_disjoint(&edge_sets[j]) {
                    visited.insert(j);
                    group.push(j);
                }
            }
        }
        if group.len() == 1 {
            continue;
        }
        let a = &candidates[seed];
        work.spend(group.iter().map(|&i| candidates[i].chain.len()).sum())?;
        let ids: BTreeSet<usize> = group
            .iter()
            .flat_map(|&i| candidates[i].chain.iter().copied())
            .collect();
        let union_edges: BTreeSet<Edge> = group
            .iter()
            .flat_map(|&i| edge_sets[i].iter().copied())
            .collect();
        let MixedLoop::Polygon { vertices } = &model.planes[a.plane].loops[a.loop_index] else {
            return Err(unproven());
        };
        work.spend(vertices.len() + ids.len())?;
        let Some(start) = (0..vertices.len()).find(|&k| {
            union_edges.contains(&edge(vertices[k], vertices[(k + 1) % vertices.len()]))
                && !union_edges.contains(&edge(
                    vertices[k],
                    vertices[(k + vertices.len() - 1) % vertices.len()],
                ))
        }) else {
            continue;
        };
        let chain: Vec<usize> = (0..ids.len())
            .map(|k| vertices[(start + k) % vertices.len()])
            .collect();
        if chain.len() != union_edges.len() + 1
            || !chain.iter().all(|v| ids.contains(v))
            || !chain
                .windows(2)
                .all(|p| union_edges.contains(&edge(p[0], p[1])))
        {
            continue;
        }
        let frame = Frame::new(model.vertices[chain[0]], a.normal);
        let points: Vec<P2> = chain
            .iter()
            .map(|&v| frame.project(model.vertices[v]).0)
            .collect();
        work.spend(points.len())?;
        let Some(f) = fit(&points, o.cylinder_fit_tolerance_mm) else {
            continue;
        };
        if !(30_f64.to_radians()..=330_f64.to_radians()).contains(&f.sweep.abs()) {
            continue;
        }
        candidates[seed].chain = chain;
        candidates[seed].center = frame.lift(f.center);
        candidates[seed].radius = f.radius;
        removed.extend(group.into_iter().filter(|&i| i != seed));
    }
    let mut index = 0;
    candidates.retain(|_| {
        let keep = !removed.contains(&index);
        index += 1;
        keep
    });
    Ok(())
}

/// A rail can border one retained plane or one other proven cylindrical wall.
/// Shared rails require identical source chains and precisely those two wall
/// patches on every segment. Endpoint coincidence alone is not an incidence.
fn shared_rails(
    patches: &[Proven],
    edges: &BTreeMap<Edge, Vec<usize>>,
    triangle_count: usize,
    work: &mut Work,
) -> Result<BTreeSet<Edge>, FabError> {
    work.spend(triangle_count)?;
    let mut patch_owner = vec![None; triangle_count];
    let mut rails: BTreeMap<Edge, Vec<(usize, usize)>> = BTreeMap::new();
    for (index, patch) in patches.iter().enumerate() {
        work.spend(patch.triangles.len())?;
        for &t in &patch.triangles {
            if patch_owner[t].replace(index).is_some() {
                return Err(unproven());
            }
        }
        for (side, chain) in patch.rails.iter().enumerate() {
            rails
                .entry(edge(chain[0], *chain.last().unwrap()))
                .or_default()
                .push((index, side));
        }
    }
    let mut shared = BTreeSet::new();
    for (key, uses) in rails {
        work.spend(uses.len())?;
        if uses.len() == 1 {
            continue;
        }
        if uses.len() != 2 || uses[0].0 == uses[1].0 {
            return Err(unproven());
        }
        let a = &patches[uses[0].0].rails[uses[0].1];
        let b = &patches[uses[1].0].rails[uses[1].1];
        work.spend(a.len() + b.len())?;
        if a != b && !a.iter().eq(b.iter().rev()) {
            return Err(unproven());
        }
        for pair in a.windows(2) {
            work.spend(1)?;
            let adjacent = &edges[&edge(pair[0], pair[1])];
            if adjacent.len() != 2
                || !adjacent.iter().any(|&t| patch_owner[t] == Some(uses[0].0))
                || !adjacent.iter().any(|&t| patch_owner[t] == Some(uses[1].0))
            {
                return Err(unproven());
            }
        }
        shared.insert(key);
    }
    Ok(shared)
}

pub(crate) fn augment(
    a: &Analysis,
    model: &mut MixedModel,
    owners: &mut [Option<MixedFaceRef>],
    o: &Options,
    work: &mut Work,
    adjacency: &[Vec<usize>],
) -> Result<Residual, FabError> {
    augment_policy(a, model, owners, o, work, adjacency, false)
}
pub(crate) fn augment_auto(
    a: &Analysis,
    model: &mut MixedModel,
    owners: &mut [Option<MixedFaceRef>],
    o: &Options,
    work: &mut Work,
    adjacency: &[Vec<usize>],
) -> Result<Residual, FabError> {
    augment_policy(a, model, owners, o, work, adjacency, true)
}
fn augment_policy(
    a: &Analysis,
    model: &mut MixedModel,
    owners: &mut [Option<MixedFaceRef>],
    o: &Options,
    work: &mut Work,
    adjacency: &[Vec<usize>],
    automatic: bool,
) -> Result<Residual, FabError> {
    let mut candidates = candidates(model, o, work)?;
    let normal_ends = if candidates.is_empty() && automatic {
        normal_bands::ends(model, a.mesh.triangles.len(), work)
    } else {
        None
    };
    if candidates.is_empty() && normal_ends.is_none() {
        return Ok(Residual::default());
    }
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
    if let Some(ends) = normal_ends {
        candidates = normal_bands::nominate(ends, &a.mesh, model, owners, &edges, o);
        if candidates.is_empty() {
            return Ok(Residual::default());
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
                adjacency,
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
    if automatic {
        patches.retain(|p| matches[p.pair[0]] == 1 && matches[p.pair[1]] == 1);
    } else if matches.iter().any(|&n| n != 1) {
        return Err(unproven());
    }
    // Only historical, fully proven patches can support this second pass.
    // Newly admitted short arcs never seed another short-arc search.
    let short = if normal_ends.is_some() {
        Vec::new()
    } else {
        short::candidates(model, &candidates, &patches, o, work)?
    };
    let first_short = candidates.len();
    let short_supports: Vec<_> = short.iter().map(|c| c.supports).collect();
    candidates.extend(short.into_iter().map(|c| c.candidate));
    let mut short_matches = vec![0_usize; candidates.len() - first_short];
    for i in first_short..candidates.len() {
        for j in i + 1..candidates.len() {
            work.spend(1)?;
            if short_supports[i - first_short] != short_supports[j - first_short] {
                continue;
            }
            if let Some(p) = prove(
                a,
                owners,
                &candidates,
                [i, j],
                adjacency,
                &edges,
                &vertex_triangles,
                o,
                work,
            )? {
                short_matches[i - first_short] += 1;
                short_matches[j - first_short] += 1;
                patches.push(p);
            }
        }
    }
    if automatic {
        patches.retain(|p| {
            p.pair
                .iter()
                .all(|&i| i < first_short || short_matches[i - first_short] == 1)
        });
    } else if short_matches.iter().any(|&count| count != 1) {
        return Err(unproven());
    }
    let shared_rails = shared_rails(&patches, &edges, a.mesh.triangles.len(), work)?;
    let mut replacements: BTreeMap<usize, Vec<Replacement>> = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    let mut residual = Residual {
        rejected: if automatic {
            matches.iter().filter(|&&n| n != 1).count()
                + short_matches.iter().filter(|&&n| n != 1).count()
        } else {
            0
        },
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
            if !shared_rails.contains(&edge(
                patch.rails[end][0],
                *patch.rails[end].last().unwrap(),
            )) {
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
        }
        model.arcs.extend(patch.arcs);
        model.partial_cylinders.push(MixedPartialCylinder {
            elliptic_start: false,
            elliptic_end: false,
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
    let lateral = |b: usize, t: usize| {
        let b = projected[b].0;
        let t = projected[t].0;
        (b[0] - t[0]).hypot(b[1] - t[1])
    };
    for (&b, &t) in [bottom.first().unwrap(), bottom.last().unwrap()]
        .into_iter()
        .zip([top.first().unwrap(), top.last().unwrap()])
    {
        if lateral(b, t) > tol.min(NATIVE_TOL)
            || projected[b].1.abs() > NATIVE_TOL
            || (projected[t].1 - height).abs() > NATIVE_TOL
        {
            return Ok(None);
        }
    }
    let mut cap: f64 = 0.;
    let mut radial: f64 = 0.;
    for (chain, station) in [(&bottom, 0.), (&top, height)] {
        for &v in chain {
            let (p, z) = projected[v];
            cap = cap.max((z - station).abs());
            radial = radial.max((p[0].hypot(p[1]) - lower.radius).abs());
        }
    }
    // Matching cap samples can use the historical chord-prism proof. With
    // independent subdivisions only the source corners correspond: require
    // full rectangular UV coverage and radial witnesses on every triangle.
    let paired_drift = (bottom.len() == top.len())
        .then(|| {
            bottom
                .iter()
                .zip(&top)
                .map(|(&b, &t)| lateral(b, t))
                .fold(0., f64::max)
        })
        .filter(|&drift| drift <= tol);
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
    let mut queue = VecDeque::new();
    if paired_drift.is_some() {
        queue.push_back(first);
    }
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
    let mut curved_chord = 0_f64;
    // A stopped traversal may already cover the bottom ring (or both rings).
    // Check its entire boundary before deciding the prism proof was complete.
    let mut prism_complete = paired_drift.is_some();
    for &t in &selected {
        work.spend(3)?;
        let tri = a.mesh.triangles[t];
        for pair in [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
            let key = edge(pair[0], pair[1]);
            if arc_edges.contains(&key) || edges[&key].iter().all(|n| selected.contains(n)) {
                continue;
            }
            let on_rail = [points[0], *points.last().unwrap()].iter().any(|endpoint| {
                pair.iter().all(|&v| {
                    let p = projected[v].0;
                    (p[0] - endpoint[0]).hypot(p[1] - endpoint[1]) <= NATIVE_TOL
                })
            });
            prism_complete &= on_rail;
        }
    }
    for key in &arc_edges {
        work.spend(1)?;
        prism_complete &= edges[key].iter().any(|t| selected.contains(t));
    }
    if !prism_complete {
        // The historical prism proof permits source vertices on cap chords.
        // A different tessellation needs a separate proof with radial witnesses
        // at EVERY wall vertex; a defective prism is not accepted by a looser fit.
        if let Some(curved) = curved_wall(
            a,
            owners,
            adjacency,
            &projected,
            &frame,
            [lower.plane, upper.plane],
            first,
            points[0],
            initial.sweep.abs(),
            lower.radius,
            height,
            inner,
            o,
            work,
        )? {
            selected = curved.triangles;
            radial = radial.max(curved.radial);
            curved_chord = curved.chord;
            wall = curved.trim;
        }
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
    if !native_endpoints_match(&arcs, &a.mesh.vertices) {
        return Ok(None);
    }
    Ok(Some(Proven {
        pair,
        arcs,
        rails: [rail0, rail1],
        rail_planes: [plane0, plane1],
        triangles: selected,
        inner,
        radial: radial.max(final_fit.radial),
        chord: final_fit.chord.max(curved_chord),
        wall: wall + cap + paired_drift.unwrap_or(0.),
    }))
}

/// Match the four corners of the native cylinder's parameter rectangle, not
/// separate transverse/axial errors between source vertices. Its frame is the
/// lower arc's axis and projected start direction; its upper corners use the
/// projected height, just as partial_cylindrical_face constructs them.
fn native_endpoints_match(arcs: &[CircularArc; 2], vertices: &[V]) -> bool {
    let [lower, upper] = arcs;
    let unit = |v| scale(v, 1. / norm(v));
    let axis = unit(lower.axis);
    let start = unit(sub(vertices[lower.vertices[0]], lower.center));
    // gp_Ax3 orthogonalizes its supplied X direction with this double cross.
    let x = unit(cross(axis, cross(start, axis)));
    let y = unit(cross(axis, x));
    let height = dot(sub(upper.center, lower.center), axis);
    for (arc, z) in [(lower, 0.), (upper, height)] {
        for (id, angle) in arc.vertices.into_iter().zip([0., lower.sweep_rad]) {
            let (sin, cos) = angle.sin_cos();
            let corner = std::array::from_fn(|i| {
                lower.center[i] + lower.radius_mm * (cos * x[i] + sin * y[i]) + z * axis[i]
            });
            // A nonfinite distance also fails this comparison.
            if norm(sub(vertices[id], corner)) <= NATIVE_TOL {
                continue;
            }
            return false;
        }
    }
    true
}
struct CurvedWall {
    triangles: BTreeSet<usize>,
    radial: f64,
    chord: f64,
    trim: f64,
}

/// Project a connected source wall onto its cylindrical parameter rectangle.
/// Each accepted triangle has strictly oriented radial-projection Jacobian
/// throughout its interior: its numerator is affine, checked at all vertices.
/// The caller must still prove the sole boundary is the two source cap chains
/// and two straight rails. Their degree-one boundary, internal-edge cancellation
/// and positive Jacobians then prove full coverage without folds or overlaps.
#[allow(clippy::too_many_arguments)]
fn curved_wall(
    a: &Analysis,
    owners: &[Option<MixedFaceRef>],
    adjacency: &[Vec<usize>],
    projected: &[(P2, f64)],
    frame: &Frame,
    caps: [usize; 2],
    first: usize,
    start: P2,
    sweep: f64,
    radius: f64,
    height: f64,
    inner: bool,
    o: &Options,
    work: &mut Work,
) -> Result<Option<CurvedWall>, FabError> {
    let mut triangles = BTreeSet::new();
    let mut examined = BTreeSet::new();
    let mut queue = VecDeque::from([first]);
    let mut radial = 0_f64;
    let mut maximum_span = 0_f64;
    let mut trim = 0_f64;
    let mut area = Sum::default();
    let orientation = if inner { -1. } else { 1. };
    // Only native numerical slack at the exact rails/cap planes, not the
    // user radial tolerance, is allowed outside the parameter rectangle.
    let angle_slack = NATIVE_TOL / radius;
    while let Some(t) = queue.pop_front() {
        if !examined.insert(t) {
            continue;
        }
        work.spend(1)?;
        if !matches!(owners[t], Some(MixedFaceRef::Plane { .. }))
            || caps.iter().any(|&plane| is_plane(&owners[t], plane))
        {
            continue;
        }
        let ids = a.mesh.triangles[t];
        let xyz = ids.map(|v| a.mesh.vertices[v]);
        let normal = cross(sub(xyz[1], xyz[0]), sub(xyz[2], xyz[0]));
        let positions = ids.map(|v| projected[v]);
        let uv = positions.map(|(p, z)| {
            let mut angle =
                (start[0] * p[1] - start[1] * p[0]).atan2(start[0] * p[0] + start[1] * p[1]);
            if angle < -angle_slack {
                angle += TAU;
            }
            [angle, z]
        });
        let residual = positions
            .iter()
            .map(|(p, _)| (p[0].hypot(p[1]) - radius).abs())
            .fold(0_f64, f64::max);
        if residual > o.cylinder_fit_tolerance_mm
            || uv.iter().any(|p| {
                p[0] < -angle_slack
                    || p[0] > sweep + angle_slack
                    || p[1] < -NATIVE_TOL
                    || p[1] > height + NATIVE_TOL
            })
        {
            continue;
        }
        let span = uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
            - uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        if span > PI / 4. + 1e-12 || span <= 0. {
            continue;
        }
        if positions.iter().any(|(p, _)| {
            let r = std::array::from_fn(|i| frame.x[i] * p[0] + frame.y[i] * p[1]);
            orientation * dot(normal, r) <= 0.
        }) {
            continue;
        }
        let twice_area = (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1])
            - (uv[1][1] - uv[0][1]) * (uv[2][0] - uv[0][0]);
        if orientation * twice_area <= 0. {
            continue;
        }
        area.add(orientation * twice_area * 0.5);
        radial = radial.max(residual);
        // Numerical admission just outside the trimmed rectangle is still a
        // real displacement: charge axial and circumferential clamp distances.
        for p in uv {
            trim = trim.max(
                (-p[1]).max(p[1] - height).max(0.) + radius * (-p[0]).max(p[0] - sweep).max(0.),
            );
        }
        maximum_span = maximum_span.max(span);
        triangles.insert(t);
        queue.extend(adjacency[t].iter().copied());
    }
    // This PL-UV area check is additional to (not a substitute for) the actual
    // radial Jacobian and exact topological boundary proof performed by caller.
    let expected_area = sweep * height;
    let numerical_area = 8. * NATIVE_TOL * (sweep + height / radius) + expected_area * 1e-10;
    if (area.get() - expected_area).abs() > numerical_area {
        return Ok(None);
    }
    Ok(Some(CurvedWall {
        triangles,
        radial,
        // Any convex combination of vertices in a sector of this width is
        // within radial_residual + R*(1-cos(width/2)) of the cylinder.
        chord: 2. * radius * (maximum_span / 4.).sin().powi(2),
        trim,
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
        MixedSegment::EllipticArc { arc, reversed } => MixedSegment::EllipticArc {
            arc: *arc,
            reversed: !*reversed,
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
            MixedSegment::EllipticArc { arc, reversed } => {
                model.vertices[model.elliptic_arcs[*arc].vertices[usize::from(*reversed)]]
            }
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
            MixedSegment::EllipticArc { arc, reversed } => {
                let a = &model.elliptic_arcs[*arc];
                let sweep = a.sweep_rad;
                // Stable value of t - sin(t) in a shallow ellipse segment.
                let difference = if sweep.abs() < 1e-2 {
                    sweep.powi(3) * (1. / 6. - sweep.powi(2) / 120. + sweep.powi(4) / 5040.)
                } else {
                    sweep - sweep.sin()
                };
                let sign = if *reversed { -1. } else { 1. };
                (
                    if *reversed {
                        [a.vertices[1], a.vertices[0]]
                    } else {
                        a.vertices
                    },
                    sign * dot(a.axis, normal) * a.major_mm * a.minor_mm * difference / 2.,
                )
            }
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
    if cylinder.elliptic_start {
        return crate::oblique_pocket::double_stop_flux(model, cylinder, reference);
    }
    let a = &model.arcs[cylinder.arcs[0]];

    let x = scale(
        sub(model.vertices[a.vertices[0]], a.center),
        1. / a.radius_mm,
    );
    let y = cross(a.axis, x);
    let theta = a.sweep_rad;
    if cylinder.elliptic_end {
        // H(u) = h + hc cos(u) + hs sin(u) is the axial height of the
        // cutting plane. Integrate (p - reference) · n over the trimmed wall.
        let end = &model.elliptic_arcs[cylinder.arcs[1]];
        let h = dot(sub(end.center, a.center), a.axis);
        let slope = -a.radius_mm / dot(end.axis, a.axis);
        let hc = slope * dot(end.axis, x);
        let hs = slope * dot(end.axis, y);
        let (sin, cos) = theta.sin_cos();
        let cos2 = (theta + sin * cos) / 2.;
        let sin2 = (theta - sin * cos) / 2.;
        let sincos = sin * sin / 2.;
        let offset = sub(a.center, reference);
        let integrated = a.radius_mm * (h * theta + hc * sin + hs * (1. - cos))
            + dot(offset, x) * (h * sin + hc * cos2 + hs * sincos)
            + dot(offset, y) * (h * (1. - cos) + hc * sincos + hs * sin2);
        return (if cylinder.inner { -1. } else { 1. }) * a.radius_mm * integrated / 3.;
    }
    let b = &model.arcs[cylinder.arcs[1]];
    let height = dot(sub(b.center, a.center), a.axis);
    let integral: V = std::array::from_fn(|i| x[i] * theta.sin() + y[i] * (1. - theta.cos()));
    (if cylinder.inner { -1. } else { 1. })
        * a.radius_mm
        * height
        * (a.radius_mm * theta + dot(sub(a.center, reference), integral))
        / 3.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independently_sampled_convex_fillet_uses_the_coarser_cap_chord() {
        use crate::{analyze_mesh, analyze_stl, recognize_mixed, ReconstructionMode};
        let options = Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        };
        let original = analyze_stl(
            include_bytes!("../../../fixtures/partial_fillets/one_edge.stl"),
            &options,
        )
        .unwrap();
        let radius = 4.;
        let coarse_chord = radius * (1. - (PI / 64.).cos());
        for station in [0., 12.] {
            // Split all 16 chords on just one end of this convex block. Each
            // new sample lies on the original circle, and the two triangles
            // incident to its edge are subdivided in their original winding.
            let mut mesh = original.mesh.clone();
            let on_arc = |v: usize| {
                let [x, y, z] = original.mesh.vertices[v];
                z == station && ((x - 16.).hypot(y + 11.) - radius).abs() < 1e-10
            };
            for (id, tri) in original.mesh.triangles.iter().enumerate() {
                if let Some(k) = (0..3).find(|&k| on_arc(tri[k]) && on_arc(tri[(k + 1) % 3])) {
                    let (u, v, w) = (tri[k], tri[(k + 1) % 3], tri[(k + 2) % 3]);
                    let a = mesh.vertices[u];
                    let b = mesh.vertices[v];
                    let x = (a[0] + b[0]) / 2. - 16.;
                    let y = (a[1] + b[1]) / 2. + 11.;
                    let r = x.hypot(y);
                    let sample = mesh.vertices.len();
                    mesh.vertices
                        .push([16. + radius * x / r, -11. + radius * y / r, station]);
                    mesh.triangles[id] = [u, sample, w];
                    mesh.triangles.push([sample, v, w]);
                }
            }
            assert_eq!(mesh.triangles.len(), original.mesh.triangles.len() + 32);
            let analysis = analyze_mesh(&mesh, &options).unwrap();
            let lifted = recognize_mixed(&analysis, &options).unwrap();
            assert_eq!(lifted.model.partial_cylinders.len(), 1);
            assert!(!lifted.model.partial_cylinders[0].inner);
            assert!((lifted.report.max_chord_error_mm - coarse_chord).abs() < 1e-9);
            let tight = Options {
                max_deviation_mm: coarse_chord * 0.9,
                ..options.clone()
            };
            assert_eq!(
                recognize_mixed(&analysis, &tight).unwrap_err().code,
                "recognition.deviation_exceeded"
            );
        }
    }

    #[test]
    fn native_rectangle_checks_each_corner_with_the_euclidean_tolerance() {
        let arcs = [0., 7.].map(|z| CircularArc {
            center: [0., 0., z],
            axis: [0., 0., 1.],
            radius_mm: 5.,
            sweep_rad: PI / 2.,
            vertices: if z == 0. { [0, 1] } else { [2, 3] },
        });
        let exact = [[5., 0., 0.], [0., 5., 0.], [5., 0., 7.], [0., 5., 7.]];
        for id in 0..4 {
            for (radial, axial, accepted) in [
                (0.6e-7, 0.6e-7, true),
                (0.8e-7, 0.8e-7, false),
                (0.95e-7, 0., true),
                (0., 0.95e-7, true),
            ] {
                let mut points = exact;
                points[id][id % 2] += radial;
                points[id][2] += axial;
                assert_eq!(native_endpoints_match(&arcs, &points), accepted, "{id}");
            }
        }
    }

    #[test]
    fn whole_union_recovers_nonmonotone_endpoint_constrained_fits() {
        // Independent translated f32 semicircle, unrelated to any private mesh.
        let vertices: Vec<V> = (0..=16)
            .map(|i| {
                let angle = 0.338 + PI * i as f64 / 16.;
                [
                    (60.308 + 1.45 * angle.cos()) as f32 as f64,
                    (-130. + 1.45 * angle.sin()) as f32 as f64,
                    0.,
                ]
            })
            .collect();
        let points: Vec<P2> = vertices.iter().map(|p| [p[0], p[1]]).collect();
        assert!(fit(&points[..10], 1e-5).is_none());
        assert!(fit(&points, 1e-5).is_some());
        for reverse in [false, true] {
            let mut chain: Vec<usize> = (0..vertices.len()).collect();
            if reverse {
                chain.reverse();
            }
            let model = MixedModel {
                vertices: vertices.clone(),
                planes: vec![crate::MixedPlane {
                    normal: [0., 0., 1.],
                    loops: vec![MixedLoop::Polygon { vertices: chain }],
                }],
                rings: vec![],
                cylinders: vec![],
                arcs: vec![],
                elliptic_arcs: vec![],
                partial_cylinders: vec![],
            };
            let found = candidates(&model, &Options::default(), &mut Work::default()).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].chain.len(), 17);
        }
    }

    fn synthetic_f32_circle(count: usize) -> Vec<V> {
        // Independent analytic dimensions, not copied from a user contour.
        (0..count)
            .map(|i| {
                let angle = 0.338 + TAU * i as f64 / count as f64;
                [
                    (60.308 + 1.45 * angle.cos()) as f32 as f64,
                    (-130. + 1.45 * angle.sin()) as f32 as f64,
                    0.,
                ]
            })
            .collect()
    }
    fn polygon_model(vertices: Vec<V>, mut order: Vec<usize>, frame: usize) -> MixedModel {
        let normal = match frame {
            0 => [0., 0., 1.],
            1 => [0., 0., -1.],
            _ => [-0.8, 0., 0.6],
        };
        let vertices = vertices
            .into_iter()
            .map(|[x, y, z]| match frame {
                0 => [x, y, z],
                1 => [x + 13., y - 7., z + 11.],
                _ => [0.6 * x - 0.8 * z + 17., y - 9., 0.8 * x + 0.6 * z + 3.],
            })
            .collect();
        order.shrink_to_fit();
        MixedModel {
            vertices,
            planes: vec![crate::MixedPlane {
                normal,
                loops: vec![MixedLoop::Polygon { vertices: order }],
            }],
            rings: vec![],
            cylinders: vec![],
            arcs: vec![],
            elliptic_arcs: vec![],
            partial_cylinders: vec![],
        }
    }
    fn loop_points(model: &MixedModel) -> Vec<P2> {
        let plane = &model.planes[0];
        let MixedLoop::Polygon { vertices } = &plane.loops[0] else {
            panic!("synthetic polygon")
        };
        let normal = scale(plane.normal, 1. / norm(plane.normal));
        let frame = Frame::new(model.vertices[vertices[0]], normal);
        vertices
            .iter()
            .map(|&v| frame.project(model.vertices[v]).0)
            .collect()
    }

    #[test]
    fn quantized_dense_complete_circles_bypass_windows_with_exact_linear_work() {
        let o = Options::default();
        for count in [128, 360] {
            for shift in [0, 1, 9, 37] {
                for reverse in [false, true] {
                    for frame in 0..3 {
                        let mut order: Vec<_> = (0..count).collect();
                        order.rotate_left(shift);
                        if reverse {
                            order.reverse();
                        }
                        let model = polygon_model(synthetic_f32_circle(count), order, frame);
                        let points = loop_points(&model);
                        assert!(constrained_circle(&points,o.cylinder_fit_tolerance_mm).is_none(),"{count}/{shift}/{reverse}/{frame}: endpoint conditioning must reproduce the defect");
                        assert_eq!(closed_circle(&points, o.cylinder_fit_tolerance_mm), None);
                        let stable =
                            crate::recognition::fit_circle(&points, o.cylinder_fit_tolerance_mm)
                                .expect(
                                    "all-vertex fit and winding must prove this independent circle",
                                );
                        assert!((stable.radius - 1.45).abs() < 1e-5);
                        assert!(stable.radial <= o.cylinder_fit_tolerance_mm);
                        let mut work = Work::default();
                        work.spend(10_000_000 - 2 * count).unwrap();
                        assert!(candidates(&model, &o, &mut work).unwrap().is_empty());
                        assert_eq!(work.spend(1).unwrap_err().code, "recognition.too_complex");
                    }
                }
            }
        }
    }

    #[test]
    fn complete_circle_fast_guard_cannot_bypass_an_exhausted_budget() {
        let count = 360;
        let model = polygon_model(synthetic_f32_circle(count), (0..count).collect(), 0);
        for remaining in [0, count - 1, 2 * count - 1] {
            let mut work = Work::default();
            work.spend(10_000_000 - remaining).unwrap();
            assert_eq!(
                candidates(&model, &Options::default(), &mut work)
                    .err()
                    .unwrap()
                    .code,
                "recognition.too_complex"
            );
        }
    }

    #[test]
    fn old_d_profile_and_sparse_full_circle_decisions_are_preserved() {
        let d: Vec<_> = (0..=16)
            .map(|i| {
                let a = PI * i as f64 / 16.;
                [5. * a.cos(), 5. * a.sin(), 0.]
            })
            .collect();
        let model = polygon_model(d, (0..17).collect(), 0);
        let points = loop_points(&model);
        assert_eq!(closed_circle(&points, 1e-5), Some(false));
        assert!(crate::recognition::fit_circle(&points, 1e-5).is_none());
        let found = candidates(&model, &Options::default(), &mut Work::default()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].chain.len(), 17);
        for degrees in [
            vec![0., 45., 90., 135., 180., 225., 270., 315.],
            vec![0., 8., 16., 24., 32., 40., 180., 270.],
        ] {
            let vertices = degrees
                .into_iter()
                .map(|degrees: f64| {
                    let a = degrees.to_radians();
                    [5. * a.cos(), 5. * a.sin(), 0.]
                })
                .collect();
            let model = polygon_model(vertices, (0..8).collect(), 0);
            let points = loop_points(&model);
            assert_eq!(closed_circle(&points, 1e-5), Some(true));
            assert!(crate::recognition::fit_circle(&points, 1e-5).is_none());
            let mut work = Work::default();
            work.spend(10_000_000 - 8).unwrap();
            assert!(candidates(&model, &Options::default(), &mut work)
                .unwrap()
                .is_empty());
            assert_eq!(work.spend(1).unwrap_err().code, "recognition.too_complex");
        }
    }

    #[test]
    fn noisy_crossed_and_nonmonotone_loops_do_not_pass_the_stable_circle_guard() {
        let count = 128;
        for defect in 0..3 {
            let mut vertices = synthetic_f32_circle(count);
            let mut order: Vec<_> = (0..count).collect();
            match defect {
                0 => vertices[0][0] = (vertices[0][0] + 0.02) as f32 as f64,
                1 => order.swap(1, 65),
                _ => order[12..30].reverse(),
            }
            let model = polygon_model(vertices, order, 0);
            let points = loop_points(&model);
            assert_eq!(closed_circle(&points, 1e-5), None);
            assert!(crate::recognition::fit_circle(&points, 1e-5).is_none());
            // Give the two full-loop guards their exact budget, but no window.
            // A false skip would return an empty candidate list instead of the
            // expected blocking error at the first historical window check.
            let mut work = Work::default();
            work.spend(10_000_000 - 2 * count).unwrap();
            assert_eq!(
                candidates(&model, &Options::default(), &mut work)
                    .err()
                    .unwrap()
                    .code,
                "recognition.too_complex"
            );
        }
    }

    fn synthetic_found(vertices: &[usize], start: usize, length: usize) -> Found {
        Found {
            start,
            chain: (0..length)
                .map(|i| vertices[(start + i) % vertices.len()])
                .collect(),
            fit: Fit {
                center: [start as f64 + 0.125, -(length as f64)],
                radius: length as f64 + 0.25,
                sweep: 0.75,
                radial: 1e-9,
                chord: 0.03125,
            },
        }
    }

    fn fit_bits(fit: &Fit) -> [u64; 6] {
        [
            fit.center[0],
            fit.center[1],
            fit.radius,
            fit.sweep,
            fit.radial,
            fit.chord,
        ]
        .map(f64::to_bits)
    }

    fn kept_evidence(
        found: &[Found],
        evidence: &MaximalEvidence,
        work: &mut Work,
    ) -> Result<Vec<usize>, FabError> {
        let mut kept = Vec::new();
        for i in 0..found.len() {
            let mut contained = false;
            for j in 0..found.len() {
                if evidence.dominates(found, i, j, work)? {
                    contained = true;
                    break;
                }
            }
            if !contained {
                kept.push(i);
            }
        }
        Ok(kept)
    }

    #[test]
    fn cyclic_maximality_matches_edge_sets_exhaustively_with_wraps_and_ties() {
        for n in 3..=12 {
            let vertices: Vec<_> = (0..n).map(|i| usize::MAX - 37 * i).collect();
            let mut found: Vec<_> = (0..n)
                .flat_map(|start| (2..=n).map(move |length| (start, length)))
                .map(|(start, length)| synthetic_found(&vertices, start, length))
                .collect();
            // Duplicate intervals deliberately exercise the historical j<i tie.
            found.push(synthetic_found(&vertices, n - 1, n));
            found.rotate_left(n);
            found.reverse();
            let before: Vec<_> = found.iter().map(|f| fit_bits(&f.fit)).collect();
            let fast = MaximalEvidence::new(
                &vertices,
                &found,
                MaximalityMode::Adaptive,
                &mut Work::default(),
            )
            .unwrap();
            assert!(matches!(fast, MaximalEvidence::Cyclic { .. }));
            let historical = MaximalEvidence::new(
                &vertices,
                &found,
                MaximalityMode::Historical,
                &mut Work::default(),
            )
            .unwrap();
            for i in 0..found.len() {
                for j in 0..found.len() {
                    assert_eq!(
                        fast.dominates(&found, i, j, &mut Work::default()).unwrap(),
                        historical
                            .dominates(&found, i, j, &mut Work::default())
                            .unwrap(),
                        "n={n}, i={i}, j={j}"
                    );
                }
            }
            assert_eq!(
                kept_evidence(&found, &fast, &mut Work::default()).unwrap(),
                kept_evidence(&found, &historical, &mut Work::default()).unwrap()
            );
            assert_eq!(
                before,
                found.iter().map(|f| fit_bits(&f.fit)).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn repeated_source_indices_use_historical_edge_evidence() {
        for vertices in [
            vec![0, 1, 2, 3, 4, 5, 6, 0],
            vec![0, 1, 2, 1, 0, 3, 4, 5],
            vec![0, 1, 2, 3, 3, 4, 5, 6],
        ] {
            let found: Vec<_> = (0..vertices.len())
                .flat_map(|start| (2..=vertices.len()).map(move |length| (start, length)))
                .map(|(start, length)| synthetic_found(&vertices, start, length))
                .collect();
            let fallback = MaximalEvidence::new(
                &vertices,
                &found,
                MaximalityMode::Adaptive,
                &mut Work::default(),
            )
            .unwrap();
            assert!(matches!(fallback, MaximalEvidence::Historical(_)));
            let historical = MaximalEvidence::new(
                &vertices,
                &found,
                MaximalityMode::Historical,
                &mut Work::default(),
            )
            .unwrap();
            for i in 0..found.len() {
                for j in 0..found.len() {
                    assert_eq!(
                        fallback
                            .dominates(&found, i, j, &mut Work::default())
                            .unwrap(),
                        historical
                            .dominates(&found, i, j, &mut Work::default())
                            .unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn maximality_charges_guard_and_constant_pair_tests_without_budget_fallback() {
        // Sorted three-element heap: 3 copied indices, 3 comparisons and
        // 4 exchanges, followed by 2 uniqueness comparisons = 12 units.
        let mut work = Work::default();
        work.spend(10_000_000 - 12).unwrap();
        assert!(unique_source_vertices(&[0, 1, 2], &mut work).unwrap());
        assert_eq!(work.spend(1).unwrap_err().code, "recognition.too_complex");
        for remaining in [0, 2, 3, 6, 11] {
            let mut work = Work::default();
            work.spend(10_000_000 - remaining).unwrap();
            assert_eq!(
                unique_source_vertices(&[0, 1, 2], &mut work)
                    .unwrap_err()
                    .code,
                "recognition.too_complex"
            );
        }
        let vertices: Vec<_> = (0..8).collect();
        let found = [
            synthetic_found(&vertices, 0, 3),
            synthetic_found(&vertices, 4, 3),
        ];
        let fast = MaximalEvidence::Cyclic { vertices: 8 };
        let mut work = Work::default();
        work.spend(10_000_000 - 4).unwrap();
        assert_eq!(kept_evidence(&found, &fast, &mut work).unwrap(), [0, 1]);
        assert_eq!(work.spend(1).unwrap_err().code, "recognition.too_complex");
        let mut work = Work::default();
        work.spend(10_000_000 - 3).unwrap();
        assert_eq!(
            kept_evidence(&found, &fast, &mut work).unwrap_err().code,
            "recognition.too_complex"
        );
        // A failed uniqueness guard must return its budget error, never retry
        // the set algorithm using the exhausted counter.
        let mut work = Work::default();
        work.spend(10_000_000).unwrap();
        assert_eq!(
            MaximalEvidence::new(&vertices, &found, MaximalityMode::Adaptive, &mut work)
                .err()
                .unwrap()
                .code,
            "recognition.too_complex"
        );
        // Zero/one candidate avoids the guard and keeps the historical path.
        let mut work = Work::default();
        work.spend(10_000_000).unwrap();
        assert!(matches!(
            MaximalEvidence::new(&vertices, &found[..1], MaximalityMode::Adaptive, &mut work)
                .unwrap(),
            MaximalEvidence::Historical(_)
        ));
    }

    type CandidateBits = (usize, usize, Vec<usize>, [u64; 7]);
    fn candidate_bits(candidates: &[Candidate]) -> Vec<CandidateBits> {
        candidates
            .iter()
            .map(|c| {
                (
                    c.plane,
                    c.loop_index,
                    c.chain.clone(),
                    [
                        c.center[0],
                        c.center[1],
                        c.center[2],
                        c.normal[0],
                        c.normal[1],
                        c.normal[2],
                        c.radius,
                    ]
                    .map(f64::to_bits),
                )
            })
            .collect()
    }

    #[test]
    #[ignore = "timing benchmark for equivalent dense arc candidates"]
    fn benchmark_cyclic_maximality_against_historical_sets() {
        for count in [97, 257] {
            let vertices = (0..count)
                .map(|i| {
                    let angle = 0.338 + PI * i as f64 / (count - 1) as f64;
                    [5. * angle.cos(), 5. * angle.sin(), 0.]
                })
                .collect();
            let mut order: Vec<_> = (0..count).collect();
            order.rotate_left(count / 3);
            let model = polygon_model(vertices, order, 0);
            let mut old_work = Work::default();
            let started = std::time::Instant::now();
            let old = candidates_with_maximality(
                &model,
                &Options::default(),
                &mut old_work,
                MaximalityMode::Historical,
            )
            .unwrap();
            let old_time = started.elapsed();
            let mut new_work = Work::default();
            let started = std::time::Instant::now();
            let new = candidates(&model, &Options::default(), &mut new_work).unwrap();
            let new_time = started.elapsed();
            assert!(!old.is_empty());
            assert_eq!(candidate_bits(&old), candidate_bits(&new));
            assert!(new_work.used_for_test() < old_work.used_for_test());
            eprintln!(
                "n={count}: historical={} units {old_time:?}; cyclic={} units {new_time:?}",
                old_work.used_for_test(),
                new_work.used_for_test()
            );
        }
    }

    #[test]
    fn candidate_maximality_preserves_historical_order_chains_and_fit_bits() {
        let o = Options::default();
        for count in [17, 49, 97] {
            let vertices: Vec<_> = (0..count)
                .map(|i| {
                    let angle = 0.338 + PI * i as f64 / (count - 1) as f64;
                    [5. * angle.cos(), 5. * angle.sin(), 0.]
                })
                .collect();
            for reverse in [false, true] {
                for frame in 0..3 {
                    let mut order: Vec<_> = (0..count).collect();
                    order.rotate_left(count / 3);
                    if reverse {
                        order.reverse();
                    }
                    let model = polygon_model(vertices.clone(), order, frame);
                    let historical = candidates_with_maximality(
                        &model,
                        &o,
                        &mut Work::default(),
                        MaximalityMode::Historical,
                    )
                    .unwrap();
                    let adaptive = candidates(&model, &o, &mut Work::default()).unwrap();
                    assert!(
                        !historical.is_empty(),
                        "nonvacuous D profile {count}/{reverse}/{frame}"
                    );
                    assert_eq!(candidate_bits(&adaptive), candidate_bits(&historical));
                }
            }
        }
    }
}
