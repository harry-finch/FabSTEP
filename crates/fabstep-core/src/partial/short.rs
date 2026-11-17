//! Conservative short arcs supported by two already-proven historical walls.
use super::*;

const MAX_POINTS: usize = 7;
const MAX_CENTER_SPAN_RATIO: f64 = 0.1;

pub(super) struct SupportedCandidate {
    pub candidate: Candidate,
    pub supports: [usize; 2],
}

pub(super) fn candidates(
    model: &MixedModel,
    historical: &[Candidate],
    patches: &[Proven],
    o: &Options,
    work: &mut Work,
) -> Result<Vec<SupportedCandidate>, FabError> {
    work.spend(patches.len())?;
    let mut proven = BTreeMap::new();
    for (index, patch) in patches.iter().enumerate() {
        for candidate in patch.pair {
            proven.insert(candidate, index);
        }
    }
    let mut result = Vec::new();
    for (plane_index, plane) in model.planes.iter().enumerate() {
        for (loop_index, lp) in plane.loops.iter().enumerate() {
            let MixedLoop::Polygon { vertices } = lp else {
                continue;
            };
            work.spend(vertices.len())?;
            let mut supported = BTreeMap::new();
            for &index in proven.keys() {
                let candidate = &historical[index];
                work.spend(1)?;
                if candidate.plane != plane_index || candidate.loop_index != loop_index {
                    continue;
                }
                work.spend(candidate.chain.len())?;
                for pair in candidate.chain.windows(2) {
                    if supported.insert(edge(pair[0], pair[1]), index).is_some() {
                        return Err(unproven());
                    }
                }
            }
            if supported.is_empty() {
                continue;
            }
            for start in 0..vertices.len() {
                work.spend(1)?;
                let previous = edge(
                    vertices[(start + vertices.len() - 1) % vertices.len()],
                    vertices[start],
                );
                let Some(&left) = supported.get(&previous) else {
                    continue;
                };
                let mut chain = vec![vertices[start]];
                let mut end = start;
                while !supported
                    .contains_key(&edge(vertices[end], vertices[(end + 1) % vertices.len()]))
                {
                    work.spend(1)?;
                    end = (end + 1) % vertices.len();
                    chain.push(vertices[end]);
                    // This is an eligibility bound, never a truncated candidate.
                    if chain.len() > MAX_POINTS {
                        break;
                    }
                }
                if !(4..=MAX_POINTS).contains(&chain.len()) {
                    continue;
                }
                let next = edge(vertices[end], vertices[(end + 1) % vertices.len()]);
                let Some(&right) = supported.get(&next) else {
                    continue;
                };
                if left == right {
                    continue;
                }
                work.spend(chain.len())?;
                if let Some(candidate) = fit_supported(
                    model,
                    plane_index,
                    loop_index,
                    chain,
                    [&historical[left], &historical[right]],
                    o,
                ) {
                    let mut supports = [proven[&left], proven[&right]];
                    supports.sort_unstable();
                    if supports[0] != supports[1] {
                        result.push(SupportedCandidate {
                            candidate,
                            supports,
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}

fn fit_supported(
    model: &MixedModel,
    plane: usize,
    loop_index: usize,
    chain: Vec<usize>,
    supports: [&Candidate; 2],
    o: &Options,
) -> Option<Candidate> {
    let normal = scale(
        model.planes[plane].normal,
        1. / norm(model.planes[plane].normal),
    );
    let frame = Frame::new(model.vertices[chain[0]], normal);
    let points: Vec<P2> = chain
        .iter()
        .map(|&v| frame.project(model.vertices[v]).0)
        .collect();
    let f = fit(&points, o.cylinder_fit_tolerance_mm)?;
    if f.sweep.abs() > PI / 4. {
        return None;
    }
    let first = points[0];
    let last = *points.last()?;
    let chord = (last[0] - first[0]).hypot(last[1] - first[1]);
    let half = chord * 0.5;
    let tolerance = o.cylinder_fit_tolerance_mm;
    if half <= 4. * tolerance {
        return None;
    }
    let mid = [(first[0] + last[0]) * 0.5, (first[1] + last[1]) * 0.5];
    let x = [(last[0] - first[0]) / chord, (last[1] - first[1]) / chord];
    let mut y = [-x[1], x[0]];
    if (points[1][0] - mid[0]) * y[0] + (points[1][1] - mid[1]) * y[1] < 0. {
        y = [-y[0], -y[1]];
    }
    let mut lower = f64::NEG_INFINITY;
    let mut upper = f64::INFINITY;
    let mut witness_min = f64::INFINITY;
    let mut witness_max = f64::NEG_INFINITY;
    let mut witnesses = 0;
    for p in &points[1..points.len() - 1] {
        let u = (p[0] - mid[0]) * x[0] + (p[1] - mid[1]) * x[1];
        let v = (p[0] - mid[0]) * y[0] + (p[1] - mid[1]) * y[1];
        if u.abs() >= half || v <= tolerance {
            return None;
        }
        let fraction = (u + half) / chord;
        if (0.2..=0.8).contains(&fraction) {
            witnesses += 1;
            witness_min = witness_min.min(fraction);
            witness_max = witness_max.max(fraction);
        }
        // For center (0,t), R=sqrt(h²+t²), radial residual <=e is
        // equivalent to (D-2vt)² <=4e²(h²+t²), D=u²+v²-h²-e².
        // R>=h>e makes both squaring steps reversible. v>e gives a
        // finite interval; intersect ALL interior witnesses, not a chosen fit.
        let d = u * u + v * v - half * half - tolerance * tolerance;
        let q = v * v - tolerance * tolerance;
        let discriminant = tolerance * (d * d + 4. * half * half * q).sqrt();
        let local_lower = (v * d - discriminant) / (2. * q);
        let local_upper = (v * d + discriminant) / (2. * q);
        if !local_lower.is_finite() || !local_upper.is_finite() {
            return None;
        }
        lower = lower.max(local_lower);
        upper = upper.min(local_upper);
    }
    if witnesses < 2
        || witness_max - witness_min < 0.2
        || !lower.is_finite()
        || !upper.is_finite()
        || lower > upper
        || upper >= 0.
        || upper - lower > MAX_CENTER_SPAN_RATIO * f.radius
    {
        return None;
    }
    let offset = (f.center[0] - mid[0]) * y[0] + (f.center[1] - mid[1]) * y[1];
    let rounding = 64. * f64::EPSILON * (1. + lower.abs().max(upper.abs()).max(f.radius));
    if !offset.is_finite() || offset < lower - rounding || offset > upper + rounding {
        return None;
    }
    let center = frame.lift(f.center);
    for (end, neighbor) in supports.iter().enumerate() {
        let vertex = if end == 0 { chain[0] } else { *chain.last()? };
        let endpoint = model.vertices[vertex];
        let neighbor_radial = sub(endpoint, neighbor.center);
        let nx = dot(neighbor_radial, frame.x) * x[0] + dot(neighbor_radial, frame.y) * x[1];
        let ny = dot(neighbor_radial, frame.x) * y[0] + dot(neighbor_radial, frame.y) * y[1];
        if nx == 0. {
            return None;
        }
        // The anchored circle that would be tangent to this existing support
        // must be compatible with every source witness. We retain the fitted
        // circle: this rejects incompatible joins, it does not impose tangency.
        let tangent_offset = (if end == 0 { half } else { -half }) * ny / nx;
        if !tangent_offset.is_finite()
            || tangent_offset < lower - rounding
            || tangent_offset > upper + rounding
        {
            return None;
        }
        let r0 = sub(model.vertices[neighbor.chain[0]], neighbor.center);
        let r1 = sub(model.vertices[neighbor.chain[1]], neighbor.center);
        let sign = dot(cross(r0, r1), normal).signum();
        let tangent = scale(cross(normal, sub(endpoint, center)), f.sweep.signum());
        let neighbor_tangent = scale(cross(normal, neighbor_radial), sign);
        let alignment = dot(tangent, neighbor_tangent);
        if !alignment.is_finite() || alignment <= 0. {
            return None;
        }
    }
    Some(Candidate {
        plane,
        loop_index,
        chain,
        center,
        normal,
        radius: f.radius,
    })
}
