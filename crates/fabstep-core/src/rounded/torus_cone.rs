//! A terminal plane, one monotone toroidal arc, its tangent cone and a
//! cylindrical land. Every proposed change of surface is a measured circle.
use super::*;

mod groove;
mod terminal;

#[derive(Clone, Copy)]
struct Candidate {
    // Indices in the ORIGINAL axial order. These four indices are in profile
    // order starting from the plane that touches the torus.
    ends: [usize; 4],
    center_z: f64,
    major: f64,
    minor: f64,
    angles: [f64; 2],
    tangent_z: f64,
    tangent_radius: f64,
    land_radius: f64,
}

fn search(nodes: &[Node], o: &Options, work: &mut Work) -> Result<Option<Candidate>, FabError> {
    let last = nodes.len() - 1;
    let mut unique = None;
    let fit = o.cylinder_fit_tolerance_mm;
    let epsilon = fit.max(o.plane_tolerance_mm).max(o.weld_tolerance_mm);
    for reverse in [false, true] {
        let index = |i: usize| if reverse { last - i } else { i };
        let point = |i| &nodes[index(i)];
        let direction = if reverse { -1. } else { 1. };
        let plane = point(0);
        let end = point(last);
        // Enumerate only suffixes that can describe a single cylinder. A
        // suffix outside a 2*fit interval cannot become admissible by extension.
        let mut minimum = end.radius;
        let mut maximum = end.radius;
        for land in (4..last).rev() {
            work.spend(1)?;
            let k = point(land);
            minimum = minimum.min(k.radius);
            maximum = maximum.max(k.radius);
            if maximum - minimum > 2. * fit {
                break;
            }
            if (end.z - k.z).abs() <= 8. * epsilon {
                continue;
            }
            let land_radius = (minimum + maximum) * 0.5;
            for tangent in 3..land {
                work.spend(1)?;
                let j = point(tangent);
                let h_land = direction * (k.z - plane.z);
                let cone_height = direction * (k.z - j.z);
                let change = land_radius - j.radius;
                if cone_height <= 8. * epsilon || change.abs() <= 8. * epsilon {
                    continue;
                }
                let slope = change / cone_height;
                let branch = slope.signum();
                let normal = slope.hypot(1.);
                // In directed height h the cone line is rho=b+slope*h. The
                // circle center is (R,r) in (rho,h). Its signed distance to
                // this line is r. The terminal plane fixes R and h=0 exactly.
                let denominator = branch / (normal + slope.abs());
                let minor = (land_radius - plane.radius - slope * h_land) / denominator;
                if !minor.is_finite()
                    || minor <= 4. * epsilon
                    || plane.radius - minor <= 4. * epsilon
                {
                    continue;
                }
                let tangent_radius = plane.radius + branch * minor / normal;
                let tangent_height = minor / (normal * (normal + slope.abs()));
                let tangent_z = plane.z + direction * tangent_height;
                if (tangent_radius - j.radius).hypot(tangent_z - j.z) > fit {
                    continue;
                }
                let center_z = plane.z + direction * minor;
                let plane_angle = -direction * PI / 2.;
                let join_angle = (-direction * slope.abs()).atan2(branch);
                let angles = [plane_angle.min(join_angle), plane_angle.max(join_angle)];
                if !(0. ..PI / 2.).contains(&(angles[1] - angles[0])) {
                    continue;
                }

                // Ring means screen the decomposition cheaply. Final proof
                // subsequently projects every vertex and bounds every triangle.
                work.spend(land + 1)?;
                let arc_height = direction * (j.z - plane.z);
                let mut interior_witnesses = 0;
                let curved = (1..tangent).all(|i| {
                    let n = point(i);
                    let h = direction * (n.z - plane.z);
                    let dr = n.radius - plane.radius;
                    let dz = n.z - center_z;
                    let chord_distance = (arc_height * dr - (j.radius - plane.radius) * h).abs()
                        / arc_height.hypot(j.radius - plane.radius);
                    if chord_distance > 4. * epsilon {
                        interior_witnesses += 1;
                    }
                    branch * dr >= -fit
                        && direction * dz <= fit
                        && (dr.hypot(dz) - minor).abs() <= fit
                });
                if !curved
                    || interior_witnesses < 2
                    || !(tangent + 1..land).all(|i| {
                        let n = point(i);
                        (n.radius - land_radius - slope * direction * (n.z - k.z)).abs() <= fit
                    })
                {
                    continue;
                }
                if unique.is_some() {
                    return Err(unproven(
                        "plusieurs décompositions tore–cône–cylindre admissibles",
                    ));
                }
                unique = Some(Candidate {
                    ends: [0, tangent, land, last].map(index),
                    center_z,
                    major: plane.radius,
                    minor,
                    angles,
                    tangent_z,
                    tangent_radius,
                    land_radius,
                });
            }
        }
    }
    Ok(unique)
}

// The original ring indices are kept until every triangle has exactly one owner.
// Each canonicalized boundary is shared by both of its incident surfaces.
struct Layout {
    sections: Vec<([usize; 2], Shape)>,
    boundaries: Vec<(usize, f64, f64)>,
}

impl Candidate {
    fn layout(self, nodes: &[Node]) -> Layout {
        let [p, j, k, e] = self.ends;
        Layout {
            sections: vec![
                (
                    [p, j],
                    Shape::Torus {
                        center_z: self.center_z,
                        major: self.major,
                        minor: self.minor,
                        v: self.angles,
                    },
                ),
                ([j, k], Shape::Cone),
                ([k, e], Shape::Cylinder),
            ],
            boundaries: vec![
                (j, self.tangent_z, self.tangent_radius),
                (k, nodes[k].z, self.land_radius),
                (e, nodes[e].z, self.land_radius),
            ],
        }
    }
}

pub(super) fn classify(
    a: &Analysis,
    triangles: &[usize],
    nodes: &mut Vec<Node>,
    rims: &[Rim],
    projected_count: usize,
    o: &Options,
    work: &mut Work,
) -> Result<Option<Vec<Section>>, FabError> {
    if nodes.len() < 5 {
        return Ok(None);
    }
    // Both terminals must be the original planar rims, even if there are
    // additional measured horizontal circles outside their axial interval.
    let last = nodes.len() - 1;
    for i in [0, last] {
        work.spend(nodes[i].vertices.len().saturating_mul(rims.len()))?;
        if !rims.iter().any(|r| nodes[i].vertices == r.vertices) {
            return Ok(None);
        }
    }
    let layout = if let Some(candidate) = search(nodes, o, work)? {
        candidate.layout(nodes)
    } else if let Some(layout) = terminal::search(nodes, o, work)? {
        layout
    } else if let Some(layout) = groove::search(nodes, o, work)? {
        layout
    } else {
        return Ok(None);
    };

    // Every vertex belongs to a complete source circle and each triangle
    // connects consecutive circles. This proves both cuts are genuine and
    // partitions the input exactly once even for two-ring straight portions.
    let count = nodes.iter().map(|n| n.vertices.len()).sum::<usize>();
    work.spend(count + triangles.len().saturating_mul(3))?;
    let mut levels = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        for &v in &n.vertices {
            if levels.insert(v, i).is_some() {
                return Err(unproven("sommet commun à plusieurs niveaux méridiens"));
            }
        }
    }
    if levels.len() != projected_count {
        return Err(unproven(
            "sommet hors des anneaux source de la chaîne torique",
        ));
    }
    let mut sections: Vec<_> = layout
        .sections
        .into_iter()
        .map(|(mut ends, shape)| {
            ends.sort_unstable();
            Section {
                ends,
                shape,
                triangles: Vec::new(),
            }
        })
        .collect();
    sections.sort_by_key(|s| s.ends[0]);
    for &t in triangles {
        let mut rings = a.mesh.triangles[t].map(|v| levels[&v]);
        rings.sort_unstable();
        if rings[2] != rings[0] + 1 {
            return Err(unproven(
                "triangle sans bande entre deux anneaux source consécutifs",
            ));
        }
        let s = sections
            .iter_mut()
            .find(|s| rings[0] >= s.ends[0] && rings[2] <= s.ends[1])
            .ok_or_else(|| unproven("triangle traversant une frontière de raccord"))?;
        s.triangles.push(t);
    }
    for (i, z, radius) in layout.boundaries {
        nodes[i].z = z;
        nodes[i].radius = radius;
    }

    // Removal of intermediate circles occurs only after checking their
    // incidence. The sections remain in increasing axial order.
    let mut used: Vec<_> = sections.iter().map(|s| s.ends[0]).collect();
    used.push(sections.last().unwrap().ends[1]);
    let original = std::mem::take(nodes);
    nodes.extend(
        original
            .into_iter()
            .enumerate()
            .filter_map(|(i, n)| used.contains(&i).then_some(n)),
    );
    for (index, section) in sections.iter_mut().enumerate() {
        section.ends = [index, index + 1];
    }
    Ok(Some(sections))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(extra_circle: bool) -> Vec<Node> {
        let node = |z, radius| Node {
            z,
            radius,
            vertices: vec![],
        };
        let sweep = PI / 2. - (1_f64 / 15.).atan();
        let mut nodes: Vec<_> = (0..=12)
            .map(|i| {
                let angle = sweep * f64::from(i) / 12.;
                node(0.5 * (1. - angle.cos()), 10. - 0.5 * angle.sin())
            })
            .collect();
        let join = nodes.last().unwrap();
        let z = join.z + 2.;
        let radius = join.radius - 2. / 15.;
        if extra_circle {
            // The last cone interval changes radius by 6.67e-6 mm. Its
            // boundaries are resolved in z but BOTH can begin the cylinder.
            nodes.push(node(z - 1e-4, radius + 1e-4 / 15.));
        }
        nodes.push(node(z, radius));
        nodes.push(node(5., radius));
        nodes
    }

    #[test]
    fn two_measured_starts_of_the_land_are_ambiguous() {
        let options = Options::default();
        assert!(search(&profile(false), &options, &mut Work::default())
            .unwrap()
            .is_some());
        let error = search(&profile(true), &options, &mut Work::default())
            .err()
            .unwrap();
        assert!(error.message.contains("plusieurs décompositions"));
    }

    #[test]
    fn exhausting_work_after_the_first_fit_does_not_establish_uniqueness() {
        let nodes = profile(false);
        let mut full = Work::default();
        assert!(search(&nodes, &Options::default(), &mut full)
            .unwrap()
            .is_some());
        let mut limited = Work::default();
        // Leave one charge missing at the end, after the nominal fit.
        limited
            .spend(10_000_000 - full.used_for_test() + 1)
            .unwrap();
        assert_eq!(
            search(&nodes, &Options::default(), &mut limited)
                .err()
                .unwrap()
                .code,
            "recognition.too_complex"
        );
    }
}
