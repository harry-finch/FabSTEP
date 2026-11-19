use super::*;

pub(super) struct Geometry {
    pub id: usize,
    pub circle_plane: usize,
    pub ellipse_plane: usize,
    pub circle: CircularArc,
    pub ellipse: EllipticArc,
    pub near_ellipse: Option<EllipticArc>,
    near_slope: [f64; 2],
    rails: [usize; 2],
    x: V,
    y: V,
    h: f64,
    slope: [f64; 2],
}
impl Geometry {
    fn low(&self, theta: f64) -> f64 {
        self.near_slope[0] * theta.cos() + self.near_slope[1] * theta.sin()
    }
    fn height(&self, theta: f64) -> f64 {
        self.h + self.slope[0] * theta.cos() + self.slope[1] * theta.sin()
    }
    fn point(&self, theta: f64, z: f64) -> V {
        let dx = scale(self.x, self.circle.radius_mm * theta.cos());
        let dy = scale(self.y, self.circle.radius_mm * theta.sin());
        add(
            self.circle.center,
            add(add(dx, dy), scale(self.circle.axis, z)),
        )
    }
    pub fn corner(&self, id: usize) -> Option<V> {
        for (k, theta) in [0., FRAC_PI_2].into_iter().enumerate() {
            if self.circle.vertices[k] == id {
                return Some(self.point(theta, self.low(theta)));
            }
            if self.ellipse.vertices[k] == id {
                return Some(self.point(theta, self.height(theta)));
            }
        }
        None
    }
}

pub(super) fn fit(mesh: &Mesh, p: &mut Partition, o: &Options) -> Result<Geometry, FabError> {
    let raw = &mesh.vertices;
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    let mut rails = Vec::new();
    let mut caps = Vec::new();
    for (&(i, other), path) in &p.cuts {
        if other == WALL {
            if straight(path, raw, raw, tol).is_some() {
                rails.push(i);
            } else {
                caps.push(i);
            }
        }
    }
    checked(rails.len() == 2 && caps.len() == 2)?;
    let axis = cross(
        p.planes[rails[0]].support.normal,
        p.planes[rails[1]].support.normal,
    );
    caps.sort_by(|&a, &b| {
        dot(axis, p.planes[b].support.normal)
            .abs()
            .total_cmp(&dot(axis, p.planes[a].support.normal).abs())
    });
    let (circle_plane, ellipse_plane) = (caps[0], caps[1]);
    fit_between(
        mesh,
        p,
        o,
        Stops {
            wall: WALL,
            rails: [rails[0], rails[1]],
            circle_plane,
            ellipse_plane,
            near: p.planes[circle_plane].support,
            far: p.planes[ellipse_plane].support,
            shared_radius: None,
        },
    )
}

pub(super) struct Stops {
    pub wall: usize,
    pub rails: [usize; 2],
    pub circle_plane: usize,
    pub ellipse_plane: usize,
    pub near: Plane,
    pub far: Plane,
    pub shared_radius: Option<f64>,
}

/// Minimum of a*cos(theta) + b*sin(theta) over the closed first quadrant.
fn minimum_quarter_offset([a, b]: [f64; 2]) -> f64 {
    if a < 0. && b < 0. {
        -a.hypot(b)
    } else {
        a.min(b)
    }
}

pub(super) fn fit_between(
    mesh: &Mesh,
    p: &mut Partition,
    o: &Options,
    stops: Stops,
) -> Result<Geometry, FabError> {
    let Stops {
        wall,
        mut rails,
        circle_plane,
        ellipse_plane,
        near,
        far,
        shared_radius,
    } = stops;
    let raw = &mesh.vertices;
    let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    checked(
        dot(
            p.planes[rails[0]].support.normal,
            p.planes[rails[1]].support.normal,
        )
        .abs()
            < 1e-6,
    )?;
    let mut axis = unit(cross(
        p.planes[rails[0]].support.normal,
        p.planes[rails[1]].support.normal,
    ))
    .ok_or_else(unsupported)?;
    let square_stop = dot(axis, near.normal).abs() > 1. - 1e-10;
    checked(
        square_stop
            || ((circle_plane >= WALL || shared_radius.is_none())
                && (0.1..0.99999).contains(&dot(axis, near.normal).abs())),
    )?;
    let near_chain = &p.cuts[&pair(circle_plane, wall)];
    let far_chain = &p.cuts[&pair(ellipse_plane, wall)];
    checked(near_chain.len() >= 7 && far_chain.len() >= 7)?;
    let near_mid = raw[near_chain[near_chain.len() / 2]];
    let far_mid = raw[far_chain[far_chain.len() / 2]];
    if dot(axis, sub(far_mid, near_mid)) < 0. {
        axis = scale(axis, -1.);
        rails.swap(0, 1);
    }
    // Choose the order of the rails so that x × y points from the circle
    // to the ellipse. Both are inward normals of the cylindrical cavity.
    checked(dot(near.normal, axis) > 0.1 && dot(far.normal, axis) < -0.1)?;
    let x = scale(p.planes[rails[0]].support.normal, -1.);
    let y = cross(axis, x);
    p.planes[rails[1]].support.normal = scale(y, -1.);
    if square_stop {
        p.planes[circle_plane].support.normal = axis;
    }
    let cap_ends = |cap: &[usize]| -> Result<[usize; 2], FabError> {
        let mut ids = [0; 2];
        for (k, &i) in rails.iter().enumerate() {
            let cut = &p.cuts[&pair(i, wall)];
            let hits: Vec<_> = [cap[0], *cap.last().unwrap()]
                .into_iter()
                .filter(|v| *v == cut[0] || *v == *cut.last().unwrap())
                .collect();
            checked(hits.len() == 1)?;
            ids[k] = hits[0];
        }
        checked(ids[0] != ids[1])?;
        Ok(ids)
    };
    let low = cap_ends(near_chain)?;
    let high = cap_ends(far_chain)?;
    let supports = rails.map(|i| p.planes[i].support);
    let base = intersection([supports[0], supports[1], near], near_mid).ok_or_else(unsupported)?;
    let radius = (supports[1].distance(raw[low[0]])
        + supports[0].distance(raw[low[1]])
        + supports[1].distance(raw[high[0]])
        + supports[0].distance(raw[high[1]]))
        / 4.;
    let radius = shared_radius.unwrap_or(radius);
    checked(radius.is_finite() && radius > 8. * tol)?;
    let mut center = add(base, scale(add(x, y), -radius));
    let near_slope = if square_stop {
        [0., 0.]
    } else {
        let cosine = dot(near.normal, axis);
        center = add(center, scale(axis, -near.distance(center) / cosine));
        [
            -radius * dot(near.normal, x) / cosine,
            -radius * dot(near.normal, y) / cosine,
        ]
    };
    let normal = scale(far.normal, dot(far.normal, axis).signum());
    let cosine = dot(normal, axis);
    checked((0.1..0.99999).contains(&cosine))?;
    let h = dot(sub(far.origin, center), normal) / cosine;
    let slope = [
        -radius * dot(normal, x) / cosine,
        -radius * dot(normal, y) / cosine,
    ];
    if shared_radius.is_some() {
        // The seam terminates this wall on its proven quarter turn. Narrow
        // pockets can have positive height here even if the bisector would
        // cross the opposite side of a hypothetical complete cylinder.
        checked(
            h + minimum_quarter_offset([slope[0] - near_slope[0], slope[1] - near_slope[1]]) > tol,
        )?;
    } else {
        // Keep the historical clearance of an isolated oblique stop.
        checked(h > (slope[0] - near_slope[0]).hypot(slope[1] - near_slope[1]) + tol)?;
    }
    let circle = CircularArc {
        center,
        axis,
        radius_mm: radius,
        sweep_rad: FRAC_PI_2,
        vertices: low,
    };
    let section =
        |origin: V, normal: V, slope: [f64; 2], vertices| -> Result<EllipticArc, FabError> {
            let cosine = dot(normal, axis);
            let major_direction = unit(sub(axis, scale(normal, cosine))).ok_or_else(unsupported)?;
            let minor_direction = cross(normal, major_direction);
            let first_radius = add(scale(x, radius), scale(axis, slope[0]));
            let major_mm = radius / cosine;
            let start_rad = (dot(first_radius, minor_direction) / radius)
                .atan2(dot(first_radius, major_direction) / major_mm);
            Ok(EllipticArc {
                center: origin,
                axis: normal,
                major_direction,
                major_mm,
                minor_mm: radius,
                start_rad,
                sweep_rad: FRAC_PI_2,
                vertices,
            })
        };
    let ellipse = section(add(center, scale(axis, h)), normal, slope, high)?;
    let near_ellipse = if square_stop {
        None
    } else {
        Some(section(center, near.normal, near_slope, low)?)
    };
    Ok(Geometry {
        id: wall,
        circle_plane,
        ellipse_plane,
        circle,
        ellipse,
        near_ellipse,
        near_slope,
        rails,
        x,
        y,
        h,
        slope,
    })
}

pub(super) struct ErrorBound {
    pub residual: f64,
    pub chord: f64,
}
pub(super) fn prove(
    mesh: &Mesh,
    part: &Partition,
    g: &Geometry,
    o: &Options,
    work: &mut Work,
) -> Result<ErrorBound, FabError> {
    let raw = &mesh.vertices;
    let count = raw.len();
    work.spend(20 * count + 8 * mesh.triangles.len())?;
    let mut chart = vec![[0.; 2]; count];
    let mut wall_vertices = BTreeSet::new();
    for (tri, &owner) in mesh.triangles.iter().zip(&part.owners) {
        if owner == g.id {
            wall_vertices.extend(tri.iter().copied());
        }
    }
    for &id in &wall_vertices {
        let d = sub(raw[id], g.circle.center);
        chart[id] = [dot(d, g.y).atan2(dot(d, g.x)), dot(d, g.circle.axis)];
    }
    for (&rail, angle) in g.rails.iter().zip([0., FRAC_PI_2]) {
        for &id in &part.cuts[&pair(rail, g.id)] {
            chart[id][0] = angle;
        }
    }
    let mut stop_samples = [Vec::new(), Vec::new()];
    for (&cap, is_ellipse) in [&g.circle_plane, &g.ellipse_plane]
        .into_iter()
        .zip([false, true])
    {
        let path = &part.cuts[&pair(cap, g.id)];
        let mut samples = Vec::with_capacity(path.len());
        for &id in path {
            let theta = chart[id][0];
            let z = if is_ellipse {
                g.height(theta)
            } else {
                g.low(theta)
            };
            chart[id][1] = z;
            samples.push([theta, z]);
        }
        if samples[0][0] > samples.last().unwrap()[0] {
            samples.reverse();
        }
        checked(samples[0][0] == 0. && samples.last().unwrap()[0] == FRAC_PI_2)?;
        checked(
            samples
                .windows(2)
                .all(|w| w[0][0] < w[1][0] && w[1][0] - w[0][0] <= FRAC_PI_2 / 2.),
        )?;
        stop_samples[usize::from(is_ellipse)] = samples;
    }
    for &rail in &g.rails {
        let path = &part.cuts[&pair(rail, g.id)];
        let ascending = chart[path[0]][1] < chart[*path.last().unwrap()][1];
        checked(path.windows(2).all(|p| {
            if ascending {
                chart[p[0]][1] < chart[p[1]][1]
            } else {
                chart[p[0]][1] > chart[p[1]][1]
            }
        }))?;
    }
    let mut residual = 0_f64;
    let mut cap_area = 0.;
    let mut span = [0_f64; 2];
    for (i, samples) in stop_samples.iter().enumerate() {
        for w in samples.windows(2) {
            let interval = w[1][0] - w[0][0];
            let trapezium = interval * (w[1][1] + w[0][1]) / 2.;
            cap_area += if i == 0 { -trapezium } else { trapezium };
            span[i] = span[i].max(interval);
        }
    }
    let height_at = |samples: &[[f64; 2]], theta: f64| {
        let i = samples
            .partition_point(|p| p[0] < theta)
            .clamp(1, samples.len() - 1);
        let [l, r] = [samples[i - 1], samples[i]];
        l[1] + (r[1] - l[1]) * (theta - l[0]) / (r[0] - l[0])
    };
    let eps = 128. * f64::EPSILON * (g.h + g.circle.radius_mm).max(1.);
    // Independent end samplings can have different knots. Their interpolants
    // are linear between consecutive knots of the union.
    for sample in stop_samples.iter().flatten() {
        checked(
            height_at(&stop_samples[1], sample[0]) > height_at(&stop_samples[0], sample[0]) + eps,
        )?;
    }
    for &id in &wall_vertices {
        let [theta, z] = chart[id];
        checked(
            (0. ..=FRAC_PI_2).contains(&theta)
                && z >= height_at(&stop_samples[0], theta) - eps
                && z <= height_at(&stop_samples[1], theta) + eps,
        )?;
        residual = residual.max(norm(sub(raw[id], g.point(theta, z))));
    }
    checked(residual <= o.cylinder_fit_tolerance_mm.min(1e-5))?;
    let mut area = Sum::default();
    let mut max_span = 0_f64;
    for (tri, &owner) in mesh.triangles.iter().zip(&part.owners) {
        if owner != g.id {
            continue;
        }
        let [a, b, c] = tri.map(|v| chart[v]);
        let signed = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        // The theta,z chart has outward normal on an exterior cylinder.
        checked(signed < 0.)?;
        area.add(-signed / 2.);
        let width = a[0].max(b[0]).max(c[0]) - a[0].min(b[0]).min(c[0]);
        max_span = max_span.max(width);
        let [p, q, r] = tri.map(|v| raw[v]);
        let theta = (a[0] + b[0] + c[0]) / 3.;
        checked(
            dot(
                cross(sub(q, p), sub(r, p)),
                add(scale(g.x, theta.cos()), scale(g.y, theta.sin())),
            ) < 0.,
        )?;
    }
    checked(max_span <= FRAC_PI_2 / 2. && (area.get() - cap_area).abs() <= cap_area * 1e-9)?;
    // Rescale each vertical interval affinely between its two stop curves.
    // The displacement is a convex combination of the two interpolation errors.
    let axial = (g.slope[0].hypot(g.slope[1]) * span[1].powi(2))
        .max(g.near_slope[0].hypot(g.near_slope[1]) * span[0].powi(2))
        / 8.;
    let near_radius = g
        .near_ellipse
        .as_ref()
        .map_or(g.circle.radius_mm, |e| e.major_mm);
    let chord = (g.circle.radius_mm * max_span.powi(2) / 8.)
        .hypot(axial)
        .max(g.ellipse.major_mm * span[1].powi(2) / 8.)
        .max(near_radius * span[0].powi(2) / 8.);
    Ok(ErrorBound { residual, chord })
}

#[cfg(test)]
mod tests {
    use super::minimum_quarter_offset;

    #[test]
    fn quarter_height_checks_endpoints_and_interior_minima() {
        for (coefficients, expected) in [
            ([3., 4.], 3.),
            ([-3., 4.], -3.),
            ([3., -4.], -4.),
            ([-3., -4.], -5.),
            ([0., 4.], 0.),
        ] {
            assert_eq!(minimum_quarter_offset(coefficients), expected);
        }
    }
}
