//! One exterior rolling blend with a positive affine radius on a straight
//! convex edge. Planar cuts of the inclined circular cone are elliptic.
use super::block::{disks, ordered_chain};
use super::*;
use crate::{EllipticArc, MixedLinearBlend, MixedSegment};
use std::f64::consts::FRAC_1_SQRT_2;

const CONE: usize = 6;
const NATIVE: f64 = 5e-8;
type Chains = BTreeMap<Edge, Vec<usize>>;

struct Block {
    origin: V,
    axes: [V; 3],
    lengths: V,
    regions: [usize; 6],
}
impl Block {
    fn vector(&self, p: V) -> V {
        std::array::from_fn(|i| (0..3).map(|j| self.axes[j][i] * p[j]).sum())
    }
    fn local(&self, p: V) -> V {
        self.axes.map(|d| dot(sub(p, self.origin), d))
    }
    fn world(&self, p: V) -> V {
        let v = self.vector(p);
        std::array::from_fn(|i| self.origin[i] + v[i])
    }
    fn normal(&self, face: usize) -> V {
        scale(self.axes[face % 3], if face < 3 { -1. } else { 1. })
    }
    fn height(&self, face: usize) -> f64 {
        if face < 3 {
            0.
        } else {
            self.lengths[face % 3]
        }
    }

    /// Constant extruded arcs can have the same six-plane incidence graph.
    /// Both tangency setbacks must change in the same resolved direction
    /// before a failed G proof can take precedence over circular recognition.
    fn has_taper(
        &self,
        a: &Analysis,
        s: &crate::planar::Segmentation,
        o: &Options,
        work: &mut Work,
    ) -> Result<bool, FabError> {
        let mut changes = [0.; 2];
        for (axis, change) in changes.iter_mut().enumerate() {
            let cycle = &s.model.faces[self.regions[axis]].loops[0];
            work.spend(cycle.len() * 2)?;
            let mut ends = [f64::INFINITY; 2];
            for &v in cycle {
                let p = self.local(a.mesh.vertices[v]);
                for (end, height) in ends.iter_mut().zip([0., self.lengths[2]]) {
                    if (p[2] - height).abs() <= o.plane_tolerance_mm {
                        *end = end.min(p[1 - axis]);
                    }
                }
            }
            if !ends.iter().all(|r| r.is_finite()) {
                return Ok(false);
            }
            *change = ends[1] - ends[0];
        }
        let margin = 2. * o.cylinder_fit_tolerance_mm;
        Ok(changes.iter().all(|&v| v > margin) || changes.iter().all(|&v| v < -margin))
    }
}

fn frame(
    top: &[(f64, usize)],
    neighbors: [u8; 6],
    a: &Analysis,
    s: &crate::planar::Segmentation,
    work: &mut Work,
) -> Result<Option<Block>, FabError> {
    let touching = |i: usize, j: usize| (neighbors[i] & (1 << j)) != 0;
    let near: Vec<_> = (0..6).filter(|&i| neighbors[i].count_ones() == 3).collect();
    if near.len() != 2
        || touching(near[0], near[1])
        || (0..6).any(|i| !near.contains(&i) && neighbors[i].count_ones() != 4)
    {
        return Ok(None);
    }
    let other = |n| (0..6).find(|&i| !near.contains(&i) && !touching(n, i));
    let (Some(far_x), Some(far_y)) = (other(near[0]), other(near[1])) else {
        return Ok(None);
    };
    if far_x == far_y {
        return Ok(None);
    }
    let caps: Vec<_> = (0..6)
        .filter(|&i| !near.contains(&i) && i != far_x && i != far_y)
        .collect();
    if caps.len() != 2 || touching(caps[0], caps[1]) {
        return Ok(None);
    }
    let faces = &s.model.faces;
    let x = scale(faces[top[near[0]].1].normal, -1.);
    let y0 = scale(faces[top[near[1]].1].normal, -1.);
    if dot(x, y0).abs() > 1e-8 {
        return Ok(None);
    }
    let y = sub(y0, scale(x, dot(y0, x)));
    let y = scale(y, 1. / norm(y));
    let z = cross(x, y);
    let (low, high) = if dot(z, faces[top[caps[0]].1].normal) < 0. {
        (caps[0], caps[1])
    } else {
        (caps[1], caps[0])
    };
    let regions = [near[0], near[1], low, far_x, far_y, high].map(|i| top[i].1);
    let mut f = Block {
        origin: [0.; 3],
        axes: [x, y, z],
        lengths: [0.; 3],
        regions,
    };
    let anchor = a.mesh.vertices[faces[regions[0]].loops[0][0]];
    let mut heights = [0.; 6];
    for (i, r) in regions.into_iter().enumerate() {
        let p = &faces[r];
        if norm(sub(p.normal, f.normal(i))) > 1e-8 {
            return Ok(None);
        }
        let lp = &p.loops[0];
        work.spend(lp.len())?;
        let mut sum = Sum::default();
        for &v in lp {
            sum.add(dot(sub(a.mesh.vertices[v], anchor), f.axes[i % 3]));
        }
        heights[i] = sum.get() / lp.len() as f64;
    }
    let offset = f.vector([heights[0], heights[1], heights[2]]);
    f.origin = std::array::from_fn(|i| anchor[i] + offset[i]);
    f.lengths = std::array::from_fn(|i| heights[i + 3] - heights[i]);
    if f.lengths.iter().any(|&h| !h.is_finite() || h <= NATIVE)
        || f.origin
            .iter()
            .chain(&f.lengths)
            .any(|v| v.abs() * 128. * f64::EPSILON > NATIVE)
    {
        return Ok(None);
    }
    Ok(Some(f))
}

fn chains(
    a: &Analysis,
    owners: &[usize],
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    work: &mut Work,
) -> Result<Chains, FabError> {
    let mut cuts: BTreeMap<Edge, Vec<Edge>> = BTreeMap::new();
    for (&e, inc) in edges {
        work.spend(1)?;
        let [left, right] = inc.as_slice() else {
            return Err(unproven("incidence source non fermée"));
        };
        let (i, j) = (owners[*left], owners[*right]);
        if i != j {
            cuts.entry(edge(i, j)).or_default().push(e);
        }
    }
    let mut wanted = BTreeSet::new();
    for i in 0..6 {
        for j in i + 1..6 {
            if i % 3 != j % 3 && (i, j) != (0, 1) {
                wanted.insert((i, j));
            }
        }
    }
    wanted.extend([0, 1, 2, 5].map(|i| (i, CONE)));
    if cuts.keys().copied().collect::<BTreeSet<_>>() != wanted {
        return Err(unproven("voisinage différent de celui du congé linéaire"));
    }
    disks::<7>(a, owners, adj, &cuts, work)?;
    cuts.into_iter()
        .map(|(key, es)| {
            work.spend(9 * es.len() + 1)?;
            Ok((key, ordered_chain(&es)?))
        })
        .collect()
}

struct Envelope {
    radii: [f64; 2],
    b: f64,
    h: f64,
    center: f64,
    major: f64,
    minor: f64,
    theta: f64,
    ends: [[usize; 2]; 2],
}
impl Envelope {
    fn radius(&self, z: f64) -> f64 {
        self.radii[0] + self.b * z
    }
    fn angle(&self, p: V) -> f64 {
        let r = self.radius(p[2]);
        let c = (2. * self.center * r - p[0] - p[1]) * FRAC_1_SQRT_2 / self.major;
        let s = (p[0] - p[1]) * FRAC_1_SQRT_2 / self.minor;
        s.atan2(c)
    }
    fn point(&self, [t, z]: [f64; 2]) -> V {
        let (sin, cos) = t.sin_cos();
        let r = self.radius(z);
        [
            r * (self.center + (-self.major * cos + self.minor * sin) * FRAC_1_SQRT_2),
            r * (self.center + (-self.major * cos - self.minor * sin) * FRAC_1_SQRT_2),
            z,
        ]
    }
    fn uv(&self, p: V, mask: u8) -> [f64; 2] {
        let z = if mask & (1 << 2) != 0 {
            0.
        } else if mask & (1 << 5) != 0 {
            self.h
        } else {
            p[2]
        };
        let t = if mask & 1 != 0 {
            -self.theta
        } else if mask & 2 != 0 {
            self.theta
        } else {
            self.angle([p[0], p[1], z])
        };
        [t, z]
    }
    fn new(f: &Block, local: &[V], cs: &mut Chains, o: &Options) -> Result<Self, FabError> {
        let e = |v: &[usize]| [v[0], v[v.len() - 1]];
        let mut ends = [[0; 2]; 2];
        let mut radii = [0.; 2];
        let eps = o
            .cylinder_fit_tolerance_mm
            .max(o.plane_tolerance_mm)
            .max(o.weld_tolerance_mm);
        for (i, cap) in [2, 5].into_iter().enumerate() {
            let yends = e(&cs[&(0, CONE)]);
            let xends = e(&cs[&(1, CONE)]);
            let chain = cs.get_mut(&(cap, CONE)).unwrap();
            if chain.len() < 5 {
                return Err(unproven("coupe elliptique insuffisamment échantillonnée"));
            }
            if yends.contains(&chain[chain.len() - 1]) {
                chain.reverse();
            }
            let [y, x] = e(chain);
            if !yends.contains(&y)
                || !xends.contains(&x)
                || xends.contains(&y)
                || yends.contains(&x)
            {
                return Err(unproven("extrémités de coupe sans génératrices distinctes"));
            }
            ends[i] = [y, x];
            let (ry, rx) = (local[y][1], local[x][0]);
            if (ry - rx).abs() > 2. * o.cylinder_fit_tolerance_mm {
                return Err(unproven("rayons terminaux incompatibles"));
            }
            radii[i] = (ry + rx) / 2.;
        }
        if ends.into_iter().flatten().collect::<BTreeSet<_>>().len() != 4
            || radii.iter().any(|&r| {
                !r.is_finite() || r <= 8. * eps || r + 8. * eps >= f.lengths[0].min(f.lengths[1])
            })
        {
            return Err(unproven("rayon ou extrémité du raccord indéterminés"));
        }
        let h = f.lengths[2];
        let delta = radii[1] - radii[0];
        // Both radii and both terminal positions carry uncertainty. Require
        // the full interval to stay away from zero and the parabolic cuts.
        if h <= 8. * eps
            || delta.abs() <= 8. * eps
            || (delta.abs() + 4. * eps) / (h - 4. * eps) >= 1.
        {
            return Err(unproven(
                "pente nulle, indéterminée ou coupe non elliptique",
            ));
        }
        let b = delta / h;
        let d = 1. - b * b;
        let g = Self {
            radii,
            b,
            h,
            center: 1. / d,
            major: (1. + b * b).sqrt() / d,
            minor: 1. / d.sqrt(),
            theta: (d / 2.).sqrt().asin(),
            ends,
        };
        let worst = (g.major * radii[0].max(radii[1])).max((radii[0] / b).abs());
        if !worst.is_finite() || worst * 128. * f64::EPSILON > NATIVE.min(eps) {
            return Err(unproven("conditionnement numérique du cône insuffisant"));
        }
        Ok(g)
    }
}

#[derive(Default)]
struct ErrorBound {
    projection: f64,
    chord: f64,
    plane: f64,
}
impl ErrorBound {
    fn record(&mut self, actual: V, ideal: V, tol: f64) -> Result<(), FabError> {
        let e = norm(sub(actual, ideal));
        if !e.is_finite() || e > tol {
            return Err(unproven("déplacement vectoriel du congé hors tolérance"));
        }
        self.projection = self.projection.max(e);
        Ok(())
    }
}
struct Chart {
    mask: Vec<u8>,
    // Exact and continuously corresponding points on every analytic boundary.
    boundary: Vec<Option<V>>,
    trims: BTreeMap<Edge, [usize; 2]>,
}

fn boundaries(
    f: &Block,
    g: &Envelope,
    local: &[V],
    cs: &Chains,
    fit: f64,
    proof: &mut ErrorBound,
    work: &mut Work,
) -> Result<Chart, FabError> {
    work.spend(2 * local.len())?;
    let mut c = Chart {
        mask: vec![0; local.len()],
        boundary: vec![None; local.len()],
        trims: BTreeMap::new(),
    };
    for &face in &[0, 1, 2, 5] {
        let chain = &cs[&(face, CONE)];
        work.spend(chain.len())?;
        for &v in chain {
            c.mask[v] |= 1 << face;
        }
    }
    for (&(i, j), chain) in cs {
        work.spend(7 * chain.len())?;
        let mut ends = [chain[0], chain[chain.len() - 1]];
        let along = if j == CONE { 2 } else { 3 - i % 3 - j % 3 };
        let flip = !(j == CONE && i % 3 == 2) && local[ends[0]][along] > local[ends[1]][along];
        if flip {
            ends.reverse();
        }
        let mut previous = f64::NEG_INFINITY;
        for k in 0..chain.len() {
            let v = chain[if flip { chain.len() - 1 - k } else { k }];
            let p = local[v];
            let endpoint = k == 0 || k + 1 == chain.len();
            let (param, ideal) = if j == CONE {
                let uv = g.uv(p, c.mask[v]);
                if uv[0].abs() > g.theta || uv[1] < 0. || uv[1] > g.h {
                    return Err(unproven("frontière du cône hors domaine"));
                }
                (if i % 3 == 2 { uv[0] } else { uv[1] }, g.point(uv))
            } else {
                let mut q = p;
                q[i % 3] = f.height(i);
                q[j % 3] = f.height(j);
                let cap_side = (i % 3 == 2 && j < 2) || (j % 3 == 2 && i < 2);
                let low = if cap_side { g.radius(q[2]) } else { 0. };
                let high = f.lengths[along];
                q[along] = if k == 0 {
                    low
                } else if k + 1 == chain.len() {
                    high
                } else {
                    p[along]
                };
                if q[along] < low || q[along] > high {
                    return Err(unproven("bord plan hors bloc"));
                }
                (q[along], q)
            };
            if !param.is_finite() || param <= previous {
                return Err(unproven("frontière repliée du congé"));
            }
            let curve_corner = c.mask[v].count_ones() == 2;
            proof.record(
                p,
                ideal,
                if endpoint && !curve_corner {
                    fit.min(NATIVE / 4.)
                } else {
                    fit
                },
            )?;
            if let Some(prior) = c.boundary[v] {
                if norm(sub(prior, ideal)) > NATIVE / 8. {
                    return Err(unproven("coin partagé incompatible"));
                }
            }
            c.boundary[v] = Some(ideal);
            previous = param;
        }
        c.trims.insert((i, j), ends);
    }
    Ok(c)
}

#[allow(clippy::too_many_arguments)]
fn triangles(
    a: &Analysis,
    f: &Block,
    g: &Envelope,
    owners: &[usize],
    local: &[V],
    c: &Chart,
    o: &Options,
    proof: &mut ErrorBound,
    work: &mut Work,
) -> Result<(), FabError> {
    let mut area = Sum::default();
    let mut witness = [f64::INFINITY, f64::NEG_INFINITY];
    let eps = o
        .cylinder_fit_tolerance_mm
        .max(o.plane_tolerance_mm)
        .max(o.weld_tolerance_mm);
    for (&ids, &owner) in a.mesh.triangles.iter().zip(owners) {
        work.spend(16)?;
        if owner < CONE {
            let i = owner % 3;
            let mut pp = [[0.; 3]; 3];
            for (j, v) in ids.into_iter().enumerate() {
                let mut p = local[v];
                let height = f.height(owner);
                let e = (p[i] - height).abs();
                if !e.is_finite() || e > o.plane_tolerance_mm {
                    return Err(unproven("sommet hors du support plan"));
                }
                proof.plane = proof.plane.max(e);
                p[i] = height;
                pp[j] = c.boundary[v].unwrap_or(p);
            }
            let [p, q, r] = pp;
            let u = sub(q, p);
            let v = sub(r, p);
            if cross(u, v)[i] * if owner < 3 { -1. } else { 1. }
                <= 128. * f64::EPSILON * norm(u) * norm(v)
            {
                return Err(unproven("triangle plan replié sur le bloc"));
            }
            continue;
        }
        let mut uv = [[0.; 2]; 3];
        for (j, v) in ids.into_iter().enumerate() {
            let p = local[v];
            let [t, z] = g.uv(p, c.mask[v]);
            if !t.is_finite() || !z.is_finite() || t.abs() > g.theta || z < 0. || z > g.h {
                return Err(unproven("sommet hors du rectangle du congé"));
            }
            proof.record(p, g.point([t, z]), o.cylinder_fit_tolerance_mm)?;
            uv[j] = [t, z];
            let r = g.radius(z);
            let circle_error = ((p[0] - r).hypot(p[1] - r) - r).abs();
            if c.mask[v] == 0
                && t.abs() < g.theta * 0.8
                && z > 8. * eps
                && z < g.h - 8. * eps
                && circle_error > 8. * eps
            {
                witness[0] = witness[0].min(z);
                witness[1] = witness[1].max(z);
            }
        }
        // Positive chart area proves multiplicity. Also check the actual
        // source normal against the physical (outward) sheet of the cone.
        let t = uv.iter().map(|p| p[0]).sum::<f64>() / 3.;
        let (sin, cos) = t.sin_cos();
        let et = [
            (g.major * sin + g.minor * cos) * FRAC_1_SQRT_2,
            (g.major * sin - g.minor * cos) * FRAC_1_SQRT_2,
            0.,
        ];
        let section = g.point([t, 0.]);
        let ez = [
            g.b * section[0] / g.radii[0],
            g.b * section[1] / g.radii[0],
            1.,
        ];
        let outward = cross(et, ez);
        let [v0, v1, v2] = ids.map(|v| local[v]);
        let source_normal = cross(sub(v1, v0), sub(v2, v0));
        if dot(source_normal, outward) <= 128. * f64::EPSILON * norm(source_normal) * norm(outward)
        {
            return Err(unproven("triangle replié sur la nappe du cône"));
        }
        let [p, q, r] = uv;
        let (dt0, dz0, dt1, dz1) = (q[0] - p[0], q[1] - p[1], r[0] - p[0], r[1] - p[1]);
        let det = dt0 * dz1 - dt1 * dz0;
        if !det.is_finite()
            || det <= 128. * f64::EPSILON * (dt0.abs() * dz1.abs() + dt1.abs() * dz0.abs())
        {
            return Err(unproven("triangle replié dans le domaine du cône"));
        }
        area.add(det / 2.);
        let span = |axis: usize| {
            uv.iter().map(|p| p[axis]).fold(f64::NEG_INFINITY, f64::max)
                - uv.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min)
        };
        let dt = span(0);
        let dz = span(1);
        let rmax = uv.iter().map(|p| g.radius(p[1])).fold(0., f64::max);
        // Barycentric Taylor remainder. Var(t), Var(z) <= range²/4,
        // |Cov(t,z)| <= range(t)*range(z)/4; S_zz vanishes identically.
        let chord = g.major * (rmax * dt * dt + 2. * g.b.abs() * dt * dz) / 8.;
        if !chord.is_finite() {
            return Err(unproven("borne continue du cône non finie"));
        }
        proof.chord = proof.chord.max(chord);
    }
    let target = 2. * g.theta * g.h;
    if witness[1] - witness[0] <= 8. * eps || (area.get() - target).abs() > target * 1e-8 {
        return Err(unproven(
            "couverture ou témoins distincts du modèle circulaire insuffisants",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn candidate(
    top: &[(f64, usize)],
    neighbors: [u8; 6],
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let Some(f) = frame(top, neighbors, a, s, work)? else {
        return Ok(None);
    };
    if !f.has_taper(a, s, o, work)? {
        return Ok(None);
    }
    work.spend(a.mesh.vertices.len() + a.mesh.triangles.len() + s.model.faces.len())?;
    let mut by_region = vec![CONE; s.model.faces.len()];
    for (face, r) in f.regions.into_iter().enumerate() {
        by_region[r] = face;
    }
    let local: Vec<_> = a.mesh.vertices.iter().map(|&p| f.local(p)).collect();
    let owners: Vec<_> = s.triangle_regions.iter().map(|&r| by_region[r]).collect();
    let mut cs = chains(a, &owners, adj, edges, work)?;
    let g = Envelope::new(&f, &local, &mut cs, o)?;
    let mut proof = ErrorBound::default();
    let c = boundaries(
        &f,
        &g,
        &local,
        &cs,
        o.cylinder_fit_tolerance_mm,
        &mut proof,
        work,
    )?;
    triangles(a, &f, &g, &owners, &local, &c, o, &mut proof, work)?;
    // Pay for the result vertex copy and final source-owner materialization.
    work.spend(a.mesh.vertices.len() + a.mesh.triangles.len())?;
    let sq = FRAC_1_SQRT_2;
    let mut vertices = a.mesh.vertices.clone();
    let mut arcs = Vec::with_capacity(2);
    let mut moved = 0;
    for (i, [y, x]) in g.ends.into_iter().enumerate() {
        let z = if i == 0 { 0. } else { g.h };
        let r = g.radii[i];
        // Only the four shared ellipse/rail corners are canonicalized. The
        // original Analysis stores the unmodified source mesh and attribution.
        for (v, q) in [(x, [r, 0., z]), (y, [0., r, z])] {
            let world = f.world(q);
            proof.record(vertices[v], world, o.cylinder_fit_tolerance_mm)?;
            if vertices[v] != world {
                moved += 1;
            }
            vertices[v] = world;
        }
        arcs.push(EllipticArc {
            center: f.world([r * g.center, r * g.center, z]),
            axis: f.axes[2],
            major_direction: f.vector([-sq, -sq, 0.]),
            major_mm: r * g.major,
            minor_mm: r * g.minor,
            start_rad: -g.theta,
            sweep_rad: 2. * g.theta,
            vertices: [y, x],
        });
    }
    let mut planes = Vec::with_capacity(6);
    for (i, rid) in f.regions.into_iter().enumerate() {
        let expected = if i % 3 == 2 { 5 } else { 4 };
        let segments = super::block::plane_segments(
            &s.model.faces[rid].loops[0],
            i,
            expected,
            edges,
            &owners,
            work,
            |ids, other| {
                let ends = c.trims[&edge(i, other)];
                let reversed = ids == [ends[1], ends[0]];
                if !reversed && ids != ends {
                    return Err(unproven("extrémités de plan incompatibles"));
                }
                Ok(if other == CONE && i % 3 == 2 {
                    let arc = usize::from(i == 5);
                    if reversed != (i == 2) {
                        return Err(unproven("orientation de coupe elliptique incohérente"));
                    }
                    MixedSegment::EllipticArc { arc, reversed }
                } else {
                    MixedSegment::Line { vertices: ids }
                })
            },
        )?;
        planes.push(MixedPlane {
            normal: f.normal(i),
            loops: vec![MixedLoop::Segments { segments }],
        });
    }
    let q = (1. + 2. * g.b * g.b).sqrt();
    let [lo, hi] = g.ends;
    let blend = MixedLinearBlend {
        apex: f.world([0., 0., -g.radii[0] / g.b]),
        axis: f.vector(scale([g.b, g.b, 1.], g.b.signum() / q)),
        semi_angle_rad: (g.b.abs() / q).asin(),
        arcs: [0, 1],
        rails: [[lo[1], hi[1]], [hi[0], lo[0]]],
    };
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                vertices,
                planes,
                elliptic_arcs: arcs,
                arcs: vec![],
                rings: vec![],
                cylinders: vec![],
                partial_cylinders: vec![],
            },
            cones: vec![],
        },
        linear_blend: Some(Box::new(blend)),
        octants: vec![],
        partial_cones: vec![],
        tori: vec![],
        spheres: vec![],
    };
    // Ordinary box corners remain the source coordinates. Their displacement
    // from the canonical line intersections is included in projection. A
    // planar disk's chordal/analytic symmetric difference is swept by the
    // certified boundary homotopy. Allow projection both ways along a plane.
    let wall = 2. * proof.plane + proof.projection;
    let deviation = o.weld_tolerance_mm + wall + proof.projection + proof.chord;
    if !deviation.is_finite() || deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le congé linéaire dépasse la borne d’écart.",
        ));
    }
    let mut diagnostics = vec![
        Diagnostic {
            code: "recognition.conic_intent_assumed".into(),
            severity: Severity::Warning,
            count: 1,
            message: "Le raccord admissible est interprété comme une portion de cône inclinée."
                .into(),
        },
        Diagnostic {
            code: "recognition.rounded_coverage".into(),
            severity: Severity::Info,
            count: owners.len(),
            message: "Tous les triangles sont attribués à six plans et un cône incliné.".into(),
        },
        Diagnostic {
            code: "recognition.bound_scope".into(),
            severity: Severity::Info,
            count: 1,
            message: "La borne cumule soudage, projections et interpolation du domaine elliptique."
                .into(),
        },
    ];
    if moved != 0 {
        diagnostics.push(Diagnostic{code:"recognition.linear_blend_endpoints_projected".into(),severity:Severity::Info,count:moved,
        message:"Les extrémités partagées ont été projetées sur les ellipses et les génératrices. Le déplacement est inclus dans la borne.".into()});
    }
    let report = RecognitionReport {
        cylinders: 0,
        cones: 1,
        spheres: 0,
        tori: 0,
        toroidal_fillets: 0,
        max_radial_residual_mm: proof.projection,
        max_wall_residual_mm: wall,
        max_chord_error_mm: proof.chord,
        max_deviation_bound_mm: deviation,
        expected_volume_mm3: volume(&model)?,
        diagnostics,
    };
    let triangle_faces = owners
        .into_iter()
        .map(|index| {
            if index == CONE {
                RoundedFaceRef::LinearBlend { index: 0 }
            } else {
                RoundedFaceRef::Plane { index }
            }
        })
        .collect();
    Ok(Some(RoundedRecognition {
        model,
        report,
        triangle_faces,
    }))
}
