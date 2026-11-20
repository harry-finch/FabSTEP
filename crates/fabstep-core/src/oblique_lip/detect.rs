use super::*;

struct Disk {
    triangles: BTreeSet<usize>,
    rim: Vec<usize>,
    normal: V,
    point: V,
}

fn disks(a: &Analysis, plane: f64, limit: usize) -> Option<Vec<Disk>> {
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
            if limit > 3 {
                continue;
            }
            return None;
        }
        disks.push(Disk {
            triangles: ids,
            point: p,
            normal: n,
            rim: loops.pop()?,
        });
        if disks.len() > limit {
            return None;
        }
    }
    Some(disks)
}
pub(crate) fn supports(a: &Analysis, fit: f64, plane: f64) -> Option<(ObliqueLip, Vec<usize>)> {
    let disks = disks(a, plane, 3)?;
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
    fit_disks(&a.mesh, &disks, i, j, k, fit)
}
// A subdivided generator strip of a conical mesh can itself be polygonally
// planar. Only a cutting chord intersecting BOTH measured radii transversely
// may be their third support plane.
pub(crate) fn band_supports(
    a: &Analysis,
    fit: f64,
    plane: f64,
) -> Option<(ObliqueLip, Vec<usize>)> {
    let disks = disks(a, plane, 64)?;
    let mut result = None;
    let mut attempts = 0;
    for i in 0..disks.len() {
        for j in i + 1..disks.len() {
            if dot(disks[i].normal, disks[j].normal) >= -1. + 1e-10 {
                continue;
            }
            for k in 0..disks.len() {
                if k == i || k == j {
                    continue;
                }
                // Bound circle fitting even on a mesh with many parallel strips.
                attempts += 1;
                if attempts > 256 {
                    return None;
                }
                let Some((m, owners)) = fit_disks(&a.mesh, &disks, i, j, k, fit) else {
                    continue;
                };
                if m.cut(0.).abs() >= 0.97 * m.outer_radius_mm
                    || m.cut(m.height_mm).abs() >= 0.97 * m.major_radius_mm
                {
                    continue;
                }
                if result.is_some() {
                    return None;
                }
                result = Some((m, owners));
            }
        }
    }
    result
}
fn fit_disks(
    m: &crate::Mesh,
    disks: &[Disk],
    i: usize,
    j: usize,
    k: usize,
    fit: f64,
) -> Option<(ObliqueLip, Vec<usize>)> {
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
