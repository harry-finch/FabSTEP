//! A blind rectangular pocket with two orthogonal concave fillets of distinct radii.
//! The smaller cylinder ends on the larger. The latter continues on the small
//! cylinder's tangent plane past the common nonplanar seam.
use super::*;
use crate::UnequalPocket;

#[path = "unequal_pocket/band.rs"]
mod band;
#[path = "unequal_pocket/boundaries.rs"]
mod boundaries;

const FLOOR: usize = 10;
const SMALL: usize = 11;
const LARGE: usize = 12;
const SIDE: [usize; 2] = [8, 6];
const STOP: [usize; 2] = [7, 9];

fn local(model: &UnequalPocket, point: V) -> V {
    model.axes.map(|n| dot(n, sub(point, model.origin)))
}

// Exact auxiliary-plane inset along one side, at the angle on the other
// cylinder. Using sin(t/2) avoids taking sqrt(h) at a noisy floor endpoint.
fn inset_at_angle(own: f64, other: f64, angle: f64) -> f64 {
    let u = (angle / 2.).sin();
    let height = 2. * own * u * u;
    if height >= other {
        return 0.;
    }
    // Rationalize other - sqrt(height*(2*other-height)) near the endpoint.
    (other - height).powi(2) / (other + (height * (2. * other - height)).sqrt())
}

// For a prescribed normal, the midplane of the extreme projections
// minimizes the maximum point residual, including all interior vertices.
fn center_plane(mesh: &Mesh, patch: &Patch, normal: V) -> Plane {
    let base = patch.support.origin;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for &t in &patch.triangles {
        for &v in &mesh.triangles[t] {
            let h = dot(sub(mesh.vertices[v], base), normal);
            low = low.min(h);
            high = high.max(h);
        }
    }
    Plane {
        origin: add(base, scale(normal, (low + high) / 2.)),
        normal,
    }
}

fn refit_supports(mesh: &Mesh, part: &mut Partition) -> Result<(), FabError> {
    // The seed triangle suffices for discovery. Binary STL rounding makes
    // its normal less reliable than the oriented area of the entire patch.
    for patch in &mut part.planes {
        let mut vector = [0.; 3];
        for &t in &patch.triangles {
            let [a, b, c] = mesh.triangles[t].map(|v| mesh.vertices[v]);
            vector = add(vector, cross(sub(b, a), sub(c, a)));
        }
        patch.support.normal = unit(vector).ok_or_else(unsupported)?;
    }
    Ok(())
}

fn parallel_normal(planes: &[Patch], reference: V) -> Result<V, FabError> {
    let mut vector = [0.; 3];
    for patch in planes {
        let agreement = dot(reference, patch.support.normal);
        if agreement.abs() > 1. - 1e-10 {
            vector = add(
                vector,
                scale(patch.support.normal, agreement.signum() * patch.area),
            );
        }
    }
    unit(vector).ok_or_else(unsupported)
}

fn frame(
    part: &Partition,
    mesh: &Mesh,
    tol: f64,
) -> Result<(UnequalPocket, [usize; WALL]), FabError> {
    let supports: Vec<_> = part
        .cuts
        .keys()
        .filter_map(|&(i, j)| (j == WALL).then_some(i))
        .collect();
    checked(supports.len() == 5)?;
    let normal = |i: usize| part.planes[i].support.normal;
    let floors: Vec<_> = supports
        .iter()
        .copied()
        .filter(|&i| {
            supports
                .iter()
                .all(|&j| i == j || dot(normal(i), normal(j)).abs() < 1e-6)
        })
        .collect();
    let [floor]: [usize; 1] = floors.try_into().map_err(|_| unsupported())?;
    let up = center_plane(
        mesh,
        &part.planes[floor],
        parallel_normal(&part.planes, normal(floor))?,
    );
    let mut sides = Vec::new();
    for &i in &supports {
        if i == floor {
            continue;
        }
        // A terminal stop touches its cylinder all the way down to the
        // floor; a tangent side starts at the fillet's own radius.
        let low = part.cuts[&(i, WALL)]
            .iter()
            .map(|&v| up.distance(mesh.vertices[v]))
            .fold(f64::INFINITY, f64::min);
        if low > 8. * tol {
            sides.push((low, i));
        } else {
            checked(low.abs() <= tol)?;
        }
    }
    sides.sort_by(|a, b| f64::total_cmp(&a.0, &b.0));
    let [(small, sy), (large, sx)]: [(f64, usize); 2] =
        sides.try_into().map_err(|_| unsupported())?;
    checked(large - small > 8. * tol)?;
    let side = parallel_normal(&part.planes, normal(sx))?;
    let x = unit(sub(side, scale(up.normal, dot(side, up.normal)))).ok_or_else(unsupported)?;
    let y_raw = cross(up.normal, x);
    let y = scale(y_raw, dot(y_raw, normal(sy)).signum());
    let axes = [x, y, up.normal];
    checked(dot(normal(sx), x) > 1. - 1e-10 && dot(normal(sy), y) > 1. - 1e-10)?;
    let origin = intersection(
        [
            center_plane(mesh, &part.planes[sx], x),
            center_plane(mesh, &part.planes[sy], y),
            up,
        ],
        up.origin,
    )
    .ok_or_else(unsupported)?;
    let mut ids = [usize::MAX; WALL];
    ids[6] = sx;
    ids[8] = sy;
    ids[FLOOR] = floor;
    let mut outer_min = [0.; 3];
    let mut outer_max = [0.; 3];
    let mut spans = [0.; 2];
    for (i, patch) in part.planes.iter().enumerate() {
        if [sx, sy, floor].contains(&i) {
            continue;
        }
        let projections = axes.map(|axis| dot(axis, normal(i)));
        let axis = projections
            .iter()
            .position(|n| n.abs() > 1. - 1e-10)
            .ok_or_else(unsupported)?;
        let coord = dot(
            sub(center_plane(mesh, patch, axes[axis]).origin, origin),
            axes[axis],
        );
        let slot = if supports.contains(&i) {
            checked(axis < 2 && projections[axis] < 0.)?;
            spans[axis] = coord;
            7 + 2 * axis
        } else {
            let positive = projections[axis] > 0.;
            if positive {
                outer_max[axis] = coord;
            } else {
                outer_min[axis] = coord;
            }
            2 * axis + usize::from(positive)
        };
        checked(ids[slot] == usize::MAX)?;
        ids[slot] = i;
    }
    checked(
        !ids.contains(&usize::MAX)
            && outer_min.iter().all(|v| *v < -tol)
            && spans[0] > large + tol
            && spans[1] > small + tol
            && outer_max[0] > spans[0] + tol
            && outer_max[1] > spans[1] + tol
            && outer_max[2] > large + tol,
    )?;
    Ok((
        UnequalPocket {
            origin,
            axes,
            radii: [small, large],
            outer_min,
            outer_max,
            pocket_spans: spans,
        },
        ids,
    ))
}

fn split(
    mesh: &Mesh,
    part: &mut Partition,
    ids: [usize; WALL],
    coords: &[V],
    radii: [f64; 2],
    tol: f64,
) -> Result<(), FabError> {
    let mut map = [0; WALL];
    for (index, id) in ids.iter().copied().enumerate() {
        map[id] = index;
    }
    for (tri, owner) in mesh.triangles.iter().zip(&mut part.owners) {
        if *owner != WALL {
            *owner = map[*owner];
            continue;
        }
        let on_wall = |k: usize| {
            tri.iter().all(|&id| {
                let p = coords[id];
                ((p[1 - k] - radii[k]).hypot(p[2] - radii[k]) - radii[k]).abs() <= tol
            })
        };
        *owner = match (on_wall(0), on_wall(1)) {
            (true, false) => SMALL,
            (false, true) => LARGE,
            _ => return Err(unsupported()),
        };
    }
    (part.cuts, part.neighbor) = interfaces(mesh, &part.owners)?;
    Ok(())
}

fn plane_geometry(model: &UnequalPocket, i: usize) -> (V, V) {
    let (axis, offset, sign) = match i {
        0..=5 => {
            let k = i / 2;
            if i % 2 == 0 {
                (k, model.outer_min[k], -1.)
            } else {
                (k, model.outer_max[k], 1.)
            }
        }
        6 => (0, 0., 1.),
        7 => (0, model.pocket_spans[0], -1.),
        8 => (1, 0., 1.),
        9 => (1, model.pocket_spans[1], -1.),
        _ => (2, 0., 1.),
    };
    (
        add(model.origin, scale(model.axes[axis], offset)),
        scale(model.axes[axis], sign),
    )
}

pub(super) fn candidate(
    a: &Analysis,
    o: &Options,
    adjacency: &[Vec<usize>],
    work: &mut Work,
    mut part: Partition,
) -> Result<RoundedRecognition, FabError> {
    let mesh = &a.mesh;
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    work.spend(100 * mesh.triangles.len() + 24 * mesh.vertices.len())?;
    refit_supports(mesh, &mut part)?;
    let (domain, ids) = frame(&part, mesh, tol)?;
    let coords: Vec<_> = mesh.vertices.iter().map(|&p| local(&domain, p)).collect();
    split(mesh, &mut part, ids, &coords, domain.radii, tol)?;
    for id in [SMALL, LARGE] {
        prove_disk(mesh, adjacency, &part.owners, id)?;
    }
    let mut planes = Vec::new();
    let mut checks = 0;
    let mut flat = 0_f64;
    for (i, raw_id) in ids.into_iter().enumerate() {
        let patch = &part.planes[raw_id];
        let (origin, normal) = plane_geometry(&domain, i);
        for &t in &patch.triangles {
            let [a, b, c] = mesh.triangles[t].map(|id| mesh.vertices[id]);
            checked(dot(cross(sub(b, a), sub(c, a)), normal) > 0.)?;
            for p in [a, b, c] {
                flat = flat.max(dot(sub(p, origin), normal).abs());
            }
        }
        checked(flat <= o.plane_tolerance_mm.min(1e-5))?;
        let before = checks;
        let contours = crate::planar::validate_loops(
            mesh,
            crate::planar::boundaries(mesh, &patch.triangles)?,
            origin,
            normal,
            &mut checks,
        )?;
        work.spend(checks - before)?;
        checked(contours.len() == if i == 5 { 2 } else { 1 })?;
        planes.push(MixedPlane {
            normal,
            loops: vec![],
        });
    }
    let (linear, adjusted) = boundaries::verify(&part, &domain, &coords, tol)?;
    let mut radial = 0_f64;
    let mut chord = 0_f64;
    for k in 0..2 {
        let e = band::prove(mesh, &part, &domain, &coords, k, tol)?;
        radial = radial.max(e.residual);
        chord = chord.max(e.chord);
    }
    let extent = mesh
        .vertices
        .iter()
        .flatten()
        .fold(1_f64, |m, &x| m.max(x.abs()));
    let rounding = 256. * f64::EPSILON * extent;
    checked(rounding < 1e-8)?;
    // Reserve 1e-5 mm for the native construction and transfer of the seam.
    let trim = linear + 2. * flat + rounding + 1e-5;
    let bound = o.weld_tolerance_mm + chord + radial + trim;
    if bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le raccord entre les deux congés de rayons différents dépasse la tolérance d’écart.",
        ));
    }
    let mixed = MixedModel {
        vertices: vec![],
        planes,
        rings: vec![],
        cylinders: vec![],
        arcs: vec![],
        elliptic_arcs: vec![],
        partial_cylinders: vec![],
    };
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed,
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        partial_cones: vec![],
        octants: vec![],
        cross_drill: None,
        linear_blend: None,
        torus_sectors: vec![],
        smooth_revolution: None,
        unequal_pocket: Some(Box::new(domain)),
    };
    let expected_volume_mm3 = crate::rounded::volume(&model)?;
    let triangle_faces = part
        .owners
        .into_iter()
        .map(|i| {
            if i < SMALL {
                RoundedFaceRef::Plane { index: i }
            } else {
                RoundedFaceRef::PartialCylinder { index: i - SMALL }
            }
        })
        .collect();
    let mut diagnostics = vec![Diagnostic {
        code: "recognition.unequal_pocket_fitted".into(),
        count: 2,
        severity: Severity::Warning,
        message:
            "Les deux congés de rayons différents sont reconstruits avec une arête commune mesurée."
                .into(),
    }];
    if adjusted != 0 {
        diagnostics.push(Diagnostic {
            code: "recognition.unequal_pocket_adjusted".into(),
            count: adjusted,
            severity: Severity::Info,
            message: "Les sommets de jonction ont été ajustés aux supports mesurés dans la borne d’écart.".into(),
        });
    }
    Ok(RoundedRecognition {
        model,
        triangle_faces,
        report: RecognitionReport {
            revolutions: 0,
            cylinders: 2,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            max_radial_residual_mm: radial,
            max_chord_error_mm: chord,
            max_wall_residual_mm: trim,
            max_deviation_bound_mm: bound,
            expected_volume_mm3,
            diagnostics,
        },
    })
}
