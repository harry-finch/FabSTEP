//! A round shaft with a transverse cylindrical bore. Only one mouth is
//! rounded. This bounded domain uses the Z shaft and the X bore directions.
use crate::{drilled_shaft::circle, Analysis, Options};
use std::collections::{BTreeMap, BTreeSet};
mod supports;

#[derive(Clone, Debug)]
pub struct RollingBore {
    pub origin: [f64; 3],
    pub height: f64,
    pub shaft: f64,
    pub bore: f64,
    pub offset: f64,
    pub level: f64,
    pub radius: f64,
    pub radius_spread: f64,
    /// Continuous displacement on the two cap disks and the two trimmed
    /// cylinders, including the separate native boundary allowance.
    pub support_bound_mm: f64,
    /// Low cap, high cap, outer cylinder, inner cylinder, rolling surface.
    pub owners: Vec<usize>,
}

// Stabilize a verified three-point seed using every point of the intact ring.
// Source vertex order on a material rim need not be angular. Its actual edge
// sequence and winding are checked separately by the support certificate.
fn ring_center(points: &[[f64; 3]], axis: usize, fit: f64) -> Option<([f64; 3], f64)> {
    let mut direction = [0.; 3];
    direction[axis] = 1.;
    let (mut center, radius) = circle(points, direction, fit)?;
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let mut projected: Vec<_> = points.iter().map(|p| [p[u], p[v]]).collect();
    let angle = |p: &[f64; 2]| (p[1] - center[v]).atan2(p[0] - center[u]);
    projected.sort_by(|a, b| angle(a).total_cmp(&angle(b)));
    if let Some(refined) = crate::recognition::fit_circle(&projected, fit) {
        [center[u], center[v]] = refined.center;
        Some((center, refined.radius))
    } else {
        Some((center, radius))
    }
}

pub fn candidate(a: &Analysis, o: &Options) -> Option<RollingBore> {
    let mesh = &a.mesh;
    let fit = o.cylinder_fit_tolerance_mm.min(1e-5);
    let [lo, hi] = a.report.bounds_mm;
    let height = hi[2] - lo[2];
    if !(80..=8192).contains(&mesh.triangles.len())
        || height > 64.
        || !(height > 0. && fit > 0.)
        || lo.iter().chain(&hi).any(|v| v.abs() > 1024.)
    {
        return None;
    }
    if !height.is_finite()
        || !o.plane_tolerance_mm.is_finite()
        || o.plane_tolerance_mm <= 0.
        || !o.max_deviation_mm.is_finite()
        || !o.weld_tolerance_mm.is_finite()
        || o.weld_tolerance_mm <= 0.
        || o.max_deviation_mm <= o.weld_tolerance_mm
        || !o.cylinder_fit_tolerance_mm.is_finite()
    {
        return None;
    }
    // The caps have to be actual source polygons, not just extrema.
    let caps: Option<Vec<_>> = [lo[2], hi[2]]
        .iter()
        .map(|&z| {
            a.model.faces.iter().find(|f| {
                f.normal[2].abs() > 1. - 1e-12
                    && f.loops.len() == 1
                    && f.loops[0].len() >= 12
                    && f.loops[0].iter().all(|&id| {
                        mesh.vertices
                            .get(id)
                            .is_some_and(|p| (p[2] - z).abs() <= o.plane_tolerance_mm)
                    })
            })
        })
        .collect();
    let caps = caps?;
    crate::recognition::validate_analysis(a).ok()?;
    let ring: Vec<_> = caps[0].loops[0].iter().map(|&i| mesh.vertices[i]).collect();
    let (origin, shaft) = ring_center(&ring, 2, fit)?;
    let local: Vec<[f64; 3]> = mesh
        .vertices
        .iter()
        .map(|v| std::array::from_fn(|i| v[i] - origin[i]))
        .collect();
    let on_shaft = |p: [f64; 3]| (p[0].hypot(p[1]) - shaft).abs() <= fit;
    let mouth: Vec<_> = local
        .iter()
        .copied()
        .filter(|&p| p[0] > 0. && p[2] > fit && p[2] < height - fit && on_shaft(p))
        .collect();
    if mouth.len() < 12 {
        return None;
    }
    let (center, bore) = ring_center(&mouth, 0, fit)?;
    let [_, offset, level] = center;
    if !(shaft <= 32.
        && bore > 0.1
        && shaft > 1.5 * (bore + offset.abs())
        && level > bore
        && height - level > bore)
    {
        return None;
    }
    let mut owners = Vec::with_capacity(mesh.triangles.len());
    let mut edges = BTreeMap::new();
    for (id, &tri) in mesh.triangles.iter().enumerate() {
        let p = tri.map(|i| local[i]);
        let owner = if p.iter().all(|v| v[2].abs() <= o.plane_tolerance_mm) {
            0
        } else if p
            .iter()
            .all(|v| (v[2] - height).abs() <= o.plane_tolerance_mm)
        {
            1
        } else if p.iter().all(|&v| on_shaft(v)) {
            2
        } else if p
            .iter()
            .all(|v| ((v[1] - offset).hypot(v[2] - level) - bore).abs() <= fit)
        {
            3
        } else {
            4
        };
        if owner == 4 && p.iter().any(|v| v[0] >= 0.) {
            return None;
        }
        owners.push(owner);
        for i in 0..3 {
            let (u, v) = (tri[i], tri[(i + 1) % 3]);
            let incident = edges.entry((u.min(v), u.max(v))).or_insert([None; 2]);
            if incident[usize::from(u < v)].replace(id).is_some() {
                return None;
            }
        }
    }
    // Each material interface must be a single, non-branching closed loop.
    let mut joins = BTreeMap::<(usize, usize), BTreeMap<usize, usize>>::new();
    for ((u, v), pair) in edges {
        let (left, right) = (owners[pair[0]?], owners[pair[1]?]);
        if left == right {
            continue;
        }
        let next = joins.entry((left.min(right), left.max(right))).or_default();
        let (from, to) = if left < right { (v, u) } else { (u, v) };
        if next.insert(from, to).is_some() {
            return None;
        }
    }
    if joins.keys().copied().collect::<Vec<_>>() != [(0, 2), (1, 2), (2, 3), (2, 4), (3, 4)] {
        return None;
    }
    let mut loops = BTreeMap::new();
    let mut rim_vertices = BTreeSet::new();
    for (pair, next) in &joins {
        let &first = next.keys().next()?;
        if next.len() < 12 {
            return None;
        }
        let mut id = first;
        let mut seen = BTreeSet::new();
        let mut vertices = Vec::with_capacity(next.len());
        while seen.insert(id) {
            if !rim_vertices.insert(id) {
                return None;
            }
            vertices.push(id);
            id = *next.get(&id)?;
        }
        if id != first || seen.len() != next.len() {
            return None;
        }
        loops.insert(*pair, vertices);
    }
    let mut estimates = Vec::new();
    for &id in joins[&(2, 4)].keys() {
        let [x, y, z] = local[id];
        if x >= 0. {
            return None;
        }
        let dy = y - offset;
        let a = (y / shaft).powi(2) - 1.;
        let b = -2. * (dy * y / shaft + bore);
        let c = dy * dy + (z - level).powi(2) - bore * bore;
        let disc = b * b - 4. * a * c;
        if !(disc >= 0. && a < -0.05) {
            return None;
        }
        let denom = -b + disc.sqrt().copysign(-b);
        let r = 2. * c / denom;
        if !(r > fit && r < bore / 2.) {
            return None;
        }
        estimates.push(r);
    }
    estimates.sort_by(f64::total_cmp);
    let radius = estimates[estimates.len() / 2];
    let radius_spread = (radius - estimates[0]).max(estimates.last()? - radius);
    if radius_spread > (0.01 * radius).min(o.max_deviation_mm / 4.) {
        return None;
    }
    let mut model = RollingBore {
        origin,
        height,
        shaft,
        bore,
        offset,
        level,
        radius,
        radius_spread,
        support_bound_mm: 0.,
        owners,
    };
    model.support_bound_mm = supports::prove(&model, mesh, &local, &loops, o, fit)?;
    Some(model)
}
