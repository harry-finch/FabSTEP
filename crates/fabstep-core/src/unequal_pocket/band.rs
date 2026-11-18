//! Oriented area and piecewise-linear boundary coverage in each cylinder chart.
use super::*;

pub(super) struct Bound {
    pub residual: f64,
    pub chord: f64,
}
pub(super) fn prove(
    mesh: &Mesh,
    part: &Partition,
    d: &UnequalPocket,
    coords: &[V],
    k: usize,
    tol: f64,
) -> Result<Bound, FabError> {
    let own = d.radii[k];
    let other = d.radii[1 - k];
    let wall = SMALL + k;
    let span = d.pocket_spans[k];
    let indices: BTreeSet<_> = mesh
        .triangles
        .iter()
        .zip(&part.owners)
        .filter(|(_, id)| **id == wall)
        .flat_map(|(t, _)| *t)
        .collect();
    let mut chart = vec![[0., 0.]; coords.len()];
    for &v in &indices {
        let p = coords[v];
        chart[v] = [(own - p[1 - k]).atan2(own - p[2]), p[k]];
    }
    let cut = |i: usize| -> &[usize] { &part.cuts[&pair(i, wall)] };
    for (side, theta) in [(FLOOR, 0.), (SIDE[k], FRAC_PI_2)] {
        for &v in cut(side) {
            chart[v][0] = theta;
        }
    }
    let cutoff = if k == 0 {
        FRAC_PI_2
    } else {
        (1. - other / own).acos()
    };
    let mut lower = Vec::new();
    let mut seam_step = 0_f64;
    for (neighbor, begin, end) in [
        (STOP[k], 0., FRAC_PI_2),
        (SMALL + 1 - k, 0., cutoff),
        (8, cutoff, FRAC_PI_2),
    ] {
        if begin == end {
            continue;
        }
        let upper = neighbor == STOP[k];
        let seam = neighbor == SMALL + 1 - k;
        let mut path = cut(neighbor).to_vec();
        checked(path.len() >= if upper || seam { 7 } else { 2 })?;
        if chart[path[0]][0] > chart[*path.last().unwrap()][0] {
            path.reverse();
        }
        chart[path[0]][0] = begin;
        chart[*path.last().unwrap()][0] = end;
        for &id in &path {
            let theta = chart[id][0];
            let a = if upper {
                span
            } else {
                inset_at_angle(own, other, theta)
            };
            chart[id][1] = a;
            if !upper && lower.last() != Some(&id) {
                lower.push(id);
            }
        }
        for ab in path.windows(2) {
            let delta = chart[ab[1]][0] - chart[ab[0]][0];
            checked(delta > 0. && delta <= FRAC_PI_2 / 2.)?;
            if seam {
                seam_step = seam_step.max(delta);
            }
        }
    }
    let mut strip_area = Sum::default();
    for ab in lower.windows(2) {
        let [a, b] = [chart[ab[0]], chart[ab[1]]];
        checked(b[0] > a[0])?;
        strip_area.add((b[0] - a[0]) * (span - (a[1] + b[1]) / 2.));
    }
    let eps = 128. * f64::EPSILON * span.max(1.);
    let mut residual = 0_f64;
    for &id in &indices {
        let [theta, a] = chart[id];
        checked((0. ..=FRAC_PI_2).contains(&theta) && a <= span + eps)?;
        let n = lower
            .partition_point(|&v| chart[v][0] < theta)
            .clamp(1, lower.len() - 1);
        let [p, q] = [chart[lower[n - 1]], chart[lower[n]]];
        let interpolated = p[1] + (q[1] - p[1]) * (theta - p[0]) / (q[0] - p[0]);
        checked(a >= interpolated - eps)?;
        let mut target = [0., 0., own * (1. - theta.cos())];
        target[k] = a;
        target[1 - k] = own * (1. - theta.sin());
        residual = residual.max(norm(sub(coords[id], target)));
    }
    checked(residual <= tol)?;
    let hand = dot(cross(d.axes[0], d.axes[1]), d.axes[2]).signum();
    let sign = if k == 0 { hand } else { -hand };
    let mut covered = Sum::default();
    let mut max_span = 0_f64;
    for (tri, &owner) in mesh.triangles.iter().zip(&part.owners) {
        if owner != wall {
            continue;
        }
        let [a, b, c] = tri.map(|v| chart[v]);
        let signed = sign * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]));
        checked(signed > 0.)?;
        covered.add(signed / 2.);
        let [p, q, r] = tri.map(|v| mesh.vertices[v]);
        let theta = (a[0] + b[0] + c[0]) / 3.;
        let inward = add(
            scale(d.axes[1 - k], theta.sin()),
            scale(d.axes[2], theta.cos()),
        );
        checked(dot(cross(sub(q, p), sub(r, p)), inward) > 0.)?;
        max_span = max_span.max(a[0].max(b[0]).max(c[0]) - a[0].min(b[0]).min(c[0]));
    }
    checked(
        max_span <= FRAC_PI_2 / 2.
            && (covered.get() - strip_area.get()).abs() <= strip_area.get() * 1e-9,
    )?;
    // At fixed theta, a linear stretch from the polygonal lower boundary
    // to the exact inset covers the entire trimmed cylinder. For
    // u=sin(theta/2), Q=other-2 sqrt(own) u sqrt(other-own*u²).
    // On the common branch own*u² <= min(own,other)/2. Differentiating
    // twice gives |Q''| <= sqrt(min(own,other)/other)*(3 own+other/2).
    // Past the junction Q is identically zero. The independent angular
    // interpolation bound is the circle's second derivative times Δθ²/8.
    let curvature = (own.min(other) / other).sqrt() * (3. * own + other / 2.);
    let chord = (own * max_span.powi(2) / 8.).hypot(curvature * seam_step.powi(2) / 8.);
    Ok(Bound { residual, chord })
}
