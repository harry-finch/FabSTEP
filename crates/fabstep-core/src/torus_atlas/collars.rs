//! Protect measured neighbors from folds caused by a newly curved rim.
use super::{unit, Source, Torus, TorusChart, TorusTrimVertex, V};
use crate::math::{cross, dot, norm, sub};
use std::collections::{BTreeMap, BTreeSet};

fn add(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] + b[k])
}
fn mul(a: V, s: f64) -> V {
    a.map(|x| x * s)
}
fn key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
#[derive(Clone, Copy)]
pub(super) struct Arc {
    pub(super) pole: V,
    pub(super) weight: f64,
    center: V,
    radius: f64,
    sweep: f64,
    normal: V,
}

fn circle_fan(arc: Arc, a: V, b: V, apex: V, n: V) -> bool {
    if dot(sub(apex, arc.center), arc.normal).abs() > 4e-6
        && arc.sweep.abs() < std::f64::consts::FRAC_PI_2
        && dot(cross(sub(b, a), sub(apex, a)), n) > 1e-14
    {
        return true;
    }
    positive_fan(arc.pole, arc.weight, a, b, apex, n)
}
pub(super) fn positive_fan(pole: V, weight: f64, a: V, b: V, apex: V, n: V) -> bool {
    let h = [sub(a, apex), mul(sub(pole, apex), weight), sub(b, apex)];
    let d = [mul(sub(h[1], h[0]), 2.), mul(sub(h[2], h[1]), 2.)];
    let mut coeff = [0.; 4];
    for i in 0..3 {
        for j in 0..2 {
            coeff[i + j] += [1., 2., 1.][i] / [1., 3., 3., 1.][i + j] * dot(cross(h[i], d[j]), n);
        }
    }
    coeff.iter().all(|&x| x > 1e-14)
}
fn trace(t: f64, a: V, b: V, arc: Arc) -> (V, V) {
    let w = arc.weight;
    let m = mul(sub(arc.pole, a), w);
    let end = sub(b, a);
    let n = add(mul(m, 2. * t * (1. - t)), mul(end, t * t));
    let den = (1. - t).powi(2) + 2. * w * t * (1. - t) + t * t;
    let dn = add(mul(m, 2. * (1. - 2. * t)), mul(end, 2. * t));
    let dw = 2. * (1. - 2. * t) * (w - 1.);
    (
        mul(n, 1. / den),
        mul(sub(mul(dn, den), mul(n, dw)), 1. / den.powi(2)),
    )
}
pub(super) fn arc(m: Torus, a: TorusTrimVertex, b: TorusTrimVertex) -> Arc {
    let [u, v] = a.uv;
    let (center, normal, radius, sweep) = if a.constant_u {
        (
            [
                m.center[0],
                m.center[1] + m.major * u.cos(),
                m.center[2] + m.major * u.sin(),
            ],
            [0., -u.sin(), u.cos()],
            m.minor,
            b.uv[1] - v,
        )
    } else {
        (
            [m.center[0] + m.minor * v.sin(), m.center[1], m.center[2]],
            [1., 0., 0.],
            m.major + m.minor * v.cos(),
            b.uv[0] - u,
        )
    };
    let weight = (sweep / 2.).cos();
    let mid = m.point([(u + b.uv[0]) / 2., (v + b.uv[1]) / 2.]);
    let pole = add(center, mul(sub(mid, center), 1. / weight));
    Arc {
        pole,
        weight,
        center,
        normal,
        radius,
        sweep,
    }
}

pub(super) fn unsafe_vertices(src: &Source, charts: &[TorusChart]) -> BTreeSet<usize> {
    let mut dst = src.points.clone();
    let mut changed = vec![false; dst.len()];
    let mut used = vec![false; src.tris.len()];
    let mut arcs = BTreeMap::new();
    for chart in charts {
        let m = Torus {
            center: chart.center_mm,
            major: chart.major_radius_mm,
            minor: chart.minor_radius_mm,
        };
        for &i in &chart.triangles {
            used[i] = true;
        }
        for path in &chart.loops {
            for (a, b) in path.iter().zip(path.iter().cycle().skip(1)) {
                dst[a.vertex] = m.point(a.uv);
                changed[a.vertex] = true;
                arcs.insert(key(a.vertex, b.vertex), arc(m, *a, *b));
            }
        }
    }
    let mut failed = BTreeSet::new();
    for (i, t) in src.tris.iter().enumerate() {
        if used[i] || !t.iter().any(|&v| changed[v]) {
            continue;
        }
        let s = t.map(|v| src.points[v]);
        let d = t.map(|v| dst[v]);
        let Some(n) = unit(cross(sub(s[1], s[0]), sub(s[2], s[0]))) else {
            failed.extend(t);
            continue;
        };
        let projected = dot(cross(sub(d[1], d[0]), sub(d[2], d[0])), n);
        let rims =
            std::array::from_fn::<_, 3, _>(|j| arcs.get(&key(t[j], t[(j + 1) % 3])).copied());
        let count = rims.iter().flatten().count();
        let good = match count {
            0 => projected > 1e-14,
            1 => {
                let j = rims.iter().position(Option::is_some).unwrap();
                circle_fan(rims[j].unwrap(), d[j], d[(j + 1) % 3], d[(j + 2) % 3], n)
            }
            2 => {
                let j = rims.iter().position(Option::is_none).unwrap();
                let a = d[(j + 2) % 3];
                let b = d[j];
                let c = d[(j + 1) % 3];
                let e1 = rims[(j + 1) % 3].unwrap();
                let e2 = rims[(j + 2) % 3].unwrap();
                if norm(sub(e1.center, e2.center)) < 1e-8 && (e1.radius - e2.radius).abs() < 1e-8 {
                    projected > 1e-14
                } else {
                    [1e-6, 0.25, 0.5, 0.75, 1.].into_iter().all(|t| {
                        let (x, dx) = trace(t, a, b, e2);
                        let (y, dy) = trace(t, a, c, e1);
                        let v = sub(y, x);
                        dot(cross(dx, v), n) > 0. && dot(cross(dy, v), n) > 0.
                    })
                }
            }
            _ => false,
        };
        if !good {
            failed.extend(t.iter().copied().filter(|&v| changed[v]));
        }
    }
    failed
}
