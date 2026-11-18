//! A single bicubic height field over an axis-aligned rectangular base.
//! The five planar charts keep their exact boundaries. A degree-six
//! Bernstein enclosure covers the entire projection of every roof facet.
use crate::math::{bounds, cross, sub, Sum, V};
use crate::{Analysis, Diagnostic, FabError, Options, RecognitionReport, Severity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const ROOF: usize = 5;
const MAX_TRIANGLES: usize = 32_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightPatch {
    pub origin: V,
    pub size: [f64; 2],
    /// Cubic Bernstein control heights above origin.z, indexed [u][v].
    pub heights: [[f64; 4]; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightPatchRecognition {
    pub model: HeightPatch,
    /// Owners in source-analysis triangle order: x-, x+, y-, y+, base, roof.
    pub triangle_faces: Vec<usize>,
    pub report: RecognitionReport,
}

fn unproven() -> FabError {
    FabError::new("recognition.height_patch_unproven",
        "Le maillage ne démontre pas une surface de hauteur bicubique à bord rectangulaire horizontal.")
}
fn check(pass: bool) -> Result<(), FabError> {
    if pass {
        Ok(())
    } else {
        Err(unproven())
    }
}

type Edge = (usize, usize);
fn edge(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}

struct Charts {
    owners: Vec<usize>,
    roof_vertices: BTreeSet<usize>,
    rim_z: f64,
}

fn certify_charts(a: &Analysis, lo: V, hi: V) -> Result<Charts, FabError> {
    let mesh = &a.mesh;
    let mut owners = Vec::with_capacity(mesh.triangles.len());
    let mut triangles_per_chart = [0_usize; 6];
    let mut chart_areas: [Sum; 6] = std::array::from_fn(|_| Sum::default());
    let mut nodes: [BTreeSet<usize>; 6] = std::array::from_fn(|_| BTreeSet::new());
    let mut used = vec![false; mesh.vertices.len()];
    let mut edge_owners = BTreeMap::<Edge, (usize, usize, Option<usize>)>::new();
    for (t, &tri) in mesh.triangles.iter().enumerate() {
        let p = tri.map(|id| mesh.vertices[id]);
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let mut owner = ROOF;
        for (id, level) in [lo[0], hi[0], lo[1], hi[1], lo[2]].into_iter().enumerate() {
            let axis = id / 2;
            if p.iter().all(|v| v[axis] == level) {
                check(owner == ROOF)?;
                owner = id;
            }
        }
        let signed = if owner % 2 == 0 {
            -n[owner / 2]
        } else {
            n[owner / 2]
        };
        check(signed.is_finite() && signed > 0.)?;
        owners.push(owner);
        nodes[owner].extend(tri);
        for id in tri {
            used[id] = true;
        }
        triangles_per_chart[owner] += 1;
        chart_areas[owner].add(signed / 2.);
        for (from, to) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            match edge_owners.entry(edge(from, to)) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert((from, t, None));
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    let (prior, _, next) = slot.get_mut();
                    check(*prior == to && next.replace(t).is_none())?;
                }
            }
        }
    }
    check(used.into_iter().all(|v| v) && triangles_per_chart.iter().all(|&n| n > 0))?;
    let mut adjacency = vec![Vec::new(); owners.len()];
    let mut edge_count = [0_usize; 6];
    let mut chains = BTreeMap::<Edge, Vec<Edge>>::new();
    let mut rim_z = None;
    for (&(u, v), &(_, t, next)) in &edge_owners {
        let other = next.ok_or_else(unproven)?;
        let (i, j) = (owners[t], owners[other]);
        edge_count[i] += 1;
        if i == j {
            adjacency[t].push(other);
            adjacency[other].push(t);
        } else {
            edge_count[j] += 1;
            check(i / 2 != j / 2)?;
            let pair = edge(i, j);
            let along = 3 - i / 2 - j / 2;
            let (from, to) = if mesh.vertices[u][along] < mesh.vertices[v][along] {
                (u, v)
            } else {
                (v, u)
            };
            check(mesh.vertices[from][along] < mesh.vertices[to][along])?;
            chains.entry(pair).or_default().push((from, to));
            if pair.1 == ROOF {
                let z = *rim_z.get_or_insert(mesh.vertices[u][2]);
                check(mesh.vertices[u][2] == z && mesh.vertices[v][2] == z)?;
            }
        }
    }
    let rim_z = rim_z.ok_or_else(unproven)?;
    check(rim_z > lo[2] && chains.len() == 12)?;
    let cuboid_hi = [hi[0], hi[1], rim_z];
    // Twelve unique source polylines. Each goes monotonically corner to corner
    // along its prescribed edge and neighboring polylines share vertex IDs.
    let mut corners = BTreeMap::new();
    for ((i, j), mut segments) in chains {
        let along = 3 - i / 2 - j / 2;
        segments.sort_unstable_by(|&(u, _), &(v, _)| {
            mesh.vertices[u][along].total_cmp(&mesh.vertices[v][along])
        });
        check(segments.windows(2).all(|q| q[0].1 == q[1].0))?;
        for (end, id) in [segments[0].0, segments.last().unwrap().1]
            .into_iter()
            .enumerate()
        {
            let mut labels = [0; 3];
            labels[along] = end;
            labels[i / 2] = i % 2;
            labels[j / 2] = j % 2;
            let p = mesh.vertices[id];
            check((0..3).all(|axis| {
                p[axis]
                    == if labels[axis] == 0 {
                        lo[axis]
                    } else {
                        cuboid_hi[axis]
                    }
            }))?;
            check(*corners.entry(labels).or_insert(id) == id)?;
        }
    }
    check(corners.len() == 8)?;
    let mut seen = vec![false; owners.len()];
    for i in 0..6 {
        // Edge-connected disk with a unique simple rectangular boundary.
        // A strictly oriented projected triangulation has degree one there.
        check(nodes[i].len() + triangles_per_chart[i] == edge_count[i] + 1)?;
        let seed = owners.iter().position(|&o| o == i).ok_or_else(unproven)?;
        let mut stack = vec![seed];
        let mut count = 0;
        seen[seed] = true;
        while let Some(t) = stack.pop() {
            count += 1;
            for &j in &adjacency[t] {
                if !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        check(count == triangles_per_chart[i])?;
        let axes = [(i / 2 + 1) % 3, (i / 2 + 2) % 3];
        let area = (cuboid_hi[axes[0]] - lo[axes[0]]) * (cuboid_hi[axes[1]] - lo[axes[1]]);
        check(area.is_finite() && (chart_areas[i].get() - area).abs() <= area * 1e-9)?;
    }
    Ok(Charts {
        owners,
        roof_vertices: std::mem::take(&mut nodes[ROOF]),
        rim_z,
    })
}

fn cubic(t: f64) -> [f64; 4] {
    let s = 1. - t;
    [s * s * s, 3. * t * s * s, 3. * t * t * s, t * t * t]
}

/// Stream a four-column QR using Givens rotations, then bound the condition
/// number of its triangular factor before solving for the four interior poles.
#[derive(Default)]
struct Fit {
    r: [[f64; 5]; 4],
}
impl Fit {
    fn add(&mut self, mut row: [f64; 5]) {
        for i in 0..4 {
            let d = self.r[i][i].hypot(row[i]);
            if d == 0. {
                continue;
            }
            let (c, s) = (self.r[i][i] / d, row[i] / d);
            for (a, b) in self.r[i][i..].iter_mut().zip(&mut row[i..]) {
                let (u, v) = (*a, *b);
                *a = c * u + s * v;
                *b = c * v - s * u;
            }
        }
    }
    fn solve(&self, mut rhs: [f64; 4]) -> [f64; 4] {
        for i in (0..4).rev() {
            rhs[i] =
                (rhs[i] - (i + 1..4).map(|j| self.r[i][j] * rhs[j]).sum::<f64>()) / self.r[i][i];
        }
        rhs
    }
    fn controls(&self) -> Result<[f64; 4], FabError> {
        check((0..4).all(|i| self.r[i][i].is_finite() && self.r[i][i] > 0.))?;
        let inv: [[_; 4]; 4] = std::array::from_fn(|j| {
            self.solve(std::array::from_fn(|i| if i == j { 1. } else { 0. }))
        });
        let norm = self
            .r
            .iter()
            .map(|row| row[..4].iter().map(|x| x.abs()).sum())
            .fold(0_f64, f64::max);
        let inv_norm = (0..4)
            .map(|i| inv.iter().map(|column| column[i].abs()).sum())
            .fold(0_f64, f64::max);
        check(norm * inv_norm < 1e5)?;
        Ok(self.solve(self.r.map(|r| r[4])))
    }
}

fn weights(uv: [f64; 2]) -> [f64; 4] {
    let [u, v] = uv.map(cubic);
    [u[1] * v[1], u[1] * v[2], u[2] * v[1], u[2] * v[2]]
}
fn project(p: V, lo: V, size: [f64; 2]) -> [f64; 2] {
    [(p[0] - lo[0]) / size[0], (p[1] - lo[1]) / size[1]]
}

const TRIPLES: [[usize; 3]; 10] = [
    [3, 0, 0],
    [0, 3, 0],
    [0, 0, 3],
    [2, 1, 0],
    [2, 0, 1],
    [1, 2, 0],
    [0, 2, 1],
    [1, 0, 2],
    [0, 1, 2],
    [1, 1, 1],
];
const FACT: [usize; 7] = [1, 1, 2, 6, 24, 120, 720];
fn multi(n: usize, a: [usize; 3]) -> usize {
    FACT[n] / (FACT[a[0]] * FACT[a[1]] * FACT[a[2]])
}

/// Coefficients of the polar form of the inner two cubic basis functions.
fn blossom(ts: [f64; 3], a: [usize; 3]) -> [f64; 2] {
    let mut b = [1., 0., 0., 0.];
    for (t, n) in ts.into_iter().zip(a) {
        for _ in 0..n {
            for k in (1..4).rev() {
                b[k] = b[k] * (1. - t) + b[k - 1] * t;
            }
            b[0] *= 1. - t;
        }
    }
    [b[1], b[2]]
}

struct Product {
    u: usize,
    v: usize,
    into: usize,
    weight: f64,
}
struct DegreeSix {
    product: Vec<Product>,
    bary: Vec<[f64; 3]>,
}
impl DegreeSix {
    fn new() -> Self {
        let mut entries = BTreeMap::new();
        let mut bary = Vec::new();
        let mut product = Vec::new();
        for (u, a) in TRIPLES.into_iter().enumerate() {
            for (v, b) in TRIPLES.into_iter().enumerate() {
                let key = std::array::from_fn(|k| a[k] + b[k]);
                let into = *entries.entry(key).or_insert_with(|| {
                    bary.push(key.map(|n| n as f64 / 6.));
                    bary.len() - 1
                });
                let weight = (multi(3, a) * multi(3, b)) as f64 / multi(6, key) as f64;
                product.push(Product { u, v, into, weight });
            }
        }
        Self { product, bary }
    }
    fn bound(&self, uv: [[f64; 2]; 3], z: [f64; 3], coeff: [f64; 4]) -> f64 {
        let u = TRIPLES.map(|a| blossom(uv.map(|p| p[0]), a));
        let v = TRIPLES.map(|a| blossom(uv.map(|p| p[1]), a));
        let rows = u.map(|[x, y]| [x * coeff[0] + y * coeff[2], x * coeff[1] + y * coeff[3]]);
        let mut controls = [0.; 28];
        for &Product {
            u,
            v: b,
            into,
            weight,
        } in &self.product
        {
            controls[into] += weight * (rows[u][0] * v[b][0] + rows[u][1] * v[b][1]);
        }
        // Elevate the linear triangle to degree six by barycentric averaging.
        // Bernstein weights are nonnegative and sum to one over the triangle.
        self.bary
            .iter()
            .zip(controls)
            .map(|(b, h)| (h - b[0] * z[0] - b[1] * z[1] - b[2] * z[2]).abs())
            .fold(0., f64::max)
    }
}

pub fn recognize_height_patch(
    a: &Analysis,
    o: &Options,
) -> Result<HeightPatchRecognition, FabError> {
    check(a.mesh.triangles.len() <= MAX_TRIANGLES && a.mesh.vertices.len() <= 3 * MAX_TRIANGLES)?;
    crate::recognition::validate_analysis(a)?;
    for t in [
        o.max_deviation_mm,
        o.weld_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.plane_tolerance_mm,
    ] {
        if !t.is_finite() || t <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Tolérance invalide.",
            ));
        }
    }
    let [lo, hi] = bounds(&a.mesh.vertices);
    let size = [hi[0] - lo[0], hi[1] - lo[1]];
    let fit_tolerance = o.cylinder_fit_tolerance_mm.min(2e-5);
    check(
        size.iter()
            .all(|v| v.is_finite() && *v > 10. * fit_tolerance),
    )?;
    let charts = certify_charts(a, lo, hi)?;
    let h = charts.rim_z - lo[2];
    let mut fit = Fit::default();
    let mut relief = 0_f64;
    for &id in &charts.roof_vertices {
        let p = a.mesh.vertices[id];
        let [a, b, c, d] = weights(project(p, lo, size));
        let z = p[2] - charts.rim_z;
        relief = relief.max(z.abs());
        fit.add([a, b, c, d, z]);
    }
    check(relief > 8. * fit_tolerance)?;
    let coeff = fit.controls()?;
    let mut heights = [[h; 4]; 4];
    for (i, v) in coeff.iter().enumerate() {
        heights[1 + i / 2][1 + i % 2] += *v;
    }
    check(
        heights
            .iter()
            .flatten()
            .all(|z| z.is_finite() && *z > fit_tolerance),
    )?;
    // Certify the rounded coefficients that are actually serialized.
    let coeff = std::array::from_fn(|i| heights[1 + i / 2][1 + i % 2] - h);
    let mut residual = 0_f64;
    for &id in &charts.roof_vertices {
        let p = a.mesh.vertices[id];
        let w = weights(project(p, lo, size));
        residual = residual
            .max(((0..4).map(|k| w[k] * coeff[k]).sum::<f64>() - (p[2] - charts.rim_z)).abs());
    }
    check(residual <= fit_tolerance)?;
    let proof = DegreeSix::new();
    let mut continuous = 0_f64;
    for (&tri, &owner) in a.mesh.triangles.iter().zip(&charts.owners) {
        if owner == ROOF {
            let points = tri.map(|id| a.mesh.vertices[id]);
            continuous = continuous.max(proof.bound(
                points.map(|p| project(p, lo, size)),
                points.map(|p| p[2] - charts.rim_z),
                coeff,
            ));
        }
    }
    // Bound floating-point chart evaluation and world-space control placement.
    let variation = coeff.into_iter().map(f64::abs).fold(0., f64::max);
    let scale = hi
        .into_iter()
        .chain(lo)
        .map(f64::abs)
        .fold(1., f64::max)
        .max(variation)
        .max(h);
    let rounding =
        1024. * f64::EPSILON * scale * (1. + 6. * variation * (1. / size[0] + 1. / size[1]));
    let chord = continuous + rounding;
    let bound = o.weld_tolerance_mm + residual + chord;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "La borne continue du relief bicubique dépasse le budget demandé.",
        ));
    }
    let volume = size[0] * size[1] * heights.iter().flatten().sum::<f64>() / 16.;
    check(volume.is_finite() && volume > 0.)?;
    Ok(HeightPatchRecognition {
        model: HeightPatch { origin: lo, size, heights },
        triangle_faces: charts.owners,
        report: RecognitionReport {
            revolutions: 0,
            cylinders: 0,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            max_radial_residual_mm: 0.,
            max_wall_residual_mm: residual,
            max_chord_error_mm: chord,
            max_deviation_bound_mm: bound,
            expected_volume_mm3: volume,
            diagnostics: vec![Diagnostic {
                code: "recognition.height_patch_fitted".into(),
                severity: Severity::Warning,
                count: 1,
                message: "Le relief est interprété comme une surface bicubique. L’écart est borné sur chaque triangle du toit.".into(),
            }],
        },
    })
}
