use crate::math::*;
use crate::{FabError, Mesh, PlanarFace, PlanarModel};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// Bounding worst-case contour validation matters for hostile/very intricate STL.
const MAX_SEGMENT_CHECKS: usize = 5_000_000;
const NORMAL_DOT_MIN: f64 = 1.0 - 1e-10;
type P2 = [f64; 2];

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
}

pub(crate) fn build_with_provenance(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    tolerance: f64,
) -> Result<Segmentation, FabError> {
    let normals: Vec<V> = mesh
        .triangles
        .iter()
        .map(|t| {
            let n = cross(
                sub(mesh.vertices[t[1]], mesh.vertices[t[0]]),
                sub(mesh.vertices[t[2]], mesh.vertices[t[0]]),
            );
            let length = norm(n);
            n.map(|component| component / length)
        })
        .collect();
    let mut assigned = vec![false; mesh.triangles.len()];
    let mut faces = Vec::new();
    let mut triangle_regions = vec![usize::MAX; mesh.triangles.len()];
    let mut approximate_faces = 0;
    let mut checks = 0;
    for seed in 0..mesh.triangles.len() {
        if assigned[seed] {
            continue;
        }
        let n = normals[seed];
        let origin = mesh.vertices[mesh.triangles[seed][0]];
        let mut region = BTreeSet::new();
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
                if !assigned[next]
                    && dot(normals[next], n) >= NORMAL_DOT_MIN
                    && mesh.triangles[next]
                        .iter()
                        .all(|&v| dot(sub(mesh.vertices[v], origin), n).abs() <= tolerance)
                {
                    assigned[next] = true;
                    pending.push_back(next);
                }
            }
        }
        if max_residual > extent * f64::EPSILON * 128.0 {
            approximate_faces += 1;
        }
        let loops = boundaries(mesh, &region)?;
        let loops = validate_loops(mesh, loops, origin, n, &mut checks)?;
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
    })
}

fn boundaries(mesh: &Mesh, region: &BTreeSet<usize>) -> Result<Vec<Vec<usize>>, FabError> {
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

fn validate_loops(
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
    for (index, lp) in projected.iter().enumerate() {
        for i in 0..lp.len() {
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
fn contour_error(message: &str) -> FabError {
    FabError::new("planar.invalid_contour", message)
}
fn budget(checks: &mut usize) -> Result<(), FabError> {
    *checks += 1;
    if *checks > MAX_SEGMENT_CHECKS {
        Err(FabError::new(
            "planar.too_complex",
            "Budget de validation des contours dépassé (5 millions de comparaisons).",
        ))
    } else {
        Ok(())
    }
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
}
