//! Spatial certificate for one already topology-validated triangular shell.
//! All predicates prove separation of the continuous primitives. No sampling.
use super::{sub, Bounds, Node, Point};
use fabstep_core::{FabError, Mesh};
mod separation;

// Four times OCCT Precision::Confusion in millimetres. Both primitive
// tolerances and roundoff must fit inside the certified separation.
const CLEARANCE: f64 = 4e-7;
const MAX_VISITS: usize = 100_000_000;
const MAX_CANDIDATES: usize = 10_000_000;
const LEAF: usize = 8;

fn unproven() -> FabError {
    FabError::new("validation.source_embedding_unproven",
        "Une intersection, un contact ou une séparation inférieure à la précision de contrôle est possible dans le maillage source.")
}
fn limit() -> FabError {
    FabError::new(
        "validation.source_embedding_budget",
        "Le contrôle des intersections du maillage source a atteint son budget.",
    )
}

/// Certify that distinct triangles of one welded, consistently oriented,
/// closed manifold meet only on their declared common simplex. Requires
/// the topology, dimensions and signed-volume checks of the core analysis.
/// A rejection can also mean the required clearance could not be certified.
/// The broad phase recursively partitions the triangle indices at their
/// median, storing O(n) records. Overlapping leaf pairs are examined once,
/// with at most 100 million tree/box visits and 10 million geometric tests.
/// A common vertex or edge is an allowed contact, provided a separating
/// plane strictly separates the other vertices. At a shared edge, both
/// pairs of nonincident bordering segments are independently separated too.
pub fn certify_source_embedding(mesh: &Mesh) -> Result<(), FabError> {
    certify(mesh, MAX_VISITS, MAX_CANDIDATES).map(|_| ())
}

struct Facet {
    id: usize,
    bounds: Bounds,
    center: Point,
}

fn build(facets: &mut [Facet], start: usize, nodes: &mut Vec<Node>) -> usize {
    let id = nodes.len();
    let bounds = facets
        .iter()
        .skip(1)
        .fold(facets[0].bounds, |b, t| b.union(t.bounds));
    nodes.push(Node {
        bounds,
        range: start..start + facets.len(),
        children: None,
    });
    if facets.len() > LEAF {
        let centers = Bounds::of(facets.iter().map(|f| f.center));
        let delta = sub(centers.max, centers.min);
        let axis = (0..3)
            .max_by(|&a, &b| delta[a].total_cmp(&delta[b]))
            .unwrap();
        let mid = facets.len() / 2;
        facets.select_nth_unstable_by(mid, |a, b| {
            a.center[axis]
                .total_cmp(&b.center[axis])
                .then(a.id.cmp(&b.id))
        });
        let (a, b) = facets.split_at_mut(mid);
        nodes[id].children = Some((build(a, start, nodes), build(b, start + mid, nodes)));
    }
    id
}

fn apart(a: Bounds, b: Bounds) -> bool {
    // Only one subtraction of two input coordinates. The factor of two
    // also makes the conclusion conservative after its final rounding.
    (0..3).any(|i| a.min[i] - b.max[i] > 2. * CLEARANCE || b.min[i] - a.max[i] > 2. * CLEARANCE)
}
#[derive(Debug)]
struct Budget {
    visits: usize,
    pairs: usize,
}
impl Budget {
    fn visit(&mut self) -> Result<(), FabError> {
        self.visits = self.visits.checked_sub(1).ok_or_else(limit)?;
        Ok(())
    }
}

struct Search<'a> {
    mesh: &'a Mesh,
    nodes: Vec<Node>,
    facets: Vec<Facet>,
}
impl Search<'_> {
    fn pair(&self, a: usize, b: usize, budget: &mut Budget) -> Result<(), FabError> {
        budget.visit()?;
        let (na, nb) = (&self.nodes[a], &self.nodes[b]);
        if a != b && apart(na.bounds, nb.bounds) {
            return Ok(());
        }
        if a == b {
            if let Some((l, r)) = na.children {
                self.pair(l, l, budget)?;
                self.pair(l, r, budget)?;
                return self.pair(r, r, budget);
            }
        } else if let Some((l, r)) = na
            .children
            .filter(|_| nb.children.is_none() || na.range.len() >= nb.range.len())
        {
            self.pair(l, b, budget)?;
            return self.pair(r, b, budget);
        } else if let Some((l, r)) = nb.children {
            self.pair(a, l, budget)?;
            return self.pair(a, r, budget);
        }
        for i in na.range.clone() {
            let start = if a == b { i + 1 } else { nb.range.start };
            for j in start..nb.range.end {
                budget.visit()?;
                let (x, y) = (&self.facets[i], &self.facets[j]);
                if apart(x.bounds, y.bounds) {
                    continue;
                }
                budget.pairs = budget.pairs.checked_sub(1).ok_or_else(limit)?;
                if !separation::certified(self.mesh, x.id, y.id) {
                    return Err(unproven());
                }
            }
        }
        Ok(())
    }
}
fn certify(mesh: &Mesh, visits: usize, pairs: usize) -> Result<Budget, FabError> {
    if mesh.triangles.is_empty()
        || mesh.triangles.len() > fabstep_core::SOURCE_MAX_TRIANGLES
        || mesh.vertices.len() > fabstep_core::SOURCE_MAX_VERTICES
        || mesh
            .vertices
            .iter()
            .flatten()
            .any(|&v| !v.is_finite() || v.abs() > 1e50)
    {
        return Err(unproven());
    }
    // The coordinate bound keeps all products in the checked predicates
    // finite, including intermediate witness directions.
    let mut facets = Vec::with_capacity(mesh.triangles.len());
    for (id, t) in mesh.triangles.iter().enumerate() {
        if t.iter().any(|&v| v >= mesh.vertices.len()) {
            return Err(unproven());
        }
        let tri = t.map(|v| mesh.vertices[v]);
        if !separation::regular(tri) {
            return Err(unproven());
        }
        let bounds = Bounds::of(tri);
        let center = std::array::from_fn(|i| 0.5 * (bounds.min[i] + bounds.max[i]));
        facets.push(Facet { id, bounds, center });
    }
    let mut nodes = Vec::with_capacity(2 * facets.len().div_ceil(LEAF).next_power_of_two());
    build(&mut facets, 0, &mut nodes);
    let mut remaining = Budget { visits, pairs };
    Search {
        mesh,
        nodes,
        facets,
    }
    .pair(0, 0, &mut remaining)?;
    Ok(remaining)
}

#[cfg(test)]
mod tests;
