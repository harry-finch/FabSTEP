//! Source hypothesis for a block with three unequal cylindrical edge blends
//! meeting at its low corner. The fourth non-planar face must still pass the
//! native interior and trim certificate before this hypothesis can be used.
use crate::{Analysis, Options};

#[derive(Clone, Debug)]
pub struct UnequalCorner {
    pub origin: [f64; 3],
    pub size: [f64; 3],
    pub radii: [f64; 3],
    /// Low planes, high planes, three cylinder walls, then the corner patch.
    pub owners: Vec<usize>,
}

pub fn unequal_corner_candidate(analysis: &Analysis, options: &Options) -> Option<UnequalCorner> {
    let mesh = &analysis.mesh;
    if !(10..=8192).contains(&analysis.model.faces.len())
        || !(10..=16_384).contains(&mesh.triangles.len())
        || mesh.vertices.len() > 49_152
    {
        return None;
    }
    let [origin, high] = analysis.report.bounds_mm;
    let size: [f64; 3] = std::array::from_fn(|i| high[i] - origin[i]);
    let fit = options.cylinder_fit_tolerance_mm;
    if !(fit > 0. && fit <= 1e-4) || size.iter().any(|s| !s.is_finite() || *s <= 0.) {
        return None;
    }
    // All six support planes must exist. This also skips axis-oblique stocks
    // before allocating the local coordinates used by this bounded domain.
    for i in 0..3 {
        for bound in [origin[i], high[i]] {
            if !analysis.model.faces.iter().any(|f| {
                f.normal[i].abs() > 1. - 1e-11
                    && f.loops.len() == 1
                    && f.loops[0]
                        .first()
                        .and_then(|&index| analysis.model.vertices.get(index))
                        .is_some_and(|v| (v[i] - bound).abs() <= options.plane_tolerance_mm)
            }) {
                return None;
            }
        }
    }
    let local: Vec<[f64; 3]> = mesh
        .vertices
        .iter()
        .map(|v| std::array::from_fn(|i| v[i] - origin[i]))
        .collect();
    let mut radii = [0.; 3];
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        // Two independent terminal tangencies locate a measured quarter-circle.
        let cap = local
            .iter()
            .filter(|v| (v[i] - size[i]).abs() <= options.plane_tolerance_mm);
        let setback = |a: usize, b: usize| {
            cap.clone()
                .filter(|v| v[a].abs() <= options.plane_tolerance_mm)
                .map(|v| v[b])
                .fold(f64::INFINITY, f64::min)
        };
        let a = setback(j, k);
        let b = setback(k, j);
        let r = (a + b) * 0.5;
        if !r.is_finite()
            || (a - b).abs() > fit
            || r < 16. * fit
            || r + 16. * fit >= size[j].min(size[k])
        {
            return None;
        }
        if radii[..i]
            .iter()
            .any(|previous| f64::abs(r - previous) < 16. * fit)
        {
            return None;
        }
        if cap
            .filter(|v| {
                v[j] > fit
                    && v[k] > fit
                    && v[j] < r
                    && v[k] < r
                    && ((v[j] - r).hypot(v[k] - r) - r).abs() <= fit
            })
            .count()
            < 3
        {
            return None;
        }
        radii[i] = r;
    }
    let mut owners = Vec::with_capacity(mesh.triangles.len());
    let mut counts = [0usize; 10];
    for tri in &mesh.triangles {
        let [a, b, c] = *tri;
        let p = [local.get(a)?, local.get(b)?, local.get(c)?];
        let plane = (0..6).find(|&index| {
            let i = index % 3;
            let level = if index < 3 { 0. } else { size[i] };
            p.iter()
                .all(|v| (v[i] - level).abs() <= options.plane_tolerance_mm)
        });
        let owner = plane.unwrap_or_else(|| {
            (0..3)
                .find(|&i| {
                    let j = (i + 1) % 3;
                    let k = (i + 2) % 3;
                    let r = radii[i];
                    p.iter().all(|v| {
                        v[j] <= r + fit
                            && v[k] <= r + fit
                            && ((v[j] - r).hypot(v[k] - r) - r).abs() <= fit
                    })
                })
                .map_or(9, |i| 6 + i)
        });
        counts[owner] += 1;
        owners.push(owner);
    }
    if counts.contains(&0) || counts[9] < 4 {
        return None;
    }
    Some(UnequalCorner {
        origin,
        size,
        radii,
        owners,
    })
}
