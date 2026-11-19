//! Extract simple oriented UV domains, retaining ambiguous rims as source facets.
use super::{Source, Torus, TorusChart, TorusTrimVertex, P};
use crate::math::{norm, sub};
use std::collections::{BTreeMap, VecDeque};

fn spans(p: [P; 3]) -> P {
    std::array::from_fn(|k| {
        p.iter().map(|q| q[k]).fold(f64::NEG_INFINITY, f64::max)
            - p.iter().map(|q| q[k]).fold(f64::INFINITY, f64::min)
    })
}
fn remainder(m: Torus, [u, v]: P) -> f64 {
    ((m.major + m.minor) * u * u + m.minor * v * v + 2. * m.minor * u * v) / 2.
}
fn constant(a: P, b: P, m: Torus) -> Option<usize> {
    let u = (a[0] - b[0]).abs() < (8e-5 / (m.major - m.minor)).max(1.5e-5);
    let v = (a[1] - b[1]).abs() < (8e-5 / m.minor).max(1.5e-5);
    match (u, v) {
        (true, false) => Some(0),
        (false, true) => Some(1),
        _ => None,
    }
}

pub(super) fn measure(src: &Source, m: Torus, blocked: &[bool], limit: f64) -> Vec<TorusChart> {
    let points: &[_] = &src.points;
    let valid: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, &p)| !blocked[i] && m.distance(p).abs() < 2e-5)
        .collect();
    let uv: Vec<_> = points.iter().map(|&p| m.uv(p)).collect();
    let mut selected: Vec<_> = src
        .tris
        .iter()
        .map(|t| {
            if !t.iter().all(|&i| valid[i]) {
                return false;
            }
            let span = spans(t.map(|i| uv[i]));
            span.iter().all(|v| *v < std::f64::consts::FRAC_PI_2)
                && remainder(m, span) < 0.8 * limit
        })
        .collect();
    let mut queue = VecDeque::new();
    let iso = |e: usize| {
        let (a, b) = src.edge(e);
        constant(uv[a], uv[b], m)
    };
    for (i, t) in selected.iter().enumerate() {
        if *t && (3 * i..3 * i + 3).any(|e| !selected[src.twin[e] / 3] && iso(e).is_none()) {
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        if !selected[i] {
            continue;
        }
        selected[i] = false;
        for e in 3 * i..3 * i + 3 {
            let neighbor = src.twin[e] / 3;
            if selected[neighbor] && iso(e).is_none() {
                queue.push_back(neighbor);
            }
        }
    }
    let mut visited = vec![false; selected.len()];
    let mut out = Vec::new();
    for root in 0..selected.len() {
        if visited[root] || !selected[root] {
            continue;
        }
        visited[root] = true;
        let mut cells = vec![root];
        let mut rim = Vec::new();
        let mut at = 0;
        while at < cells.len() {
            let i = cells[at];
            at += 1;
            for e in 3 * i..3 * i + 3 {
                let neighbor = src.twin[e] / 3;
                if !selected[neighbor] {
                    rim.push(e);
                } else if !visited[neighbor] {
                    visited[neighbor] = true;
                    cells.push(neighbor);
                }
            }
        }
        if cells.len() < 30 || rim.len() < 4 || rim.len() > 24000 {
            continue;
        }
        if let Some(patch) = qualify(src, m, &uv, cells, rim, limit) {
            out.push(patch);
        }
    }
    out
}

fn qualify(
    src: &Source,
    m: Torus,
    uv: &[P],
    mut cells: Vec<usize>,
    rim: Vec<usize>,
    limit: f64,
) -> Option<TorusChart> {
    let mut entries = BTreeMap::<usize, (usize, usize)>::new();
    for e in rim {
        let (a, b) = src.edge(e);
        if entries.insert(a, (b, constant(uv[a], uv[b], m)?)).is_some() {
            return None;
        }
    }
    let mut loops = Vec::new();
    let mut snapped = BTreeMap::<usize, P>::new();
    while let Some((&start, _)) = entries.first_key_value() {
        let mut next = start;
        let mut path = Vec::new();
        loop {
            let (b, k) = entries.remove(&next)?;
            path.push((next, k));
            next = b;
            if b == start {
                break;
            }
        }
        let len = path.len();
        if len < 4 {
            return None;
        }
        let first = (0..len).find(|&i| path[i].1 != path[(i + len - 1) % len].1)?;
        path.rotate_left(first);
        let mut i = 0;
        while i < len {
            let axis = path[i].1;
            let end = (i + 1..len).find(|&j| path[j].1 != axis).unwrap_or(len);
            let ids = (i..=end).map(|j| path[j % len].0);
            let vals: Vec<_> = ids.clone().map(|v| uv[v][axis]).collect();
            let spread = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - vals.iter().copied().fold(f64::INFINITY, f64::min);
            if spread
                * (if axis == 0 {
                    m.major + m.minor
                } else {
                    m.minor
                })
                > 0.8 * limit
            {
                return None;
            }
            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
            for v in ids {
                snapped.entry(v).or_insert(uv[v])[axis] = mean;
            }
            i = end;
        }
        loops.push(
            path.into_iter()
                .map(|(v, k)| TorusTrimVertex {
                    vertex: v,
                    uv: snapped[&v],
                    constant_u: k == 0,
                })
                .collect::<Vec<_>>(),
        );
    }
    let mut sign = 0;
    let mut drift = 0.0_f64;
    let mut error = 0.0_f64;
    for &i in &cells {
        let t = src.tris[i];
        let q = t.map(|v| snapped.get(&v).copied().unwrap_or(uv[v]));
        let det =
            (q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[1][1] - q[0][1]) * (q[2][0] - q[0][0]);
        let direction = if det > 1e-12 {
            1
        } else if det < -1e-12 {
            -1
        } else {
            return None;
        };
        if sign != 0 && direction != sign {
            return None;
        }
        sign = direction;
        for (v, actual) in t.into_iter().zip(q) {
            drift = drift.max(norm(sub(m.point(actual), src.points[v])));
        }
        error = error.max(remainder(m, spans(q)));
    }
    let bound = error + drift;
    if bound >= limit || !disjoint(&loops) {
        return None;
    }
    let mut outer = None;
    for (i, path) in loops.iter().enumerate() {
        let area = path
            .iter()
            .zip(path.iter().cycle().skip(1))
            .map(|(a, b)| a.uv[0] * b.uv[1] - b.uv[0] * a.uv[1])
            .sum::<f64>();
        if area * sign as f64 > 0. && outer.replace(i).is_some() {
            return None;
        }
    }
    loops.swap(0, outer?);
    cells.sort_unstable();
    Some(TorusChart {
        center_mm: m.center,
        major_radius_mm: m.major,
        minor_radius_mm: m.minor,
        orientation: sign,
        triangles: cells,
        loops,
        bound_mm: bound,
    })
}

fn disjoint(loops: &[Vec<TorusTrimVertex>]) -> bool {
    let mut segments = Vec::new();
    for path in loops {
        for (a, b) in path.iter().zip(path.iter().cycle().skip(1)) {
            segments.push((
                [
                    a.uv[0].min(b.uv[0]),
                    a.uv[0].max(b.uv[0]),
                    a.uv[1].min(b.uv[1]),
                    a.uv[1].max(b.uv[1]),
                ],
                a.vertex,
                b.vertex,
            ));
        }
    }
    segments.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    let mut visits = 0;
    for (i, (s, a, b)) in segments.iter().enumerate() {
        for (t, c, d) in &segments[i + 1..] {
            if t[0] > s[1] + 1e-12 {
                break;
            }
            visits += 1;
            if visits > 8_000_000
                || (a != c
                    && b != c
                    && a != d
                    && b != d
                    && s[2] <= t[3] + 1e-12
                    && t[2] <= s[3] + 1e-12)
            {
                return false;
            }
        }
    }
    true
}
