//! Nominate a narrow cylindrical strip from the turning normals of the
//! faceted wall at its two ends. The historical full patch proof still owns
//! the trim, rails, projection coverage and continuous deviation bound.
use super::*;
use crate::MixedPlane;

const MAX_SOURCE_TRIANGLES: usize = 8_192;
const MAX_LOOP: usize = 256;

fn end_loop(plane: &MixedPlane) -> Option<&[usize]> {
    match plane.loops.as_slice() {
        [MixedLoop::Polygon { vertices }]
            if (MIN_POINTS + 1..=MAX_LOOP).contains(&vertices.len()) =>
        {
            Some(vertices)
        }
        _ => None,
    }
}

/// Initial domain: a single component with two parallel polygonal ends,
/// no existing analytic patches and a bounded number of source triangles.
pub(super) fn ends(model: &MixedModel, count: usize, work: &mut Work) -> Option<[usize; 2]> {
    if count > MAX_SOURCE_TRIANGLES || !model.rings.is_empty() || !model.arcs.is_empty() {
        return None;
    }
    let mut candidates = model
        .planes
        .iter()
        .enumerate()
        .filter(|(_, p)| end_loop(p).is_some());
    let (a, p) = candidates.next()?;
    let (b, q) = candidates.next()?;
    if candidates.next().is_some() || dot(p.normal, q.normal) >= -1. + 1e-10 {
        return None;
    }
    // Reserve the adjacency, source normals, bounded cyclic scans and at
    // most four fits per run before allocating any optional search tables.
    let vertices = end_loop(p)?.len() + end_loop(q)?.len();
    work.reserve_optional(count * 16 + vertices * 32)
        .then_some([a, b])
}

pub(super) fn nominate(
    ends: [usize; 2],
    mesh: &crate::Mesh,
    model: &MixedModel,
    owners: &[Option<MixedFaceRef>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for plane in ends {
        let face = &model.planes[plane];
        let Some(vertices) = end_loop(face) else {
            return vec![];
        };

        let frame = Frame::new(mesh.vertices[vertices[0]], face.normal);
        let points: Vec<_> = vertices
            .iter()
            .map(|&v| frame.project(mesh.vertices[v]).0)
            .collect();
        // A sparsely sampled complete circle must never become a prettier
        // partial circle by dropping the segment with the largest turn.
        if constrained_circle(&points, o.cylinder_fit_tolerance_mm).is_some() {
            return vec![];
        }
        let mut normals = Vec::with_capacity(vertices.len());
        for i in 0..vertices.len() {
            let e = edge(vertices[i], vertices[(i + 1) % vertices.len()]);
            let normal = edges.get(&e).and_then(|neighbors| {
                let &t = neighbors.iter().find(|&&t| !is_plane(&owners[t], plane))?;
                if !matches!(owners[t], Some(MixedFaceRef::Plane { .. })) {
                    return None;
                }
                let [a, b, c] = mesh.triangles[t].map(|v| mesh.vertices[v]);
                let raw = cross(sub(b, a), sub(c, a));
                let n = scale(raw, 1. / norm(raw));
                (dot(n, frame.axis).abs() < 1e-5).then_some(n)
            });
            normals.push(normal);
        }
        let n = normals.len();
        let smooth = |i: usize| {
            normals[i]
                .zip(normals[(i + 1) % n])
                .is_some_and(|(a, b)| dot(a, b) > 8f64.to_radians().cos())
        };
        let Some(stop) = (0..n).find(|&i| !smooth(i)) else {
            return vec![];
        };
        let first = (stop + 1) % n;
        let mut offset = 0;
        while offset < n {
            let start = offset;
            offset += 1;
            while offset < n && smooth((first + offset - 1) % n) {
                offset += 1;
            }
            if offset - start + 1 < MIN_POINTS {
                continue;
            }
            let indices: Vec<_> = (start..=offset).map(|j| (first + j) % n).collect();
            if let Some((chain, f)) = ring_arc(
                &indices,
                &points,
                &normals,
                &frame,
                o.cylinder_fit_tolerance_mm,
            ) {
                candidates.push(Candidate {
                    plane,
                    loop_index: 0,
                    chain: chain.iter().map(|&i| vertices[i]).collect(),
                    center: frame.lift(f.center),
                    normal: frame.axis,
                    radius: f.radius,
                });
            }
        }
    }
    if candidates.len() == 2 && candidates[0].plane != candidates[1].plane {
        candidates
    } else {
        vec![]
    }
}

/// A long flat adjoining a real fillet can be tangent enough to belong to
/// its smooth-normal run. Its far endpoint must be clearly off the support
/// and its normal at the junction must meet the axis. Interior samples and
/// ambiguous competing endpoint assignments are never removed.
fn ring_arc<'a>(
    indices: &'a [usize],
    points: &[P2],
    normals: &[Option<V>],
    frame: &Frame,
    tol: f64,
) -> Option<(&'a [usize], Fit)> {
    let mut accepted = None;
    for (left, right) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let chain = &indices[left..indices.len() - right];
        if chain.len() < MIN_POINTS {
            continue;
        }
        let samples: Vec<_> = chain.iter().map(|&i| points[i]).collect();
        let Some(fit) = fit(&samples, tol) else {
            continue;
        };
        if !(15f64.to_radians()..30f64.to_radians()).contains(&fit.sweep.abs())
            || 2. * fit.radius * (fit.sweep / 4.).sin().powi(2) <= 256. * tol
        {
            continue;
        }
        let ray = |i: usize, j: usize, sample: P2| {
            let Some(normal) = normals[i] else {
                return false;
            };
            let nxy = [dot(normal, frame.x), dot(normal, frame.y)];
            let d = [sample[0] - fit.center[0], sample[1] - fit.center[1]];
            let length = (points[i][0] - points[j][0]).hypot(points[i][1] - points[j][1]);
            // Directional uncertainty of a chord is proportional to the
            // absolute coordinate error divided by its length.
            length > 0.
                && (d[0] * nxy[1] - d[1] * nxy[0]).abs() <= tol * (1. + 2. * fit.radius / length)
        };
        if !chain.windows(2).all(|w| {
            let (i, j) = (w[0], w[1]);
            ray(
                i,
                j,
                [
                    (points[i][0] + points[j][0]) * 0.5,
                    (points[i][1] + points[j][1]) * 0.5,
                ],
            )
        }) {
            continue;
        }
        let first = indices[0];
        let last = indices[indices.len() - 1];
        let off = |i: usize| {
            ((points[i][0] - fit.center[0]).hypot(points[i][1] - fit.center[1]) - fit.radius).abs()
                > 8. * tol
        };
        if left != 0 && (!off(first) || !ray(first, chain[0], points[chain[0]])) {
            continue;
        }
        let end = chain[chain.len() - 1];
        if right != 0 && (!off(last) || !ray(end, last, points[end])) {
            continue;
        }
        if accepted.is_some() {
            return None;
        }
        accepted = Some((chain, fit));
    }
    accepted
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_declines_before_allocating_when_work_is_exhausted() {
        let a = crate::analyze_stl(
            include_bytes!("../../../../fixtures/r117-narrow-cylinder.stl"),
            &Options::default(),
        )
        .unwrap();
        let model = MixedModel {
            vertices: a.mesh.vertices,
            planes: a
                .model
                .faces
                .into_iter()
                .map(|f| MixedPlane {
                    normal: f.normal,
                    loops: f
                        .loops
                        .into_iter()
                        .map(|vertices| MixedLoop::Polygon { vertices })
                        .collect(),
                })
                .collect(),
            rings: vec![],
            cylinders: vec![],
            arcs: vec![],
            elliptic_arcs: vec![],
            partial_cylinders: vec![],
        };
        let mut work = Work::default();
        assert!(ends(&model, a.mesh.triangles.len(), &mut work).is_some());
        let mut exhausted = Work::default();
        exhausted.spend(10_000_000).unwrap();
        assert!(ends(&model, a.mesh.triangles.len(), &mut exhausted).is_none());
        assert_eq!(exhausted.used_for_test(), 10_000_000);
    }
}
