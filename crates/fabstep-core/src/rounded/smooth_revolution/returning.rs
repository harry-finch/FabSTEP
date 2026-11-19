//! A single cubic axial graph over a strictly monotone affine radius.
use super::*;

fn turning_interval(z: [f64; 4], tol: f64) -> Option<[f64; 2]> {
    let [a, b, c, d] = z;
    let qa = -a + 3. * b - 3. * c + d;
    let qb = 2. * (a - 2. * b + c);
    let qc = b - a;
    let disc = qb * qb - 4. * qa * qc;
    if qa <= tol || b - a <= tol || d - c <= tol || disc <= tol * tol {
        return None;
    }
    let hi = (-qb + disc.sqrt()) / (2. * qa);
    let lo = qc / (qa * hi);
    if !(lo > 0. && lo < hi && hi < 1.) {
        return None;
    }
    let peak = radius(&z, lo);
    let valley = radius(&z, hi);
    (peak < d - tol && valley > a + tol && peak - valley > tol).then_some([lo, hi])
}

struct Meridian {
    radial: [f64; 2],
    z: [f64; 4],
}
impl Meridian {
    fn point(&self, p: [f64; 2]) -> V {
        let [theta, t] = p;
        let r = self.radial[0] + (self.radial[1] - self.radial[0]) * t;
        [r * theta.cos(), r * theta.sin(), radius(&self.z, t)]
    }
    fn cell(
        &self,
        uv: [[f64; 2]; 3],
        source: [V; 3],
        available: f64,
        depth: usize,
        work: &mut Work,
    ) -> Result<f64, FabError> {
        work.spend(18)?;
        let residual = (0..3)
            .map(|i| norm(sub(self.point(uv[i]), source[i])))
            .fold(0., f64::max);
        let range = |k: usize| {
            let a = uv.map(|p| p[k]);
            (
                a.into_iter().fold(f64::INFINITY, f64::min),
                a.into_iter().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        let (u, v) = range(0);
        let (a, b) = range(1);
        let w = v - u;
        let dt = b - a;
        let [z0, z1, z2, z3] = self.z;
        let d2 = |t: f64| 6. * ((1. - t) * (z0 - 2. * z1 + z2) + t * (z1 - 2. * z2 + z3)).abs();
        // For the linear barycentric UV map Var(u)<=w²/4 and Var(t)<=dt²/4.
        // The mixed derivative has length |r'|, independently of the return.
        let bound = residual
            + (self.radial[0].max(self.radial[1]) * w * w
                + 2. * (self.radial[1] - self.radial[0]).abs() * dt * w
                + d2(a).max(d2(b)) * dt * dt)
                / 8.;
        if bound <= available || residual > available || depth == 0 {
            return Ok(bound);
        }
        fn mids<const N: usize>(points: [[f64; N]; 3]) -> [[[f64; N]; 3]; 4] {
            let [a, b, c] = points;
            let mid = |x: [f64; N], y: [f64; N]| std::array::from_fn(|k| (x[k] + y[k]) / 2.);
            let (ab, bc, ca) = (mid(a, b), mid(b, c), mid(c, a));
            [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]
        }
        let mut max = 0_f64;
        for (p, q) in mids(uv).into_iter().zip(mids(source)) {
            max = max.max(self.cell(p, q, available, depth - 1, work)?);
            if max > available {
                return Ok(bound);
            }
        }
        Ok(max)
    }
}

pub(in super::super) fn candidate(
    a: &Analysis,
    s: &Segmentation,
    rims: &[Rim],
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    if rims.len() != 2 || a.mesh.triangles.len() > MAX_TRIANGLES {
        return Ok(None);
    }
    let frame = Frame::new(rims[0].center, rims[0].axis);
    let mut ends = [&rims[0], &rims[1]];
    ends.sort_by(|a, b| frame.project(a.center)[2].total_cmp(&frame.project(b.center)[2]));
    let nodes = ends.map(|r| Node {
        vertices: r.vertices.clone(),
        z: frame.project(r.center)[2],
        radius: r.radius,
    });
    let tol = o.cylinder_fit_tolerance_mm;
    let h = nodes[1].z - nodes[0].z;
    let r0 = nodes[0].radius;
    let r1 = nodes[1].radius;
    let dr = r1 - r0;
    if h < 16. * tol || dr.abs() < 16. * tol || r0.min(r1) < 8. * tol {
        return Ok(None);
    }
    for (end, r) in ends.iter().enumerate() {
        let [x, y, _] = frame.project(r.center);
        let face = &s.model.faces[r.region];
        let sign = if end == 0 { -1. } else { 1. };
        if x.hypot(y) > tol
            || face.loops.len() != 1
            || face.loops[0] != r.vertices
            || sign * dot(face.normal, frame.axis) < 1. - 1e-10
        {
            return Ok(None);
        }
    }
    work.spend(a.mesh.triangles.len() * 12 + a.mesh.vertices.len() * 8)?;
    let local: Vec<_> = a
        .mesh
        .vertices
        .iter()
        .map(|&p| {
            let [x, y, z] = frame.project(p);
            [x, y, z - nodes[0].z]
        })
        .collect();
    let mut wall = vec![false; local.len()];
    for (&tri, &r) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        if !ends.iter().any(|e| e.region == r) {
            for v in tri {
                wall[v] = true;
            }
        }
    }
    let mut m11 = 0.;
    let mut m12 = 0.;
    let mut m22 = 0.;
    let mut rhs = [0.; 2];
    let mut uv = vec![[0.; 2]; local.len()];
    for (i, &[x, y, z]) in local.iter().enumerate() {
        if !wall[i] {
            continue;
        }
        let t = (x.hypot(y) - r0) / dr;
        if t < -tol / dr.abs() || t > 1. + tol / dr.abs() {
            return Ok(None);
        }
        uv[i] = [y.atan2(x), t];
        let p = 3. * (1. - t).powi(2) * t;
        let q = 3. * (1. - t) * t * t;
        let value = z - t.powi(3) * h;
        m11 += p * p;
        m12 += p * q;
        m22 += q * q;
        rhs[0] += p * value;
        rhs[1] += q * value;
    }
    let det = m11 * m22 - m12 * m12;
    if !det.is_finite() || det <= 1e-4 * m11 * m22 {
        return Ok(None);
    }
    let z = [
        0.,
        (rhs[0] * m22 - rhs[1] * m12) / det,
        (rhs[1] * m11 - rhs[0] * m12) / det,
        h,
    ];
    let Some([peak, valley]) = turning_interval(z, 8. * tol) else {
        return Ok(None);
    };
    // Observe the downturn on the actual mesh around the entire axis. A
    // polynomial inferred between distant stations alone cannot create it.
    let middle = (peak + valley) / 2.;
    let mut highs = [f64::NEG_INFINITY; 8];
    let mut lows = [f64::INFINITY; 8];
    for (i, &[theta, t]) in uv.iter().enumerate() {
        if wall[i] {
            let sector = ((theta.rem_euclid(TAU) * 8. / TAU) as usize).min(7);
            if t < middle && t > 0. {
                highs[sector] = highs[sector].max(local[i][2]);
            } else if t > middle && t < 1. {
                lows[sector] = lows[sector].min(local[i][2]);
            }
        }
    }
    if (0..8).any(|i| highs[i] - lows[i] <= 8. * tol + 2. * o.weld_tolerance_mm) {
        return Ok(None);
    }
    let origin = frame.lift([0., 0., nodes[0].z]);
    let roundoff = 512.
        * f64::EPSILON
        * origin
            .into_iter()
            .chain(z)
            .chain([r0, r1])
            .map(f64::abs)
            .fold(1., f64::max);
    if roundoff > tol.min(5e-8) {
        return Ok(None);
    }
    let f = SmoothRevolution {
        origin,
        axis: frame.axis,
        height_mm: h,
        radii_mm: [r0, r0 + dr / 3., r1 - dr / 3., r1],
        axial_controls_mm: Some([z[1], z[2]]),
        second_span: None,
        pole_vertex: None,
        rings: [0, 1],
        witness_tolerance_mm: 0.,
    };
    let mut bound = Bound {
        residual: 0.,
        curvature: 0.,
    };
    let rings = ring_points(a, &frame, &nodes, &f, tol, &mut bound)?;
    let meridian = Meridian {
        radial: [r0, r1],
        z,
    };
    for (i, p) in uv.iter_mut().enumerate() {
        if !wall[i] {
            continue;
        }
        if let Some(end) = rings[i].level {
            p[1] = end as f64
        } else if !(p[1] > 0. && p[1] < 1.) {
            return Err(unproven("rayon intérieur hors du domaine"));
        }
        bound.charge_projection(norm(sub(local[i], meridian.point(*p))), tol)?;
    }
    let available =
        o.max_deviation_mm - o.weld_tolerance_mm - roundoff - NATIVE_RESERVE_MM - bound.residual;
    let mut owners = Vec::with_capacity(a.mesh.triangles.len());
    // Each wall triangle maps with positive orientation. Its shared edges
    // have identical linear lifts modulo 2π. Both boundary circles have
    // degree one (ring_points), so every interior regular chart point has
    // exactly one preimage, including where the physical height turns back.
    for (&ids, &r) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        if let Some(end) = ends.iter().position(|e| e.region == r) {
            cap_triangle(
                ids.map(|i| local[i]),
                ids.map(|i| rings[i]),
                end,
                end,
                end as f64 * h,
                o.plane_tolerance_mm,
                &mut bound,
            )?;
            owners.push(RoundedFaceRef::Plane { index: end });
        } else {
            let mut pts = ids.map(|i| uv[i]);
            let u = delta(pts[0][0], pts[1][0]);
            let v = delta(pts[0][0], pts[2][0]);
            if u.abs().max(v.abs()).max((u - v).abs()) >= PI / 2. {
                return Err(unproven("couture angulaire de la sous-coupe ambiguë"));
            }
            let x = u * (pts[2][1] - pts[0][1]);
            let y = v * (pts[1][1] - pts[0][1]);
            if x - y <= 128. * f64::EPSILON * (x.abs() + y.abs()) {
                return Err(unproven("projection angle–rayon de la paroi repliée"));
            }
            pts[1][0] = pts[0][0] + u;
            pts[2][0] = pts[0][0] + v;
            let chord = meridian.cell(pts, ids.map(|i| local[i]), available, 3, work)?;
            bound.curvature = bound.curvature.max(chord);
            owners.push(RoundedFaceRef::SmoothRevolution { index: 0 });
        }
    }
    let proof = (
        "Une méridienne cubique à retour axial relie les deux disques mesurés.",
        "La projection angle–rayon couvre chaque triangle, les deux disques et la méridienne à rayon monotone.",
    );
    finish(a, o, &nodes, &frame, bound, roundoff, f, owners, proof)
}
