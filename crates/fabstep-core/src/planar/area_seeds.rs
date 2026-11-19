//! Optional coarsening of an already valid planar partition on a quantized mesh.
use super::*;

pub(crate) const LIMIT: usize = 100_000;

struct Recovery<'a> {
    mesh: &'a Mesh,
    adjacency: &'a [Vec<usize>],
    normals: &'a [V],
    tolerance: f64,
    owners: &'a [usize],
    planes: &'a [V],
}

pub(crate) fn candidate(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    normals: &[V],
    tolerance: f64,
    checks: &mut usize,
    owners: &[usize],
    planes: &[V],
) -> Option<(Segmentation, crate::Diagnostic)> {
    let count = mesh.triangles.len();
    if count > LIMIT || planes.len() <= 1 {
        return None;
    }
    // Seeds that can both align with one normal differ by at most twice the
    // growth angle. A shared edge is necessary for a connected union.
    let split = adjacency.iter().enumerate().any(|(t, neighbors)| {
        neighbors.iter().any(|&n| {
            n > t
                && owners[t] != owners[n]
                && dot(planes[owners[t]], planes[owners[n]]) >= SHARED_SEED_DOT_MIN
        })
    });
    if !split {
        return None;
    }
    // Reserve sorting, a second traversal and its linear verification before
    // allocating the candidate. Contours still charge the remaining budget.
    budget_units(checks, count * (count.ilog2() as usize + 16)).ok()?;
    let areas: Vec<_> = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|v| mesh.vertices[v]);
            norm(cross(sub(b, a), sub(c, a)))
        })
        .collect();
    let recovery = Recovery {
        mesh,
        adjacency,
        normals,
        tolerance,
        owners,
        planes,
    };
    let mut order: Vec<_> = (0..count).collect();
    order.sort_unstable_by(|&a, &b| areas[b].total_cmp(&areas[a]).then(a.cmp(&b)));
    if let Some(alternate) = recovery.prove_order(checks, &order) {
        let diagnostic = recovered_diagnostic(planes.len() - alternate.model.faces.len());
        return Some((alternate, diagnostic));
    }

    // The auxiliary copy only changes the seed order of the same proof.
    // Sorting and two more linear passes must fit in the remaining budget.
    budget_units(checks, count * (count.ilog2() as usize + 32)).ok()?;
    taubin::order_by_stability(mesh, normals, &areas, &mut order);
    let alternate = recovery.prove_order(checks, &order)?;
    let diagnostic = crate::Diagnostic {
        code: "planar.taubin_seeds_recovered".into(),
        severity: crate::Severity::Info,
        count: planes.len() - alternate.model.faces.len(),
        message: "Des plans fragmentés ont été réunis à partir de normales source stables.".into(),
    };
    Some((alternate, diagnostic))
}

impl Recovery<'_> {
    fn prove_order(&self, checks: &mut usize, order: &[usize]) -> Option<Segmentation> {
        let Self {
            mesh,
            adjacency,
            normals,
            tolerance,
            owners,
            planes,
        } = *self;
        let alternate = segment(
            mesh,
            adjacency,
            normals,
            tolerance,
            checks,
            false,
            order.iter().copied(),
        )
        .ok()?;
        if alternate.model.faces.len() >= planes.len()
            || !coarsens(planes.len(), owners, &alternate.triangle_regions)
        {
            return None;
        }
        // Preserve an existing crease even when the two neighboring normals
        // separately fit the angular cone around the new seed.
        if adjacency.iter().enumerate().any(|(t, neighbors)| {
            neighbors.iter().any(|&n| {
                n > t
                    && owners[t] != owners[n]
                    && alternate.triangle_regions[t] == alternate.triangle_regions[n]
                    && !aligned(normals[t], normals[n])
            })
        }) {
            return None;
        }
        // The native plane is anchored at the first vertex of the outer wire.
        // Auto additionally reserves half the kernel tolerance for shared edges.
        for f in &alternate.model.faces {
            let origin = mesh.vertices[f.loops[0][0]];
            if f.loops.iter().flatten().any(|&v| {
                dot(sub(mesh.vertices[v], origin), f.normal).abs()
                    > tolerance.min(SHARED_CONTOUR_TOLERANCE_MM)
            }) {
                return None;
            }
        }
        for (t, &region) in mesh.triangles.iter().zip(&alternate.triangle_regions) {
            let f = &alternate.model.faces[region];
            let origin = mesh.vertices[f.loops[0][0]];
            if t.iter()
                .any(|&v| dot(sub(mesh.vertices[v], origin), f.normal).abs() > tolerance)
            {
                return None;
            }
        }
        Some(alternate)
    }
}

/// Every previous region must survive whole within exactly one new plane.
fn coarsens(count: usize, owners: &[usize], alternate: &[usize]) -> bool {
    let mut mapping = vec![None; count];
    owners
        .iter()
        .zip(alternate)
        .all(|(&from, &to)| *mapping[from].get_or_insert(to) == to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_may_join_a_neighbor_but_must_not_be_split() {
        assert!(coarsens(3, &[0, 1, 0, 2], &[1, 0, 1, 0]));
        assert!(!coarsens(3, &[0, 1, 0, 2], &[0, 0, 1, 0]));
    }

    #[test]
    fn exhausted_contour_budget_preserves_the_original_partition() {
        let options = crate::Options::default();
        let source = include_bytes!("../../../../fixtures/r115-rotated-crease.stl");
        let mesh = crate::analyze_stl(source, &options).unwrap().mesh;
        let adjacency = crate::pipeline::validate_topology(&mesh).unwrap();
        let normals = triangle_normals(&mesh);
        let mut checks = 0;
        let first = initial_partition(
            &mesh,
            &adjacency,
            &normals,
            options.plane_tolerance_mm,
            &mut checks,
        )
        .unwrap();
        let planes: Vec<_> = first.model.faces.iter().map(|f| f.normal).collect();
        let retry = |work: &mut usize| {
            candidate(
                &mesh,
                &adjacency,
                &normals,
                options.plane_tolerance_mm,
                work,
                &first.triangle_regions,
                &planes,
            )
        };
        let mut exhausted = MAX_SEGMENT_CHECKS;
        assert!(retry(&mut exhausted).is_none());
        assert_eq!(retry(&mut checks).unwrap().0.model.faces.len(), 7);
        assert_eq!(first.model.faces.len(), 9);
    }
}
