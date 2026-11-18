//! Convex box with one rounded trihedral corner. Every surface and trim is
//! established from a partition of one closed source component.
use super::*;
use crate::{CircularArc, MixedPartialCylinder, MixedSegment, MixedSphericalOctant};
use std::f64::consts::FRAC_PI_2;

const SPHERE: usize = 9;
const NATIVE: f64 = 5e-8;
type Pair = (usize, usize);
type Cuts = BTreeMap<Pair, Vec<Edge>>;

struct Frame {
    origin: V,
    // Near support OUTWARD normals in right-handed order.
    directions: [V; 3],
    lengths: V,
    radius: f64,
    // Near plane axis i at index i, far plane at i+3.
    regions: [usize; 6],
}
impl Frame {
    fn local(&self, p: V) -> V {
        self.directions.map(|d| -dot(sub(p, self.origin), d))
    }
    fn world(&self, p: V) -> V {
        std::array::from_fn(|i| {
            self.origin[i] - (0..3).map(|j| self.directions[j][i] * p[j]).sum::<f64>()
        })
    }
}

fn seed(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Frame>, FabError> {
    let p = &a.mesh.vertices;
    // Fixed upper bounds apply only to attempting this additional hypothesis.
    if !(10..=8192).contains(&s.model.faces.len()) || a.mesh.triangles.len() > 32_000 {
        return Ok(None);
    }
    let mut top = Vec::<(f64, usize)>::new();
    for (i, plane) in s.model.faces.iter().enumerate() {
        if plane.loops.len() != 1 {
            continue;
        }
        let cycle = &plane.loops[0];
        work.spend(cycle.len() + 1)?;
        let base = p[cycle[0]];
        let area = (0..cycle.len())
            .map(|j| {
                dot(
                    cross(
                        sub(p[cycle[j]], base),
                        sub(p[cycle[(j + 1) % cycle.len()]], base),
                    ),
                    plane.normal,
                )
            })
            .sum::<f64>()
            .abs()
            / 2.;
        top.push((area, i));
        top.sort_by(|x, y| y.0.total_cmp(&x.0).then_with(|| x.1.cmp(&y.1)));
        top.truncate(7);
    }
    if top.len() < 7 || top[5].0 <= 4. * top[6].0 || !top[0].0.is_finite() {
        return Ok(None);
    }
    let mut map = vec![None; s.model.faces.len()];
    for (i, &(_, r)) in top[..6].iter().enumerate() {
        map[r] = Some(i);
    }
    let mut neighbors = [0u8; 6];
    for (t, list) in adj.iter().enumerate() {
        work.spend(1)?;
        let Some(i) = map[s.triangle_regions[t]] else {
            continue;
        };
        for &other in list {
            if let Some(j) = map[s.triangle_regions[other]] {
                if i != j {
                    neighbors[i] |= 1 << j;
                }
            }
        }
    }
    let near: Vec<_> = (0..6).filter(|&i| neighbors[i].count_ones() == 2).collect();
    let far: Vec<_> = (0..6).filter(|&i| neighbors[i].count_ones() == 4).collect();
    if near.len() != 3
        || far.len() != 3
        || near
            .iter()
            .any(|&i| near.iter().any(|&j| neighbors[i] & (1 << j) != 0))
    {
        return Ok(None);
    }
    let mut ids = [0; 6];
    for (k, &i) in near.iter().enumerate() {
        let others: Vec<_> = far
            .iter()
            .copied()
            .filter(|j| neighbors[i] & (1 << j) == 0)
            .collect();
        if others.len() != 1 {
            return Ok(None);
        }
        ids[k] = top[i].1;
        ids[k + 3] = top[others[0]].1;
    }
    if ids.iter().copied().collect::<BTreeSet<_>>().len() != 6 {
        return Ok(None);
    }
    let mut d = [0, 1, 2].map(|i| s.model.faces[ids[i]].normal);
    if dot(cross(d[0], d[1]), d[2]) < 0. {
        d.swap(1, 2);
        ids.swap(1, 2);
        ids.swap(4, 5);
    }
    let tangent = sub(d[1], scale(d[0], dot(d[0], d[1])));
    if norm(tangent) < 0.99 {
        return Ok(None);
    }
    d[1] = scale(tangent, 1. / norm(tangent));
    d[2] = cross(d[0], d[1]);
    let anchor = p[s.model.faces[ids[0]].loops[0][0]];
    let mut offsets = [0.; 6];
    for (i, &region) in ids.iter().enumerate() {
        let plane = &s.model.faces[region];
        let expected = scale(d[i % 3], if i < 3 { 1. } else { -1. });
        if dot(expected, plane.normal) < 1. - 1e-10 {
            return Ok(None);
        }
        let lp = &plane.loops[0];
        work.spend(lp.len())?;
        offsets[i] = lp
            .iter()
            .map(|&v| dot(sub(p[v], anchor), d[i % 3]))
            .sum::<f64>()
            / lp.len() as f64;
    }
    let origin =
        std::array::from_fn(|i| anchor[i] + (0..3).map(|j| offsets[j] * d[j][i]).sum::<f64>());
    let mut f = Frame {
        origin,
        directions: d,
        lengths: std::array::from_fn(|i| offsets[i] - offsets[i + 3]),
        radius: 0.,
        regions: ids,
    };
    let mut minima = Vec::with_capacity(6);
    for (i, &region) in ids[..3].iter().enumerate() {
        let lp = &s.model.faces[region].loops[0];
        work.spend(lp.len() * 2)?;
        for j in (0..3).filter(|&j| j != i) {
            minima.push(
                lp.iter()
                    .map(|&v| f.local(p[v])[j])
                    .fold(f64::INFINITY, f64::min),
            );
        }
    }
    f.radius = minima.iter().sum::<f64>() / 6.;
    let eps = o
        .cylinder_fit_tolerance_mm
        .max(o.plane_tolerance_mm)
        .max(o.weld_tolerance_mm);
    let r = f.radius;
    if !r.is_finite()
        || r <= 8. * eps
        || f.lengths
            .iter()
            .any(|&v| !v.is_finite() || v - r <= 8. * eps)
        || minima
            .iter()
            .any(|v| (v - r).abs() > o.cylinder_fit_tolerance_mm)
        || f.origin
            .iter()
            .chain(f.lengths.iter())
            .any(|v| v.abs() * f64::EPSILON * 128. > eps.min(NATIVE))
    {
        return Ok(None);
    }
    Ok(Some(f))
}

fn pair(a: usize, b: usize) -> Pair {
    (a.min(b), a.max(b))
}
fn cylinder(axis: usize) -> usize {
    6 + axis
}
fn arc_index(a: usize, b: usize) -> Option<usize> {
    for i in 0..3 {
        if pair(a, b) == pair(cylinder(i), SPHERE) {
            return Some(2 * i);
        }
        if pair(a, b) == pair(cylinder(i), 3 + i) {
            return Some(2 * i + 1);
        }
    }
    None
}

fn ordered_chain(edges: &[Edge]) -> Result<Vec<usize>, FabError> {
    let mut next = BTreeMap::<usize, Vec<usize>>::new();
    for &(a, b) in edges {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    let ends: Vec<_> = next
        .iter()
        .filter(|(_, v)| v.len() == 1)
        .map(|(&k, _)| k)
        .collect();
    if ends.len() != 2 || next.values().any(|v| v.len() > 2) {
        return Err(unproven("frontière ouverte, fermée ou ramifiée du raccord"));
    }
    let mut chain = vec![ends[0]];
    let mut previous = usize::MAX;
    loop {
        let current = *chain.last().unwrap();
        let Some(&neighbor) = next[&current].iter().find(|&&v| v != previous) else {
            break;
        };
        chain.push(neighbor);
        previous = current;
        if chain.len() > next.len() {
            return Err(unproven("contour de raccord multiple"));
        }
    }
    if chain.len() != edges.len() + 1 || chain.last() != Some(&ends[1]) {
        return Err(unproven("contour de raccord disjoint"));
    }
    Ok(chain)
}

fn cylinder_uv(f: &Frame, p: V, i: usize) -> [f64; 2] {
    let r = f.radius;
    [(r - p[(i + 1) % 3]).atan2(r - p[(i + 2) % 3]), p[i] - r]
}
fn ideal_cylinder(f: &Frame, i: usize, uv: [f64; 2]) -> V {
    let mut q = [f.radius; 3];
    q[i] += uv[1];
    q[(i + 1) % 3] -= f.radius * uv[0].sin();
    q[(i + 2) % 3] -= f.radius * uv[0].cos();
    q
}

fn owner(a: &Analysis, f: &Frame, local: &[V], t: usize, fit: f64) -> Result<usize, FabError> {
    let triangle = a.mesh.triangles[t].map(|v| local[v]);
    let mut found = None;
    for i in 0..4 {
        let ok = triangle.iter().all(|&p| {
            if i < 3 {
                let uv = cylinder_uv(f, p, i);
                let trim = [
                    uv[0].clamp(0., FRAC_PI_2),
                    uv[1].clamp(0., f.lengths[i] - f.radius),
                ];
                norm(sub(p, ideal_cylinder(f, i, trim))) <= fit
            } else {
                let offsets = p.map(|x| f.radius - x);
                offsets.iter().all(|&v| v >= -fit) && (norm(offsets) - f.radius).abs() <= fit
            }
        });
        if ok
            && found
                .replace(if i < 3 { cylinder(i) } else { SPHERE })
                .is_some()
        {
            return Err(unproven(
                "triangle attribuable à plusieurs supports arrondis",
            ));
        }
    }
    found.ok_or_else(|| unproven("triangle sans support arrondi démontré"))
}

struct Proof {
    residual: f64,
    chord: f64,
    endpoint: f64,
    plane: f64,
}
impl Proof {
    fn check(&mut self, value: f64, limit: f64) -> Result<(), FabError> {
        if !value.is_finite() || value > limit {
            return Err(unproven("déplacement hors tolérance du raccord local"));
        }
        self.residual = self.residual.max(value);
        Ok(())
    }
}

struct Trim {
    ids: [usize; 2],
    arc: Option<usize>,
}
struct Boundaries {
    arcs: Vec<CircularArc>,
    trim: BTreeMap<Pair, Trim>,
    constraints: BTreeSet<(usize, usize)>,
}

fn boundaries(
    f: &Frame,
    local: &[V],
    cuts: Cuts,
    fit: f64,
    proof: &mut Proof,
    work: &mut Work,
) -> Result<Boundaries, FabError> {
    let mut expect = BTreeSet::new();
    for i in 0..3 {
        expect.insert(pair(SPHERE, cylinder(i)));
        expect.insert(pair(3 + i, cylinder(i)));
        for j in 0..3 {
            if i != j {
                expect.insert(pair(j, cylinder(i)));
                expect.insert(pair(i, 3 + j));
                expect.insert(pair(3 + i, 3 + j));
            }
        }
    }
    if cuts.keys().copied().collect::<BTreeSet<_>>() != expect {
        return Err(unproven("incidences du coin arrondi non prouvées"));
    }
    let mut arcs = [const { None }; 6];
    let mut trim = BTreeMap::new();
    let mut constraints = BTreeSet::new();
    for ((a, b), edges) in cuts {
        work.spend(edges.len().saturating_mul(9) + 1)?;
        let mut chain = ordered_chain(&edges)?;
        for &v in &chain {
            constraints.insert((a, v));
            constraints.insert((b, v));
        }
        let mut ends = [chain[0], *chain.last().unwrap()];
        let arc = arc_index(a, b);
        if let Some(idx) = arc {
            let i = idx / 2;
            let h = if idx % 2 == 0 {
                0.
            } else {
                f.lengths[i] - f.radius
            };
            if chain.len() < 4 {
                return Err(unproven("arc quart de cercle insuffisamment échantillonné"));
            }
            if cylinder_uv(f, local[ends[0]], i)[0] > cylinder_uv(f, local[ends[1]], i)[0] {
                chain.reverse();
                ends.reverse();
            }
            let mut previous = 0.;
            for (k, &v) in chain.iter().enumerate() {
                let angle = if k == 0 {
                    0.
                } else if k + 1 == chain.len() {
                    FRAC_PI_2
                } else {
                    cylinder_uv(f, local[v], i)[0]
                };
                let tol = if k == 0 || k + 1 == chain.len() {
                    fit.min(NATIVE)
                } else {
                    fit
                };
                let residual = norm(sub(local[v], ideal_cylinder(f, i, [angle, h])));
                proof.check(residual, tol)?;
                if k == 0 || k + 1 == chain.len() {
                    proof.endpoint = proof.endpoint.max(residual);
                }
                if k > 0 {
                    let step = angle - previous;
                    if step <= 0. || step > FRAC_PI_2 / 2. + 1e-12 || angle > FRAC_PI_2 {
                        return Err(unproven("arc avec repli ou pas supérieur à 45°"));
                    }
                    proof.chord = proof.chord.max(f.radius * step * step / 8.);
                }
                previous = angle;
            }
            let mut c = [f.radius; 3];
            c[i] += h;
            arcs[idx] = Some(CircularArc {
                center: f.world(c),
                axis: scale(f.directions[i], -1.),
                radius_mm: f.radius,
                sweep_rad: FRAC_PI_2,
                vertices: ends,
            });
        } else {
            let (axis, mut start, finish) = if b >= 6 {
                let i = b - 6;
                let mut lo = [f.radius; 3];
                lo[a] = 0.;
                (i, lo, f.lengths[i])
            } else {
                let i = a % 3;
                let j = b % 3;
                let axis = 3 - i - j;
                let mut lo = [0.; 3];
                lo[i] = if a < 3 { 0. } else { f.lengths[i] };
                lo[j] = if b < 3 { 0. } else { f.lengths[j] };
                lo[axis] = if a < 3 { f.radius } else { 0. };
                (axis, lo, f.lengths[axis])
            };
            if local[ends[0]][axis] > local[ends[1]][axis] {
                chain.reverse();
                ends.reverse();
            }
            let lower = start[axis];
            let mut previous = f64::NEG_INFINITY;
            for (k, &v) in chain.iter().enumerate() {
                let endpoint = k == 0 || k + 1 == chain.len();
                let h = if k == 0 {
                    lower
                } else if k + 1 == chain.len() {
                    finish
                } else {
                    local[v][axis]
                };
                if h <= previous || h > finish {
                    return Err(unproven("génératrice repliée ou hors de ses extrémités"));
                }
                start[axis] = h;
                let residual = norm(sub(local[v], start));
                proof.check(residual, if endpoint { fit.min(NATIVE) } else { fit })?;
                if endpoint {
                    proof.endpoint = proof.endpoint.max(residual);
                }
                previous = h;
            }
        }
        trim.insert((a, b), Trim { ids: ends, arc });
    }
    Ok(Boundaries {
        arcs: arcs.into_iter().flatten().collect(),
        trim,
        constraints,
    })
}

fn det2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

// Check that each face really is a single source disk with one simple
// boundary cycle. Closed source incidence has already been validated. This
// prevents isolated extra islands or pinched seams from contributing chart area.
fn disks(
    a: &Analysis,
    owners: &[usize],
    adj: &[Vec<usize>],
    cuts: &Cuts,
    work: &mut Work,
) -> Result<(), FabError> {
    let mut tags = vec![false; owners.len()];
    let mut seen = [false; 10];
    let mut degrees: [BTreeMap<usize, Vec<usize>>; 10] = std::array::from_fn(|_| BTreeMap::new());
    for (&(a, b), edges) in cuts {
        for &face in &[a, b] {
            work.spend(edges.len() * 2)?;
            for &(x, y) in edges {
                degrees[face].entry(x).or_default().push(y);
                degrees[face].entry(y).or_default().push(x);
            }
        }
    }
    for t in 0..owners.len() {
        if tags[t] {
            continue;
        }
        let face = owners[t];
        if std::mem::replace(&mut seen[face], true) {
            return Err(unproven("face source en plusieurs composantes"));
        }
        let boundary = &degrees[face];
        let Some((&start, _)) = boundary.first_key_value() else {
            return Err(unproven("face du raccord sans contour"));
        };
        work.spend(boundary.len() * 2)?;
        if boundary.values().any(|n| n.len() != 2) {
            return Err(unproven("contour source pincé"));
        }
        let mut previous = usize::MAX;
        let mut vertex = start;
        let mut n = 0;
        loop {
            let pair = &boundary[&vertex];
            let next = if pair[0] == previous {
                pair[1]
            } else {
                pair[0]
            };
            previous = vertex;
            vertex = next;
            n += 1;
            if vertex == start {
                break;
            }
            if n >= boundary.len() {
                return Err(unproven("contour source non simple"));
            }
        }
        if n != boundary.len() {
            return Err(unproven("plusieurs contours de face"));
        }
        let mut vertices = BTreeSet::new();
        let mut stack = vec![t];
        tags[t] = true;
        let mut triangles = 0isize;
        while let Some(t) = stack.pop() {
            work.spend(7)?;
            triangles += 1;
            vertices.extend(a.mesh.triangles[t]);
            for &neighbor in &adj[t] {
                if owners[neighbor] == face && !tags[neighbor] {
                    tags[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        let twice_edges = 3 * triangles + n as isize;
        if 2 * vertices.len() as isize - twice_edges + 2 * triangles != 2 {
            return Err(unproven("topologie de face différente d’un disque"));
        }
    }
    if seen.contains(&false) {
        return Err(unproven("face du raccord absente"));
    }
    Ok(())
}

// Keep the source, measured frame, topology, and shared budget explicit.
#[allow(clippy::too_many_arguments)]
fn prove_triangles(
    a: &Analysis,
    f: &Frame,
    local: &[V],
    owners: &[usize],
    bound: &Boundaries,
    o: &Options,
    proof: &mut Proof,
    work: &mut Work,
) -> Result<(), FabError> {
    let fit = o.cylinder_fit_tolerance_mm;
    let mut areas: [Sum; 4] = std::array::from_fn(|_| Sum::default());
    let mut interior = [false; 4];
    for (&ids, &face) in a.mesh.triangles.iter().zip(owners) {
        work.spend(10)?;
        if face < 6 {
            let i = face % 3;
            let positive = face >= 3;
            let height = if positive { f.lengths[i] } else { 0. };
            let normal = scale(f.directions[i], if positive { -1. } else { 1. });
            let [x, y, z] = ids.map(|v| a.mesh.vertices[v]);
            if dot(cross(sub(y, x), sub(z, x)), normal) <= 0. {
                return Err(unproven("face plane du coin repliée"));
            }
            for v in ids {
                let offset = (local[v][i] - height).abs();
                proof.check(offset, fit.min(NATIVE * 0.5))?;
                proof.plane = proof.plane.max(offset);
            }
            continue;
        }
        let i = face - 6;
        let mut chart = [[0.; 2]; 3];
        let mut sphere_points = [[0.; 3]; 3];
        for (j, v) in ids.into_iter().enumerate() {
            let p = local[v];
            if i < 3 {
                let mut uv = cylinder_uv(f, p, i);
                uv[0] = uv[0].clamp(0., FRAC_PI_2);
                uv[1] = uv[1].clamp(0., f.lengths[i] - f.radius);
                if bound.constraints.contains(&((i + 1) % 3, v)) {
                    uv[0] = FRAC_PI_2;
                }
                if bound.constraints.contains(&((i + 2) % 3, v)) {
                    uv[0] = 0.;
                }
                if bound.constraints.contains(&(SPHERE, v)) {
                    uv[1] = 0.;
                }
                if bound.constraints.contains(&(3 + i, v)) {
                    uv[1] = f.lengths[i] - f.radius;
                }
                proof.check(norm(sub(p, ideal_cylinder(f, i, uv))), fit)?;
                chart[j] = uv;
                if uv[0] > 0. && uv[0] < FRAC_PI_2 && uv[1] > 0. && uv[1] < f.lengths[i] - f.radius
                {
                    interior[i] = true;
                }
            } else {
                let mut d = p.map(|q| (f.radius - q).max(0.));
                for (k, c) in d.iter_mut().enumerate() {
                    if bound.constraints.contains(&(cylinder(k), v)) {
                        *c = 0.;
                    }
                }
                let n = norm(d);
                if n <= fit {
                    return Err(unproven("projection sphérique indéterminée"));
                }
                let unit = scale(d, 1. / n);
                let ideal = unit.map(|c| f.radius * (1. - c));
                proof.check(norm(sub(p, ideal)), fit)?;
                let sum = unit.iter().sum::<f64>();
                chart[j] = [unit[1] / sum, unit[2] / sum];
                sphere_points[j] = unit;
                if unit.iter().all(|&x| x * f.radius > 4. * fit) {
                    interior[i] = true;
                }
            }
        }
        let det = det2(chart[0], chart[1], chart[2]);
        if !det.is_finite() || det <= 1e-14 {
            return Err(unproven("triangle replié ou paramétrisation dégénérée"));
        }
        areas[i].add(0.5 * det);
        let chord = if i < 3 {
            let angles = chart.map(|q| q[0]);
            let width = angles.into_iter().fold(f64::NEG_INFINITY, f64::max)
                - angles.into_iter().fold(f64::INFINITY, f64::min);
            if width > FRAC_PI_2 / 2. + 1e-12 {
                return Err(unproven("pas cylindrique supérieur à 45°"));
            }
            // Each chart triangle covers a triangle in (angle,height).
            // The cylinder has only one nonzero second derivative, with
            // norm r in angle. Taylor's remainder times half the angular
            // variance bounds the difference from barycentric interpolation.
            f.radius * width * width / 8.
        } else {
            // The normalized ideal chordal facet maps radially onto its
            // spherical triangle. Normalizing once more by sum of the three
            // coordinates maps its cone to a straight triangle in the unit
            // simplex, so boundary degree and chart area count multiplicity.
            let [a, b, c] = sphere_points;
            f.radius * (1. - min_triangle_norm(a, b, c)).max(0.)
        };
        if !chord.is_finite() {
            return Err(unproven("borne de corde du coin indéterminée"));
        }
        proof.chord = proof.chord.max(chord);
    }
    for i in 0..4 {
        let exact = if i < 3 {
            FRAC_PI_2 * (f.lengths[i] - f.radius)
        } else {
            0.5
        };
        if !interior[i] || (areas[i].get() - exact).abs() > 1e-8 * exact {
            return Err(unproven("domaine du raccord non couvert une seule fois"));
        }
    }
    Ok(())
}

fn planes(
    s: &crate::planar::Segmentation,
    f: &Frame,
    edges: &BTreeMap<Edge, Vec<usize>>,
    owners: &[usize],
    bound: &Boundaries,
    work: &mut Work,
) -> Result<Vec<MixedPlane>, FabError> {
    let mut result = Vec::with_capacity(6);
    for (i, &rid) in f.regions.iter().enumerate() {
        let lp = &s.model.faces[rid].loops[0];
        work.spend(lp.len() * 3)?;
        let adjacent: Vec<_> = (0..lp.len())
            .map(|k| {
                edges
                    .get(&edge(lp[k], lp[(k + 1) % lp.len()]))
                    .and_then(|incidence| {
                        incidence
                            .iter()
                            .map(|&t| owners[t])
                            .find(|&other| other != i)
                    })
                    .ok_or_else(|| unproven("bord plan sans face incidente"))
            })
            .collect::<Result<_, _>>()?;
        let breaks: Vec<_> = (0..lp.len())
            .filter(|&k| adjacent[k] != adjacent[(k + lp.len() - 1) % lp.len()])
            .collect();
        if breaks.len() != if i < 3 { 4 } else { 5 } {
            return Err(unproven("extrémité plane sans le contour attendu"));
        }
        let mut segments = Vec::new();
        for (k, &from) in breaks.iter().enumerate() {
            let to = breaks[(k + 1) % breaks.len()];
            let ids = [lp[from], lp[to]];
            let t = bound
                .trim
                .get(&pair(i, adjacent[from]))
                .ok_or_else(|| unproven("frontière du plan absente"))?;
            let reversed = ids == [t.ids[1], t.ids[0]];
            if !reversed && ids != t.ids {
                return Err(unproven("extrémités du contour plan incompatibles"));
            }
            segments.push(match t.arc {
                Some(arc) => MixedSegment::Arc { arc, reversed },
                None => MixedSegment::Line { vertices: ids },
            });
        }
        result.push(MixedPlane {
            normal: scale(f.directions[i % 3], if i < 3 { 1. } else { -1. }),
            loops: vec![MixedLoop::Segments { segments }],
        });
    }
    Ok(result)
}

pub(super) fn candidate(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let Some(f) = seed(a, s, adj, o, work)? else {
        return Ok(None);
    };
    work.spend(a.mesh.vertices.len() + a.mesh.triangles.len().saturating_mul(13))?;
    let local: Vec<_> = a.mesh.vertices.iter().map(|&p| f.local(p)).collect();
    let mut face_map = vec![None; s.model.faces.len()];
    for (i, &r) in f.regions.iter().enumerate() {
        face_map[r] = Some(i);
    }
    let owners = s
        .triangle_regions
        .iter()
        .enumerate()
        .map(|(t, &r)| {
            if let Some(i) = face_map[r] {
                Ok(i)
            } else {
                owner(a, &f, &local, t, o.cylinder_fit_tolerance_mm)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut cuts = Cuts::new();
    for (&edge, incidence) in edges {
        work.spend(1)?;
        let [a, b] = incidence.as_slice() else {
            return Err(unproven("incidence de coin non fermée"));
        };
        let (a, b) = (owners[*a], owners[*b]);
        if a != b {
            cuts.entry(pair(a, b)).or_default().push(edge);
        }
    }
    disks(a, &owners, adj, &cuts, work)?;
    let mut proof = Proof {
        endpoint: 0.,
        plane: 0.,
        residual: 0.,
        chord: 0.,
    };
    let bound = boundaries(
        &f,
        &local,
        cuts,
        o.cylinder_fit_tolerance_mm,
        &mut proof,
        work,
    )?;
    prove_triangles(a, &f, &local, &owners, &bound, o, &mut proof, work)?;
    let planes = planes(s, &f, edges, &owners, &bound, work)?;
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                vertices: a.mesh.vertices.clone(),
                planes,
                rings: vec![],
                cylinders: vec![],
                arcs: bound.arcs,
                partial_cylinders: (0..3)
                    .map(|i| MixedPartialCylinder {
                        arcs: [2 * i, 2 * i + 1],
                        inner: false,
                    })
                    .collect(),
            },
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        partial_cones: vec![],
        octants: vec![MixedSphericalOctant {
            center: f.world([f.radius; 3]),
            radius_mm: f.radius,
            directions: f.directions,
            arc_uses: [(4, true), (0, true), (2, true)],
            inner: false,
        }],
    };
    // Projecting each planar disk onto its support moves points by at most
    // `plane`. Its sole simple contour has a pointwise correspondence to the
    // proven analytic contour: linear rails interpolate the measured ends;
    // quarter-circle intervals add their Taylor chord bound. A point in the
    // planar symmetric difference lies in that boundary homotopy by winding.
    // Native arcs use the projected direction to their measured first end.
    // For |v-r*d|<=e, |r*v/|v|-r*d|<=2e by the reverse triangle inequality.
    // Allow that rotation plus displacement of a planar origin/line endpoint.
    let native = 3. * proof.endpoint + 2. * proof.plane;
    let deviation = o.weld_tolerance_mm + proof.residual + proof.chord + native;
    if deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le raccord dépasse la borne d’écart.",
        ));
    }
    let volume = volume(&model)?;
    let triangle_faces = owners
        .into_iter()
        .map(|i| {
            if i < 6 {
                RoundedFaceRef::Plane { index: i }
            } else if i < 9 {
                RoundedFaceRef::PartialCylinder { index: i - 6 }
            } else {
                RoundedFaceRef::SphericalOctant { index: 0 }
            }
        })
        .collect();
    let diagnostics = vec![
        Diagnostic {
            code: "recognition.rounded_intent_assumed".into(), severity: Severity::Warning,
            count: 1, message: "Le patch admissible est interprété comme une portion sphérique.".into(),
        },
        Diagnostic {
            code: "recognition.rounded_coverage".into(), severity: Severity::Info,
            count: a.mesh.triangles.len(), message: "Tous les triangles sont attribués : 1 sphère, 0 tore, 0 cône, 3 cylindres et 6 plans. Frontières et raccords issus de la source.".into(),
        },
        Diagnostic {
            code: "recognition.bound_scope".into(), severity: Severity::Info,
            count: 1, message: "Borne composée : soudage, résidus de projection, flèche sphérique ou borne paramétrique des triangles. Validation native et audit bidirectionnel requis ; pas une mesure Hausdorff globale.".into(),
        },
    ];
    let report = RecognitionReport {
        cylinders: 3,
        cones: 0,
        spheres: 1,
        tori: 0,
        toroidal_fillets: 0,
        max_radial_residual_mm: proof.residual,
        max_chord_error_mm: proof.chord,
        max_wall_residual_mm: native,
        max_deviation_bound_mm: deviation,
        expected_volume_mm3: volume,
        diagnostics,
    };
    Ok(Some(RoundedRecognition {
        model,
        triangle_faces,
        report,
    }))
}
