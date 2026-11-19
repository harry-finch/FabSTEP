//! A genuinely unstructured annulus can be certified by its radial projection.
//! Curve fitting uses measured vertices distributed over the height. The two
//! actual disk boundaries are the only cycles used as trim constraints.
use super::*;

pub(super) fn samples(
    a: &Analysis,
    s: &Segmentation,
    nodes: &[Node],
    caps: [usize; 2],
    frame: &Frame,
    work: &mut Work,
) -> Result<Option<Vec<Node>>, FabError> {
    let h = nodes.last().unwrap().z - nodes[0].z;
    let mut on_ring = vec![false; a.mesh.vertices.len()];
    for n in nodes {
        for &v in &n.vertices {
            on_ring[v] = true;
        }
    }
    let mut seen = vec![false; on_ring.len()];
    let mut bins = 0u16;
    let mut off_range = [1_f64, 0_f64];
    let mut stations = [None; 17];
    work.spend(6 * a.mesh.triangles.len() + 5 * a.mesh.vertices.len())?;
    for (ids, region) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        if caps.contains(region) {
            continue;
        }
        for &v in ids {
            if std::mem::replace(&mut seen[v], true) {
                continue;
            }
            let [x, y, z] = frame.project(a.mesh.vertices[v]);
            let t = (z - nodes[0].z) / h;
            if !(0.0..1.0).contains(&t) {
                continue;
            }
            if t > 1. / 32. && t < 31. / 32. {
                let third = (3. * t) as usize;
                let sector = usize::from(x >= 0.) + 2 * usize::from(y >= 0.);
                bins |= 1 << (third * 4 + sector);
                if !on_ring[v] {
                    off_range[0] = off_range[0].min(t);
                    off_range[1] = off_range[1].max(t);
                }
            }
            let k = (t * 16.).round() as usize;
            if k > 0 && k < 16 {
                let error = (t - k as f64 / 16.).abs();
                if stations[k].is_none_or(|(old, _, _, _)| error < old) {
                    stations[k] = Some((error, v, z, x.hypot(y)));
                }
            }
        }
    }
    // A single damaged complete ring is not an unstructured wall. Demand
    // off-ring witnesses separated by 1/8 of the height. The entire wall must also
    // provide independent samples in each azimuth quadrant of every third.
    if bins != 0xfff || off_range[1] - off_range[0] < 0.125 {
        return Ok(None);
    }
    let mut measured = vec![nodes[0].clone()];
    for (_, v, z, radius) in stations.into_iter().flatten() {
        measured.push(Node {
            vertices: vec![v],
            z,
            radius,
        });
    }
    measured.push(nodes.last().unwrap().clone());
    if measured.len() < 11 || measured.windows(2).any(|w| (w[1].z - w[0].z) > h * 0.16) {
        return Ok(None);
    }
    Ok(Some(measured))
}

fn eval(f: &SmoothRevolution, t: f64) -> f64 {
    let (lo, hi, b) = spans(f).find(|(_, hi, _)| t <= *hi).unwrap();
    radius(&b, (t - lo) / (hi - lo))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn corrected(
    a: &Analysis,
    s: &Segmentation,
    caps: [usize; 2],
    points: &[Point],
    nodes: &[Node],
    frame: &Frame,
    f: &SmoothRevolution,
    tol: f64,
    bound: &mut Bound,
) -> Result<Vec<V>, FabError> {
    let mut mapped = vec![[f64::NAN; 3]; points.len()];
    for (ids, region) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        if caps.contains(region) {
            continue;
        }
        for &v in ids {
            if !mapped[v][2].is_nan() {
                continue;
            }
            if let Some(i) = points[v].level {
                mapped[v] = [
                    points[v].xy[0],
                    points[v].xy[1],
                    (nodes[i].z - nodes[0].z) / f.height_mm,
                ];
            } else {
                let [x, y, z] = frame.project(a.mesh.vertices[v]);
                let t = (z - nodes[0].z) / f.height_mm;
                if !(0. < t && t < 1.) {
                    return Err(unproven("sommet de paroi hors des deux disques"));
                }
                let rho = x.hypot(y);
                if rho <= tol {
                    return Err(unproven("paroi irrégulière sur l’axe"));
                }
                let r = eval(f, t);
                bound.charge_projection((rho - r).abs(), tol)?;
                mapped[v] = [r * x / rho, r * y / rho, t];
            }
        }
    }
    Ok(mapped)
}

fn cell_bound(p: [V; 3], f: &SmoothRevolution) -> Result<(f64, f64), FabError> {
    let theta = p.map(|q| q[1].atan2(q[0]));
    let u = delta(theta[0], theta[1]);
    let v = delta(theta[0], theta[2]);
    let width = 0_f64.max(u).max(v) - 0_f64.min(u).min(v);
    if width >= PI / 2. {
        return Err(unproven("triangle traversant l’axe ou la couture"));
    }
    let low = p.iter().map(|q| q[2]).fold(1., f64::min);
    let high = p.iter().map(|q| q[2]).fold(0., f64::max);
    let mut curvature = 0_f64;
    for (a, b, c) in spans(f) {
        if low >= b || high <= a {
            continue;
        }
        for t in [low.max(a), high.min(b)] {
            let s = (t - a) / (b - a);
            let second = ((1. - s) * (c[0] - 2. * c[1] + c[2]) + s * (c[1] - 2. * c[2] + c[3]))
                * 6.
                / (b - a).powi(2);
            curvature = curvature.max(second.abs());
        }
    }
    let mut rmax = 0_f64;
    let mut residual = 0_f64;
    for [x, y, t] in p {
        let r = x.hypot(y);
        rmax = rmax.max(r);
        residual = residual.max((r - eval(f, t)).abs());
    }
    // For any barycentric point in the flat triangle, its angular direction
    // is arg(sum lambda_i * (x_i,y_i)). Projecting onto the angular bisector
    // bounds the loss in length by R(1-cos(width/2)). Scalar radial
    // interpolation contributes max|r''| Var(t)/2 at the same axial height.
    let bound =
        residual + curvature * (high - low).powi(2) / 8. + 2. * rmax * (width / 4.).sin().powi(2);
    Ok((residual, bound))
}

fn certificate(
    p: [V; 3],
    f: &SmoothRevolution,
    available: f64,
    depth: u8,
    work: &mut Work,
) -> Result<f64, FabError> {
    work.spend(30)?;
    let (residual, bound) = cell_bound(p, f)?;
    if bound <= available || residual > available || depth == 0 {
        return Ok(bound);
    }
    let [a, b, c] = p;
    let mid = |x: V, y: V| std::array::from_fn(|k| (x[k] + y[k]) / 2.);
    let (ab, bc, ca) = (mid(a, b), mid(b, c), mid(c, a));
    let mut worst = 0_f64;
    for cell in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
        worst = worst.max(certificate(cell, f, available, depth - 1, work)?);
        if worst > available {
            break;
        }
    }
    Ok(worst)
}

pub(super) fn triangle(
    pts: [V; 3],
    f: &SmoothRevolution,
    available: f64,
    work: &mut Work,
) -> Result<f64, FabError> {
    let [a, b, c] = pts;
    let (d, e) = (sub(b, a), sub(c, a));
    // The numerator of det(d theta,d t) is affine on the source triangle.
    // Its three positive vertex values give positive orientation everywhere.
    // The two end cycles have degree one and adjacent triangles share their
    // mapped edge. Degree on the open annulus is thus one, proving coverage
    // in both directions, independently of any intermediate rings.
    for [x, y, _] in pts {
        let p = (x * d[1] - y * d[0]) * e[2];
        let q = (x * e[1] - y * e[0]) * d[2];
        let scale = x.abs() * (d[1] * e[2]).abs()
            + y.abs() * (d[0] * e[2]).abs()
            + x.abs() * (e[1] * d[2]).abs()
            + y.abs() * (e[0] * d[2]).abs();
        if p - q <= 128. * f64::EPSILON * scale {
            return Err(unproven("projection radiale du triangle repliée"));
        }
    }
    certificate(pts, f, available, 3, work)
}
