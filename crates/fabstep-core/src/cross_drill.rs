//! Orthogonal through bores with two transverse, closed branches.
//! The cylinder charts are periodic. The oriented boundary of each domain
//! fixes its degree; pointwise projection alone cannot prove the two holes.
use crate::math::*;
use crate::planar::Segmentation;
use crate::recognition::Work;
use crate::rounded::Rim;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

const SMALL_LOW: usize = 6;
const SMALL_HIGH: usize = 7;
const LARGE: usize = 8;
const MAX_CYCLE: usize = 4096;
const NATIVE_BOUND: f64 = 4e-7;
type Edge = (usize, usize);
fn key(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn fail(detail: &str) -> FabError {
    FabError::new(
        "recognition.surface_unproven",
        format!("Perçages sécants non prouvés : {detail}."),
    )
}
fn midpoint(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * 0.5)
}
fn positive(mut dir: V) -> V {
    let i = (0..3)
        .max_by(|&i, &j| dir[i].abs().total_cmp(&dir[j].abs()))
        .unwrap();
    if dir[i] < 0. {
        dir = scale(dir, -1.)
    }
    dir
}
fn local(f: &CrossDrill, p: V) -> V {
    f.axes.map(|d| dot(sub(p, f.center), d))
}
fn world(f: &CrossDrill, p: V) -> V {
    std::array::from_fn(|i| f.center[i] + (0..3).map(|j| p[j] * f.axes[j][i]).sum::<f64>())
}
fn face_axis(i: usize) -> usize {
    i % 3
}
fn side(i: usize) -> f64 {
    if i < 3 {
        -1.
    } else {
        1.
    }
}
fn height(f: &CrossDrill, i: usize) -> f64 {
    side(i) * f.half_lengths[face_axis(i)]
}
fn mouth(face: usize) -> Option<(usize, usize)> {
    match face {
        2 => Some((0, SMALL_LOW)),
        5 => Some((1, SMALL_HIGH)),
        0 => Some((2, LARGE)),
        3 => Some((3, LARGE)),
        _ => None,
    }
}
fn branch_position(f: &CrossDrill, t: f64, sign: f64) -> V {
    let [r, big] = f.radii;
    let (s, c) = t.sin_cos();
    let [dx, dy] = f.small_offset;
    let y = dy + r * s;
    [dx + r * c, y, sign * ((big - y) * (big + y)).sqrt()]
}
fn delta_angle(a: f64, b: f64) -> f64 {
    let d = b - a;
    if d > PI {
        d - TAU
    } else if d < -PI {
        d + TAU
    } else {
        d
    }
}

fn bore_direction(first: &Rim, last: &Rim, tolerance: f64) -> Option<V> {
    let delta = sub(last.center, first.center);
    let length = norm(delta);
    (length > 2. * tolerance).then(|| positive(scale(delta, 1. / length)))
}

struct Seed {
    model: CrossDrill,
    regions: [usize; 6],
    plane_by_region: Vec<usize>,
    roundoff: f64,
}
fn seed(
    a: &Analysis,
    s: &Segmentation,
    rims: &[Rim],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Seed>, FabError> {
    if rims.len() != 4 || a.mesh.triangles.len() > 32_000 || s.model.faces.len() > 8192 {
        return Ok(None);
    }
    work.spend(rims.iter().map(|r| r.vertices.len()).sum())?;
    let mut order = [0, 1, 2, 3];
    order.sort_by(|&i, &j| rims[i].radius.total_cmp(&rims[j].radius));
    let [s0, s1, b0, b1] = order.map(|i| &rims[i]);
    if rims.iter().any(|r| {
        s.model.faces[r.region].loops.len() != 2 || r.vertices != s.model.faces[r.region].loops[1]
    }) {
        return Ok(None);
    }
    let fit = o.cylinder_fit_tolerance_mm;
    let eps = fit.max(o.weld_tolerance_mm).max(o.plane_tolerance_mm);
    let r = (s0.radius + s1.radius) * 0.5;
    let big = (b0.radius + b1.radius) * 0.5;
    let c0 = midpoint(s0.center, s1.center);
    let c1 = midpoint(b0.center, b1.center);
    if (s0.radius - s1.radius).abs() > 2. * fit
        || (b0.radius - b1.radius).abs() > 2. * fit
        || r <= 1e-4
        || big - 1.5 * r <= 4. * eps.max(1e-7)
        || dot(s0.axis, b0.axis).abs() > 1e-6
    {
        return Ok(None);
    }
    // Opposite centers measure the direction across the entire block. An
    // individual mouth normal may come from a thin binary STL triangle.
    let (Some(z), Some(raw_x)) = (bore_direction(s0, s1, fit), bore_direction(b0, b1, fit)) else {
        return Ok(None);
    };
    if dot(z, raw_x).abs() > 1e-6 {
        return Ok(None);
    }
    let x = sub(raw_x, scale(z, dot(z, raw_x)));
    let x = scale(x, 1. / norm(x));
    let y = cross(z, x);
    let mut model = CrossDrill {
        small_offset: [0.; 2],
        witness_tolerance_mm: 0.,
        center: if norm(sub(c0, c1)) <= 2. * fit {
            midpoint(c0, c1)
        } else {
            c1
        },
        axes: [x, y, z],
        half_lengths: [0.; 3],
        radii: [r, big],
        rims: [0, 1, 2, 3],
        branches: [vec![], vec![]],
    };
    if rims.iter().any(|rim| {
        let axis = if (rim.radius - r).abs() < (rim.radius - big).abs() {
            z
        } else {
            x
        };
        norm(cross(axis, rim.axis)) * rim.radius > fit
    }) {
        return Ok(None);
    }
    work.spend(a.mesh.vertices.len() * 4 + s.model.faces.len())?;
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for &p in &a.mesh.vertices {
        let q = local(&model, p);
        for i in 0..3 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    for i in 0..3 {
        if (hi[i] + lo[i]).abs() > 2. * fit {
            return Ok(None);
        }
        model.half_lengths[i] = (hi[i] - lo[i]) * 0.5;
    }
    let longest = model.half_lengths.into_iter().fold(0., f64::max);
    let mut regions = [usize::MAX; 6];
    for (region, face) in s.model.faces.iter().enumerate() {
        let q = local(&model, a.mesh.vertices[face.loops[0][0]]);
        for (i, slot) in regions.iter_mut().enumerate() {
            if (q[i % 3] - height(&model, i)).abs() > fit
                || norm(sub(face.normal, scale(model.axes[i % 3], side(i)))) * longest > fit
            {
                continue;
            }
            if *slot != usize::MAX || face.loops.len() != if mouth(i).is_some() { 2 } else { 1 } {
                return Ok(None);
            }
            *slot = region;
        }
    }
    if regions.contains(&usize::MAX) {
        return Ok(None);
    }
    // The bounding box locates candidate supports, but its outermost sample
    // systematically inflates a measured block. Place each plane halfway
    // between its two extreme source projections, the minimax parallel fit.
    work.spend(s.model.faces.len() + a.mesh.triangles.len() * 4)?;
    let mut plane_by_region = vec![usize::MAX; s.model.faces.len()];
    for (i, region) in regions.into_iter().enumerate() {
        plane_by_region[region] = i;
    }
    let mut low = [f64::INFINITY; 6];
    let mut high = [f64::NEG_INFINITY; 6];
    for (triangle, &region) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        let i = plane_by_region[region];
        if i == usize::MAX {
            continue;
        }
        for &v in triangle {
            let d = dot(
                sub(a.mesh.vertices[v], model.center),
                model.axes[face_axis(i)],
            );
            low[i] = low[i].min(d);
            high[i] = high[i].max(d);
        }
    }
    let heights: [f64; 6] = std::array::from_fn(|i| (low[i] + high[i]) * 0.5);
    let offset = std::array::from_fn(|i| (heights[i + 3] + heights[i]) * 0.5);
    model.center = world(&model, offset);
    model.half_lengths = std::array::from_fn(|i| (heights[i + 3] - heights[i]) * 0.5);
    let measured = local(&model, c0);
    // Absorb only deviations already covered by the measured mouth residual.
    // This preserves historical centered cylinders on float32 placements.
    if measured[0].hypot(measured[1]) > fit {
        model.small_offset = [measured[0], measured[1]];
    }
    let [dx, dy] = model.small_offset;
    let l = model.half_lengths;
    if big - 1.5 * (r + dy.abs()) <= (if dy == 0. { 4. } else { 5.5 }) * eps.max(1e-7)
        || dy.abs() > r
        || l[0] <= dx.abs() + r + 8. * eps
        || l[1] <= big + 8. * eps
        || l[2] <= big + 8. * eps
    {
        return Ok(None);
    }
    let roundoff = 128.
        * f64::EPSILON
        * model
            .center
            .into_iter()
            .chain(l)
            .map(f64::abs)
            .fold(1., f64::max);
    if roundoff > fit.min(5e-8) {
        return Ok(None);
    }
    for rim in rims {
        let Some(i) = regions.iter().position(|&rr| rr == rim.region) else {
            return Ok(None);
        };
        let Some((slot, _)) = mouth(i) else {
            return Ok(None);
        };
        let mut q = if slot < 2 { [dx, dy, 0.] } else { [0.; 3] };
        q[i % 3] = height(&model, i);
        if norm(sub(rim.center, world(&model, q))) > fit
            || (rim.radius - model.radii[usize::from(slot >= 2)]).abs() > fit
        {
            return Ok(None);
        }
    }
    Ok(Some(Seed {
        model,
        regions,
        plane_by_region,
        roundoff,
    }))
}

#[derive(Default)]
struct Bound {
    projection: f64,
    wall: f64,
    trim: f64,
    plane: f64,
}
impl Bound {
    fn record(&mut self, a: V, b: V, tol: f64) -> Result<(), FabError> {
        let e = norm(sub(a, b));
        if !e.is_finite() || e > tol {
            return Err(fail("résidu vectoriel hors tolérance"));
        }
        self.projection = self.projection.max(e);
        Ok(())
    }
}
fn owners(
    a: &Analysis,
    s: &Segmentation,
    seed: &Seed,
    points: &[V],
    adj: &[Vec<usize>],
    fit: f64,
    work: &mut Work,
) -> Result<Option<Vec<usize>>, FabError> {
    work.spend(a.mesh.triangles.len() * 13)?;
    let [r, big] = seed.model.radii;
    let [dx, dy] = seed.model.small_offset;
    let owners: Vec<_> = a
        .mesh
        .triangles
        .iter()
        .zip(&s.triangle_regions)
        .map(|(ids, &rgn)| {
            if seed.plane_by_region[rgn] != usize::MAX {
                return seed.plane_by_region[rgn];
            }
            let p = ids.map(|v| points[v]);
            let small = p
                .iter()
                .all(|q| ((q[0] - dx).hypot(q[1] - dy) - r).abs() <= fit);
            let large = p.iter().all(|q| (q[1].hypot(q[2]) - big).abs() <= fit);
            match (small, large) {
                (false, true) => LARGE,
                // Both intersection branches satisfy |z| >= sqrt(5) R/3 when r/R < 2/3.
                (true, false) if p.iter().all(|q| q[2] < -big / 2.) => SMALL_LOW,
                (true, false) if p.iter().all(|q| q[2] > big / 2.) => SMALL_HIGH,
                _ => usize::MAX,
            }
        })
        .collect();
    // Four blind bores with conical tips can present the same supported
    // planes and four mouths. A common source edge between *different*
    // uniquely classified bore walls is necessary to enter the F proof.
    if !owners.iter().zip(adj).any(|(&wall, neighbors)| {
        matches!(wall, SMALL_LOW | SMALL_HIGH) && neighbors.iter().any(|&t| owners[t] == LARGE)
    }) {
        return Ok(None);
    }
    if owners.contains(&usize::MAX) {
        return Err(fail(
            "triangle de paroi absent ou attribuable à plusieurs supports",
        ));
    }
    Ok(Some(owners))
}
fn ordered_cycle(edges: &[(usize, usize)]) -> Result<Vec<usize>, FabError> {
    if !(12..=MAX_CYCLE).contains(&edges.len()) {
        return Err(fail("nombre de segments de boucle hors limites"));
    }
    let mut next = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for &(a, b) in edges {
        if next.insert(a, b).is_some() || !incoming.insert(b) {
            return Err(fail("boucle de découpe ramifiée"));
        }
    }
    let start = *next.first_key_value().unwrap().0;
    let mut v = start;
    let mut out = Vec::with_capacity(edges.len());
    loop {
        out.push(v);
        v = next
            .remove(&v)
            .ok_or_else(|| fail("boucle de découpe ouverte"))?;
        if v == start {
            break;
        }
    }
    if !next.is_empty() {
        return Err(fail("plusieurs cycles sur la même branche"));
    }
    Ok(out)
}
fn directed_edge(tri: [usize; 3], e: Edge) -> Option<Edge> {
    [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])]
        .into_iter()
        .find(|&(a, b)| key(a, b) == e)
}
struct Topology {
    branch: [Vec<usize>; 2],
    boundary: Vec<Option<V>>,
}
#[allow(clippy::too_many_arguments)]
fn topology(
    a: &Analysis,
    s: &Segmentation,
    seed: &Seed,
    own: &[usize],
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    points: &[V],
    o: &Options,
    err: &mut Bound,
    work: &mut Work,
) -> Result<Topology, FabError> {
    let mut cuts = BTreeMap::<Edge, Vec<Edge>>::new();
    let mut counts = [0isize; 9];
    let mut incident: [BTreeSet<usize>; 9] = std::array::from_fn(|_| BTreeSet::new());
    let mut edge_counts = [0isize; 9];
    work.spend(4 * a.mesh.triangles.len() + a.mesh.vertices.len() + 2 * edges.len())?;
    for (ids, &i) in a.mesh.triangles.iter().zip(own) {
        counts[i] += 1;
        incident[i].extend(ids);
    }
    for (&e, inc) in edges {
        let [lt, rt] = inc.as_slice() else {
            return Err(fail("incidence non fermée"));
        };
        let (l, r) = (own[*lt], own[*rt]);
        edge_counts[l] += 1;
        if l != r {
            edge_counts[r] += 1;
            let t = if l < r { *lt } else { *rt };
            let use_edge = directed_edge(a.mesh.triangles[t], e)
                .ok_or_else(|| fail("arête source absente"))?;
            cuts.entry(key(l, r)).or_default().push(use_edge);
        }
    }
    let mut want = BTreeSet::new();
    for i in 0..6 {
        for j in i + 1..6 {
            if i % 3 != j % 3 {
                want.insert((i, j));
            }
        }
    }
    for i in [0, 2, 3, 5] {
        want.insert((i, mouth(i).unwrap().1));
    }
    want.extend([(SMALL_LOW, LARGE), (SMALL_HIGH, LARGE)]);
    if cuts.keys().copied().collect::<BTreeSet<_>>() != want {
        return Err(fail("incidences des neuf domaines incompatibles"));
    }
    let mut visited = vec![false; own.len()];
    let mut seen = [false; 9];
    for (i, &owner) in own.iter().enumerate() {
        if visited[i] {
            continue;
        }
        if std::mem::replace(&mut seen[owner], true) {
            return Err(fail("domaine en plusieurs composantes"));
        }
        visited[i] = true;
        let mut stack = vec![i];
        while let Some(t) = stack.pop() {
            work.spend(3)?;
            for &n in &adj[t] {
                if own[n] == owner && !visited[n] {
                    visited[n] = true;
                    stack.push(n);
                }
            }
        }
    }
    for i in 0..9 {
        let expected = match i {
            1 | 4 => 1,
            LARGE => -2,
            _ => 0,
        };
        if !seen[i] || incident[i].len() as isize - edge_counts[i] + counts[i] != expected {
            return Err(fail("caractéristique d’Euler du domaine incorrecte"));
        }
    }
    let mut t = Topology {
        branch: [vec![], vec![]],
        boundary: vec![None; a.mesh.vertices.len()],
    };
    let f = &seed.model;
    let [dx, dy] = f.small_offset;
    for face in [0, 2, 3, 5] {
        let (slot, wall) = mouth(face).unwrap();
        let es = &cuts[&(face, wall)];
        work.spend(8 * es.len())?;
        let lp = &s.model.faces[seed.regions[face]].loops[1];
        let actual: BTreeSet<_> = es.iter().map(|&(a, b)| key(a, b)).collect();
        if lp.len() != es.len()
            || (0..lp.len()).any(|i| !actual.contains(&key(lp[i], lp[(i + 1) % lp.len()])))
        {
            return Err(fail("embouchure différente de la boucle du plan"));
        }
        let ring = ordered_cycle(es)?;
        let axis = face % 3;
        let radius = f.radii[usize::from(slot >= 2)];
        // The cylindrical frame x/y for z, or y/z for x, is right-handed.
        // Directed edge origins here come from the planar side of each mouth.
        let center = if slot < 2 { [dx, dy, 0.] } else { [0.; 3] };
        let angle = |p: V| {
            let q = sub(p, center);
            q[(axis + 2) % 3].atan2(q[(axis + 1) % 3])
        };
        let mut total = Sum::default();
        for (i, &v) in ring.iter().enumerate() {
            let a = angle(points[v]);
            let b = angle(points[ring[(i + 1) % ring.len()]]);
            let step = -side(face) * delta_angle(a, b);
            if !(0. < step && step <= PI / 4. + 1e-10) {
                return Err(fail("embouchure repliée ou insuffisamment résolue"));
            }
            total.add(step);
            err.wall = err.wall.max(radius * step * step / 8.);
            let mut q = center;
            q[axis] = height(f, face);
            q[(axis + 1) % 3] += radius * a.cos();
            q[(axis + 2) % 3] += radius * a.sin();
            err.record(points[v], q, o.cylinder_fit_tolerance_mm)?;
            if t.boundary[v].replace(q).is_some() {
                return Err(fail("embouchures partagées entre domaines"));
            }
        }
        if (total.get() - TAU).abs() > 1e-8 {
            return Err(fail("degré de l’embouchure différent d’un"));
        }
    }
    let [r, big] = f.radii;
    let extent = r + dy.abs();
    let floor = ((big - extent) * (big + extent)).sqrt();
    let (small_accel, large_accel) = if dy == 0. {
        let ratio = r / big;
        (
            r * r / floor + r.powi(4) / (4. * floor.powi(3)),
            r.hypot(big * ratio / (1. - ratio * ratio).powf(1.5)),
        )
    } else {
        // y=d+r sin(t), h=sqrt(R²-y²). Bound |h''| on the small chart,
        // and |(asin(y/R))''|*R together with |x''| on the large chart.
        (
            (r * r + extent * r) / floor + (extent * r).powi(2) / floor.powi(3),
            r.hypot(big * (r / floor + extent * r * r / floor.powi(3))),
        )
    };
    for j in 0..2 {
        let es = &cuts[&(SMALL_LOW + j, LARGE)];
        work.spend(8 * es.len())?;
        let mut cycle = ordered_cycle(es)?;
        if j == 1 {
            cycle.reverse();
        }
        let phase = |v: usize| (points[v][1] - dy).atan2(points[v][0] - dx);
        let mut total = Sum::default();
        for (i, &v) in cycle.iter().enumerate() {
            let angle = phase(v);
            let step = delta_angle(angle, phase(cycle[(i + 1) % cycle.len()]));
            if !(0. < step && step <= PI / 4. + 1e-10) {
                return Err(fail("branche transverse repliée ou pas angulaire excessif"));
            }
            total.add(step);
            let q = branch_position(f, angle, if j == 0 { -1. } else { 1. });
            err.record(points[v], q, o.cylinder_fit_tolerance_mm)?;
            if t.boundary[v].replace(q).is_some() {
                return Err(fail("branches non disjointes"));
            }
            // The two cylindrical charts use different straight interpolants
            // of the common curve. Bound each interpolant to the ideal branch.
            err.trim = err
                .trim
                .max(small_accel.max(large_accel) * step * step / 8.);
        }
        if (total.get() - TAU).abs() > 1e-8 {
            return Err(fail("degré différent d’un sur la branche transverse"));
        }
        t.branch[j] = cycle;
    }
    Ok(t)
}

fn planes(
    s: &Segmentation,
    seed: &Seed,
    t: &mut Topology,
    local: &[V],
    err: &mut Bound,
    fit: f64,
    work: &mut Work,
) -> Result<Vec<MixedPlane>, FabError> {
    let mut result = Vec::with_capacity(6);
    let f = &seed.model;
    for (i, &region) in seed.regions.iter().enumerate() {
        let outer = &s.model.faces[region].loops[0];
        work.spend(8 * outer.len())?;
        let normal = scale(f.axes[i % 3], side(i));
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let mut sides = Vec::with_capacity(outer.len());
        for n in 0..outer.len() {
            let p = local[outer[n]];
            let q = local[outer[(n + 1) % outer.len()]];
            let mut selected = None;
            for &fixed in &[j, k] {
                for sign in [-1., 1.] {
                    if [p, q]
                        .iter()
                        .all(|v| (v[fixed] - sign * f.half_lengths[fixed]).abs() <= fit)
                    {
                        if selected.is_some() {
                            return Err(fail("arête du bloc ambiguë"));
                        }
                        selected = Some((fixed, sign));
                    }
                }
            }
            let (fixed, sign) = selected.ok_or_else(|| fail("bord extérieur non rectangulaire"))?;
            let along = if fixed == j { k } else { j };
            let forward = if fixed == j { sign } else { -sign } * side(i);
            if (q[along] - p[along]) * forward <= 0. {
                return Err(fail("bord du bloc replié"));
            }
            sides.push((fixed, sign));
        }
        let mut corners = Vec::new();
        for n in 0..outer.len() {
            let v = outer[n];
            let mut q = local[v];
            q[i % 3] = height(f, i);
            let (fixed, sign) = sides[n];
            q[fixed] = sign * f.half_lengths[fixed];
            let previous = sides[(n + outer.len() - 1) % outer.len()];
            if previous != sides[n] {
                if previous.0 == fixed {
                    return Err(fail("angle du rectangle absent"));
                }
                q[previous.0] = previous.1 * f.half_lengths[previous.0];
                corners.push(v);
            }
            err.record(local[v], q, fit)?;
            if let Some(prev) = t.boundary[v] {
                if norm(sub(q, prev)) > seed.roundoff {
                    return Err(fail("coins des supports incompatibles"));
                }
            }
            t.boundary[v] = Some(q);
        }
        if corners.len() != 4 {
            return Err(fail(
                "contour extérieur différent du rectangle parcouru une fois",
            ));
        }
        let mut loops = vec![MixedLoop::Polygon { vertices: corners }];
        if let Some((slot, _)) = mouth(i) {
            loops.push(MixedLoop::Circle { ring: slot });
        }
        result.push(MixedPlane { normal, loops });
    }
    Ok(result)
}

// A strip can run from mouth to seam without any intermediate vertices. Its
// internal edges still contain measured points between the two trims. Demand
// two resolved, widely separated radial directions on each such wall.
fn prove_strip_interiors(
    f: &CrossDrill,
    local: &[V],
    owners: &[usize],
    edges: &BTreeMap<Edge, Vec<usize>>,
    interior: &mut [bool; 3],
    o: &Options,
    work: &mut Work,
) -> Result<(), FabError> {
    let fit = o.cylinder_fit_tolerance_mm;
    let clearance = 4. * fit.max(o.weld_tolerance_mm).max(o.plane_tolerance_mm);
    let mut minima = [[f64::INFINITY; 2]; 3];
    let mut maxima = [[f64::NEG_INFINITY; 2]; 3];
    work.spend(edges.len() * 8)?;
    for (&(a, b), incident) in edges {
        // topology() already required exactly two incident triangles per edge.
        let wall = owners[incident[0]];
        if wall < SMALL_LOW || interior[wall - SMALL_LOW] || wall != owners[incident[1]] {
            continue;
        }
        let large = wall == LARGE;
        let axis = if large { 0 } else { 2 };
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        let radius = f.radii[usize::from(large)];
        let (p, q) = (local[a], local[b]);
        let m = midpoint(p, q);
        let [dx, dy] = f.small_offset;
        let center = if large { [0.; 3] } else { [dx, dy, 0.] };
        let other_distance = if large {
            (m[0] - dx).hypot(m[1] - dy) - f.radii[0]
        } else {
            m[1].hypot(m[2]) - f.radii[1]
        };
        if (p[axis] - q[axis]).abs() <= clearance
            || f.half_lengths[axis] - m[axis].abs() <= clearance
            || other_distance <= clearance
            || ((m[j] - center[j]).hypot(m[k] - center[k]) - radius).abs() > fit
        {
            continue;
        }
        let slot = wall - SMALL_LOW;
        for (i, value) in [m[j], m[k]].into_iter().enumerate() {
            minima[slot][i] = minima[slot][i].min(value);
            maxima[slot][i] = maxima[slot][i].max(value);
            // A single radial component spanning more than one radius
            // certifies two distinct points even with both fit errors.
            interior[slot] |= maxima[slot][i] - minima[slot][i] > radius + 2. * fit;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn proof(
    a: &Analysis,
    f: &CrossDrill,
    local: &[V],
    owners: &[usize],
    edges: &BTreeMap<Edge, Vec<usize>>,
    t: &Topology,
    o: &Options,
    err: &mut Bound,
    work: &mut Work,
) -> Result<(), FabError> {
    let mut interior = [false; 3];
    for (&ids, &wall) in a.mesh.triangles.iter().zip(owners) {
        work.spend(15)?;
        if wall < 6 {
            let axis = face_axis(wall);
            let h = height(f, wall);
            let mut projected = [[0.; 3]; 3];
            for (i, v) in ids.into_iter().enumerate() {
                let p = local[v];
                let dist = (p[axis] - h).abs();
                if dist > o.plane_tolerance_mm {
                    return Err(fail("triangle hors du plan canonique"));
                }
                err.plane = err.plane.max(dist);
                let mut q = p;
                q[axis] = h;
                projected[i] = t.boundary[v].unwrap_or(q);
            }
            let [p, q, r] = projected;
            let u = sub(q, p);
            let v = sub(r, p);
            if side(wall) * cross(u, v)[axis] <= 128. * f64::EPSILON * norm(u) * norm(v) {
                return Err(fail("triangle plan replié"));
            }
            continue;
        }
        let axis = if wall == LARGE { 0 } else { 2 };
        let radius = f.radii[usize::from(wall == LARGE)];
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        let [dx, dy] = f.small_offset;
        let center = if wall == LARGE { [0.; 3] } else { [dx, dy, 0.] };
        let mut uv = [[0.; 2]; 3];
        for (i, v) in ids.into_iter().enumerate() {
            let p = local[v];
            let mut q = t.boundary[v].unwrap_or(p);
            let angle = (q[k] - center[k]).atan2(q[j] - center[j]);
            q[j] = center[j] + radius * angle.cos();
            q[k] = center[k] + radius * angle.sin();
            err.record(p, q, o.cylinder_fit_tolerance_mm)?;
            uv[i] = [angle, q[axis]];
            if t.boundary[v].is_none() {
                interior[wall - 6] = true;
            }
        }
        let raw0 = uv[0][0];
        for p in &mut uv {
            p[0] = delta_angle(raw0, p[0]);
        }
        let lo = uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let hi = uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        if hi - lo >= PI - 1e-10 {
            return Err(fail(
                "triangle cylindrique sans relèvement angulaire unique",
            ));
        }
        let [p, q, r] = uv;
        let (u, h, v, g) = (q[0] - p[0], q[1] - p[1], r[0] - p[0], r[1] - p[1]);
        let signed = u * g - v * h;
        if !signed.is_finite() || signed >= -128. * f64::EPSILON * ((u * g).abs() + (v * h).abs()) {
            return Err(fail("carte cylindrique repliée ou indéterminée"));
        }
        let [p, q, r] = ids.map(|v| local[v]);
        let n = cross(sub(q, p), sub(r, p));
        let mut radial = [0.; 3];
        let angle = raw0 + (lo + hi) * 0.5;
        radial[j] = angle.cos();
        radial[k] = angle.sin();
        if dot(n, radial) >= -128. * f64::EPSILON * norm(n) {
            return Err(fail("normale de paroi dirigée vers la matière"));
        }
        err.wall = err.wall.max(radius * (hi - lo).powi(2) / 8.);
    }
    if interior.contains(&false) {
        prove_strip_interiors(f, local, owners, edges, &mut interior, o, work)?;
    }
    if interior.contains(&false) {
        return Err(fail("paroi sans témoin intérieur"));
    }
    // The source is a closed oriented 2-manifold. Connected patches and their
    // explicit simple cycles have the required Euler characteristics. Each
    // affine cylinder chart triangle now has strictly negative orientation.
    // The sum of triangle degrees vanishes outside the axial extent. Crossing
    // each directed boundary changes it by the boundary degree, so interior
    // multiplicity is one precisely in the intended region, zero elsewhere.
    // On the large cylinder, each ideal branch in (x, angle) is congruent to
    // (r cos t, asin(k sin t)), k=r/R<2/3. Its signed curvature is
    // positive: its numerator has the sign of 1-k²(1-cos⁴(t)) >= 1-k². The sampled
    // polygons are therefore simple, convex and in disjoint half-cylinders.
    // With transverse displacement d, the chart is (dx+r cos t, asin(y/R)),
    // y=d+r sin t. Its curvature determinant has the sign of
    // 1 - sin(t)*y*r*cos²(t)/(R²-y²), positive since |y|,r < 2R/3.
    // In the small chart the two trims are strictly single-valued axial graphs.
    // The homotopy from each straight chart edge to its exact trim is charged
    // to 'trim'. Cylinder interpolation has S_theta_theta of norm radius,
    // giving radius*range(theta)^2/8 via barycentric variance.
    Ok(())
}

pub(crate) fn volume(f: &CrossDrill) -> Result<f64, FabError> {
    let [r, big] = f.radii;
    let [x, y, z] = f.half_lengths;
    if f.small_offset[1] != 0. {
        let a = f.small_offset[1].abs() / big;
        let b = r / big;
        // Section through the narrow bore: integrate sqrt(1-v²) against
        // the semicircle density on v in [a-b,a+b]. Integration by parts
        // of v^n*(b²-(v-a)²)^(3/2) gives this recurrence for its moments.
        let mut previous = 1.;
        let mut moment = a;
        let mut coefficient = 1.;
        let mut sum = Sum::default();
        sum.add(1.);
        for n in 1..96 {
            let k = n as f64;
            let next = (a * (2. * k + 3.) * moment + k * (b * b - a * a) * previous) / (k + 3.);
            previous = moment;
            moment = next;
            if n % 2 == 1 {
                let j = (n / 2) as f64;
                coefficient *= (j - 0.5) / (j + 1.);
                sum.add(coefficient * moment);
            }
        }
        let v = 8. * x * y * z - 2. * PI * (z * r * r + x * big * big)
            + 2. * PI * r * r * big * sum.get();
        return if v.is_finite() && v > 0. {
            Ok(v)
        } else {
            Err(fail("volume de matière non positif"))
        };
    }
    // F0 overlap integral. Moments of cos²(t)*sin^(2k)(t) have ratio
    // (2k+1)/(2k+4). The absolute ratio of successive terms is at most q.
    // The native certificate independently encloses the volume with intervals.
    let ratio = (r / big).powi(2);
    let mut term = PI / 4.;
    let mut series = Sum::default();
    // Preserve the historical sum for q <= 1/4. In the extended domain
    // the omitted series is bounded by (pi/4)*(4/9)^48/(1-4/9) < 1.8e-17.
    let terms = if ratio <= 0.25 { 20 } else { 48 };
    for k in 0..terms {
        series.add(term);
        let k = k as f64;
        term *= ratio * (k - 0.5) / (k + 1.) * (2. * k + 1.) / (2. * k + 4.);
    }
    let result =
        8. * x * y * z - 2. * PI * (z * r * r + x * big * big) + 8. * r * r * big * series.get();
    if result.is_finite() && result > 0. {
        Ok(result)
    } else {
        Err(fail("volume de matière non positif"))
    }
}

pub(crate) fn candidate(
    a: &Analysis,
    s: &Segmentation,
    rims: &[Rim],
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let Some(mut seed) = seed(a, s, rims, o, work)? else {
        return Ok(None);
    };
    work.spend(a.mesh.vertices.len())?;
    let points: Vec<_> = a
        .mesh
        .vertices
        .iter()
        .map(|&p| local(&seed.model, p))
        .collect();
    let Some(own) = owners(a, s, &seed, &points, adj, o.cylinder_fit_tolerance_mm, work)? else {
        return Ok(None);
    };
    let mut err = Bound::default();
    let mut t = topology(a, s, &seed, &own, adj, edges, &points, o, &mut err, work)?;
    let planes = planes(
        s,
        &seed,
        &mut t,
        &points,
        &mut err,
        o.cylinder_fit_tolerance_mm,
        work,
    )?;
    proof(a, &seed.model, &points, &own, edges, &t, o, &mut err, work)?;
    seed.model.branches = t.branch;
    seed.model.witness_tolerance_mm = err.projection + err.plane + seed.roundoff;
    let f = &seed.model;
    // Each measured radius can vary by w. Require R - w > 1.5 (r + w)
    // even after the reconstruction and the CAD frame each charge roundoff.
    let witness = f.witness_tolerance_mm + seed.roundoff;
    let [dx, dy] = f.small_offset;
    let margin = if dy == 0. { 2.5 } else { 5.5 };
    if f.radii[1] - 1.5 * (f.radii[0] + dy.abs()) <= margin * witness {
        return Ok(None);
    }
    let mut rings = Vec::with_capacity(4);
    for face in [2, 5, 0, 3] {
        let i = face % 3;
        let mut p = if i == 2 { [dx, dy, 0.] } else { [0.; 3] };
        p[i] = height(f, face);
        rings.push(CircularRing {
            center: world(f, p),
            axis: f.axes[i],
            radius_mm: f.radii[usize::from(i == 0)],
        });
    }
    let wall = err.plane + err.trim + seed.roundoff + NATIVE_BOUND;
    let deviation = o.weld_tolerance_mm + err.projection + err.wall + wall;
    if !deviation.is_finite() || deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le perçage sécant dépasse la borne d’écart.",
        ));
    }
    let report=RecognitionReport{revolutions: 0,cylinders:3,cones:0,spheres:0,tori:0,toroidal_fillets:0,max_radial_residual_mm:err.projection,
        max_chord_error_mm:err.wall,max_wall_residual_mm:wall,max_deviation_bound_mm:deviation,expected_volume_mm3:volume(f)?,
        diagnostics:vec![
        Diagnostic{code:"recognition.circular_intent_assumed".into(),severity:Severity::Warning,count:3,message:"Trois parois sont interprétées comme deux perçages cylindriques sécants.".into()},
        Diagnostic{code:"recognition.rounded_coverage".into(),severity:Severity::Info,count:own.len(),message:"Les neuf domaines et les deux branches transverses sont attribués aux triangles source.".into()},
        Diagnostic{code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,message:"La borne inclut les résidus source, les branches communes, les cordes et la réserve du certificat natif.".into()},
    ]};
    work.spend(a.mesh.vertices.len() + own.len())?;
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                vertices: a.mesh.vertices.clone(),
                rings,
                planes,
                cylinders: vec![],
                arcs: vec![],
                elliptic_arcs: vec![],
                partial_cylinders: vec![],
            },
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        partial_cones: vec![],
        octants: vec![],
        linear_blend: None,
        torus_sectors: vec![],
        smooth_revolution: None,
        unequal_pocket: None,
        cross_drill: Some(Box::new(seed.model)),
    };
    let triangle_faces = own
        .into_iter()
        .map(|i| {
            if i < 6 {
                RoundedFaceRef::Plane { index: i }
            } else {
                RoundedFaceRef::CrossDrill { index: i - 6 }
            }
        })
        .collect();
    Ok(Some(RoundedRecognition {
        model,
        report,
        triangle_faces,
    }))
}
