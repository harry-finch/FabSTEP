//! Rings propose axes and meridian circles. Fits only nominate a support;
//! every accepted input cell is independently certified in the UV chart.
use super::{Source, Torus, P, V};
use std::collections::{BTreeMap, BTreeSet};

fn median(v: &mut [f64]) -> f64 {
    v.sort_unstable_by(f64::total_cmp);
    (v[(v.len() - 1) / 2] + v[v.len() / 2]) / 2.
}

fn circle(points: &[P]) -> Option<(P, f64, f64)> {
    if points.len() < 3 {
        return None;
    }
    let n = points.len() as f64;
    let mean = points
        .iter()
        .fold([0.; 2], |p, q| [p[0] + q[0] / n, p[1] + q[1] / n]);
    let (mut xx, mut xy, mut yy, mut xr, mut yr) = (0., 0., 0., 0., 0.);
    for q in points {
        let x = q[0] - mean[0];
        let y = q[1] - mean[1];
        let r = x * x + y * y;
        xx += x * x;
        xy += x * y;
        yy += y * y;
        xr += x * r;
        yr += y * r;
    }
    let det = xx * yy - xy * xy;
    if det <= f64::EPSILON * (xx + yy).powi(2) {
        return None;
    }
    let center = [
        mean[0] + (yy * xr - xy * yr) / (2. * det),
        mean[1] + (xx * yr - xy * xr) / (2. * det),
    ];
    let (mut lo, mut hi, mut radius) = (f64::INFINITY, 0.0_f64, 0.);
    for q in points {
        let r = (q[0] - center[0]).hypot(q[1] - center[1]);
        lo = lo.min(r);
        hi = hi.max(r);
        radius += r / n;
    }
    Some((center, radius, hi - lo))
}

struct Section {
    center: P,
    ids: Vec<usize>,
}
fn section(source: &Source, ids: &[usize]) -> Option<Section> {
    if ids.len() < 8 {
        return None;
    }
    let x = source.points[ids[0]][0];
    if ids.iter().any(|&v| (source.points[v][0] - x).abs() > 3e-5) {
        return None;
    }
    let yz: Vec<_> = ids
        .iter()
        .map(|&i| [source.points[i][1], source.points[i][2]])
        .collect();
    let (center, radius, error) = circle(&yz)?;
    let extent = std::array::from_fn::<_, 2, _>(|i| {
        let lo = yz.iter().map(|v| v[i]).fold(f64::INFINITY, f64::min);
        let hi = yz.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
        hi - lo
    });
    (radius > 0.05
        && error < f64::max(3e-5, 3e-6 * radius)
        && extent[0].hypot(extent[1]) >= radius * 0.12)
        .then(|| Section {
            center,
            ids: ids.to_vec(),
        })
}

fn sections(source: &Source) -> Vec<(P, Vec<usize>)> {
    let nv = source.points.len();
    let mut neighbors = vec![[usize::MAX; 2]; nv];
    let mut degree = vec![0u32; nv];
    for (e, &twin) in source.twin.iter().enumerate() {
        let (a, b) = source.edge(e);
        if a > b
            || (source.points[a][0] - source.points[b][0]).abs() >= 5e-6
            || source.normal_x[e / 3].min(source.normal_x[twin / 3]) >= 0.999
        {
            continue;
        }
        for (v, w) in [(a, b), (b, a)] {
            if degree[v] < 2 {
                neighbors[v][degree[v] as usize] = w;
            }
            degree[v] += 1;
        }
    }
    let mut visited = vec![false; nv];
    let mut fits = Vec::new();
    let mut total = 0;
    // Chains ending at a branch start at that endpoint, then closed cycles.
    for closed in [false, true] {
        for start in 0..nv {
            if visited[start] || !(1..=2).contains(&degree[start]) {
                continue;
            }
            let adjacent = neighbors[start]
                .iter()
                .filter(|&&v| v != usize::MAX && degree[v] <= 2)
                .count();
            if !closed && adjacent == 2 {
                continue;
            }
            let mut chain = Vec::new();
            let mut current = start;
            loop {
                visited[current] = true;
                chain.push(current);
                let next = neighbors[current]
                    .iter()
                    .find(|&&v| v != usize::MAX && degree[v] <= 2 && !visited[v]);
                let Some(&v) = next else { break };
                current = v;
            }
            if chain.len() < 8 {
                continue;
            }
            if let Some(rec) = section(source, &chain) {
                total += rec.ids.len();
                fits.push(rec);
            } else {
                for window in [32, 12] {
                    for sub in chain.windows(window).step_by(window / 2) {
                        if let Some(rec) = section(source, sub) {
                            total += rec.ids.len();
                            fits.push(rec);
                        }
                    }
                }
            }
            if fits.len() > 25000 || total > 8 * nv {
                return Vec::new();
            }
        }
    }
    let mut bins = BTreeMap::<[i64; 2], Vec<usize>>::new();
    for (i, f) in fits.iter().enumerate() {
        bins.entry(f.center.map(|v| (v / 0.005).floor() as i64))
            .or_default()
            .push(i);
    }
    let mut groups = Vec::new();
    while let Some((cell, mut items)) = bins.pop_first() {
        let mut todo = vec![cell];
        while let Some([x, y]) = todo.pop() {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let other = [x.saturating_add(dx), y.saturating_add(dy)];
                    if let Some(mut neighbors) = bins.remove(&other) {
                        items.append(&mut neighbors);
                        todo.push(other);
                    }
                }
            }
        }
        if items.len() >= 6 {
            groups.push(items);
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    groups
        .into_iter()
        .take(32)
        .map(|items| {
            let center = std::array::from_fn(|k| {
                median(&mut items.iter().map(|&i| fits[i].center[k]).collect::<Vec<_>>())
            });
            let mut verts: Vec<_> = items
                .iter()
                .flat_map(|&i| fits[i].ids.iter().copied())
                .collect();
            verts.sort_unstable();
            verts.dedup();
            (center, verts)
        })
        .collect()
}

struct Rng(u64);
impl Rng {
    fn index(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % (n as u64)) as usize
    }
    fn sample(&mut self, ids: &[usize], n: usize) -> Vec<usize> {
        if ids.len() <= n {
            return ids.to_vec();
        }
        let mut out = BTreeSet::new();
        for j in ids.len() - n..ids.len() {
            let i = self.index(j + 1);
            if !out.insert(i) {
                out.insert(j);
            }
        }
        out.into_iter().map(|i| ids[i]).collect()
    }
}

fn solve(mut matrix: [[f64; 6]; 5]) -> Option<[f64; 5]> {
    for j in 0..5 {
        let pivot = (j..5).max_by(|&i, &k| matrix[i][j].abs().total_cmp(&matrix[k][j].abs()))?;
        matrix.swap(j, pivot);
        let d = matrix[j][j];
        if d.abs() < 1e-14 {
            return None;
        }
        for k in j..6 {
            matrix[j][k] /= d;
        }
        for i in 0..5 {
            if i == j {
                continue;
            }
            let factor = matrix[i][j];
            for k in j..6 {
                matrix[i][k] -= matrix[j][k] * factor;
            }
        }
    }
    Some(matrix.map(|row| row[5]))
}
fn update(x: Torus, d: [f64; 5], t: f64) -> Torus {
    Torus {
        center: std::array::from_fn(|k| x.center[k] + t * d[k]),
        major: x.major + t * d[3],
        minor: x.minor + t * d[4],
    }
}
fn refine(p: &[V], available: &[bool], mut h: Torus) -> Option<Torus> {
    for band in [0.02, 0.002, 0.0002, 0.00002] {
        let ids: Vec<_> = p
            .iter()
            .enumerate()
            .filter(|(i, q)| available[*i] && h.distance(**q).abs() < band)
            .map(|(i, _)| i)
            .collect();
        if ids.len() < 80 {
            return None;
        }
        let step = (ids.len() / 1500).max(1);
        let sample: Vec<_> = ids.iter().step_by(step).map(|&i| p[i]).collect();
        let scale = band / 10.;
        let score = |model: Torus| {
            sample
                .iter()
                .map(|&q| model.distance(q).hypot(scale))
                .sum::<f64>()
        };
        for _ in 0..25 {
            let mut m = [[0.; 6]; 5];
            for &q in &sample {
                let dx = q[0] - h.center[0];
                let dy = q[1] - h.center[1];
                let dz = q[2] - h.center[2];
                let rho = dy.hypot(dz);
                let dr = rho - h.major;
                let dist = dx.hypot(dr);
                if rho < 1e-10 || dist < 1e-10 {
                    return None;
                }
                let err = dist - h.minor;
                let g = [
                    -dx / dist,
                    -dr / dist * dy / rho,
                    -dr / dist * dz / rho,
                    -dr / dist,
                    -1.,
                ];
                let w = scale / scale.hypot(err);
                for i in 0..5 {
                    for j in 0..5 {
                        m[i][j] += w * g[i] * g[j];
                    }
                    m[i][5] -= w * g[i] * err;
                }
            }
            let Some(delta) = solve(m) else { break };
            if delta.iter().all(|v| v.abs() < 1e-11) {
                break;
            }
            let current = score(h);
            let next = [1., 0.5, 0.25, 0.125, 0.0625]
                .into_iter()
                .map(|t| update(h, delta, t))
                .find(|q| q.major > q.minor && q.minor > 0.05 && score(*q) < current);
            let Some(q) = next else { break };
            h = q;
        }
    }
    Some(h)
}

pub(super) fn discover(src: &Source) -> Vec<Torus> {
    let mut candidates = Vec::<(usize, Torus)>::new();
    let mut rng = Rng(386);
    let mut trials = 0;
    for ([cy, cz], rings) in sections(src) {
        let q: Vec<_> = src
            .points
            .iter()
            .map(|p| [p[0], (p[1] - cy).hypot(p[2] - cz)])
            .collect();
        let mut available = vec![true; q.len()];
        let mut rejected = vec![false; q.len()];
        for _ in 0..20 {
            if trials >= 160 {
                break;
            }
            let pool: Vec<_> = rings
                .iter()
                .copied()
                .filter(|&i| available[i] && !rejected[i])
                .collect();
            if pool.len() < 80 {
                break;
            }
            let ids = rng.sample(&pool, 1800);
            let sample: Vec<_> = ids.iter().map(|&i| q[i]).collect();
            let mut best = None;
            for _ in 0..512 {
                let mut triple = [0; 3];
                for i in 0..3 {
                    let mut k = rng.index(sample.len());
                    while triple[..i].contains(&k) {
                        k = (k + 1) % sample.len();
                    }
                    triple[i] = k;
                }
                let points = triple.map(|i| sample[i]);
                let Some(([x, major], minor, _)) = circle(&points) else {
                    continue;
                };
                let spread = points
                    .iter()
                    .map(|p| p[0])
                    .fold(f64::NEG_INFINITY, f64::max)
                    - points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                if major <= minor || minor <= 0.05 || spread < 0.1_f64.min(minor / 4.) {
                    continue;
                }
                let score = sample
                    .iter()
                    .filter(|p| ((p[0] - x).hypot(p[1] - major) - minor).abs() < 0.0004)
                    .count();
                if best.as_ref().is_none_or(|&(n, _)| score > n) {
                    best = Some((
                        score,
                        Torus {
                            center: [x, cy, cz],
                            major,
                            minor,
                        },
                    ));
                }
            }
            let Some((score, seed)) = best else { break };
            if score < 8 {
                break;
            }
            trials += 1;
            let Some(model) = refine(&src.points, &available, seed) else {
                for &i in &pool {
                    if seed.distance(src.points[i]).abs() < 0.001 {
                        rejected[i] = true;
                    }
                }
                continue;
            };
            let mask: Vec<_> = src
                .points
                .iter()
                .enumerate()
                .map(|(i, &p)| available[i] && model.distance(p).abs() < 2e-5)
                .collect();
            let used = src
                .tris
                .iter()
                .filter(|t| t.iter().all(|&v| mask[v]))
                .count();
            for (i, &yes) in mask.iter().enumerate() {
                if yes {
                    available[i] = false;
                }
            }
            if used >= 100
                && !candidates.iter().any(|(_, h)| {
                    h.center
                        .iter()
                        .zip(model.center)
                        .all(|(a, b)| (a - b).abs() < 0.0004)
                        && (h.major - model.major).abs() < 0.0004
                        && (h.minor - model.minor).abs() < 0.0004
                })
            {
                candidates.push((used, model));
            }
        }
    }
    candidates.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    candidates.into_iter().take(64).map(|(_, h)| h).collect()
}
