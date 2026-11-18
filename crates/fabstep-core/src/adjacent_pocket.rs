//! Split two adjacent concave quarter cylinders along the witnessed bisector.
//! All distances and signs refer to the measured inward planar supports.
use super::*;

struct Supports {
    sides: [usize; 2],
    floor: usize,
    stops: [usize; 2],
}

fn supports(mesh: &Mesh, p: &Partition, tol: f64) -> Result<Supports, FabError> {
    // The two side rails are straight. The shared floor has two rails at
    // right angles, and each independent stop contributes a circular chain.
    let (sides, rest): (Vec<_>, Vec<_>) = p
        .cuts
        .iter()
        .filter(|((_, j), _)| *j == WALL)
        .map(|(&(i, _), chain)| (i, chain))
        .partition(|(_, c)| straight(c, &mesh.vertices, &mesh.vertices, tol).is_some());
    let sides: [usize; 2] = sides
        .into_iter()
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| unsupported())?;
    let normals = sides.map(|i| p.planes[i].support.normal);
    checked(dot(normals[0], normals[1]).abs() < 1e-6)?;
    let mut floor = None;
    let mut stops = [None; 2];
    for (i, _) in rest {
        let n = p.planes[i].support.normal;
        if normals.iter().all(|&v| dot(n, v).abs() < 1e-6) {
            checked(floor.replace(i).is_none())?;
        } else {
            // A fillet on side k runs parallel to the other side normal,
            // and stops on the plane opposite that other side.
            let k = (0..2)
                .find(|&k| dot(n, normals[1 - k]) < -1. + 1e-10)
                .ok_or_else(unsupported)?;
            checked(stops[k].replace(i).is_none())?;
        }
    }
    Ok(Supports {
        sides,
        floor: floor.ok_or_else(unsupported)?,
        stops: [
            stops[0].ok_or_else(unsupported)?,
            stops[1].ok_or_else(unsupported)?,
        ],
    })
}

fn same_ellipse(a: &EllipticArc, b: &EllipticArc) -> Result<(), FabError> {
    checked(a.vertices == [b.vertices[1], b.vertices[0]])?;
    checked(dot(a.axis, b.axis) < -1. + 1e-12)?;
    // For a quarter ellipse the offsets at either endpoint are exactly
    // the two coefficients of C*cos(t) + S*sin(t), t in [0, pi/2].
    // Reversing t interchanges these coefficients. The sum below bounds
    // the distance of both charts along the entire shared edge.
    let coefficients = |e: &EllipticArc| {
        [e.start_rad, e.start_rad + FRAC_PI_2].map(|t| {
            add(
                scale(e.major_direction, e.major_mm * t.cos()),
                scale(cross(e.axis, e.major_direction), e.minor_mm * t.sin()),
            )
        })
    };
    let [ca, sa] = coefficients(a);
    let [cb, sb] = coefficients(b);
    checked(norm(sub(a.center, b.center)) + norm(sub(ca, sb)) + norm(sub(sa, cb)) < 1e-8)
}

pub(super) fn fit(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    p: &mut Partition,
    o: &Options,
    work: &mut Work,
) -> Result<Vec<wall::Geometry>, FabError> {
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    let Supports {
        sides,
        floor,
        stops,
    } = supports(mesh, p, tol)?;
    work.spend(48 * mesh.triangles.len())?;
    // Use one orthonormal frame. Both charts meet the same pair of centered
    // axes even when STL rounding perturbs the measured face normals.
    let up = p.planes[floor].support.normal;
    let n = p.planes[sides[0]].support.normal;
    let n0 = unit(sub(n, scale(up, dot(up, n)))).ok_or_else(unsupported)?;
    let n1 = cross(up, n0);
    let sign = dot(n1, p.planes[sides[1]].support.normal).signum();
    p.planes[sides[0]].support.normal = n0;
    p.planes[sides[1]].support.normal = scale(n1, sign);
    let lateral = sides.map(|i| p.planes[i].support);
    for k in 0..2 {
        p.planes[stops[k]].support.normal = scale(lateral[1 - k].normal, -1.);
    }
    let base = p.planes[floor].support;
    let corner =
        intersection([lateral[0], lateral[1], base], base.origin).ok_or_else(unsupported)?;
    // Height of each straight side rail above the floor is its radius.
    let mut heights = Vec::new();
    for side in sides {
        let path = &p.cuts[&(side, WALL)];
        for &id in [path.first().unwrap(), path.last().unwrap()] {
            heights.push(base.distance(mesh.vertices[id]));
        }
    }
    let radius = heights.iter().sum::<f64>() / 4.;
    checked(radius > 8. * tol && heights.iter().all(|&r| (r - radius).abs() <= tol))?;
    let bisector = unit(sub(lateral[0].normal, lateral[1].normal)).ok_or_else(unsupported)?;
    for (t, id) in p.owners.iter_mut().enumerate() {
        if *id != WALL {
            continue;
        }
        // The smaller side distance identifies the material boundary. A
        // triangle may touch the bisector but cannot straddle it. The union
        // of shared source edges will be tested in both cylindrical charts.
        let d = mesh.triangles[t].map(|v| dot(sub(mesh.vertices[v], corner), bisector));
        let lo = d.into_iter().fold(f64::INFINITY, f64::min);
        let hi = d.into_iter().fold(f64::NEG_INFINITY, f64::max);
        match (lo < -tol, hi > tol) {
            (true, false) => {}
            (false, true) => *id = WALL + 1,
            _ => return Err(unsupported()),
        }
    }
    (p.cuts, p.neighbor) = interfaces(mesh, &p.owners)?;
    let mut result = Vec::new();
    for k in 0..2 {
        let id = WALL + k;
        let other = WALL + 1 - k;
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
        checked(neighbors == BTreeSet::from([sides[k], floor, stops[k], other]))?;
        let n = scale(
            bisector,
            -dot(bisector, p.planes[stops[k]].support.normal).signum(),
        );
        result.push(wall::fit_between(
            mesh,
            p,
            o,
            wall::Stops {
                wall: id,
                rails: [sides[k], floor],
                circle_plane: stops[k],
                ellipse_plane: other,
                far: Plane {
                    origin: corner,
                    normal: n,
                },
                shared_radius: Some(radius),
            },
        )?);
    }
    same_ellipse(&result[0].ellipse, &result[1].ellipse)?;
    Ok(result)
}
