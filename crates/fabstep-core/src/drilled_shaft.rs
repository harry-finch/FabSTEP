//! A convex round shaft pierced by one concave orthogonal through bore.
//! Four disjoint charts, four closed rims and opposite source incidences are
//! required. Neither CAD labels nor nominal fixture dimensions enter the fit.
use crate::math::{cross, dot, norm, scale, sub, Sum, V};
use crate::recognition::Work;
use crate::rounded::Rim;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

const MAX_TRIANGLES: usize = 4096;
const NATIVE: f64 = 5e-7;

fn along(p: V, dir: V, t: f64) -> V {
    std::array::from_fn(|i| p[i] + t * dir[i])
}
fn unit(v: V) -> Option<V> {
    let n = norm(v);
    (n > 1e-12 && n.is_finite()).then(|| scale(v, 1. / n))
}
fn radial(p: V, d: V) -> V {
    along(p, d, -dot(p, d))
}
fn turn(d: f64) -> f64 {
    d - TAU * (d / TAU).round()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrilledShaft {
    pub origin: V,
    pub shaft_axis: V,
    pub bore_axis: V,
    pub height_mm: f64,
    pub radius_mm: f64,
    pub bore_radius_mm: f64,
    pub bore_offset_mm: f64,
    pub bore_height_mm: f64,
    /// Bottom, top, positive and negative bore mouths, oriented on the shaft.
    pub cycles: [Vec<usize>; 4],
    pub witness_tolerance_mm: f64,
}
impl DrilledShaft {
    fn local(&self, p: V) -> V {
        let d = sub(p, self.origin);
        [
            dot(d, self.bore_axis),
            dot(d, cross(self.shaft_axis, self.bore_axis)),
            dot(d, self.shaft_axis),
        ]
    }

    pub(crate) fn volume_mm3(&self) -> f64 {
        // At y=offset+r*sin(t), removed area is the product of two chord
        // lengths. Reflection about the endpoints makes this a periodic
        // midpoint rule. With (|off|+r)/R < 2/3, the integrand remains analytic
        // for |Im(t)| < 0.5; the native verifier also encloses the volume by
        // a separate outward-rounded series on the transferred geometry.
        let (r, big, off) = (self.bore_radius_mm, self.radius_mm, self.bore_offset_mm);
        let mut sum = Sum::default();
        for i in 0..256 {
            let t = -PI / 2. + PI * (i as f64 + 0.5) / 256.;
            let y = off + r * t.sin();
            sum.add(t.cos().powi(2) * ((big - y) * (big + y)).sqrt());
        }
        PI * big * big * self.height_mm - 4. * r * r * PI / 256. * sum.get()
    }

    pub fn valid_witnesses(&self, points: &[V]) -> bool {
        // The interpolation and high-order volume quadrature here are limited
        // to separated mouths, finite machine-part scales and orthogonal axes.
        let t = self.witness_tolerance_mm;
        let r = self.bore_radius_mm;
        let big = self.radius_mm;
        let limit = self.height_mm.max(big).max(1.);
        if ![
            t,
            r,
            big,
            self.height_mm,
            self.bore_height_mm,
            self.bore_offset_mm,
        ]
        .into_iter()
        .chain(self.origin)
        .chain(self.shaft_axis)
        .chain(self.bore_axis)
        .all(f64::is_finite)
            || self.origin.iter().any(|v| v.abs() > 1024.)
            || !(0.0..=1e-5).contains(&t)
            || t == 0.
            || big > 32.
            || self.height_mm > 32.
            || r <= 0.1
            || big - 1.5 * (self.bore_offset_mm.abs() + r) <= 4. * t
            || self.bore_height_mm <= r + 0.1
            || self.height_mm - self.bore_height_mm <= r + 0.1
            || (norm(self.shaft_axis) - 1.).abs() * limit > 1e-10
            || (norm(self.bore_axis) - 1.).abs() * limit > 1e-10
            || dot(self.shaft_axis, self.bore_axis).abs() * limit > 1e-10
        {
            return false;
        }
        if points.len() > 3 * MAX_TRIANGLES
            || points
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || x.abs() > 1024.)
        {
            return false;
        }
        let mut distinct = BTreeSet::new();
        self.cycles.iter().enumerate().all(|(i, ids)| {
            (12..=MAX_TRIANGLES).contains(&ids.len())
                && ids.iter().all(|&v| v < points.len() && distinct.insert(v))
                && self.rim(i, ids, points).is_some()
        })
    }
    /// Bound the chart displacement of the entire rim by its endpoint errors
    /// and max(second derivative)*angular step squared / 8. Cylinder charts
    /// use physical arc length, so mapping a chart back into 3D is nonexpansive.
    fn rim(&self, which: usize, ids: &[usize], points: &[V]) -> Option<[f64; 2]> {
        let (r, big, y0, z0) = (
            self.bore_radius_mm,
            self.radius_mm,
            self.bore_offset_mm,
            self.bore_height_mm,
        );
        let cap = which < 2;
        let sign = if which == 2 { 1. } else { -1. };
        let winding = if which == 0 || which == 3 { 1. } else { -1. };
        let mut total = Sum::default();
        let (mut gap2, mut ds, mut db) = (0_f64, 0_f64, 0_f64);
        let phase = |p: V| {
            if cap {
                p[1].atan2(p[0])
            } else {
                (p[2] - z0).atan2(p[1] - y0)
            }
        };
        for j in 0..ids.len() {
            let p = self.local(points[ids[j]]);
            let q = self.local(points[ids[(j + 1) % ids.len()]]);
            let t = phase(p);
            let delta = turn(phase(q) - t);
            if winding * delta <= 1e-10 || delta.abs() >= PI / 2. {
                return None;
            }
            total.add(delta);
            gap2 = gap2.max(delta * delta / 8.);
            if cap {
                let dz = (p[2] - if which == 0 { 0. } else { self.height_mm }).abs();
                let dr = (p[0].hypot(p[1]) - big).abs();
                if dr.hypot(dz) > self.witness_tolerance_mm {
                    return None;
                }
                ds = ds.max(dz);
                db = db.max(dr);
            } else {
                let y = y0 + r * t.cos();
                let z = z0 + r * t.sin();
                let x = sign * ((big - y) * (big + y)).sqrt();
                if norm(sub(p, [x, y, z])) > self.witness_tolerance_mm {
                    return None;
                }
                ds = ds.max((big * turn(p[1].atan2(p[0]) - y.atan2(x))).hypot(p[2] - z));
                db = db.max((p[0] - x).abs());
            }
        }
        if (total.get() - winding * TAU).abs() > 1e-8 {
            return None;
        }
        if cap {
            return Some([ds, db + big * gap2]);
        }
        let ymax = y0.abs() + r;
        let xmin = ((big - ymax) * (big + ymax)).sqrt();
        let ddx = r * ymax / xmin + r * r * big * big / xmin.powi(3);
        let dds = r * big / xmin + r * r * big * ymax / xmin.powi(3);
        Some([ds + dds * gap2, db + ddx * gap2])
    }
}

fn circle(points: &[V], axis: V, tol: f64) -> Option<(V, f64)> {
    let origin = *points.first()?;
    let q: Vec<_> = points
        .iter()
        .map(|&p| radial(sub(p, origin), axis))
        .collect();
    let a = *q
        .iter()
        .max_by(|&&x, &&y| dot(x, x).total_cmp(&dot(y, y)))?;
    let b = *q.iter().max_by(|&&x, &&y| {
        dot(cross(a, x), axis)
            .abs()
            .total_cmp(&dot(cross(a, y), axis).abs())
    })?;
    let den = 2. * dot(cross(a, b), axis);
    if den.abs() <= 1e-6_f64.max(0.1 * dot(a, a)) {
        return None;
    }
    let c = along(
        along(origin, cross(b, axis), dot(a, a) / den),
        cross(axis, a),
        dot(b, b) / den,
    );
    let radii: Vec<_> = points
        .iter()
        .map(|&p| norm(radial(sub(p, c), axis)))
        .collect();
    let r = radii.iter().sum::<f64>() / radii.len() as f64;
    radii.iter().all(|v| (v - r).abs() <= tol).then_some((c, r))
}
fn area(uv: [[f64; 2]; 3]) -> f64 {
    let [a, b, c] = uv;
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn wall(tri: [V; 3], r: f64, sense: f64, tol: f64, area3: f64) -> Option<f64> {
    let mut uv = tri.map(|p| [p[1].atan2(p[0]), p[2]]);
    let start = uv[0][0];
    for p in &mut uv {
        p[0] = turn(p[0] - start);
    }
    let angles = uv.map(|p| p[0]);
    let span = angles.into_iter().fold(f64::NEG_INFINITY, f64::max)
        - angles.into_iter().fold(f64::INFINITY, f64::min);
    let err = tri
        .into_iter()
        .map(|p| (p[0].hypot(p[1]) - r).abs())
        .fold(0., f64::max);
    (err <= tol && span < PI && sense * r * area(uv) > 1e-10_f64.max(area3 * 0.5))
        .then_some(err + r * span * span / 8.)
}
fn single_cycle(edges: &[(usize, usize)]) -> Option<Vec<usize>> {
    let successors: BTreeMap<_, _> = edges.iter().copied().collect();
    if successors.len() != edges.len() || edges.len() < 12 {
        return None;
    }
    let start = *successors.keys().next()?;
    let mut curr = start;
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    while seen.insert(curr) {
        ids.push(curr);
        curr = *successors.get(&curr)?;
    }
    (curr == start && ids.len() == edges.len()).then_some(ids)
}
/// Quantization can break both analytic rim seeds at a thin cap triangle.
/// Nominate planar clusters by adjacent normals, then recheck the whole
/// planar disk and its circle using the source coordinates and absolute fit.
fn recovered_caps(mesh: &Mesh, adjacency: &[Vec<usize>], tol: f64) -> Option<[Rim; 2]> {
    let areas: Vec<_> = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|v| mesh.vertices[v]);
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    let normals = areas
        .iter()
        .copied()
        .map(unit)
        .collect::<Option<Vec<_>>>()?;
    let mut visited = vec![false; areas.len()];
    let mut caps = Vec::new();
    for seed in 0..areas.len() {
        if visited[seed] {
            continue;
        }
        visited[seed] = true;
        let mut region = vec![seed];
        let mut next = 0;
        while next < region.len() {
            let t = region[next];
            next += 1;
            for &n in &adjacency[t] {
                if !visited[n] && dot(normals[t], normals[n]) > 1. - 5e-7 {
                    visited[n] = true;
                    region.push(n);
                }
            }
        }
        if region.len() < 6 {
            continue;
        }
        let mut edges = BTreeSet::new();
        let mut sum = [0.; 3];
        for &t in &region {
            for (k, c) in sum.iter_mut().enumerate() {
                *c += areas[t][k];
            }
            let ids = mesh.triangles[t];
            for i in 0..3 {
                let (a, b) = (ids[i], ids[(i + 1) % 3]);
                if !edges.remove(&(b, a)) && !edges.insert((a, b)) {
                    return None;
                }
            }
        }
        let Some(axis) = unit(sum) else {
            continue;
        };
        let Some(vertices) = single_cycle(&edges.into_iter().collect::<Vec<_>>()) else {
            continue;
        };
        let points: Vec<_> = vertices.iter().map(|&v| mesh.vertices[v]).collect();
        let Some((center, radius)) = circle(&points, axis, tol) else {
            continue;
        };
        if region
            .iter()
            .flat_map(|&i| mesh.triangles[i])
            .any(|v| dot(sub(mesh.vertices[v], center), axis).abs() > tol)
        {
            continue;
        }
        caps.push(Rim {
            center,
            axis,
            radius,
            vertices,
            region: seed,
        });
        if caps.len() > 2 {
            return None;
        }
    }
    caps.try_into().ok()
}

pub(crate) fn candidate(a: &Analysis, o: &Options, work: &mut Work) -> Option<RoundedRecognition> {
    let mesh = &a.mesh;
    let n = mesh.triangles.len();
    if !(48..=MAX_TRIANGLES).contains(&n) || mesh.vertices.len() > 3 * MAX_TRIANGLES {
        return None;
    }
    // The new pass runs only on a fully faceted Auto result. Charge the
    // finite topology, clustering, both fits, four charts and rim cycles
    // before allocating optional tables. Historical budgets are untouched.
    if !work.reserve_optional(n * 400 + mesh.vertices.len() * 64 + 8192)
        || mesh
            .vertices
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1024.)
        || !a
            .model
            .faces
            .iter()
            .any(|f| f.loops.len() == 1 && f.loops[0].len() >= 8)
    {
        return None;
    }
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    let budget = o.max_deviation_mm.min(0.05);
    let adjacency = crate::pipeline::validate_topology(mesh).ok()?;
    let [bottom, top] = recovered_caps(mesh, &adjacency, tol)?;
    let inward = scale(bottom.axis, -1.);
    let axis = unit(sub(top.center, bottom.center))?;
    let height = norm(sub(top.center, bottom.center));
    let radius = (top.radius + bottom.radius) * 0.5;
    if (top.radius - bottom.radius).abs() > tol || dot(inward, axis) < 1. - 1e-10 {
        return None;
    }
    let normals: Vec<_> = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|v| mesh.vertices[v]);
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    let refdir = unit(radial(
        sub(mesh.vertices[bottom.vertices[0]], bottom.center),
        axis,
    ))?;
    let side = cross(axis, refdir);
    let mut unknown = Vec::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        let zs = t.map(|v| dot(sub(mesh.vertices[v], bottom.center), axis));
        if zs.iter().all(|z| z.abs() <= tol) || zs.iter().all(|z| (z - height).abs() <= tol) {
            continue;
        }
        let tri = t.map(|v| {
            let p = sub(mesh.vertices[v], bottom.center);
            [dot(p, refdir), dot(p, side), dot(p, axis)]
        });
        if wall(tri, radius, 1., tol, norm(normals[i])).is_none() {
            unknown.push(i);
        }
    }
    let n0 = unit(normals[*unknown.first()?])?;
    let n1 = unknown
        .iter()
        .filter_map(|&i| unit(normals[i]))
        .min_by(|&a, &b| dot(a, n0).abs().total_cmp(&dot(b, n0).abs()))?;
    if dot(n0, n1).abs() >= 0.5 {
        return None;
    }
    let inferred = unit(cross(n0, n1))?;
    if dot(inferred, axis).abs() >= 1e-5 {
        return None;
    }
    let bx = unit(radial(inferred, axis))?;
    let by = cross(axis, bx);
    let ids: BTreeSet<_> = unknown.iter().flat_map(|&i| mesh.triangles[i]).collect();
    let samples: Vec<_> = ids.iter().map(|&v| mesh.vertices[v]).collect();
    let (bore, r) = circle(&samples, bx, tol)?;
    let d = sub(bore, bottom.center);
    let mut model = DrilledShaft {
        origin: bottom.center,
        shaft_axis: axis,
        bore_axis: bx,
        height_mm: height,
        radius_mm: radius,
        bore_radius_mm: r,
        bore_offset_mm: dot(d, by),
        bore_height_mm: dot(d, axis),
        cycles: Default::default(),
        witness_tolerance_mm: tol,
    };
    // Classify against ALL four surfaces. Assigning whatever is left over
    // to the bore would silently admit overlaps and missing evidence.
    let p: Vec<_> = mesh.vertices.iter().map(|&p| model.local(p)).collect();
    let mut owners = Vec::with_capacity(n);
    let mut bounds = [0_f64; 4];
    let mut radial_error = 0_f64;
    let mut borders: BTreeMap<(usize, usize), (usize, usize, usize)> = BTreeMap::new();
    let mut curves: [Vec<(usize, usize)>; 4] = Default::default();
    for (i, t) in mesh.triangles.iter().enumerate() {
        let tri = t.map(|v| p[v]);
        let a3 = norm(normals[i]);
        let cap = |z: f64, sign: f64| {
            let err = tri.iter().map(|p| (p[2] - z).abs()).fold(0., f64::max);
            (err <= tol && sign * area(tri.map(|p| [p[0], p[1]])) > 1e-10_f64.max(a3 * 0.5))
                .then_some(err)
        };
        let bo = tri.map(|p| {
            [
                p[1] - model.bore_offset_mm,
                p[2] - model.bore_height_mm,
                p[0],
            ]
        });
        let fits = [
            cap(0., -1.),
            cap(height, 1.),
            wall(tri, radius, 1., tol, a3),
            wall(bo, r, -1., tol, a3),
        ];
        let mut valid = fits
            .into_iter()
            .enumerate()
            .filter_map(|(i, b)| Some((i, b?)))
            .filter(|(_, b)| *b <= budget);
        let (owner, b) = valid.next()?;
        if valid.next().is_some() {
            return None;
        }
        let residual = match owner {
            2 => tri
                .iter()
                .map(|p| (p[0].hypot(p[1]) - radius).abs())
                .fold(0., f64::max),
            3 => bo
                .iter()
                .map(|p| (p[0].hypot(p[1]) - r).abs())
                .fold(0., f64::max),
            _ => 0.,
        };
        radial_error = radial_error.max(residual);
        bounds[owner] = bounds[owner].max(b - residual);
        owners.push(owner);
        for j in 0..3 {
            let (u, v) = (t[j], t[(j + 1) % 3]);
            let key = (u.min(v), u.max(v));
            if let Some((other, c, e)) = borders.remove(&key) {
                if (c, e) != (v, u) {
                    return None;
                }
                if owner == other {
                    continue;
                }
                let (o, x, y) = if owner == 2 {
                    (other, u, v)
                } else if other == 2 {
                    (owner, c, e)
                } else {
                    return None;
                };
                let which = if o == 3 {
                    if p[x][0] * p[y][0] <= 0. {
                        return None;
                    }
                    if p[x][0] > 0. {
                        2
                    } else {
                        3
                    }
                } else {
                    o
                };
                curves[which].push((x, y));
            } else {
                borders.insert(key, (owner, u, v));
            }
        }
    }
    if !borders.is_empty() {
        return None;
    }
    for (i, edges) in curves.iter().enumerate() {
        model.cycles[i] = single_cycle(edges)?;
    }
    if !model.valid_witnesses(&mesh.vertices) {
        return None;
    }
    let mut rims = [0_f64; 4];
    for (i, ids) in model.cycles.iter().enumerate() {
        let [shaft, other] = model.rim(i, ids, &mesh.vertices)?;
        rims[2] = rims[2].max(shaft);
        let adj = if i < 2 { i } else { 3 };
        rims[adj] = rims[adj].max(other);
    }
    let face_bound = (0..4).map(|i| bounds[i] + rims[i]).fold(0., f64::max);
    let bound = radial_error + face_bound + NATIVE + o.weld_tolerance_mm;
    if bound > budget {
        return None;
    }
    let planes = (0..2)
        .map(|i| MixedPlane {
            normal: scale(axis, if i == 0 { -1. } else { 1. }),
            loops: vec![MixedLoop::Circle { ring: i }],
        })
        .collect();
    let rings = [0., height]
        .into_iter()
        .map(|h| CircularRing {
            center: along(model.origin, axis, h),
            axis,
            radius_mm: radius,
        })
        .collect();
    let report = RecognitionReport {
        cylinders: 2,
        cones: 0,
        spheres: 0,
        tori: 0,
        toroidal_fillets: 0,
        revolutions: 0,
        max_radial_residual_mm: radial_error,
        max_chord_error_mm: face_bound,
        max_wall_residual_mm: NATIVE,
        max_deviation_bound_mm: bound,
        expected_volume_mm3: model.volume_mm3(),
        diagnostics: vec![Diagnostic {
            code: "recognition.circular_intent_assumed".into(),
            severity: Severity::Warning,
            count: 2,
            message: "L’arbre et son perçage transversal sont interprétés comme deux cylindres."
                .into(),
        }],
    };
    Some(RoundedRecognition {
        report,
        triangle_faces: owners
            .into_iter()
            .map(|i| {
                if i < 2 {
                    RoundedFaceRef::Plane { index: i }
                } else {
                    RoundedFaceRef::ShaftWall { index: i - 2 }
                }
            })
            .collect(),
        model: RoundedModel {
            drilled_shaft: Some(Box::new(model)),
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices: mesh.vertices.clone(),
                    planes,
                    rings,
                    cylinders: vec![],
                    partial_cylinders: vec![],
                    arcs: vec![],
                    elliptic_arcs: vec![],
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
            cross_drill: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_search_declines_before_allocations_when_budget_or_size_excludes_it() {
        let o = Options::default();
        let mut data = crate::analyze_stl(
            include_bytes!("../../../fixtures/r113-cross-bored-shaft.stl"),
            &o,
        )
        .unwrap();
        let mut work = Work::default();
        work.spend(9_999_999).unwrap();
        assert!(candidate(&data, &o, &mut work).is_none());
        assert_eq!(work.used_for_test(), 9_999_999);
        data.mesh.triangles.resize(MAX_TRIANGLES + 1, [0, 1, 2]);
        let mut available = Work::default();
        assert!(candidate(&data, &o, &mut available).is_none());
        assert_eq!(available.used_for_test(), 0);
    }
}
