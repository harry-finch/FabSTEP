//! Full-turn profiles with a terminal plane tangent to a quarter-circle fillet
//! and two lands, or to an obtuse fillet followed directly by its tangent cone.
use super::*;

struct Profile<'a> {
    nodes: &'a [Node],
    reverse: bool,
    fit: f64,
    epsilon: f64,
}

impl Profile<'_> {
    fn last(&self) -> usize {
        self.nodes.len() - 1
    }

    fn index(&self, i: usize) -> usize {
        if self.reverse {
            self.last() - i
        } else {
            i
        }
    }

    fn node(&self, i: usize) -> &Node {
        &self.nodes[self.index(i)]
    }

    fn direction(&self) -> f64 {
        if self.reverse {
            -1.
        } else {
            1.
        }
    }

    fn height(&self, i: usize) -> f64 {
        self.direction() * (self.node(i).z - self.node(0).z)
    }

    fn line_fits(
        &self,
        i: usize,
        j: usize,
        r0: f64,
        r1: f64,
        work: &mut Work,
    ) -> Result<bool, FabError> {
        work.spend(j - i + 1)?;
        let h0 = self.height(i);
        let slope = (r1 - r0) / (self.height(j) - h0);
        Ok((i..=j)
            .all(|k| (self.node(k).radius - r0 - slope * (self.height(k) - h0)).abs() <= self.fit))
    }
}

#[derive(Clone, Copy)]
struct Arc {
    branch: f64,
    minor: f64,
    sweep: f64,
}

impl Arc {
    fn end(&self, p: &Profile<'_>) -> (f64, f64) {
        (
            self.minor * (1. - self.sweep.cos()),
            p.node(0).radius + self.branch * self.minor * self.sweep.sin(),
        )
    }

    fn shape(&self, p: &Profile<'_>) -> Shape {
        let direction = p.direction();
        let plane_angle = -direction * PI / 2.;
        // Keep the reflected sweep continuous even when it crosses ±π.
        let join_angle = plane_angle + direction * self.branch * self.sweep;
        Shape::Torus {
            center_z: p.node(0).z + direction * self.minor,
            major: p.node(0).radius,
            minor: self.minor,
            v: [plane_angle.min(join_angle), plane_angle.max(join_angle)],
        }
    }

    fn fits(&self, p: &Profile<'_>, join: usize, work: &mut Work) -> Result<bool, FabError> {
        let major = p.node(0).radius;
        let (height, radius) = self.end(p);
        if !self.minor.is_finite()
            || self.minor <= 4. * p.epsilon
            || major - self.minor <= 4. * p.epsilon
            || (p.height(join) - height).hypot(p.node(join).radius - radius) > p.fit
        {
            return Ok(false);
        }
        work.spend(join)?;
        let mut curved = 0;
        let mut previous = 0.;
        let mut max_radial_offset = 0_f64;
        let chord = height.hypot(radius - major);
        for i in 1..join {
            let dr = p.node(i).radius - major;
            let h = p.height(i);
            let radial_offset = self.branch * dr;
            let angle = radial_offset.atan2(self.minor - h);
            if angle <= previous
                || angle >= self.sweep
                || (dr.hypot(h - self.minor) - self.minor).abs() > p.fit
            {
                return Ok(false);
            }
            previous = angle;
            max_radial_offset = max_radial_offset.max(radial_offset);
            if (height * dr - (radius - major) * h).abs() > 4. * p.epsilon * chord {
                curved += 1;
            }
        }
        // An obtuse fillet actually turns back in radius. Require a measured
        // inner circle past the endpoint radius by more than uncertainty.
        let resolved_turn = self.sweep <= PI / 2.
            || max_radial_offset - self.branch * (radius - major) > 4. * p.epsilon;
        Ok(curved >= 2 && resolved_turn)
    }
}

fn record(unique: &mut Option<Layout>, layout: Layout) -> Result<(), FabError> {
    if unique.replace(layout).is_some() {
        return Err(unproven(
            "plusieurs découpages du profil torique terminal admissibles",
        ));
    }
    Ok(())
}

fn obtuse_cone(
    p: &Profile<'_>,
    join: usize,
    unique: &mut Option<Layout>,
    work: &mut Work,
) -> Result<(), FabError> {
    let end = p.last();
    let dz = p.height(end) - p.height(join);
    let dr = p.node(end).radius - p.node(join).radius;
    if dz <= 8. * p.epsilon || dr.abs() <= 8. * p.epsilon {
        return Ok(());
    }
    let slope = dr / dz;
    let branch = -slope.signum();
    let normal = slope.hypot(1.);
    // In directed height h from the plane, the tangent point at sweep φ is
    // (h,ρ)=(q*(1-cos φ), R+b*q*sin φ). With slope b*cot φ, the cone line
    // is ρ=R+m*h+q*(b*hypot(1,m)-m). The far source ring fixes q.
    let minor =
        (p.node(end).radius - p.node(0).radius - slope * p.height(end)) / (branch * normal - slope);
    let arc = Arc {
        branch,
        minor,
        sweep: 1_f64.atan2(branch * slope),
    };
    if arc.sweep <= PI / 2.
        || arc.sweep >= PI
        || !arc.fits(p, join, work)?
        || !p.line_fits(join, end, p.node(join).radius, p.node(end).radius, work)?
    {
        return Ok(());
    }
    let (height, radius) = arc.end(p);
    let j = p.index(join);
    record(
        unique,
        Layout {
            sections: vec![
                ([p.index(0), j], arc.shape(p)),
                ([j, p.index(end)], Shape::Cone),
            ],
            boundaries: vec![(j, p.node(0).z + p.direction() * height, radius)],
        },
    )
}

#[derive(Clone, Copy)]
struct Land {
    min: f64,
    max: f64,
}

impl Land {
    fn new(radius: f64) -> Self {
        Self {
            min: radius,
            max: radius,
        }
    }

    fn include(&mut self, radius: f64, fit: f64) -> Option<f64> {
        self.min = self.min.min(radius);
        self.max = self.max.max(radius);
        (self.max - self.min <= 2. * fit).then_some((self.min + self.max) * 0.5)
    }
}

fn quarter_lands(
    p: &Profile<'_>,
    join: usize,
    unique: &mut Option<Layout>,
    work: &mut Work,
) -> Result<(), FabError> {
    let end = p.last();
    if end - join < 4 {
        return Ok(());
    }
    let major = p.node(0).radius;
    let offset = p.node(join).radius - major;
    // A possible plane-to-land quarter obeys h=|ρ-R|. Its measured
    // endpoint can move by at most fit in the vector proof.
    if (offset.abs() - p.height(join)).abs() > 2. * p.fit {
        return Ok(());
    }
    let mut first_land = Land::new(p.node(join).radius);
    for a in join + 1..=end - 3 {
        work.spend(1)?;
        let Some(r1) = first_land.include(p.node(a).radius, p.fit) else {
            break;
        };
        let arc = Arc {
            branch: offset.signum(),
            minor: (r1 - major).abs(),
            sweep: PI / 2.,
        };
        let land_height = p.height(a) - arc.minor;
        if land_height <= 8. * p.epsilon || !arc.fits(p, join, work)? {
            continue;
        }
        for b in a + 1..=end - 2 {
            work.spend(1)?;
            let mut second_land = Land::new(p.node(b).radius);
            for c in b + 1..end {
                work.spend(1)?;
                let Some(r2) = second_land.include(p.node(c).radius, p.fit) else {
                    break;
                };
                let e = p.epsilon;
                let profile = [
                    [p.height(a), r1],
                    [p.height(b), r2],
                    [p.height(c), r2],
                    [p.height(end), p.node(end).radius],
                ];
                // Resolve the first cone against BOTH lands. The existing
                // opposed-chamfer guard proves its central land and the
                // other cone with the same endpoint-uncertainty margins.
                if ((r2 - r1).abs() - 2. * e) * (land_height - 2. * e)
                    <= 8. * e * (p.height(b) - p.height(a) + 2. * e)
                    || !crate::revolved::resolved_double_chamfer(profile, e)
                    || !p.line_fits(a, b, r1, r2, work)?
                    || !p.line_fits(c, end, r2, p.node(end).radius, work)?
                {
                    continue;
                }
                let [plane, j, a_id, b_id, c_id, last] =
                    [0, join, a, b, c, end].map(|i| p.index(i));
                record(
                    unique,
                    Layout {
                        sections: vec![
                            ([plane, j], arc.shape(p)),
                            ([j, a_id], Shape::Cylinder),
                            ([a_id, b_id], Shape::Cone),
                            ([b_id, c_id], Shape::Cylinder),
                            ([c_id, last], Shape::Cone),
                        ],
                        boundaries: vec![
                            (j, p.node(0).z + p.direction() * arc.minor, r1),
                            (a_id, p.node(a).z, r1),
                            (b_id, p.node(b).z, r2),
                            (c_id, p.node(c).z, r2),
                        ],
                    },
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn search(
    nodes: &[Node],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Layout>, FabError> {
    let mut unique = None;
    for reverse in [false, true] {
        let p = Profile {
            nodes,
            reverse,
            fit: o.cylinder_fit_tolerance_mm,
            epsilon: o
                .cylinder_fit_tolerance_mm
                .max(o.plane_tolerance_mm)
                .max(o.weld_tolerance_mm),
        };
        for join in 3..p.last() {
            work.spend(1)?;
            obtuse_cone(&p, join, &mut unique, work)?;
            quarter_lands(&p, join, &mut unique, work)?;
        }
    }
    Ok(unique)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(z: f64, radius: f64) -> Node {
        Node {
            z,
            radius,
            vertices: vec![],
        }
    }

    fn arc(branch: f64, sweep: f64) -> Vec<Node> {
        (0..=20)
            .map(|i| {
                let phi = sweep * f64::from(i) / 20.;
                node(2. * (1. - phi.cos()), 10. + branch * 2. * phi.sin())
            })
            .collect()
    }

    fn lands(ambiguous: bool) -> Vec<Node> {
        let mut rings = arc(1., PI / 2.);
        rings.push(node(5., 12.));
        if ambiguous {
            // Either this circle or the one just below can start the cone
            // within tolerance; even though the main radius change is clear.
            rings.push(node(5.0001, 12.00001));
        }
        rings.extend([node(9., 12.4), node(12., 12.4), node(16., 11.)]);
        rings
    }

    #[test]
    fn quarter_profile_requires_a_unique_resolved_pair_of_opposed_cones() {
        let o = Options::default();
        let valid = lands(false);
        let layout = search(&valid, &o, &mut Work::default()).unwrap().unwrap();
        assert_eq!(layout.sections.len(), 5);
        let error = search(&lands(true), &o, &mut Work::default())
            .err()
            .unwrap();
        assert!(error.message.contains("plusieurs découpages"));
        let mut same_sign = valid.clone();
        same_sign.last_mut().unwrap().radius = 13.;
        assert!(search(&same_sign, &o, &mut Work::default())
            .unwrap()
            .is_none());
        let mut unresolved = valid;
        unresolved[22].radius = 12.00001;
        unresolved[23].radius = 12.00001;
        assert!(search(&unresolved, &o, &mut Work::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn obtuse_profile_must_demonstrate_a_radial_turn_and_finish_its_search() {
        let o = Options::default();
        let sweep = 100_f64.to_radians();
        let mut rings = arc(-1., sweep);
        let last = rings.last().unwrap();
        rings.push(node(last.z + 4., last.radius - 4. / sweep.tan()));
        let mut work = Work::default();
        let layout = search(&rings, &o, &mut work).unwrap().unwrap();
        let Shape::Torus { v, .. } = layout.sections[0].1 else {
            panic!()
        };
        assert!(v[0] < -PI && v[1] > -PI);
        assert!((v[1] - v[0] - sweep).abs() < 1e-10);

        let mut limited = Work::default();
        limited
            .spend(10_000_000 - work.used_for_test() + 1)
            .unwrap();
        let error = search(&rings, &o, &mut limited).err().unwrap();
        assert_eq!(error.code, "recognition.too_complex");

        // The same exact outer circles, but remove every sample of the
        // radial overshoot. Two other curved interior witnesses remain.
        rings.drain(16..20);
        assert!(search(&rings, &o, &mut Work::default()).unwrap().is_none());
    }
}
