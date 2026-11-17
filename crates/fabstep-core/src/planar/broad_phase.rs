//! Conservative closed-AABB rejection; the existing floating predicates make
//! every geometric decision. Construction and traversal share the 5M budget.
use super::{budget, budget_units, contour_error, intersects, too_complex, FabError, P2};
use std::cmp::Ordering;

const LEAF_SIZE: usize = 8;

pub(super) fn eligible(polygons: &[Vec<P2>]) -> bool {
    // This is a filter eligibility window, NOT a geometric tolerance. For
    // binary64 coordinates 0 or 2^-200 <= |x| <= 2^200, every nonzero rounded
    // coordinate difference is >=2^-252 and <=2^201. Nonzero orientation
    // products are therefore in [2^-504,2^402]; their subtraction and the
    // 16*EPSILON uncertainty stay normal and finite. The standard gamma_3
    // product error plus the final subtraction is strictly below that filter.
    // Hence every nonzero orientation sign agrees with the exact sign for the
    // represented projected points. Proper crossings require overlapping
    // AABBs; all collinear clauses independently require on_segment's closed
    // AABB. Outside this proved window we run the unmodified exhaustive path,
    // preserving even its numerical edge cases rather than assuming a bound.
    let minimum = 2.0_f64.powi(-200);
    let maximum = 2.0_f64.powi(200);
    polygons
        .iter()
        .flatten()
        .flatten()
        .all(|x| x.is_finite() && (*x == 0. || (minimum..=maximum).contains(&x.abs())))
}

#[derive(Clone, Copy)]
struct Bounds {
    min: P2,
    max: P2,
}
impl Bounds {
    fn segment(a: P2, b: P2) -> Self {
        Self {
            min: [a[0].min(b[0]), a[1].min(b[1])],
            max: [a[0].max(b[0]), a[1].max(b[1])],
        }
    }
    fn union(self, other: Self) -> Self {
        Self {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }
    fn overlaps(self, other: Self) -> bool {
        // Strict separation only. Equal endpoints, horizontal/vertical edges,
        // signed zeros, and touching boxes are always sent to the old predicate.
        (0..2).all(|axis| self.min[axis] <= other.max[axis] && other.min[axis] <= self.max[axis])
    }
    fn center(self, axis: usize) -> f64 {
        self.min[axis] * 0.5 + self.max[axis] * 0.5
    }
}
struct Segment {
    bounds: Bounds,
    polygon: usize,
    index: usize,
}
struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<(usize, usize)>,
}
struct Tree {
    segments: Vec<Segment>,
    order: Vec<usize>,
    nodes: Vec<Node>,
}

// Fallible deterministic merge sort. Each output slot, including a tail copy,
// consumes a unit, so no library sort can hide unbounded work after exhaustion.
fn sort(
    order: &mut Vec<usize>,
    segments: &[Segment],
    axis: usize,
    checks: &mut usize,
) -> Result<(), FabError> {
    let compare = |a: usize, b: usize| -> Ordering {
        segments[a]
            .bounds
            .center(axis)
            .total_cmp(&segments[b].bounds.center(axis))
            .then_with(|| {
                segments[a]
                    .bounds
                    .center(1 - axis)
                    .total_cmp(&segments[b].bounds.center(1 - axis))
            })
            .then(a.cmp(&b))
    };
    budget_units(checks, order.len())?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(order.len())
        .map_err(|_| too_complex())?;
    scratch.extend_from_slice(order);
    let mut width = 1;
    while width < order.len() {
        for start in (0..order.len()).step_by(2 * width) {
            let middle = (start + width).min(order.len());
            let end = (start + 2 * width).min(order.len());
            let (mut left, mut right) = (start, middle);
            for target in &mut scratch[start..end] {
                budget(checks)?;
                if left < middle
                    && (right == end || compare(order[left], order[right]) != Ordering::Greater)
                {
                    *target = order[left];
                    left += 1;
                } else {
                    *target = order[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(order, &mut scratch);
        width *= 2;
    }
    Ok(())
}
impl Tree {
    fn build(&mut self, start: usize, end: usize, checks: &mut usize) -> Result<usize, FabError> {
        budget(checks)?;
        let (bounds, children) = if end - start <= LEAF_SIZE {
            let mut bounds = self.segments[self.order[start]].bounds;
            for &segment in &self.order[start + 1..end] {
                budget(checks)?;
                bounds = bounds.union(self.segments[segment].bounds);
            }
            (bounds, None)
        } else {
            let middle = start + (end - start) / 2;
            let left = self.build(start, middle, checks)?;
            let right = self.build(middle, end, checks)?;
            (
                self.nodes[left].bounds.union(self.nodes[right].bounds),
                Some((left, right)),
            )
        };
        let id = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            start,
            end,
            children,
        });
        Ok(id)
    }
    fn pair(
        &self,
        a: usize,
        b: usize,
        polygons: &[Vec<P2>],
        checks: &mut usize,
    ) -> Result<(), FabError> {
        budget(checks)?;
        let (first, second) = (&self.nodes[a], &self.nodes[b]);
        if !first.bounds.overlaps(second.bounds) {
            return Ok(());
        }
        if let (None, None) = (first.children, second.children) {
            for i in first.start..first.end {
                for j in second.start..second.end {
                    if a == b && j <= i {
                        continue;
                    }
                    budget(checks)?;
                    let (x, y) = (&self.segments[self.order[i]], &self.segments[self.order[j]]);
                    let adjacent = x.polygon == y.polygon
                        && ((x.index + 1) % polygons[x.polygon].len() == y.index
                            || (y.index + 1) % polygons[y.polygon].len() == x.index);
                    if adjacent || !x.bounds.overlaps(y.bounds) {
                        continue;
                    }
                    let (lp, other) = (&polygons[x.polygon], &polygons[y.polygon]);
                    if intersects(
                        lp[x.index],
                        lp[(x.index + 1) % lp.len()],
                        other[y.index],
                        other[(y.index + 1) % other.len()],
                    ) {
                        return Err(contour_error(
                            "Contours planaires auto-intersectés ou se touchant.",
                        ));
                    }
                }
            }
        } else if a == b {
            let (left, right) = first.children.unwrap();
            self.pair(left, left, polygons, checks)?;
            self.pair(left, right, polygons, checks)?;
            self.pair(right, right, polygons, checks)?;
        } else if second.children.is_none()
            || (first.children.is_some() && first.end - first.start >= second.end - second.start)
        {
            let (left, right) = first.children.unwrap();
            self.pair(left, b, polygons, checks)?;
            self.pair(right, b, polygons, checks)?;
        } else {
            let (left, right) = second.children.unwrap();
            self.pair(a, left, polygons, checks)?;
            self.pair(a, right, polygons, checks)?;
        }
        Ok(())
    }
}

pub(super) fn prove(
    polygons: &[Vec<P2>],
    count: usize,
    checks: &mut usize,
) -> Result<(), FabError> {
    budget_units(checks, count)?;
    let mut segments = Vec::new();
    segments
        .try_reserve_exact(count)
        .map_err(|_| too_complex())?;
    for (polygon, points) in polygons.iter().enumerate() {
        for index in 0..points.len() {
            segments.push(Segment {
                bounds: Bounds::segment(points[index], points[(index + 1) % points.len()]),
                polygon,
                index,
            });
        }
    }
    let mut centers = [(f64::INFINITY, f64::NEG_INFINITY); 2];
    for segment in &segments {
        budget(checks)?;
        for (axis, range) in centers.iter_mut().enumerate() {
            let c = segment.bounds.center(axis);
            range.0 = range.0.min(c);
            range.1 = range.1.max(c);
        }
    }
    let axis = usize::from(centers[1].1 - centers[1].0 > centers[0].1 - centers[0].0);
    budget_units(checks, count)?;
    let mut order = Vec::new();
    order.try_reserve_exact(count).map_err(|_| too_complex())?;
    order.extend(0..count);
    sort(&mut order, &segments, axis, checks)?;
    // Balanced leaves have at least four segments, so this bounds node storage
    // linearly. Recursion depth is logarithmic; no candidate-pair array exists.
    budget(checks)?;
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(count / 2 + 1)
        .map_err(|_| too_complex())?;
    let mut tree = Tree {
        segments,
        order,
        nodes,
    };
    let root = tree.build(0, count, checks)?;
    tree.pair(root, root, polygons, checks)
}
