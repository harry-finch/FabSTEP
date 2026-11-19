//! Three quarter cylinders around the floor of a rectangular pocket.
use super::*;

struct Supports {
    sides: [usize; 3], // flanks first, central wall last
    floor: usize,
    stop: usize,
    radius: f64,
}

fn supports(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    p: &mut Partition,
    tol: f64,
) -> Result<Supports, FabError> {
    refit_supports(mesh, p)?;
    let mut boundary = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (t, &id) in p.owners.iter().enumerate() {
        if id != WALL {
            continue;
        }
        for &other in &adjacency[t] {
            let owner = p.owners[other];
            if owner != WALL {
                let chain = boundary.entry(owner).or_default();
                chain.extend(
                    mesh.triangles[t]
                        .into_iter()
                        .filter(|v| mesh.triangles[other].contains(v)),
                );
            }
        }
    }
    checked(boundary.len() == 5)?;
    let perpendicular = |i: usize, j: usize| {
        dot(p.planes[i].support.normal, p.planes[j].support.normal).abs() < 1e-6
    };
    let floors: Vec<_> = boundary
        .keys()
        .copied()
        .filter(|&i| boundary.keys().all(|&j| i == j || perpendicular(i, j)))
        .collect();
    let [floor]: [usize; 1] = floors.try_into().map_err(|_| unsupported())?;
    let base = p.planes[floor].support;
    let mut rails = Vec::new();
    let mut stop = None;
    let mut heights = Vec::new();
    for (&i, vertices) in &boundary {
        if i == floor {
            continue;
        }
        let lo = vertices
            .iter()
            .map(|&v| base.distance(mesh.vertices[v]))
            .fold(f64::INFINITY, f64::min);
        let hi = vertices
            .iter()
            .map(|&v| base.distance(mesh.vertices[v]))
            .fold(f64::NEG_INFINITY, f64::max);
        if hi - lo <= tol && lo > 8. * tol {
            rails.push(i);
            heights.extend([lo, hi]);
        } else {
            checked(stop.replace(i).is_none() && lo.abs() < tol && hi > 8. * tol)?;
        }
    }
    let stop = stop.ok_or_else(unsupported)?;
    checked(rails.len() == 3)?;
    let r = heights.iter().sum::<f64>() / heights.len() as f64;
    checked(heights.iter().all(|&h| (h - r).abs() <= tol))?;
    let stop_normal = p.planes[stop].support.normal;
    let central = rails
        .iter()
        .position(|&i| dot(p.planes[i].support.normal, stop_normal) < -1. + 1e-10)
        .ok_or_else(unsupported)?;
    let center = rails.remove(central);
    let [left, right] = [rails[0], rails[1]];
    checked(
        perpendicular(center, left)
            && dot(
                p.planes[left].support.normal,
                p.planes[right].support.normal,
            ) < -1. + 1e-10,
    )?;
    let up = base.normal;
    let x0 = p.planes[left].support.normal;
    let x = unit(sub(x0, scale(up, dot(up, x0)))).ok_or_else(unsupported)?;
    let y = cross(up, x);
    let y = scale(y, dot(y, p.planes[center].support.normal).signum());
    for (i, normal) in [
        (floor, up),
        (left, x),
        (right, scale(x, -1.)),
        (center, y),
        (stop, scale(y, -1.)),
    ] {
        p.planes[i].support = center_plane(mesh, &p.planes[i], normal);
    }
    Ok(Supports {
        sides: [left, right, center],
        floor,
        stop,
        radius: r,
    })
}

fn split(mesh: &Mesh, p: &mut Partition, lateral: [Plane; 3], tol: f64) -> Result<(), FabError> {
    // The inward distance to the closest curved side determines the owner.
    // Every vertex of each triangle must lie in that same closed cell.
    for (t, id) in p.owners.iter_mut().enumerate() {
        if *id != WALL {
            continue;
        }
        let d = mesh.triangles[t].map(|v| lateral.map(|s| s.distance(mesh.vertices[v])));
        let mean = |k| d.iter().map(|row| row[k]).sum::<f64>() / 3.;
        let mut ks = [0, 1, 2];
        ks.sort_by(|&i, &j| mean(i).total_cmp(&mean(j)));
        let k = ks[0];
        checked(
            mean(ks[1]) - mean(k) > tol
                && d.iter().all(|row| (0..3).all(|j| row[k] <= row[j] + tol)),
        )?;
        *id = WALL + k;
    }
    (p.cuts, p.neighbor) = interfaces(mesh, &p.owners)?;
    Ok(())
}

pub(super) fn fit(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    p: &mut Partition,
    o: &Options,
    work: &mut Work,
) -> Result<Vec<wall::Geometry>, FabError> {
    work.spend(80 * mesh.triangles.len())?;
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    let Supports {
        sides,
        floor,
        stop,
        radius,
    } = supports(mesh, adjacency, p, tol)?;
    let lateral = sides.map(|i| p.planes[i].support);
    split(mesh, p, lateral, tol)?;
    let bisector = |k: usize| -> Result<Plane, FabError> {
        Ok(Plane {
            origin: intersection(
                [lateral[k], lateral[2], p.planes[floor].support],
                p.planes[floor].support.origin,
            )
            .ok_or_else(unsupported)?,
            normal: unit(sub(lateral[k].normal, lateral[2].normal)).ok_or_else(unsupported)?,
        })
    };
    let ends = [bisector(0)?, bisector(1)?];
    let mut result = Vec::new();
    for (k, &side) in sides.iter().enumerate() {
        let id = WALL + k;
        let (low, high, mut near, mut far) = if k < 2 {
            (stop, WALL + 2, p.planes[stop].support, ends[k])
        } else {
            (WALL, WALL + 1, ends[0], ends[1])
        };
        prove_disk(mesh, adjacency, &p.owners, id)?;
        let neighbors: BTreeSet<_> = p
            .cuts
            .keys()
            .filter_map(|&(a, b)| {
                if a == id {
                    Some(b)
                } else if b == id {
                    Some(a)
                } else {
                    None
                }
            })
            .collect();
        checked(neighbors == BTreeSet::from([floor, side, low, high]))?;
        let mid = |j| {
            let c = &p.cuts[&pair(j, id)];
            mesh.vertices[c[c.len() / 2]]
        };
        let axis = cross(
            p.planes[floor].support.normal,
            p.planes[side].support.normal,
        );
        let axis = scale(axis, dot(sub(mid(high), mid(low)), axis).signum());
        near.normal = scale(near.normal, dot(near.normal, axis).signum());
        far.normal = scale(far.normal, -dot(far.normal, axis).signum());
        result.push(wall::fit_between(
            mesh,
            p,
            o,
            wall::Stops {
                wall: id,
                rails: [side, floor],
                circle_plane: low,
                ellipse_plane: high,
                near,
                far,
                shared_radius: Some(radius),
            },
        )?);
    }
    adjacent::same_ellipse(
        &result[0].ellipse,
        result[2].near_ellipse.as_ref().ok_or_else(unsupported)?,
    )?;
    adjacent::same_ellipse(&result[1].ellipse, &result[2].ellipse)?;
    Ok(result)
}
