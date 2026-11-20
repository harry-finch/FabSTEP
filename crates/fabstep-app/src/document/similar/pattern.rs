//! Pattern labels describe already certified cylindrical walls. A grid never
//! supplies a missing candidate or changes the coordinates used by guidance.
use super::*;
use std::f64::consts::TAU;

type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn scale(v: V, s: f64) -> V {
    v.map(|x| x * s)
}
fn dot(a: V, b: V) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(v: V) -> f64 {
    dot(v, v).sqrt()
}
fn unit(v: V) -> Option<V> {
    let length = norm(v);
    (length > 0. && length.is_finite()).then(|| scale(v, 1. / length))
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Wall {
    center: V,
    axis: V,
    radius: f64,
    height: f64,
}

impl Wall {
    pub(super) fn from_owner(r: &RoundedRecognition, owner: RoundedFaceRef) -> Option<Self> {
        let mixed = &r.model.revolved.mixed;
        let (a, b, axis, radius) = match owner {
            RoundedFaceRef::Cylinder { index } => {
                let [a, b] = mixed.cylinders[index].rings.map(|i| &mixed.rings[i]);
                (a.center, b.center, a.axis, a.radius_mm)
            }
            RoundedFaceRef::PartialCylinder { index } => {
                let half = &mixed.partial_cylinders[index];
                if half.elliptic_start || half.elliptic_end {
                    return None;
                }
                let [a, b] = half.arcs.map(|i| &mixed.arcs[i]);
                (a.center, b.center, a.axis, a.radius_mm)
            }
            _ => return None,
        };
        Some(Self {
            center: add(a, scale(sub(b, a), 0.5)),
            axis,
            radius,
            height: norm(sub(b, a)),
        })
    }

    /// The null direction of the area-weighted normal Gram matrix is the
    /// generator of a triangulated cylinder, including an open arc. Rank the
    /// row pairs by conditioning and then check all original triangle vertices.
    pub(super) fn verified(self, mesh: &Mesh, region: &[usize], tolerance: f64) -> Option<Self> {
        let mut gram = [[0.; 3]; 3];
        let mut sum = 0.;
        for &id in region {
            let [a, b, c] = mesh.triangles[id].map(|i| mesh.vertices[i]);
            let area = cross(sub(b, a), sub(c, a));
            let weight = norm(area);
            let normal = unit(area)?;
            sum += weight;
            for i in 0..3 {
                for j in 0..3 {
                    gram[i][j] += weight * normal[i] * normal[j];
                }
            }
        }
        if sum <= 0. || !sum.is_finite() {
            return None;
        }
        let [a, b, c] = gram.map(|v| scale(v, 1. / sum));
        let best = [cross(a, b), cross(a, c), cross(b, c)]
            .into_iter()
            .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))?;
        // A single plane, or normals indistinguishable from a single plane,
        // cannot witness a unique axis.
        if norm(best) < 1e-8 {
            return None;
        }
        let axis = unit(best)?;
        if norm(cross(axis, self.axis)) * self.height.max(self.radius) > tolerance {
            return None;
        }
        for &id in region {
            for &v in &mesh.triangles[id] {
                let radial = norm(cross(sub(mesh.vertices[v], self.center), axis));
                if (radial - self.radius).abs() > tolerance {
                    return None;
                }
            }
        }
        Some(Self { axis, ..self })
    }
}

/// Return a label only if all the supplied measured walls lie on the same
/// lattice through the reference. An irregular arrangement remains selectable.
pub(super) fn classify(walls: &[Wall], tol: f64) -> Option<&'static str> {
    let anchor = *walls.first()?;
    if !(3..=16).contains(&walls.len()) {
        return None;
    }
    let mut points = Vec::with_capacity(walls.len());
    for wall in walls {
        let delta = sub(wall.center, anchor.center);
        if norm(cross(wall.axis, anchor.axis)) * wall.height.max(wall.radius) > tol
            || dot(delta, anchor.axis).abs() > tol
            || (wall.height - anchor.height).abs() > tol
        {
            return None;
        }
        points.push(delta);
    }
    let far = *points
        .iter()
        .max_by(|a, b| dot(**a, **a).total_cmp(&dot(**b, **b)))?;
    let x = unit(far)?;
    let y = unit(cross(anchor.axis, x))?;
    let mut coordinates: Vec<_> = points.iter().map(|p| dot(*p, x)).collect();
    if points
        .iter()
        .all(|p| norm(sub(*p, scale(x, dot(*p, x)))) <= tol)
    {
        coordinates.sort_by(f64::total_cmp);
        let step = coordinates
            .windows(2)
            .map(|ab| ab[1] - ab[0])
            .fold(f64::INFINITY, f64::min);
        if step <= 4. * tol {
            return None;
        }
        let origin = coordinates[0];
        let valid = coordinates.iter().all(|p| {
            let slot = ((p - origin) / step).round();
            slot <= 31. && ((p - origin) - slot * step).abs() <= tol
        });
        return valid.then_some("linear");
    }
    if walls.len() < 4 {
        return None;
    }
    let third = *points
        .iter()
        .max_by(|a, b| dot(**a, y).abs().total_cmp(&dot(**b, y).abs()))?;
    let altitude = dot(third, y);
    if altitude.abs() <= 4. * tol {
        return None;
    }
    let mid = norm(far) * 0.5;
    let center = add(
        scale(x, mid),
        scale(
            y,
            (dot(third, third) * 0.5 - mid * dot(third, x)) / altitude,
        ),
    );
    let radius = norm(center);
    let start = unit(scale(center, -1.))?;
    let side = unit(cross(anchor.axis, start))?;
    for count in walls.len()..=32 {
        if radius * (TAU / count as f64).sin() <= 4. * tol {
            continue;
        }
        let mut occupied = 0u32;
        let valid = points.iter().all(|p| {
            let q = sub(*p, center);
            let angle = dot(q, side).atan2(dot(q, start));
            let slot = ((angle * count as f64 / TAU).round() as i32).rem_euclid(count as i32);
            let mark = 1u32 << slot;
            if occupied & mark != 0 {
                return false;
            }
            occupied |= mark;
            let (s, c) = (TAU * slot as f64 / count as f64).sin_cos();
            norm(sub(
                q,
                add(scale(start, radius * c), scale(side, radius * s)),
            )) <= tol
        });
        if valid {
            return Some("circular");
        }
    }
    None
}

impl Search {
    pub(super) fn label_patterns(
        &mut self,
        anchor: Wall,
        reference: &GuidanceReport,
        body: &AnalyzedBody,
        mapping: &BodySourceTriangles,
        tolerance: f64,
        checkpoint: &mut impl FnMut() -> Result<(), FabError>,
    ) -> Result<(), FabError> {
        let ids = self
            .proposals
            .iter()
            .enumerate()
            .filter(|(_, p)| p.cylinder.is_some() && p.hint.analysis_body_id == body.id)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if ids.len() < 2 || self.truncated {
            return Ok(());
        }
        let mut walls = Vec::with_capacity(1 + ids.len());
        let sources = &mapping.analysis_to_source_triangles;
        let mut triangles = Vec::new();
        let candidates = ids.iter().map(|&i| {
            let p = &self.proposals[i];
            (
                p.cylinder.unwrap(),
                p.hint.selected_source_triangle_ids.as_slice(),
            )
        });
        let per_triangle = 2 + sources.len().max(1).ilog2() as usize + 1;
        // Optional discovery spends only the budget left by R-120. Reserve
        // the bounded (at most 32 slots per wall) grid verification first.
        let Some(mut budget) = self.work.checked_sub((ids.len() + 1) * 32) else {
            return Ok(());
        };
        for (wall, source_ids) in std::iter::once((
            anchor,
            reference.reconstructed_source_triangle_ids.as_slice(),
        ))
        .chain(candidates)
        {
            checkpoint()?;
            let Some(left) = budget.checked_sub(source_ids.len() * per_triangle) else {
                return Ok(());
            };
            budget = left;
            triangles.clear();
            for &source in source_ids {
                let Ok(i) = sources.binary_search(&source) else {
                    return Ok(());
                };
                triangles.push(i);
            }
            let Some(verified) = wall.verified(&body.analysis.mesh, &triangles, tolerance) else {
                return Ok(());
            };
            walls.push(verified);
        }
        if let Some(kind) = classify(&walls, tolerance) {
            for i in ids {
                self.proposals[i].display.pattern = Some(kind);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotated(p: V) -> V {
        // Proper rotation with no cardinal axis fixed.
        let [x, y, z] = p;
        [
            0.6 * x - 0.8 * y,
            0.48 * x + 0.36 * y - 0.8 * z,
            0.64 * x + 0.48 * y + 0.6 * z,
        ]
    }

    #[test]
    fn area_normals_recover_open_arc_axis_at_both_densities_and_after_rotation() {
        for n in [12, 41] {
            for turn in [false, true] {
                let transform = |p| if turn { rotated(p) } else { p };
                let wall = Wall {
                    center: transform([0., 0., 1.]),
                    axis: transform([0., 0., 1.]),
                    height: 2.,
                    radius: 3.,
                };
                let mut mesh = Mesh {
                    vertices: Vec::new(),
                    triangles: Vec::new(),
                };
                for i in 0..=n {
                    let angle = 1.4 * (i as f64 / n as f64).powf(1.3);
                    for z in [0., 2.] {
                        mesh.vertices
                            .push(transform([3. * angle.cos(), 3. * angle.sin(), z]));
                    }
                    if i != 0 {
                        let j = i * 2;
                        mesh.triangles
                            .extend([[j - 2, j, j + 1], [j - 2, j + 1, j - 1]]);
                    }
                }
                let region = (0..mesh.triangles.len()).collect::<Vec<_>>();
                let proved = wall.verified(&mesh, &region, 1e-8).unwrap();
                assert!(norm(cross(proved.axis, wall.axis)) < 1e-10);
                assert!(wall.verified(&mesh, &[0, 1], 1e-8).is_none());
                mesh.vertices[4][0] += 0.03;
                assert!(wall.verified(&mesh, &region, 1e-5).is_none());
            }
        }
    }

    #[test]
    fn insufficient_optional_budget_leaves_the_original_walls_selectable() {
        let source = include_bytes!("../../../../../fixtures/repeated_patterns/flange.stl");
        let (document, mappings) =
            analyze_document_with_provenance(source, InputFormat::Stl, &Options::default())
                .unwrap();
        let body = &document.bodies[0];
        let wall = Wall {
            center: [0.; 3],
            axis: [0., 0., 1.],
            height: 5.,
            radius: 2.,
        };
        let hint = GuidanceReport {
            hint_id: 0,
            family: "cylinder",
            preview_body_id: mappings[0].preview_body_id.clone(),
            analysis_body_id: body.id.clone(),
            selected_face_ids: vec![0],
            selected_source_triangle_ids: vec![0],
            reconstructed_source_triangle_ids: vec![0],
        };
        let mut search = Search {
            proposals: Vec::new(),
            capacity: 16,
            truncated: false,
            work: 0,
        };
        let support = Support {
            family: ResultFaceFamily::Cylinder,
            partial: false,
            inner: true,
            radius: 2.,
            other_radius: 2.,
            height: 5.,
            meridian: None,
        };
        for id in [1, 2] {
            search.record(support, body, &mappings[0], &[id], vec![id], Some(wall));
        }
        search
            .label_patterns(wall, &hint, body, &mappings[0], 1e-5, &mut || {
                panic!("budget checked before reading wall triangles")
            })
            .unwrap();
        assert_eq!(search.proposals.len(), 2);
        assert!(search.proposals.iter().all(|p| p.display.pattern.is_none()));
    }

    #[test]
    fn incomplete_and_irregular_rows_preserve_only_measured_grid_evidence() {
        let wall = |x| Wall {
            center: [x, 0., 0.],
            axis: [0., 0., 1.],
            radius: 2.,
            height: 5.,
        };
        assert_eq!(
            classify(&[wall(16.), wall(-16.), wall(0.), wall(48.)], 1e-5),
            Some("linear")
        );
        assert!(classify(&[wall(16.), wall(-16.), wall(0.), wall(48.01)], 1e-5).is_none());
        let mut tilted = wall(32.);
        tilted.axis = [0., 1., 0.];
        assert!(classify(&[wall(0.), wall(16.), tilted], 1e-5).is_none());
        assert!(classify(&[wall(0.), wall(0.), wall(16.)], 1e-5).is_none());
    }
}
