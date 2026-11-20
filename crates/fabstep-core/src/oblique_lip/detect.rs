use super::*;

struct Disk {
    triangles: BTreeSet<usize>,
    rim: Vec<usize>,
    normal: V,
    point: V,
}

pub(crate) fn supports(a: &Analysis, fit: f64, plane: f64) -> Option<(ObliqueLip, Vec<usize>)> {
    let m = &a.mesh;
    let adj = crate::pipeline::validate_topology(m).ok()?;
    let area: Vec<_> = m
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|v| m.vertices[v]);
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    let normals: Vec<_> = area.iter().copied().map(unit).collect::<Option<_>>()?;
    let mut seen = vec![false; m.triangles.len()];
    let mut disks = Vec::new();
    for seed in 0..seen.len() {
        if std::mem::replace(&mut seen[seed], true) {
            continue;
        }
        let mut pending = vec![seed];
        let mut ids = BTreeSet::new();
        while let Some(i) = pending.pop() {
            ids.insert(i);
            for &j in &adj[i] {
                if !seen[j] && dot(normals[i], normals[j]) > 1. - 1e-8 {
                    seen[j] = true;
                    pending.push(j);
                }
            }
        }
        if ids.len() < 6 {
            continue;
        }
        let n = unit(ids.iter().fold([0.; 3], |v, &i| plus(v, area[i])))?;
        let p = m.vertices[m.triangles[seed][0]];
        if ids.iter().any(|&i| {
            m.triangles[i]
                .iter()
                .any(|&v| dot(sub(m.vertices[v], p), n).abs() > plane)
        }) {
            continue;
        }
        let mut loops = crate::planar::boundaries(m, &ids).ok()?;
        if loops.len() != 1 || !(8..=512).contains(&loops[0].len()) {
            return None;
        }
        disks.push(Disk {
            triangles: ids,
            point: p,
            normal: n,
            rim: loops.pop()?,
        });
        if disks.len() > 3 {
            return None;
        }
    }
    if disks.len() != 3 {
        return None;
    }
    let mut pairs = [(0, 1, 2), (0, 2, 1), (1, 2, 0)]
        .into_iter()
        .filter(|&(i, j, _)| dot(disks[i].normal, disks[j].normal) < -1. + 1e-10);
    let (i, j, k) = pairs.next()?;
    if pairs.next().is_some() {
        return None;
    }
    let cut = &disks[k];
    let cap = |i: usize| {
        let d = &disks[i];
        let points: Vec<_> = d
            .rim
            .iter()
            .map(|&v| m.vertices[v])
            .filter(|&p| dot(sub(p, cut.point), cut.normal).abs() > fit)
            .collect();
        if points.len() < 6 {
            return None;
        }
        crate::drilled_shaft::circle(&points, d.normal, fit).map(|(p, r)| (d, p, r))
    };
    let mut bottom = cap(i)?;
    let mut top = cap(j)?;
    if top.2 > bottom.2 {
        std::mem::swap(&mut top, &mut bottom);
    }
    let (bot, origin, outer) = bottom;
    let (upper, center, major) = top;
    let axis = upper.normal;
    let d = sub(center, origin);
    let height = dot(d, axis);
    let minor = outer - major;
    if norm(sub(d, scale(axis, height))) > fit || minor <= 10. * fit {
        return None;
    }
    let nz = dot(cut.normal, axis);
    let radial = unit(sub(cut.normal, scale(axis, nz)))?;
    let nx = dot(radial, cut.normal);
    if nz.abs() <= 1e-3 || nx <= 0.25 {
        return None;
    }
    let model = ObliqueLip {
        origin,
        axis,
        radial,
        outer_radius_mm: outer,
        major_radius_mm: major,
        height_mm: height,
        cut_offset_mm: dot(sub(cut.point, origin), cut.normal) / nx,
        slope: -nz / nx,
    };
    let mut owners = vec![usize::MAX; m.triangles.len()];
    for (id, disk) in [bot, upper, cut].into_iter().enumerate() {
        for &j in &disk.triangles {
            owners[j] = id;
        }
    }
    Some((model, owners))
}
