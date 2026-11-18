use super::*;

pub(super) struct Geometry {
    pub circle_plane: usize,
    pub ellipse_plane: usize,
    pub circle: CircularArc,
    pub ellipse: EllipticArc,
    rails: [usize; 2],
    x: V,
    y: V,
    h: f64,
    slope: [f64; 2],
}
impl Geometry {
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
                return Some(self.point(theta, 0.));
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
    let mut rails = [rails[0], rails[1]];
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
    caps.sort_by(|&a, &b| {
        dot(axis, p.planes[b].support.normal)
            .abs()
            .total_cmp(&dot(axis, p.planes[a].support.normal).abs())
    });
    let (circle_plane, ellipse_plane) = (caps[0], caps[1]);
    let near = p.planes[circle_plane].support;
    let far = p.planes[ellipse_plane].support;
    checked(dot(axis, near.normal).abs() > 1. - 1e-10)?;
    let near_chain = &p.cuts[&(circle_plane, WALL)];
    let far_chain = &p.cuts[&(ellipse_plane, WALL)];
    checked(near_chain.len() >= 7 && far_chain.len() >= 7)?;
    let near_mid = raw[near_chain[near_chain.len() / 2]];
    let far_mid = raw[far_chain[far_chain.len() / 2]];
    if dot(axis, sub(far_mid, near_mid)) < 0. {
        axis = scale(axis, -1.);
        rails.swap(0, 1);
    }
    // Choose the order of the rails so that x × y points from the circle
    // to the ellipse. Both are inward normals of the cylindrical cavity.
    checked(dot(near.normal, axis) > 1. - 1e-10 && dot(far.normal, axis) < -0.1)?;
    let x = scale(p.planes[rails[0]].support.normal, -1.);
    let y = cross(axis, x);
    p.planes[rails[1]].support.normal = scale(y, -1.);
    p.planes[circle_plane].support.normal = scale(axis, dot(axis, near.normal).signum());
    let cap_ends = |cap: &[usize]| -> Result<[usize; 2], FabError> {
        let mut ids = [0; 2];
        for (k, &i) in rails.iter().enumerate() {
            let cut = &p.cuts[&(i, WALL)];
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
    let base = intersection(
        [supports[0], supports[1], p.planes[circle_plane].support],
        near_mid,
    )
    .ok_or_else(unsupported)?;
    let radius = (supports[1].distance(raw[low[0]])
        + supports[0].distance(raw[low[1]])
        + supports[1].distance(raw[high[0]])
        + supports[0].distance(raw[high[1]]))
        / 4.;
    checked(radius.is_finite() && radius > 8. * tol)?;
    let center = add(base, scale(add(x, y), -radius));
    let normal = scale(far.normal, dot(far.normal, axis).signum());
    let cosine = dot(normal, axis);
    checked((0.1..0.99999).contains(&cosine))?;
    let h = dot(sub(far.origin, center), normal) / cosine;
    let slope = [
        -radius * dot(normal, x) / cosine,
        -radius * dot(normal, y) / cosine,
    ];
    checked(h > slope[0].hypot(slope[1]) + tol)?;
    let circle = CircularArc {
        center,
        axis,
        radius_mm: radius,
        sweep_rad: FRAC_PI_2,
        vertices: low,
    };
    let ellipse_center = add(center, scale(axis, h));
    let major_direction = unit(sub(axis, scale(normal, cosine))).ok_or_else(unsupported)?;
    let minor_direction = cross(normal, major_direction);
    let first_radius = add(scale(x, radius), scale(axis, slope[0]));
    let major_mm = radius / cosine;
    let start_rad = (dot(first_radius, minor_direction) / radius)
        .atan2(dot(first_radius, major_direction) / major_mm);
    let ellipse = EllipticArc {
        center: ellipse_center,
        axis: normal,
        major_direction,
        major_mm,
        minor_mm: radius,
        start_rad,
        sweep_rad: FRAC_PI_2,
        vertices: high,
    };
    Ok(Geometry {
        circle_plane,
        ellipse_plane,
        circle,
        ellipse,
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
        if owner == WALL {
            wall_vertices.extend(tri.iter().copied());
        }
    }
    for &id in &wall_vertices {
        let d = sub(raw[id], g.circle.center);
        chart[id] = [dot(d, g.y).atan2(dot(d, g.x)), dot(d, g.circle.axis)];
    }
    for (&rail, angle) in g.rails.iter().zip([0., FRAC_PI_2]) {
        for &id in &part.cuts[&(rail, WALL)] {
            chart[id][0] = angle;
        }
    }
    let mut stop_samples = Vec::new();
    for (&cap, is_ellipse) in [&g.circle_plane, &g.ellipse_plane]
        .into_iter()
        .zip([false, true])
    {
        let path = &part.cuts[&(cap, WALL)];
        let mut samples = Vec::with_capacity(path.len());
        for &id in path {
            let theta = chart[id][0];
            let z = if is_ellipse { g.height(theta) } else { 0. };
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
        if is_ellipse {
            stop_samples = samples;
        }
    }
    for &rail in &g.rails {
        let path = &part.cuts[&(rail, WALL)];
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
    let mut span = 0_f64;
    for w in stop_samples.windows(2) {
        let interval = w[1][0] - w[0][0];
        cap_area += interval * (w[1][1] + w[0][1]) / 2.;
        span = span.max(interval);
    }
    let eps = 128. * f64::EPSILON * (g.h + g.circle.radius_mm).max(1.);
    for &id in &wall_vertices {
        let [theta, z] = chart[id];
        checked((0. ..=FRAC_PI_2).contains(&theta) && z >= -eps)?;
        let index = stop_samples
            .partition_point(|p| p[0] < theta)
            .clamp(1, stop_samples.len() - 1);
        let [l, r] = [stop_samples[index - 1], stop_samples[index]];
        let ceiling = l[1] + (r[1] - l[1]) * (theta - l[0]) / (r[0] - l[0]);
        checked(z <= ceiling + eps)?;
        residual = residual.max(norm(sub(raw[id], g.point(theta, z))));
    }
    checked(residual <= o.cylinder_fit_tolerance_mm.min(1e-5))?;
    let mut area = Sum::default();
    let mut max_span = 0_f64;
    for (tri, &owner) in mesh.triangles.iter().zip(&part.owners) {
        if owner != WALL {
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
    // P is the piecewise-linear stop height through the cap witnesses.
    // The map z -> z*H(theta)/P(theta) is onto the exact strip and moves
    // axially by at most ||H-P||. The independent radial chord bound comes
    // from Taylor interpolation of the unit circle on each chart triangle.
    let axial = g.slope[0].hypot(g.slope[1]) * span.powi(2) / 8.;
    let chord = (g.circle.radius_mm * max_span.powi(2) / 8.)
        .hypot(axial)
        .max(g.ellipse.major_mm * span.powi(2) / 8.);
    Ok(ErrorBound { residual, chord })
}
