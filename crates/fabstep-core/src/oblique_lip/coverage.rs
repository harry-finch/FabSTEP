use super::*;

pub(crate) struct Certificate {
    pub bound: f64,
    pub rim: [f64; 5],
    pub drift: f64,
    pub chord: f64,
}

fn mid<const N: usize>(a: [f64; N], b: [f64; N]) -> [f64; N] {
    std::array::from_fn(|i| (a[i] + b[i]) * 0.5)
}
fn split<const N: usize>([a, b, c]: [[f64; N]; 3]) -> [[[f64; N]; 3]; 4] {
    let (ab, bc, ca) = (mid(a, b), mid(b, c), mid(c, a));
    [[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
}
fn width<const N: usize>(q: [[f64; N]; 3], k: usize) -> f64 {
    let [a, b, c] = q.map(|p| p[k]);
    a.max(b).max(c) - a.min(b).min(c)
}
fn det([[a, b], [c, d], [e, f]]: [UV; 3]) -> f64 {
    (c - a) * (f - b) - (d - b) * (e - a)
}

/// A graph of revolution trimmed by two caps and the same oblique plane.
pub(crate) struct Chart<'a> {
    pub frame: &'a ObliqueLip,
    pub profile: &'a dyn Fn(f64) -> (f64, f64),
    pub ordinate: &'a dyn Fn(V) -> f64,
    pub hessian: [f64; 3],
    pub limits: (usize,usize,f64),
    pub cylindrical: bool,
    pub depth: usize,
}
impl Chart<'_> {
    fn span(&self,v:f64)->f64 {
        let (r,z)=(self.profile)(v);
        (-self.frame.cut(z)/r).acos()
    }
    fn point(&self,[s,v]: UV)->V {
        let (r,z)=(self.profile)(v);
        let u=s*self.span(v);
        [-r*u.cos(),-r*u.sin(),z]
    }
}
struct TriangleBound<'a, 'm> {
    chart: &'a Chart<'m>,
    limit: f64,
    work: &'a mut usize,
}
impl TriangleBound<'_, '_> {
    fn triangle(&mut self, q: [UV; 3], p: [V; 3], depth: usize) -> Option<f64> {
        // Bound the weighted Taylor remainder on the same parameter triangle
        // as the affine source. Its coordinate variances are <= span²/4.
        *self.work += 1;
        if *self.work > 300_000 {
            return None;
        }
        let drift = (0..3)
            .map(|j| norm(sub(self.chart.point(q[j]), p[j])))
            .fold(0., f64::max);
        let (ds, dv) = (width(q, 0), width(q, 1));
        let bound = drift
            + (self.chart.hessian[0] * ds * ds + 2. * self.chart.hessian[1] * ds * dv + self.chart.hessian[2] * dv * dv) / 8.
            + 1e-10;
        if bound <= self.limit {
            return Some(bound);
        }
        if depth == self.chart.depth {
            return None;
        }
        let mut high = 0_f64;
        for (u, x) in split(q).into_iter().zip(split(p)) {
            high = high.max(self.triangle(u, x, depth + 1)?);
        }
        Some(high)
    }
}
type Border = (usize, usize, usize, usize);
#[allow(clippy::too_many_arguments)]
pub(super) fn chart(
    m: &ObliqueLip,
    p: &[V],
    tris: &[[usize; 3]],
    owners: &[usize],
    masks: &[u8],
    borders: &[Border],
    f: usize,
    fit: f64,
    limit: f64,
    work: &mut usize,
) -> Option<Certificate> {
    let chart = Chart{
        frame:m,
        profile:&|v| m.profile(f,v),
        ordinate:&|[x,y,z]| if f==CYLINDER {z} else {(z-m.shoulder()).atan2(x.hypot(y)-m.major_radius_mm)},
        hessian:m.hessian(f)?,
        limits:if f==CYLINDER {(BOTTOM,TORUS,m.shoulder())} else {(CYLINDER,TOP,FRAC_PI_2)},
        cylindrical:f==CYLINDER,
        depth:4,
    };
    prove_chart(&chart,p,tris,owners,masks,borders,f,fit,limit,work)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prove_chart(chart:&Chart<'_>,p:&[V],tris:&[[usize;3]],owners:&[usize],masks:&[u8],
    borders:&[Border],f:usize,fit:f64,limit:f64,work:&mut usize)->Option<Certificate>{
    let m=chart.frame;
    let (lo,hi,vmax)=chart.limits;
    let h=chart.hessian;
    let mut uv = vec![[0.; 2]; p.len()];
    let mut drift = vec![0.; p.len()];
    let mut cert = Certificate {
        bound: 0.,
        rim: [0.; 5],
        drift: 0.,
        chord: 0.,
    };
    for (i, (&[x, y, z], &mask)) in p.iter().zip(masks).enumerate() {
        let has = |id: usize| mask & (1 << id) != 0;
        if !has(f) {
            continue;
        }
        let v = if has(lo) {
            0.
        } else if has(hi) {
            vmax
        } else {
            (chart.ordinate)([x,y,z])
        };
        let angle = (-y).atan2(-x);
        let s = if has(CUT) {
            angle.signum()
        } else {
            angle / chart.span(v)
        };
        if !(-1.0..=1.).contains(&s) || !(0.0..=vmax).contains(&v) {
            return None;
        }
        uv[i] = [s, v];
        drift[i] = norm(sub(chart.point(uv[i]), p[i]));
        if drift[i] > fit {
            return None;
        }
        cert.drift = cert.drift.max(drift[i]);
    }
    let mut bound = TriangleBound {
        chart,
        limit,
        work,
    };
    for (&t, &owner) in tris.iter().zip(owners) {
        if f != owner {
            continue;
        }
        let q = t.map(|i| uv[i]);
        if det(q) <= 1e-12 {
            return None;
        }
        let upper = if !chart.cylindrical {
            bound.triangle(q, t.map(|i| p[i]), 0)?
        } else {
            // Develop the wall in angle/height. Generatrices contribute zero
            // curvature. Its two non-linear trim branches are bounded below.
            let developed = q.map(|[s, v]| [s * chart.span(v), v]);
            if det(developed) <= 1e-12 {
                return None;
            }
            let chord = m.outer_radius_mm * width(developed, 0).powi(2) / 8.;
            cert.chord = cert.chord.max(chord);
            chord + t.map(|i| drift[i]).into_iter().fold(0., f64::max) + 1e-10
        };
        cert.bound = cert.bound.max(upper);
    }
    let mut sides: [Vec<(f64, f64, usize, usize)>; 4] = Default::default();
    let mut trim = 0_f64;
    for &(i, j, a, b) in borders {
        let (other, a, b) = match (owners[i] == f, owners[j] == f) {
            (true, false) => (owners[j], a, b),
            (false, true) => (owners[i], b, a),
            _ => continue,
        };
        let ([s, v], [t, w]) = (uv[a], uv[b]);
        let (side, start, end, remainder) = if other == CUT {
            if s != t || s.abs() != 1. {
                return None;
            }
            if chart.cylindrical {
                let q = m.cut(v).abs().max(m.cut(w).abs()) / m.outer_radius_mm;
                let second = q * (m.slope / m.outer_radius_mm).powi(2) / (1. - q * q).powf(1.5);
                trim = trim.max(m.outer_radius_mm * second * (w - v).powi(2) / 8.);
            }
            (
                if s > 0. { 1 } else { 3 },
                s * v,
                s * w,
                h[2] * (w - v).powi(2) / 8.,
            )
        } else {
            if (other != lo && other != hi) || v != w || v != if other == lo { 0. } else { vmax } {
                return None;
            }
            let (r, _) = (chart.profile)(v);
            let remainder = r * (chart.span(v) * (t - s)).powi(2) / 8.;
            if other == lo {
                (0, s, t, remainder)
            } else {
                (2, -s, -t, remainder)
            }
        };
        if start >= end {
            return None;
        }
        cert.rim[other] = cert.rim[other].max(drift[a].max(drift[b]) + remainder + 1e-10);
        cert.chord = cert.chord.max(remainder);
        sides[side].push((start, end, a, b));
    }
    let mut endpoints = [(0, 0); 4];
    for (k, (chain, (start, end))) in sides
        .iter_mut()
        .zip([(-1., 1.), (0., vmax), (-1., 1.), (-vmax, 0.)])
        .enumerate()
    {
        chain.sort_by(|a, b| a.0.total_cmp(&b.0));
        let a = chain.first()?;
        let b = chain.last()?;
        if a.0 != start
            || b.1 != end
            || chain
                .windows(2)
                .any(|w| w[0].1 != w[1].0 || w[0].3 != w[1].2)
        {
            return None;
        }
        endpoints[k] = (a.2, b.3);
    }
    if (0..4).any(|i| endpoints[i].1 != endpoints[(i + 1) % 4].0) {
        return None;
    }
    cert.bound += trim;
    (cert.bound <= limit).then_some(cert)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plane(
    m: &ObliqueLip,
    mesh: &crate::Mesh,
    p: &[V],
    owners: &[usize],
    borders: &[Border],
    f: usize,
    tolerance: f64,
    fit: f64,
    mut rim: f64,
    checks: &mut usize,
) -> Option<f64> {
    let ids: BTreeSet<_> = owners
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v == f).then_some(i))
        .collect();
    let ring = crate::planar::boundaries(mesh, &ids).ok()?;
    if ring.len() != 1 || ring[0].len() > 512 {
        return None;
    }
    let world = match f {
        BOTTOM => scale(m.axis, -1.),
        TOP => m.axis,
        _ => unit(sub(m.radial, scale(m.axis, m.slope)))?,
    };
    crate::planar::validate_loops(mesh, ring, m.origin, world, checks).ok()?;
    let mut drift = 0_f64;
    for t in ids {
        let [a, b, c] = mesh.triangles[t].map(|i| mesh.vertices[i]);
        if dot(cross(sub(b, a), sub(c, a)), world) <= 1e-12 {
            return None;
        }
        for v in mesh.triangles[t] {
            let [x, _, z] = p[v];
            let err = match f {
                BOTTOM => z,
                TOP => z - m.height_mm,
                _ => (x - m.cut(z)) / 1f64.hypot(m.slope),
            };
            drift = drift.max(err.abs());
        }
    }
    if drift > tolerance {
        return None;
    }
    let mut straight: [Vec<(usize, usize)>; 2] = Default::default();
    for &(i, j, a, b) in borders {
        let (other, a, b) = match (owners[i] == f, owners[j] == f) {
            (true, false) => (owners[j], a, b),
            (false, true) => (owners[i], b, a),
            _ => continue,
        };
        if other <= CUT {
            let cap = if f == CUT { other } else { f };
            if cap > TOP || f != CUT && other != CUT {
                return None;
            }
            straight[cap].push((a, b));
        }
    }
    for (cap, edges) in straight.iter_mut().enumerate() {
        if cap != f && f != CUT {
            if !edges.is_empty() {
                return None;
            }
            continue;
        }
        let z = if cap == BOTTOM { 0. } else { m.height_mm };
        let r = if cap == BOTTOM {
            m.outer_radius_mm
        } else {
            m.major_radius_mm
        };
        let x = m.cut(z);
        let y = (r * r - x * x).sqrt();
        let direction = p[edges.first()?.0][1] < p[edges.first()?.1][1];
        if !direction {
            for pair in edges.iter_mut() {
                *pair = (pair.1, pair.0);
            }
        }
        edges.sort_by(|a, b| p[a.0][1].total_cmp(&p[b.0][1]));
        if edges.windows(2).any(|w| w[0].1 != w[1].0)
            || norm(sub(p[edges.first()?.0], [x, -y, z])) > fit
            || norm(sub(p[edges.last()?.1], [x, y, z])) > fit
        {
            return None;
        }
        for &(a, b) in edges.iter() {
            if p[a][1] >= p[b][1] {
                return None;
            }
            for v in [a, b] {
                rim = rim.max(norm(sub(p[v], [x, p[v][1].clamp(-y, y), z])));
            }
        }
    }
    // The projected disk has positive triangles and a simple boundary. A
    // homotopy of Jordan boundaries with displacement <= rim preserves winding
    // outside both rim-neighborhoods and bounds BOTH interior distances.
    Some(drift + rim)
}
