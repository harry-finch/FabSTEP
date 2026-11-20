//! Bound the two optional cylindrical intervals next to the external cap rims.
use super::*;

pub(super) struct Lands {
    pub inner: [Node; 2],
    pub owners: Vec<Option<usize>>,
    pub indices: [Option<usize>; 2],
    pub lengths: [f64; 2],
    pub residual: f64,
    pub chord: f64,
}

pub(super) fn prove(
    a: &Analysis,
    s: &Segmentation,
    caps: [usize; 2],
    ends: &[Node; 2],
    local: &[V],
    tol: f64,
    work: &mut Work,
) -> Result<Option<Lands>, FabError> {
    let mut proof = Lands {
        inner: ends.clone(),
        owners: vec![None; a.mesh.triangles.len()],
        lengths: [0.; 2],
        indices: [None; 2],
        residual: 0.,
        chord: 0.,
    };
    let mut count = 0;
    for end in 0..2 {
        let radius = ends[end].radius;
        let mut rim = BTreeSet::new();
        let mut area = Sum::default();
        for (i, (&ids, region)) in a.mesh.triangles.iter().zip(&s.triangle_regions).enumerate() {
            if caps.contains(region) {
                continue;
            }
            let points = ids.map(|j| local[j]);
            let radial = points.map(|[x, y, _]| (x.hypot(y) - radius).abs());
            if radial.iter().any(|&r| r > tol) {
                continue;
            }
            work.spend(9)?;
            if proof.owners[i].replace(count).is_some() {
                return Ok(None);
            }
            proof.residual = radial.into_iter().fold(proof.residual, f64::max);
            let [[x, y, z], p, q] = points;
            let u = delta(y.atan2(x), p[1].atan2(p[0]));
            let v = delta(y.atan2(x), q[1].atan2(q[0]));
            let span = u.abs().max(v.abs()).max((u - v).abs());
            let det = u * (q[2] - z) - v * (p[2] - z);
            if span > PI / 4. + 1e-12 || det <= 0. {
                return Ok(None);
            }
            proof.chord = proof.chord.max(radius * span * span / 8.);
            area.add(det / 2.);
            for j in 0..3 {
                let pair = edge(ids[j], ids[(j + 1) % 3]);
                if !rim.insert(pair) {
                    rim.remove(&pair);
                }
            }
        }
        if rim.is_empty() {
            continue;
        }
        for e in boundary(&ends[end].vertices) {
            if !rim.remove(&e) {
                return Ok(None);
            }
        }
        let ids: BTreeSet<_> = rim.iter().flat_map(|&(a, b)| [a, b]).collect();
        if ids.len() < 12 || ids.len() > 2048 {
            return Ok(None);
        }
        let mut cycle: Vec<_> = ids.into_iter().collect();
        cycle.sort_by(|&i, &j| {
            local[i][1]
                .atan2(local[i][0])
                .total_cmp(&local[j][1].atan2(local[j][0]))
        });
        if boundary(&cycle) != rim {
            return Ok(None);
        }
        let mut center = Sum::default();
        for &i in &cycle {
            center.add(local[i][2]);
        }
        let z = center.get() / cycle.len() as f64;
        let cap_z = ends[end].z - ends[0].z;
        let len = (z - cap_z) * if end == 0 { 1. } else { -1. };
        if len <= 16. * tol || (area.get() - TAU * len).abs() > 2. * TAU * tol {
            return Ok(None);
        }
        for &i in &cycle {
            let dz = (local[i][2] - z).abs();
            if dz > tol {
                return Ok(None);
            }
            proof.residual = proof
                .residual
                .max((local[i][0].hypot(local[i][1]) - radius).hypot(dz));
        }
        proof.inner[end] = Node {
            vertices: cycle,
            z: z + ends[0].z,
            radius,
        };
        proof.indices[end] = Some(count);
        proof.lengths[end] = len;
        count += 1;
    }
    Ok(Some(proof))
}
