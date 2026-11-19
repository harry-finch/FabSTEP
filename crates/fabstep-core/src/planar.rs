use crate::math::*;
use crate::{FabError, Mesh, PlanarFace, PlanarModel};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// Bounding worst-case contour validation matters for hostile/very intricate STL.
const MAX_SEGMENT_CHECKS: usize = 5_000_000;
const NORMAL_DOT_MIN: f64 = 1.0 - 1e-10;
fn aligned(a: V, b: V) -> bool {
    dot(a, b) >= NORMAL_DOT_MIN
}
type P2 = [f64; 2];
mod broad_phase;
mod perforated;
pub(crate) use perforated::cross_drill_supports;

const EXHAUSTIVE_EDGE_LIMIT: usize = 64;

#[cfg(test)]
pub(crate) fn build(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    tolerance: f64,
) -> Result<(PlanarModel, usize), FabError> {
    let result = build_with_provenance(mesh, adjacency, tolerance)?;
    Ok((result.model, result.approximate_faces))
}

pub(crate) struct Segmentation {
    pub model: PlanarModel,
    pub approximate_faces: usize,
    /// Exact source-triangle membership; index is the original triangle index.
    pub triangle_regions: Vec<usize>,
    // When a seed split a nearly parallel region, a qualified recognizer can
    // request one alternate segmentation with the remaining contour budget.
    checks_used_before_retry: Option<usize>,
}

pub(crate) fn build_with_provenance(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    tolerance: f64,
) -> Result<Segmentation, FabError> {
    let normals = triangle_normals(mesh);
    let mut checks = 0;
    match segment(mesh, adjacency, &normals, tolerance, &mut checks, false) {
        Err(e)
            if e.code == "planar.invalid_contour"
                && mesh.triangles.len() <= perforated::MAX_TRIANGLES =>
        {
            segment(mesh, adjacency, &normals, tolerance, &mut checks, true)
        }
        other => other,
    }
}

fn triangle_normals(mesh: &Mesh) -> Vec<V> {
    mesh.triangles
        .iter()
        .map(|t| {
            let n = cross(
                sub(mesh.vertices[t[1]], mesh.vertices[t[0]]),
                sub(mesh.vertices[t[2]], mesh.vertices[t[0]]),
            );
            let length = norm(n);
            n.map(|component| component / length)
        })
        .collect()
}

/// Alternate candidates never replace the validated faceted model. A caller
/// must prove its analytic result independently before using the partition.
pub(crate) fn retry_perforations(
    previous: &Segmentation,
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    tolerance: f64,
) -> Result<Option<Segmentation>, FabError> {
    let Some(mut checks) = previous.checks_used_before_retry else {
        return Ok(None);
    };
    match segment(
        mesh,
        adjacency,
        &triangle_normals(mesh),
        tolerance,
        &mut checks,
        true,
    ) {
        Ok(result) => Ok(Some(result)),
        // A validated partition remains available when this optional proof
        // exhausts its remaining budget. No further contour pass is allowed.
        Err(e)
            if matches!(
                e.code.as_str(),
                "planar.invalid_contour" | "planar.too_complex"
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

fn segment(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    normals: &[V],
    tolerance: f64,
    checks: &mut usize,
    recover_perforations: bool,
) -> Result<Segmentation, FabError> {
    // Speculative annular fits are bounded to small/medium documents. The
    // first rejected seed distance allocates this per-triangle visit marker.
    let mut attempted = Vec::new();
    let mut assigned = vec![false; mesh.triangles.len()];
    let mut faces = Vec::new();
    let mut triangle_regions = vec![usize::MAX; mesh.triangles.len()];
    let mut approximate_faces = 0;
    let mut split_observed = false;
    for seed in 0..mesh.triangles.len() {
        if assigned[seed] {
            continue;
        }
        let n = normals[seed];
        let origin = mesh.vertices[mesh.triangles[seed][0]];
        let mut region = BTreeSet::new();
        let mut split_by_distance = false;
        let mut pending = VecDeque::from([seed]);
        assigned[seed] = true;
        let mut max_residual: f64 = 0.0;
        let mut extent: f64 = 0.0;
        while let Some(face) = pending.pop_front() {
            region.insert(face);
            for &v in &mesh.triangles[face] {
                let offset = sub(mesh.vertices[v], origin);
                max_residual = max_residual.max(dot(offset, n).abs());
                extent = extent.max(norm(offset));
            }
            for &next in &adjacency[face] {
                // Compare to the immutable seed, never a drifting local/average plane.
                if assigned[next] || !aligned(normals[next], n) {
                    continue;
                }
                if mesh.triangles[next]
                    .iter()
                    .all(|&v| dot(sub(mesh.vertices[v], origin), n).abs() <= tolerance)
                {
                    assigned[next] = true;
                    pending.push_back(next);
                } else {
                    split_by_distance = true;
                }
            }
        }
        split_observed |= split_by_distance;
        if split_by_distance && recover_perforations {
            let ctx = perforated::Search {
                mesh,
                adjacency,
                normals,
                assigned: &assigned,
                tolerance,
            };
            if let Some((patch, face, approximate)) =
                ctx.recover(&region, &mut attempted, checks)?
            {
                for t in patch {
                    assigned[t] = true;
                    triangle_regions[t] = faces.len();
                }
                approximate_faces += usize::from(approximate);
                faces.push(face);
                continue;
            }
        }
        if max_residual > extent * f64::EPSILON * 128.0 {
            approximate_faces += 1;
        }
        let loops = boundaries(mesh, &region)?;
        let loops = validate_loops(mesh, loops, origin, n, checks)?;
        for &triangle in &region {
            triangle_regions[triangle] = faces.len();
        }
        faces.push(PlanarFace { loops, normal: n });
    }
    Ok(Segmentation {
        model: PlanarModel {
            vertices: mesh.vertices.clone(),
            faces,
        },
        approximate_faces,
        triangle_regions,
        checks_used_before_retry: (split_observed
            && !recover_perforations
            && mesh.triangles.len() <= perforated::MAX_TRIANGLES)
            .then_some(*checks),
    })
}

/// Construct a single indicated patch without changing source vertex positions.
/// Unlike automatic segmentation, this tests the whole patch against one plane,
/// not the nearly parallel triangle-normal heuristic. The 1e-7 contour ceiling
/// is a conservative interoperability constraint, not a dependency on a kernel.
pub(crate) fn guided_face(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    region: &BTreeSet<usize>,
    tolerance: f64,
    checks: &mut usize,
) -> Result<PlanarFace, FabError> {
    let unproven = || {
        FabError::new("guidance.plane_unproven", "La sélection doit former une zone plane connectée dont les contours sont conservés sans déplacer les sommets.")
    };
    let seed = *region.first().ok_or_else(unproven)?;
    let mut reached = BTreeSet::from([seed]);
    let mut pending = VecDeque::from([seed]);
    while let Some(triangle) = pending.pop_front() {
        for &neighbor in &adjacency[triangle] {
            if region.contains(&neighbor) && reached.insert(neighbor) {
                pending.push_back(neighbor);
            }
        }
    }
    if reached.len() != region.len() {
        return Err(unproven());
    }
    let normal = region_normal(mesh, region).ok_or_else(unproven)?;
    let loops = boundaries(mesh, region)?;
    let origin = mesh.vertices[loops[0][0]];
    for &triangle in region {
        let vertices = mesh.triangles[triangle].map(|i| mesh.vertices[i]);
        if dot(
            cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0])),
            normal,
        ) <= 0.
            || vertices
                .iter()
                .any(|&p| dot(sub(p, origin), normal).abs() > tolerance)
        {
            return Err(unproven());
        }
    }
    let loops = validate_loops(mesh, loops, origin, normal, checks)?;
    // validate_loops may move the outer contour to index zero. The native
    // face uses that first boundary vertex as its plane origin, so check the
    // actual output plane, including the full patch, once more.
    let origin = mesh.vertices[loops[0][0]];
    if loops
        .iter()
        .flatten()
        .any(|&i| dot(sub(mesh.vertices[i], origin), normal).abs() > tolerance.min(1e-7))
        || region.iter().any(|&t| {
            mesh.triangles[t]
                .iter()
                .any(|&i| dot(sub(mesh.vertices[i], origin), normal).abs() > tolerance)
        })
    {
        return Err(unproven());
    }
    Ok(PlanarFace { loops, normal })
}

fn region_normal(mesh: &Mesh, region: &BTreeSet<usize>) -> Option<V> {
    let mut normal = [Sum::default(), Sum::default(), Sum::default()];
    for &triangle in region {
        let [a, b, c] = mesh.triangles[triangle].map(|i| mesh.vertices[i]);
        let cross = cross(sub(b, a), sub(c, a));
        for axis in 0..3 {
            normal[axis].add(cross[axis]);
        }
    }
    let normal = normal.map(|sum| sum.get());
    let length = norm(normal);
    if !length.is_finite() || length <= 0. {
        return None;
    }
    Some(scale(normal, 1. / length))
}

pub(crate) fn boundaries(
    mesh: &Mesh,
    region: &BTreeSet<usize>,
) -> Result<Vec<Vec<usize>>, FabError> {
    let mut edges = BTreeSet::new();
    for &f in region {
        let [a, b, c] = mesh.triangles[f];
        for (u, v) in [(a, b), (b, c), (c, a)] {
            if !edges.remove(&(v, u)) {
                edges.insert((u, v));
            }
        }
    }
    let mut next = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for &(u, v) in &edges {
        if next.insert(u, v).is_some() || !incoming.insert(v) {
            return Err(contour_error("Contour planaire ramifié ou pincé."));
        }
    }
    if next.is_empty() || next.keys().any(|v| !incoming.contains(v)) {
        return Err(contour_error("Contour planaire vide ou ouvert."));
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.first_key_value() {
        let mut current = start;
        let mut contour = Vec::new();
        loop {
            contour.push(current);
            current = next
                .remove(&current)
                .ok_or_else(|| contour_error("Contour non fermé."))?;
            if current == start {
                break;
            }
        }
        if contour.len() < 3 {
            return Err(contour_error("Contour avec moins de trois sommets."));
        }
        loops.push(contour);
    }
    Ok(loops)
}

pub(crate) fn validate_loops(
    mesh: &Mesh,
    mut loops: Vec<Vec<usize>>,
    origin: V,
    normal: V,
    checks: &mut usize,
) -> Result<Vec<Vec<usize>>, FabError> {
    let least = (0..3)
        .min_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
        .unwrap();
    let mut helper = [0.0; 3];
    helper[least] = 1.0;
    let u = cross(helper, normal);
    let u = scale(u, 1.0 / norm(u));
    let v = cross(normal, u);
    let mut projected: Vec<Vec<P2>> = loops
        .iter()
        .map(|lp| {
            lp.iter()
                .map(|&i| {
                    let p = sub(mesh.vertices[i], origin);
                    [dot(p, u), dot(p, v)]
                })
                .collect()
        })
        .collect();
    let areas: Vec<f64> = projected.iter().map(|lp| area(lp)).collect();
    let outer = areas
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0;
    if areas[outer] <= 0.0
        || areas
            .iter()
            .enumerate()
            .any(|(i, a)| i != outer && *a >= 0.0)
    {
        return Err(contour_error(
            "Orientation ou inclusion des contours planaires incohérente.",
        ));
    }
    loops.swap(0, outer);
    projected.swap(0, outer);
    validate_edge_pairs(&projected, checks)?;
    for i in 1..projected.len() {
        if !inside(projected[i][0], &projected[0], checks)? {
            return Err(contour_error("Contour intérieur situé hors de la face."));
        }
        for j in 1..projected.len() {
            if i != j && inside(projected[i][0], &projected[j], checks)? {
                return Err(contour_error(
                    "Trous imbriqués non représentables par une seule face.",
                ));
            }
        }
    }
    // No per-face collinear simplification: every shared boundary vertex is kept.
    Ok(loops)
}

fn validate_edge_pairs(projected: &[Vec<P2>], checks: &mut usize) -> Result<(), FabError> {
    let count = projected
        .iter()
        .try_fold(0usize, |n, lp| n.checked_add(lp.len()));
    let Some(count) = count.filter(|n| *n <= 3 * crate::SOURCE_MAX_TRIANGLES) else {
        return Err(too_complex());
    };
    if count <= EXHAUSTIVE_EDGE_LIMIT || !broad_phase::eligible(projected) {
        return validate_edge_pairs_exhaustive(projected, checks);
    }
    for lp in projected {
        for i in 0..lp.len() {
            budget(checks)?;
            consecutive(lp, i)?;
        }
    }
    broad_phase::prove(projected, count, checks)
}

fn consecutive(lp: &[P2], i: usize) -> Result<(), FabError> {
    let previous = lp[(i + lp.len() - 1) % lp.len()];
    let current = lp[i];
    let following = lp[(i + 1) % lp.len()];
    let a = [previous[0] - current[0], previous[1] - current[1]];
    let b = [following[0] - current[0], following[1] - current[1]];
    if orientation(previous, current, following) == 0 && a[0] * b[0] + a[1] * b[1] > 0.0 {
        return Err(contour_error(
            "Arêtes consécutives superposées dans un contour.",
        ));
    }
    Ok(())
}

// Retained verbatim as the small/extreme-coordinate path and differential oracle.
fn validate_edge_pairs_exhaustive(
    projected: &[Vec<P2>],
    checks: &mut usize,
) -> Result<(), FabError> {
    for (index, lp) in projected.iter().enumerate() {
        for i in 0..lp.len() {
            let current = lp[i];
            let following = lp[(i + 1) % lp.len()];
            consecutive(lp, i)?;
            for (other_index, other) in projected.iter().enumerate().skip(index) {
                for j in 0..other.len() {
                    if index == other_index
                        && (j <= i || j == i + 1 || (i == 0 && j + 1 == lp.len()))
                    {
                        continue;
                    }
                    budget(checks)?;
                    if intersects(current, following, other[j], other[(j + 1) % other.len()]) {
                        return Err(contour_error(
                            "Contours planaires auto-intersectés ou se touchant.",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
fn contour_error(message: &str) -> FabError {
    FabError::new("planar.invalid_contour", message)
}
fn budget(checks: &mut usize) -> Result<(), FabError> {
    budget_units(checks, 1)
}
fn budget_units(checks: &mut usize, units: usize) -> Result<(), FabError> {
    *checks = checks.saturating_add(units);
    if *checks > MAX_SEGMENT_CHECKS {
        Err(too_complex())
    } else {
        Ok(())
    }
}
fn too_complex() -> FabError {
    FabError::new(
        "planar.too_complex",
        "Budget de validation des contours dépassé (5 millions d’unités de travail).",
    )
}
fn area(lp: &[P2]) -> f64 {
    let origin = lp[0];
    let mut sum = Sum::default();
    for i in 0..lp.len() {
        let a = [lp[i][0] - origin[0], lp[i][1] - origin[1]];
        let b = [
            lp[(i + 1) % lp.len()][0] - origin[0],
            lp[(i + 1) % lp.len()][1] - origin[1],
        ];
        sum.add(a[0] * b[1] - a[1] * b[0]);
    }
    sum.get() * 0.5
}
// Filtered floating-point orientation: ambiguity is treated conservatively as
// collinearity. This is not an exact-predicate computational geometry package.
fn orientation(a: P2, b: P2, c: P2) -> i8 {
    let left = (b[0] - a[0]) * (c[1] - a[1]);
    let right = (b[1] - a[1]) * (c[0] - a[0]);
    let value = left - right;
    let uncertainty = (left.abs() + right.abs()) * f64::EPSILON * 16.0;
    if value > uncertainty {
        1
    } else if value < -uncertainty {
        -1
    } else {
        0
    }
}
fn on_segment(a: P2, b: P2, p: P2) -> bool {
    (0..2).all(|i| p[i] >= a[i].min(b[i]) && p[i] <= a[i].max(b[i]))
}
fn intersects(a: P2, b: P2, c: P2, d: P2) -> bool {
    let ab_c = orientation(a, b, c);
    let ab_d = orientation(a, b, d);
    let cd_a = orientation(c, d, a);
    let cd_b = orientation(c, d, b);
    (ab_c != 0 && ab_d != 0 && ab_c != ab_d && cd_a != 0 && cd_b != 0 && cd_a != cd_b)
        || (ab_c == 0 && on_segment(a, b, c))
        || (ab_d == 0 && on_segment(a, b, d))
        || (cd_a == 0 && on_segment(c, d, a))
        || (cd_b == 0 && on_segment(c, d, b))
}
fn inside(p: P2, polygon: &[P2], checks: &mut usize) -> Result<bool, FabError> {
    let mut inside = false;
    for i in 0..polygon.len() {
        budget(checks)?;
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if orientation(a, b, p) == 0 && on_segment(a, b, p) {
            return Ok(false);
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1])
        {
            inside = !inside;
        }
    }
    Ok(inside)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geometric_contours_reject_crossings() {
        assert!(intersects([0., 0.], [2., 2.], [0., 2.], [2., 0.]));
        assert!(intersects([0., 0.], [2., 0.], [1., 0.], [3., 0.]));
        assert!(!intersects([0., 0.], [1., 0.], [2., 0.], [3., 0.]));
        let mut checks = 0;
        assert!(inside(
            [1., 1.],
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            &mut checks
        )
        .unwrap());
        assert!(!inside(
            [3., 1.],
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            &mut checks
        )
        .unwrap());
    }

    #[test]
    fn region_growth_keeps_the_original_plane() {
        // Shallow bent strip: each new segment is close to its neighbour's
        // plane, but the last is too far from the first plane.
        let mesh = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [0., 1., 0.],
                [1., 0., 0.],
                [1., 1., 0.],
                [2., 0., 0.8e-6],
                [2., 1., 0.8e-6],
                [3., 0., 2.4e-6],
                [3., 1., 2.4e-6],
            ],
            triangles: vec![
                [0, 2, 3],
                [0, 3, 1],
                [2, 4, 5],
                [2, 5, 3],
                [4, 6, 7],
                [4, 7, 5],
            ],
        };
        let adjacency = vec![
            vec![1, 3],
            vec![0],
            vec![3, 5],
            vec![0, 2],
            vec![5],
            vec![2, 4],
        ];
        let (model, _) = build(&mesh, &adjacency, 1e-6).unwrap();
        assert_eq!(model.faces.len(), 2);
        assert!(model.faces[0].loops[0]
            .iter()
            .all(|&v| mesh.vertices[v][2] <= 1e-6));
        // The straight, shared seam stays in both regions.
        assert!(model
            .faces
            .iter()
            .all(|f| f.loops[0].contains(&4) && f.loops[0].contains(&5)));
    }

    #[test]
    fn binary_tilted_perforations_keep_both_contours_and_all_source_triangles() {
        let source = include_bytes!("../../../fixtures/cross_drill/freecad_mid_rotated.stl");
        let analysis = crate::analyze_stl(source, &crate::Options::default()).unwrap();
        let mut mesh = analysis.mesh;
        for _ in 0..2 {
            let adjacency = crate::pipeline::validate_topology(&mesh).unwrap();
            let normals = triangle_normals(&mesh);
            for tolerance in [1e-6, 1e-5] {
                let mut first = build_with_provenance(&mesh, &adjacency, tolerance).unwrap();
                let alternate = retry_perforations(&first, &mesh, &adjacency, tolerance).unwrap();
                let result = alternate.as_ref().unwrap_or(&first);
                assert_eq!(result.model.vertices, mesh.vertices);
                let mut perforations = 0;
                for (id, face) in result.model.faces.iter().enumerate() {
                    if face.loops.len() != 2 {
                        continue;
                    }
                    perforations += 1;
                    assert_eq!((face.loops[0].len(), face.loops[1].len()), (4, 63));
                    let seed = result
                        .triangle_regions
                        .iter()
                        .position(|&r| r == id)
                        .unwrap();
                    // Unmodified historical faces are checked against their
                    // seed plane. Recovered faces use the outer-loop anchor.
                    let anchor = if face.normal == normals[seed] {
                        mesh.triangles[seed][0]
                    } else {
                        face.loops[0][0]
                    };
                    let origin = mesh.vertices[anchor];
                    let mut count = 0;
                    for (t, &region) in mesh.triangles.iter().zip(&result.triangle_regions) {
                        if region == id {
                            count += 1;
                            for &v in t {
                                let distance =
                                    dot(sub(mesh.vertices[v], origin), face.normal).abs();
                                assert!(distance <= tolerance);
                            }
                        }
                    }
                    assert_eq!(count, 67);
                }
                assert_eq!(perforations, 4);
                first.checks_used_before_retry = Some(MAX_SEGMENT_CHECKS);
                assert!(retry_perforations(&first, &mesh, &adjacency, tolerance)
                    .unwrap()
                    .is_none());
            }
            mesh.triangles.reverse();
        }
    }

    #[test]
    fn rejects_self_intersections_and_nested_or_exterior_holes() {
        let normal = [0., 0., 1.];
        let origin = [0.; 3];
        let mut checks = 0;
        let mut mesh = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [4., 0., 0.],
                [4., 4., 0.],
                [0., 4., 0.],
                [1., 1., 0.],
                [1., 2., 0.],
                [2., 2., 0.],
                [2., 1., 0.],
            ],
            triangles: vec![],
        };
        assert!(validate_loops(
            &mesh,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
            origin,
            normal,
            &mut checks
        )
        .is_ok());
        assert!(
            validate_loops(&mesh, vec![vec![0, 2, 1, 3]], origin, normal, &mut checks).is_err()
        );
        for p in &mut mesh.vertices[4..] {
            p[0] += 10.;
        }
        assert!(validate_loops(
            &mesh,
            vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
            origin,
            normal,
            &mut checks
        )
        .is_err());
    }

    #[test]
    fn contour_work_budget_is_enforced() {
        let mut checks = MAX_SEGMENT_CHECKS;
        assert_eq!(budget(&mut checks).unwrap_err().code, "planar.too_complex");
    }

    fn circle(count: usize, scale: f64) -> Vec<P2> {
        (0..count)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / count as f64;
                [scale * angle.cos(), scale * angle.sin()]
            })
            .collect()
    }
    fn rectangle(x: f64, y: f64, right: f64, top: f64) -> Vec<P2> {
        let mut out = Vec::new();
        for (a, b) in [
            ([x, y], [right, y]),
            ([right, y], [right, top]),
            ([right, top], [x, top]),
            ([x, top], [x, y]),
        ] {
            for i in 0..8 {
                let t = i as f64 / 8.;
                out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
        out
    }
    fn equivalent(polygons: &[Vec<P2>]) -> (usize, usize) {
        let (mut old_checks, mut new_checks) = (0, 0);
        let old = validate_edge_pairs_exhaustive(polygons, &mut old_checks);
        let new = validate_edge_pairs(polygons, &mut new_checks);
        assert_eq!(
            old.as_ref()
                .err()
                .map(|e| (e.code.as_str(), e.message.as_str())),
            new.as_ref()
                .err()
                .map(|e| (e.code.as_str(), e.message.as_str()))
        );
        (old_checks, new_checks)
    }

    #[test]
    fn indexed_edges_preserve_crossings_contacts_collinearity_and_small_path() {
        let small = vec![circle(EXHAUSTIVE_EDGE_LIMIT, 1.)];
        let (old, new) = equivalent(&small);
        assert_eq!(old, new);
        let ordinary = vec![circle(128, 1.)];
        let (old, new) = equivalent(&ordinary);
        assert!(new < old);
        let mut crossing = ordinary;
        crossing[0].swap(1, 65);
        equivalent(&crossing);
        for second in [
            rectangle(6., 2., 8., 4.),
            rectangle(4., 2., 6., 4.),
            rectangle(4., 4., 6., 6.),
        ] {
            equivalent(&[
                rectangle(0., 0., 10., 10.),
                rectangle(2., 2., 4., 4.),
                second,
            ]);
        }
        let gap = f64::from_bits(1_f64.to_bits() + 1);
        equivalent(&[
            rectangle(-2., -2., 3., 3.),
            rectangle(0., 0., 1., 1.),
            rectangle(gap, 0., 2., 1.),
        ]);
        // Adjacent collinear edges are retained, including vertical edges and
        // signed zero. Their only common point is never treated as a crossing.
        equivalent(&[
            rectangle(-0., -0., 4., 4.),
            rectangle(1., 1., 2., 2.),
            rectangle(2.5, 1., 3.5, 2.),
        ]);
    }

    #[test]
    fn dyadic_filter_window_and_extreme_fallback_match_the_original_predicate() {
        for exponent in [-200, -190, 190, 200] {
            let s = 2_f64.powi(exponent);
            let polygons = vec![
                rectangle(0., 0., s, s),
                rectangle(s / 4., s / 4., s / 2., s / 2.),
                rectangle(s * 0.75, s / 4., s * 0.875, s / 2.),
            ];
            // At the lower bound, subdivision can create coordinates outside
            // the window. The fallback must preserve those cases as well.
            equivalent(&polygons);
        }
        for exponent in [-600, 600] {
            let polygons = vec![circle(96, 2_f64.powi(exponent))];
            assert!(!broad_phase::eligible(&polygons));
            let (old, new) = equivalent(&polygons);
            assert_eq!(old, new);
        }
        let low = 2_f64.powi(-200);
        let high = 2_f64.powi(200);
        assert!(broad_phase::eligible(&[vec![[0., -0.], [low, high]]]));
        assert!(!broad_phase::eligible(&[vec![[
            f64::from_bits(low.to_bits() - 1),
            1.
        ]]]));
        assert!(!broad_phase::eligible(&[vec![[
            f64::from_bits(high.to_bits() + 1),
            1.
        ]]]));
    }

    #[test]
    fn indexed_work_remains_bounded_for_overlapping_boxes_and_exhausted_setup() {
        let polygons = vec![circle(128, 1.)];
        let mut checks = MAX_SEGMENT_CHECKS;
        assert_eq!(
            validate_edge_pairs(&polygons, &mut checks)
                .unwrap_err()
                .code,
            "planar.too_complex"
        );
        // Parallel, disjoint diagonals have heavily overlapping AABBs. This
        // isolates broad-phase work; no complete polygon/solid is claimed.
        let polygons: Vec<_> = (0..256)
            .map(|i| {
                let offset = i as f64 / 1024.;
                vec![[-1., -1. + offset], [1., 1. + offset]]
            })
            .collect();
        let mut checks = MAX_SEGMENT_CHECKS - 10_000;
        assert_eq!(
            broad_phase::prove(&polygons, 512, &mut checks)
                .unwrap_err()
                .code,
            "planar.too_complex"
        );
        assert!(checks > MAX_SEGMENT_CHECKS);
    }

    #[test]
    fn deterministic_radial_corpus_matches_exhaustive_after_rigid_transforms_and_crossings() {
        let mut seed = 0x43dd_80aa_77e2_9011u64;
        for _variation in 0..4 {
            let points: Vec<_> = (0..96)
                .map(|i| {
                    seed = seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let radius = 1. + ((seed >> 32) & 1023) as f64 / 4096.;
                    let angle = std::f64::consts::TAU * i as f64 / 96.;
                    [radius * angle.cos(), radius * angle.sin()]
                })
                .collect();
            for angle in [0_f64, 0.37, 1.2] {
                let (sine, cosine) = angle.sin_cos();
                for offset in [[0., 0.], [-13.25, 7.5], [1e8, -1e8]] {
                    let transformed: Vec<_> = points
                        .iter()
                        .map(|p| {
                            [
                                cosine * p[0] - sine * p[1] + offset[0],
                                sine * p[0] + cosine * p[1] + offset[1],
                            ]
                        })
                        .collect();
                    let polygons = vec![transformed.clone()];
                    assert!(broad_phase::eligible(&polygons));
                    assert!(validate_edge_pairs_exhaustive(&polygons, &mut 0).is_ok());
                    let (_, first) = equivalent(&polygons);
                    let (_, second) = equivalent(&polygons);
                    assert_eq!(first, second, "index work must be deterministic");
                    let mut crossed = transformed;
                    crossed.swap(1, 49);
                    let polygons = vec![crossed];
                    assert_eq!(
                        validate_edge_pairs_exhaustive(&polygons, &mut 0)
                            .unwrap_err()
                            .code,
                        "planar.invalid_contour"
                    );
                    equivalent(&polygons);
                }
            }
        }
    }

    #[test]
    fn indexed_proof_still_runs_exhaustive_outer_and_nested_hole_classification() {
        for (hole, expected) in [
            (rectangle(6., 6., 8., 8.), None),
            (
                rectangle(20., 2., 22., 4.),
                Some("Contour intérieur situé hors de la face."),
            ),
            (
                rectangle(2.5, 2.5, 3.5, 3.5),
                Some("Trous imbriqués non représentables par une seule face."),
            ),
        ] {
            let polygons = [rectangle(0., 0., 12., 12.), rectangle(2., 2., 4., 4.), hole];
            let mut mesh = Mesh {
                vertices: Vec::new(),
                triangles: Vec::new(),
            };
            let mut loops = Vec::new();
            for (index, mut polygon) in polygons.into_iter().enumerate() {
                if index > 0 {
                    polygon.reverse();
                }
                let start = mesh.vertices.len();
                mesh.vertices
                    .extend(polygon.iter().map(|p| [p[0], p[1], 0.]));
                loops.push((start..mesh.vertices.len()).collect());
            }
            assert!(mesh.vertices.len() > EXHAUSTIVE_EDGE_LIMIT);
            let result = validate_loops(&mesh, loops, [0.; 3], [0., 0., 1.], &mut 0);
            if let Some(message) = expected {
                let error = result.unwrap_err();
                assert_eq!(error.code, "planar.invalid_contour");
                assert_eq!(error.message, message);
            } else {
                assert!(result.is_ok());
            }
        }
    }

    #[test]
    #[ignore = "manual contour-only timing; production integration is tested separately"]
    fn benchmark_dense_contour_exhaustive_against_bvh() {
        use std::time::Instant;
        for count in [512, 4096] {
            let polygons = vec![circle(count, 10.)];
            let mut old_checks = 0;
            let start = Instant::now();
            let old = validate_edge_pairs_exhaustive(&polygons, &mut old_checks);
            let old_time = start.elapsed();
            let mut new_checks = 0;
            let start = Instant::now();
            let new = validate_edge_pairs(&polygons, &mut new_checks);
            let new_time = start.elapsed();
            eprintln!("contour n={count}: exhaustive checks={old_checks} time={old_time:?} result={:?}; BVH work={new_checks} time={new_time:?} result={:?}",old.as_ref().err().map(|e|&e.code),new.as_ref().err().map(|e|&e.code));
            assert!(new.is_ok());
            assert!(new_checks < old_checks);
            if count == 512 {
                assert!(old.is_ok());
            } else {
                assert_eq!(old.unwrap_err().code, "planar.too_complex");
            }
        }
    }
}
