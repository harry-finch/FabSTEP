//! Source-certified toroidal regions with interrupted, isoparametric boundaries.
use crate::math::{cross, dot, norm, sub};
use crate::{FabError, Mesh, SOURCE_MAX_TRIANGLES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod charts;
mod collars;
mod fit;
mod planes;
pub use planes::{residual_planes, ResidualPlane};

type V = [f64; 3];
type P = [f64; 2];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TorusAtlas {
    pub axis: V,
    pub radial: V,
    pub charts: Vec<TorusChart>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TorusChart {
    pub center_mm: V,
    pub major_radius_mm: f64,
    pub minor_radius_mm: f64,
    pub orientation: i8,
    pub triangles: Vec<usize>,
    pub loops: Vec<Vec<TorusTrimVertex>>,
    pub bound_mm: f64,
}

/// The flag identifies the isoparametric arc leading to the next vertex.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TorusTrimVertex {
    pub vertex: usize,
    pub uv: P,
    pub constant_u: bool,
}

#[derive(Clone, Copy)]
struct Frame([V; 3]);

impl Frame {
    fn local(self, p: V) -> V {
        self.0.map(|direction| dot(p, direction))
    }
    fn world(self, p: V) -> V {
        std::array::from_fn(|i| (0..3).map(|j| self.0[j][i] * p[j]).sum())
    }
}

fn unit(p: V) -> Option<V> {
    let length = norm(p);
    (length.is_finite() && length > 1e-14).then(|| p.map(|v| v / length))
}

#[derive(Clone, Copy, Debug)]
struct Torus {
    center: V,
    major: f64,
    minor: f64,
}

impl Torus {
    fn radial(self, p: V) -> f64 {
        (p[1] - self.center[1]).hypot(p[2] - self.center[2])
    }
    fn distance(self, p: V) -> f64 {
        (p[0] - self.center[0]).hypot(self.radial(p) - self.major) - self.minor
    }
    fn uv(self, p: V) -> P {
        fn lift(t: f64) -> f64 {
            if t < -0.075 {
                t + std::f64::consts::TAU
            } else {
                t
            }
        }
        [
            lift((p[2] - self.center[2]).atan2(p[1] - self.center[1])),
            lift((p[0] - self.center[0]).atan2(self.radial(p) - self.major)),
        ]
    }
    fn point(self, [u, v]: P) -> V {
        let rho = self.major + self.minor * v.cos();
        [
            self.center[0] + self.minor * v.sin(),
            self.center[1] + rho * u.cos(),
            self.center[2] + rho * u.sin(),
        ]
    }
}

struct Source<'a> {
    points: Vec<V>,
    tris: &'a [[usize; 3]],
    twin: Vec<usize>,
    normal_x: Vec<f64>,
    frame: Frame,
}
impl<'a> Source<'a> {
    fn edge(&self, e: usize) -> (usize, usize) {
        let t = self.tris[e / 3];
        (t[e % 3], t[(e + 1) % 3])
    }
    fn prepare(mesh: &'a Mesh) -> Option<Self> {
        let n = mesh.triangles.len();
        let mut areas = BTreeMap::<[i16; 3], (f64, V)>::new();
        let mut normals = Vec::with_capacity(n);
        for t in &mesh.triangles {
            if t.iter().any(|v| *v >= mesh.vertices.len()) {
                return None;
            }
            let [a, b, c] = t.map(|i| mesh.vertices[i]);
            let raw = cross(sub(b, a), sub(c, a));
            let weight = norm(raw);
            let mut direction = unit(raw)?;
            let k = (0..3).max_by(|&i, &j| direction[i].abs().total_cmp(&direction[j].abs()))?;
            if direction[k] < 0. {
                direction = direction.map(|v| -v);
            }
            let cell = areas
                .entry(direction.map(|v| (v * 1000.).round() as i16))
                .or_default();
            cell.0 += weight;
            for (i, x) in direction.iter().enumerate() {
                cell.1[i] += weight * x;
            }
            normals.push(direction);
        }
        let axis = unit(areas.values().max_by(|a, b| a.0.total_cmp(&b.0))?.1)?;
        let j = (0..3).min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))?;
        let radial = unit(std::array::from_fn(|i| {
            if i == j {
                1. - axis[j] * axis[i]
            } else {
                -axis[j] * axis[i]
            }
        }))?;
        let frame = Frame([axis, radial, cross(axis, radial)]);
        let mut coedges = Vec::with_capacity(3 * n);
        for (i, t) in mesh.triangles.iter().enumerate() {
            for j in 0..3 {
                let (a, b) = (t[j], t[(j + 1) % 3]);
                coedges.push((a.min(b) as u32, a.max(b) as u32, (3 * i + j) as u32));
            }
        }
        coedges.sort_unstable();
        let mut twin = vec![0; 3 * n];
        for pair in coedges.chunks_exact(2) {
            let [(a, b, left), (c, d, right)] = pair else {
                unreachable!()
            };
            let (l, r) = (*left as usize, *right as usize);
            if a == b
                || a != c
                || b != d
                || mesh.triangles[l / 3][l % 3] != mesh.triangles[r / 3][(r + 1) % 3]
            {
                return None;
            }
            twin[l] = r;
            twin[r] = l;
        }
        if !coedges.len().is_multiple_of(2)
            || coedges
                .windows(3)
                .any(|p| p[0].0 == p[2].0 && p[0].1 == p[2].1)
        {
            return None;
        }
        // The document pipeline invokes recognition per connected body.
        // Refuse an external caller that mixes disjoint roots in one atlas.
        let mut seen = vec![false; n];
        let mut todo = vec![0];
        seen[0] = true;
        let mut faces = 0;
        while let Some(i) = todo.pop() {
            faces += 1;
            for &coedge in &twin[3 * i..3 * i + 3] {
                let j = coedge / 3;
                if !seen[j] {
                    seen[j] = true;
                    todo.push(j);
                }
            }
        }
        if faces != n {
            return None;
        }
        Some(Self {
            points: mesh.vertices.iter().map(|p| frame.local(*p)).collect(),
            tris: &mesh.triangles,
            twin,
            normal_x: normals.iter().map(|v| dot(*v, axis).abs()).collect(),
            frame,
        })
    }
}

/// Called only when an analytical or oversized source is eligible for this pass.
/// Source triangles are identified in the welded analysis mesh. The native solid
/// and the reimported document must independently validate the returned charts.
pub fn recognize(mesh: &Mesh, max_deviation_mm: f64) -> Result<Option<TorusAtlas>, FabError> {
    let n = mesh.triangles.len();
    if !(256..=SOURCE_MAX_TRIANGLES).contains(&n)
        || mesh.vertices.len() > 3 * SOURCE_MAX_TRIANGLES
        || mesh.vertices.iter().flatten().any(|v| !v.is_finite())
        || !max_deviation_mm.is_finite()
        || max_deviation_mm <= 0.
    {
        return Ok(None);
    }
    let Some(src) = Source::prepare(mesh) else {
        return Ok(None);
    };
    let limits = max_deviation_mm.min(0.05);
    let supports = fit::discover(&src);
    if supports.is_empty() {
        return Ok(None);
    }
    let mut guard = vec![false; src.points.len()];
    for _ in 0..5 {
        let mut blocked = guard.clone();
        let mut selected = Vec::new();
        let mut coverage = 0;
        for model in &supports {
            let mut patches = charts::measure(&src, *model, &blocked, limits);
            patches.sort_by_key(|p| std::cmp::Reverse(p.triangles.len()));
            for patch in patches {
                if patch
                    .triangles
                    .iter()
                    .any(|&i| src.tris[i].iter().any(|&v| blocked[v]))
                {
                    continue;
                }
                for &i in &patch.triangles {
                    for &v in &src.tris[i] {
                        blocked[v] = true;
                    }
                }
                coverage += patch.triangles.len();
                selected.push(patch);
            }
        }
        if coverage == 0 || n - coverage + selected.len() > 100_000 {
            return Ok(None);
        }
        let failed = collars::unsafe_vertices(&src, &selected);
        if failed.is_empty() {
            for p in &mut selected {
                p.center_mm = src.frame.world(p.center_mm);
            }
            return Ok(Some(TorusAtlas {
                axis: src.frame.0[0],
                radial: src.frame.0[1],
                charts: selected,
            }));
        }
        for v in failed {
            guard[v] = true;
        }
    }
    Ok(None)
}
