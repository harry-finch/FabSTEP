//! Annular spherical patches. Both limits must be actual source plane loops.
use super::*;

pub(super) fn candidate(
    nodes: &[Node],
    projected: &BTreeMap<usize, V>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<SphericalPatch>, FabError> {
    work.spend(projected.len().saturating_mul(4))?;
    let mut ends = [0, 1];
    if nodes[0].z > nodes[1].z {
        ends.swap(0, 1);
    }
    let a = &nodes[ends[0]];
    let b = &nodes[ends[1]];
    let Some((center_z, radius)) = parameters(a, b, o) else {
        return Ok(None);
    };
    let height = b.z - a.z;
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let slope = (b.radius - a.radius) / height;
    let mut evidence = 0;
    let mut lowest = f64::INFINITY;
    let mut highest = f64::NEG_INFINITY;
    for p in projected.values() {
        let depth = p[2] - a.z;
        let rho = p[0].hypot(p[1]);
        if depth < -tol
            || depth > height + tol
            || (rho.hypot(p[2] - center_z) - radius).abs() > o.cylinder_fit_tolerance_mm
        {
            return Ok(None);
        }
        // Endpoints alone also fit a cone (including its cylindrical limit).
        // Demand resolved non-affine meridional curvature at multiple depths.
        if depth > 4. * tol
            && height - depth > 4. * tol
            && (rho - (a.radius + slope * depth)).abs() / slope.hypot(1.)
                > o.cylinder_fit_tolerance_mm
        {
            evidence += 1;
            lowest = lowest.min(depth);
            highest = highest.max(depth);
        }
    }
    Ok(
        (evidence >= 12 && highest - lowest > 4. * tol).then_some(SphericalPatch {
            ends,
            center_z,
            radius,
        }),
    )
}

fn parameters(a: &Node, b: &Node, o: &Options) -> Option<(f64, f64)> {
    let height = b.z - a.z;
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    if height <= 4. * tol || a.radius.min(b.radius) <= 4. * tol {
        return None;
    }
    // Shift to the lower plane and normalize before squaring. Factoring the
    // difference of squares avoids cancellation for nearly equal rings.
    let scale = a.radius.max(b.radius).max(height);
    let h = height / scale;
    let r0 = a.radius / scale;
    let r1 = b.radius / scale;
    let offset = scale * ((r1 - r0) * (r1 + r0) + h * h) / (2. * h);
    let radius = a.radius.hypot(offset);
    let center_z = a.z + offset;
    // Account for the division by the normalized plane separation as well
    // as the eventual radius. A fit below numerical resolution is refused.
    if !radius.is_finite()
        || !center_z.is_finite()
        || radius.max(scale / h) * f64::EPSILON * 128. > o.cylinder_fit_tolerance_mm
    {
        return None;
    }
    Some((center_z, radius))
}

pub(super) fn prove(
    a: &Analysis,
    p: &Patch,
    sphere: SphericalPatch,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<(Errors, bool), FabError> {
    let rings = sphere.ends.map(|n| &p.nodes[n]);
    work.spend(p.triangles.len().saturating_mul(4))?;
    topology(a, &p.triangles, &rings, edges)?;
    let radius = sphere.radius;
    let limits = rings.map(|n| n.z - sphere.center_z);
    let ids = rings.map(|n| n.vertices.iter().copied().collect::<BTreeSet<_>>());
    if !ids[0].is_disjoint(&ids[1]) || limits[0] >= limits[1] {
        return Err(unproven("anneaux sphériques confondus ou inversés"));
    }
    let mut projected = BTreeMap::new();
    let mut errors = Errors::default();
    for v in p
        .triangles
        .iter()
        .flat_map(|&t| a.mesh.triangles[t])
        .collect::<BTreeSet<_>>()
    {
        work.spend(1)?;
        let actual = sub(
            p.frame.project(a.mesh.vertices[v]),
            [0., 0., sphere.center_z],
        );
        let rho = actual[0].hypot(actual[1]);
        if rho == 0.
            || actual[2] < limits[0] - o.cylinder_fit_tolerance_mm
            || actual[2] > limits[1] + o.cylinder_fit_tolerance_mm
        {
            return Err(unproven("sommet hors de la zone sphérique sans pôle"));
        }
        let ideal = project_vertex(
            actual,
            radius,
            rings.map(|r| r.radius),
            limits,
            [ids[0].contains(&v), ids[1].contains(&v)],
        );
        let residual = norm(sub(actual, ideal));
        if !residual.is_finite() || residual > o.cylinder_fit_tolerance_mm {
            return Err(unproven("projection sphérique hors tolérance"));
        }
        errors.radial = errors.radial.max(residual);
        projected.insert(v, ideal);
    }
    let mut area = Sum::default();
    let mut inner = None;
    for &t in &p.triangles {
        work.spend(1)?;
        let [u, v, w] = a.mesh.triangles[t].map(|i| projected[&i]);
        let n = [u, v, w].map(|q| scale(q, 1. / radius));
        if (0..3).any(|i| dot(n[i], n[(i + 1) % 3]) < (PI / 4.).cos()) {
            return Err(unproven("triangle sphérique trop peu échantillonné"));
        }
        let triple = dot(n[0], cross(n[1], n[2]));
        if !triple.is_finite()
            || triple.abs() < 1e-15
            || inner.is_some_and(|old| old != (triple < 0.))
        {
            return Err(unproven(
                "projection de zone sphérique dégénérée ou repliée",
            ));
        }
        inner = Some(triple < 0.);
        area.add(
            2. * triple
                .abs()
                .atan2(1. + dot(n[0], n[1]) + dot(n[1], n[2]) + dot(n[2], n[0])),
        );
        errors.chord = errors.chord.max(radius - min_triangle_norm(u, v, w));
    }
    // Choose the nearer auxiliary pole to reduce subtraction cancellation.
    let reference_sign = if limits[0] + limits[1] >= 0. { 1. } else { -1. };
    let mut fans = [0.; 2];
    let mut latitude_ranges = [[0.; 2]; 2];
    for j in 0..2 {
        let rim = rings[j];
        let latitude = limits[j].atan2(rim.radius);
        latitude_ranges[j] = [latitude; 2];
        let mut winding = Sum::default();
        let mut fan = Sum::default();
        let mut direction = None;
        let mut fringe: f64 = 0.;
        for i in 0..rim.vertices.len() {
            work.spend(1)?;
            let u = projected[&rim.vertices[i]];
            let v = projected[&rim.vertices[(i + 1) % rim.vertices.len()]];
            let du = (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]);
            if du.abs() < 1e-12
                || du.abs() > PI / 4.
                || direction.is_some_and(|old| old != (du < 0.))
            {
                return Err(unproven(
                    "contour de zone sphérique replié ou sous-échantillonné",
                ));
            }
            direction = Some(du < 0.);
            winding.add(du.abs());
            let u = scale(u, 1. / radius);
            let v = scale(v, 1. / radius);
            // Auxiliary pole is an integration reference only, never a mesh/model pole.
            fan.add(
                2. * (u[0] * v[1] - u[1] * v[0])
                    .abs()
                    .atan2(1. + reference_sign * (u[2] + v[2]) + dot(u, v)),
            );
            let geodesic = limits[j].atan2(rim.radius * (du / 2.).cos());
            latitude_ranges[j][0] = latitude_ranges[j][0].min(geodesic);
            latitude_ranges[j][1] = latitude_ranges[j][1].max(geodesic);
            fringe = fringe.max(2. * radius * ((geodesic - latitude).abs() / 2.).sin());
        }
        if (winding.get() - TAU).abs() > 1e-6 {
            return Err(unproven("contour de zone sphérique sans tour unique"));
        }
        fans[j] = fan.get();
        // Sum both boundary fringes conservatively, not merely one or their max.
        errors.chord += fringe;
    }
    let expected = reference_sign * (fans[0] - fans[1]);
    let rounding = 128.
        * f64::EPSILON
        * (p.triangles.len() + rings.iter().map(|r| r.vertices.len()).sum::<usize>()) as f64;
    let area_tolerance = 1e-6_f64.min(expected * 1e-6);
    // Strict separation makes the two simple geodesic polygons nested. Their
    // fan difference is the annulus area, including non-equatorial boundaries.
    // Together with consistent local orientation and annular source topology,
    // equality establishes degree one, excluding holes and multiple coverage.
    if latitude_ranges[0][1] >= latitude_ranges[1][0]
        || !expected.is_finite()
        || area_tolerance <= rounding
        || expected <= rounding * 16.
        || (area.get() - expected).abs() > area_tolerance
    {
        return Err(unproven("couverture annulaire sphérique non démontrée"));
    }
    Ok((
        errors,
        inner.ok_or_else(|| unproven("zone sphérique vide"))?,
    ))
}

/// Clamp the radial image to either small circle; callers account the complete
/// vector displacement, including the latitude correction, in the fit residual.
fn project_vertex(
    actual: V,
    radius: f64,
    radii: [f64; 2],
    limits: [f64; 2],
    boundary: [bool; 2],
) -> V {
    let rho = actual[0].hypot(actual[1]);
    let mut ideal = scale(actual, radius / norm(actual));
    for i in 0..2 {
        if boundary[i] || (i == 0 && ideal[2] < limits[0]) || (i == 1 && ideal[2] > limits[1]) {
            ideal = [
                radii[i] * actual[0] / rho,
                radii[i] * actual[1] / rho,
                limits[i],
            ];
        }
    }
    ideal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_latitude_clamps_count_the_full_vector_displacement() {
        let radii = [96_f64.sqrt(), 6.];
        let limits = [2., 8.];
        let delta = 2e-6;
        for (i, actual) in [
            (0, [radii[0] + delta, 0., limits[0] + delta * 0.1]),
            (1, [radii[1] - delta, 0., limits[1] - delta * 0.1]),
        ] {
            let radial = scale(actual, 10. / norm(actual));
            assert!(if i == 0 {
                radial[2] < limits[0]
            } else {
                radial[2] > limits[1]
            });
            let ideal = project_vertex(actual, 10., radii, limits, [false; 2]);
            assert_eq!(ideal[2], limits[i]);
            assert!(norm(sub(ideal, [radii[i], 0., limits[i]])) < 1e-14);
            assert!(norm(sub(actual, ideal)) > (norm(actual) - 10.).abs());
            assert!((norm(sub(actual, ideal)) - delta * 1.01_f64.sqrt()).abs() < 1e-14);
        }
    }

    #[test]
    fn unresolved_planes_conditioning_and_work_budget_cannot_create_a_zone() {
        let o = Options::default();
        let nodes = |height: f64| {
            vec![
                Node {
                    vertices: vec![],
                    z: 0.,
                    radius: 10.,
                },
                Node {
                    vertices: vec![],
                    z: height,
                    radius: 10.,
                },
            ]
        };
        let thin = nodes(1e-6);
        assert!(parameters(&thin[0], &thin[1], &o).is_none());
        let regular = nodes(10.);
        assert!(parameters(&regular[0], &regular[1], &o).is_some());
        // Even an otherwise resolved separation cannot support a fit finer
        // than the error amplification of its normalized center computation.
        let mut huge = nodes(1.);
        for n in &mut huge {
            n.radius = 1e8;
        }
        assert!(parameters(&huge[0], &huge[1], &o).is_none());
        let mut work = Work::default();
        work.spend(10_000_000).unwrap();
        let points = BTreeMap::from([(0, [10., 0., 0.])]);
        assert_eq!(
            candidate(&nodes(10.), &points, &o, &mut work)
                .err()
                .unwrap()
                .code,
            "recognition.too_complex"
        );
    }
}
