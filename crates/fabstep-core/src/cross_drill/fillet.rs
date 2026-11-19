//! A full-length quarter round measured from the unclaimed triangles.
//! Its two caps and two tangencies must belong to the same six stock planes.
use super::*;

fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}

pub(super) const FILLET: usize = 9;

pub(super) fn fit(
    f: &CrossDrill,
    mesh: &Mesh,
    points: &[V],
    owners: &[usize],
    tolerance: f64,
) -> Option<StockFillet> {
    if f.large_axis_cosine != 0. {
        return None;
    }
    let mut signs = None;
    let mut minimum = [f64::INFINITY; 2];
    for (&triangle, &owner) in mesh.triangles.iter().zip(owners) {
        if owner != usize::MAX {
            continue;
        }
        for id in triangle {
            let [_, y, z] = points[id];
            let current = [y.signum() as i32, z.signum() as i32];
            if signs.is_some_and(|s| s != current) {
                return None;
            }
            signs = Some(current);
            minimum[0] = minimum[0].min(y.abs());
            minimum[1] = minimum[1].min(z.abs());
        }
    }
    let [sy, sz] = signs?;
    let a = f.half_lengths[1] - minimum[0];
    let b = f.half_lengths[2] - minimum[1];
    if (a - b).abs() > 2. * tolerance {
        return None;
    }
    let cut = StockFillet {
        radius_mm: (a + b) * 0.5,
        y_sign: sy,
        z_sign: sz,
    };
    cut.fits(f, tolerance).then_some(cut)
}

#[derive(Clone, Copy)]
struct Segment {
    start: V,
    end: V,
    center: Option<V>,
}

impl Segment {
    fn project(self, point: V, tol: f64) -> Option<(f64, V)> {
        let (t, q) = if let Some(c) = self.center {
            let u = sub(self.start, c);
            let v = sub(self.end, c);
            let d = sub(point, c);
            let theta = dot(d, v).atan2(dot(d, u)).clamp(0., PI / 2.);
            (
                theta / (PI / 2.),
                add(c, add(scale(u, theta.cos()), scale(v, theta.sin()))),
            )
        } else {
            let v = sub(self.end, self.start);
            let t = (dot(sub(point, self.start), v) / dot(v, v)).clamp(0., 1.);
            (t, add(self.start, scale(v, t)))
        };
        (norm(sub(point, q)) <= tol).then_some((t, q))
    }
}

impl StockFillet {
    /// The whole quarter round is outside both bore envelopes even after the witness margin.
    pub fn fits(&self, f: &CrossDrill, tolerance: f64) -> bool {
        let r = self.radius_mm;
        f.large_axis_cosine == 0.
            && matches!(self.y_sign, -1 | 1)
            && matches!(self.z_sign, -1 | 1)
            && r.is_finite()
            && r > 8. * tolerance
            && f.half_lengths[2] - r > 4. * tolerance
            && f.half_lengths[1] - r
                > f.radii[1].max(f.small_offset[1].abs() + f.radii[0]) + 4. * tolerance
    }

    /// Snap one oriented outer source loop to the bounded stock and require one
    /// traversal of every line or arc, in the outward face orientation.
    pub fn project_stock_outline(
        &self,
        half: [f64; 3],
        face: usize,
        points: &[[f64; 3]],
        tol: f64,
    ) -> Option<Vec<[f64; 3]>> {
        if face >= 6 || !(3..=MAX_CYCLE).contains(&points.len()) {
            return None;
        }
        let [hx, hy, hz] = half;
        let (sy, sz) = (self.y_sign as f64, self.z_sign as f64);
        let r = self.radius_mm;
        let mirror = |x, y, z| [x, y * sy, z * sz];
        let axis = face % 3;
        let sign = side(face);
        let mut center = None;
        let corners = if axis == 0 {
            let x = sign * hx;
            center = Some(mirror(x, hy - r, hz - r));
            vec![
                mirror(x, -hy, -hz),
                mirror(x, hy, -hz),
                mirror(x, hy, hz - r),
                mirror(x, hy - r, hz),
                mirror(x, -hy, hz),
            ]
        } else {
            let j = (axis + 1) % 3;
            let k = (axis + 2) % 3;
            let mut lo = half.map(|x| -x);
            let mut hi = half;
            if axis == 1 && sign == sy {
                if sz > 0. {
                    hi[2] -= r;
                } else {
                    lo[2] += r;
                }
            } else if axis == 2 && sign == sz {
                if sy > 0. {
                    hi[1] -= r;
                } else {
                    lo[1] += r;
                }
            }
            [
                (lo[j], lo[k]),
                (hi[j], lo[k]),
                (hi[j], hi[k]),
                (lo[j], hi[k]),
            ]
            .map(|(a, b)| {
                let mut p = [0.; 3];
                p[axis] = sign * half[axis];
                p[j] = a;
                p[k] = b;
                p
            })
            .to_vec()
        };
        let mut segments: Vec<_> = (0..corners.len())
            .map(|i| Segment {
                start: corners[i],
                end: corners[(i + 1) % corners.len()],
                center: if i == 2 { center } else { None },
            })
            .collect();
        let n = cross(sub(corners[1], corners[0]), sub(corners[2], corners[0]));
        if n[axis] * sign < 0. {
            segments.reverse();
            for segment in &mut segments {
                std::mem::swap(&mut segment.start, &mut segment.end);
            }
        }
        let mut path = Vec::with_capacity(points.len());
        for (&p, &q) in points.iter().zip(points.iter().cycle().skip(1)) {
            let mut choice = None;
            for (i, &segment) in segments.iter().enumerate() {
                if let (Some((a, projected)), Some((b, _))) =
                    (segment.project(p, tol), segment.project(q, tol))
                {
                    // A chamfer joining only the two tangency points does not
                    // witness circular intent. Demand samples across the arc.
                    if b > a && (segment.center.is_none() || b - a <= 0.25 + 1e-10) {
                        if choice.is_some() {
                            return None;
                        }
                        choice = Some((i, projected));
                    }
                }
            }
            path.push(choice?);
        }
        let mut transitions = 0;
        for i in 0..path.len() {
            let before = path[(i + path.len() - 1) % path.len()].0;
            let current = path[i].0;
            if before != current {
                let corner = segments[current].start;
                if current != (before + 1) % segments.len() || norm(sub(points[i], corner)) > tol {
                    return None;
                }
                path[i].1 = corner;
                transitions += 1;
            }
        }
        (transitions == segments.len()).then(|| path.into_iter().map(|(_, q)| q).collect())
    }

    pub(super) fn neighboring_planes(self) -> (usize, usize) {
        (
            if self.y_sign < 0 { 1 } else { 4 },
            if self.z_sign < 0 { 2 } else { 5 },
        )
    }

    pub(super) fn prove_triangle(
        self,
        f: &CrossDrill,
        points: [V; 3],
        boundary: [Option<V>; 3],
        tol: f64,
        err: &mut Bound,
    ) -> Result<(), FabError> {
        let (sy, sz) = (self.y_sign as f64, self.z_sign as f64);
        let [hx, hy, hz] = f.half_lengths;
        let r = self.radius_mm;
        let mut uv = [[0.; 2]; 3];
        for (i, source) in points.into_iter().enumerate() {
            let p = boundary[i].unwrap_or(source);
            let a = p[1] * sy - (hy - r);
            let b = p[2] * sz - (hz - r);
            let angle = b.atan2(a).clamp(0., PI / 2.);
            let x = p[0].clamp(-hx, hx);
            err.record(
                source,
                [
                    x,
                    sy * (hy - r + r * angle.cos()),
                    sz * (hz - r + r * angle.sin()),
                ],
                tol,
            )?;
            uv[i] = [angle, x];
        }
        let [p, q, s] = uv;
        let u = (q[0] - p[0]) * (s[1] - p[1]);
        let v = (s[0] - p[0]) * (q[1] - p[1]);
        if (u - v) * sy * sz <= 128. * f64::EPSILON * (u.abs() + v.abs()) {
            return Err(fail("carte du congé repliée"));
        }
        let low = uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let high = uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        err.wall = err.wall.max(r * (high - low).powi(2) / 8.);
        Ok(())
    }
}
