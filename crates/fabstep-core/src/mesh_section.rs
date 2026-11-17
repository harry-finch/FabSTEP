//! Bounded polygonal sections of indexed source meshes, with no joining by proximity.
use crate::math::{cross, dot, norm, sub, Sum};
use crate::{
    validate_section_profile, validate_section_request, FabError, Mesh, ProfileFrame,
    ProfileSegment, SectionBodyProfile, SectionLoop, SectionProfile, SectionRequest,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TRIANGLES: usize = 100_000;
const MAX_VERTICES: usize = 300_000;
const MAX_LOOPS: usize = 4096;
const MAX_WORK: usize = 5_000_000;
// Keep enough exponent headroom for an exact FMA product residual (up to 106
// significant bits). Smaller nonzero products are refused, never rounded to On.
const MIN_PREDICATE_PRODUCT: f64 = f64::MIN_POSITIVE / (f64::EPSILON * f64::EPSILON);
type P2 = [f64; 2];
fn ambiguous(message: &str) -> FabError {
    FabError::new("dxf.section_ambiguous", message)
}
fn budget() -> FabError {
    FabError::new("dxf.too_complex", "Budget de coupe du maillage dépassé.")
}

struct Work<'a> {
    used: usize,
    segments: usize,
    loops: usize,
    check: &'a mut dyn FnMut() -> Result<(), FabError>,
}
impl Work<'_> {
    fn tick(&mut self) -> Result<(), FabError> {
        self.used += 1;
        if self.used > MAX_WORK {
            return Err(budget());
        }
        if self.used % 512 == 0 {
            (self.check)()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Vertex(usize),
    Edge(usize, usize),
}

// Error-free transforms separate proved on-plane values from uncertainty.
// No distance tolerance classifies or moves a vertex onto the cutting plane.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let value = a + b;
    let bv = value - a;
    (value, (a - (value - bv)) + (b - bv))
}
fn two_diff(a: f64, b: f64) -> (f64, f64) {
    two_sum(a, -b)
}
fn predicate_product(a: f64, b: f64) -> Result<(f64, f64), FabError> {
    let product = a * b;
    if !product.is_finite() || (a != 0. && b != 0. && product.abs() < MIN_PREDICATE_PRODUCT) {
        return Err(ambiguous(
            "Produit géométrique hors plage numérique résolue.",
        ));
    }
    Ok((product, a.mul_add(b, -product)))
}
fn plane_distance(point: [f64; 3], frame: &ProfileFrame) -> Result<f64, FabError> {
    let mut terms = [0.; 3];
    let mut error = 0.;
    for i in 0..3 {
        let (delta, residual) = two_diff(point[i], frame.origin_mm[i]);
        let n = frame.normal[i];
        let (product, product_error) = predicate_product(delta, n)?;
        let (tail, tail_error) = predicate_product(residual, n)?;
        if !delta.is_finite()
            || !product.is_finite()
            || !tail.is_finite()
            || (delta != 0. && n != 0. && product == 0.)
            || (residual != 0. && n != 0. && tail == 0.)
        {
            return Err(ambiguous("Distance au plan hors plage numérique."));
        }
        terms[i] = product;
        error += product_error.abs() + tail.abs() + tail_error.abs();
    }
    let (sum, e1) = two_sum(terms[0], terms[1]);
    let (distance, e2) = two_sum(sum, terms[2]);
    error = (error + e1.abs() + e2.abs()) * 8.;
    if !distance.is_finite()
        || !error.is_finite()
        || (distance.abs() <= error && !(distance == 0. && error == 0.))
    {
        return Err(ambiguous("Le côté du plan est numériquement indéterminé."));
    }
    Ok(distance)
}
fn project(point: [f64; 3], frame: &ProfileFrame) -> Result<P2, FabError> {
    let offset = sub(point, frame.origin_mm);
    let p = [dot(offset, frame.x_axis), dot(offset, frame.y_axis)];
    if p.iter().any(|v| !v.is_finite()) {
        return Err(ambiguous("Coordonnée de coupe hors plage numérique."));
    }
    Ok(p)
}
fn validate_mesh(mesh: &Mesh, work: &mut Work<'_>) -> Result<(), FabError> {
    if mesh.triangles.is_empty() {
        return Err(FabError::new("mesh.empty", "Maillage vide."));
    }
    for p in &mesh.vertices {
        work.tick()?;
        if p.iter().any(|v| !v.is_finite()) {
            return Err(FabError::new("mesh.non_finite", "Coordonnée non finie."));
        }
    }
    let mut unique = BTreeSet::new();
    for triangle in &mesh.triangles {
        work.tick()?;
        if triangle.iter().any(|v| *v >= mesh.vertices.len()) {
            return Err(FabError::new(
                "mesh.invalid_index",
                "Indice de sommet hors du maillage.",
            ));
        }
        let [a, b, c] = triangle.map(|v| mesh.vertices[v]);
        let ab = sub(b, a);
        let ac = sub(c, a);
        let area = norm(cross(ab, ac));
        if !area.is_finite() || area <= norm(ab) * norm(ac) * f64::EPSILON * 32. {
            return Err(FabError::new(
                "mesh.degenerate_triangle",
                "Triangle dégénéré.",
            ));
        }
        let mut sorted = *triangle;
        sorted.sort_unstable();
        if !unique.insert(sorted) {
            return Err(FabError::new(
                "mesh.duplicate_triangle",
                "Triangle dupliqué.",
            ));
        }
    }
    (work.check)()?;
    crate::pipeline::validate_topology_controlled(mesh, work.check)?;
    (work.check)()?;
    Ok(())
}

pub fn section_meshes(
    meshes: &[&Mesh],
    request: &SectionRequest,
) -> Result<SectionProfile, FabError> {
    section_meshes_controlled(meshes, request, &mut || Ok(()))
}

/// Extracts from the full ordered source document in millimetres. Topology is
/// checked here; native spatial validity and document separation remain the
/// application's responsibility before it authorizes an export.
pub fn section_meshes_controlled(
    meshes: &[&Mesh],
    request: &SectionRequest,
    check_current: &mut impl FnMut() -> Result<(), FabError>,
) -> Result<SectionProfile, FabError> {
    check_current()?;
    validate_section_request(request)?;
    if meshes.is_empty() || meshes.len() > 32 {
        return Err(budget());
    }
    if request.solid_ids.iter().any(|id| *id >= meshes.len()) {
        return Err(FabError::new(
            "dxf.invalid_profile",
            "Corps absent du document source.",
        ));
    }
    let (mut triangles, mut vertices) = (0usize, 0usize);
    for mesh in meshes {
        triangles = triangles
            .checked_add(mesh.triangles.len())
            .filter(|n| *n <= MAX_TRIANGLES)
            .ok_or_else(budget)?;
        vertices = vertices
            .checked_add(mesh.vertices.len())
            .filter(|n| *n <= MAX_VERTICES)
            .ok_or_else(budget)?;
    }
    let mut work = Work {
        used: 0,
        segments: 0,
        loops: 0,
        check: check_current,
    };
    // Invalid unselected source bodies invalidate the whole extraction too.
    for mesh in meshes {
        validate_mesh(mesh, &mut work)?;
    }
    let mut bodies = Vec::with_capacity(request.solid_ids.len());
    for &solid_id in &request.solid_ids {
        (work.check)()?;
        let loops = extract(meshes[solid_id], &request.frame, &mut work)?;
        bodies.push(SectionBodyProfile { solid_id, loops });
    }
    (work.check)()?;
    let result = SectionProfile {
        frame: request.frame.clone(),
        bodies,
    };
    validate_section_profile(&result)?;
    (work.check)()?;
    Ok(result)
}

struct Graph {
    points: BTreeMap<Node, P2>,
    adjacent: BTreeMap<Node, Vec<Node>>,
}
impl Graph {
    fn add(&mut self, a: Node, b: Node, work: &mut Work<'_>) -> Result<(), FabError> {
        if a == b || self.points[&a] == self.points[&b] {
            return Err(ambiguous("Segment de coupe dégénéré."));
        }
        work.segments += 1;
        if work.segments > MAX_TRIANGLES {
            return Err(budget());
        }
        for (from, to) in [(a, b), (b, a)] {
            let list = self.adjacent.entry(from).or_default();
            if list.contains(&to) || list.len() >= 2 {
                return Err(ambiguous("Coupe ramifiée ou segment dupliqué."));
            }
            list.push(to);
        }
        Ok(())
    }
    fn crossing(
        &mut self,
        u: usize,
        v: usize,
        mesh: &Mesh,
        distances: &[f64],
        frame: &ProfileFrame,
    ) -> Result<Node, FabError> {
        let (u, v) = (u.min(v), u.max(v));
        let node = Node::Edge(u, v);
        if let std::collections::btree_map::Entry::Vacant(entry) = self.points.entry(node) {
            let (da, db) = (distances[u].abs(), distances[v].abs());
            let scale = da.max(db);
            let t = (da / scale) / (da / scale + db / scale);
            if !t.is_finite() || t <= 0. || t >= 1. {
                return Err(ambiguous(
                    "Intersection trop proche d’un sommet pour être résolue.",
                ));
            }
            let a = mesh.vertices[u];
            let b = mesh.vertices[v];
            let point = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
            entry.insert(project(point, frame)?);
        }
        Ok(node)
    }
}

fn extract(
    mesh: &Mesh,
    frame: &ProfileFrame,
    work: &mut Work<'_>,
) -> Result<Vec<SectionLoop>, FabError> {
    let mut distances = Vec::with_capacity(mesh.vertices.len());
    let mut graph = Graph {
        points: BTreeMap::new(),
        adjacent: BTreeMap::new(),
    };
    for (id, &point) in mesh.vertices.iter().enumerate() {
        work.tick()?;
        let distance = plane_distance(point, frame)?;
        distances.push(distance);
        if distance == 0. {
            graph
                .points
                .insert(Node::Vertex(id), project(point, frame)?);
        }
    }
    let mut plane_edges = BTreeMap::<(usize, usize), Vec<f64>>::new();
    for triangle in &mesh.triangles {
        work.tick()?;
        let on: Vec<_> = triangle
            .iter()
            .copied()
            .filter(|&v| distances[v] == 0.)
            .collect();
        match on.len() {
            3 => {
                return Err(FabError::new(
                    "dxf.section_coplanar",
                    "Le plan contient une face triangulaire source.",
                ))
            }
            2 => {
                let third = *triangle.iter().find(|v| !on.contains(v)).unwrap();
                plane_edges
                    .entry((on[0].min(on[1]), on[0].max(on[1])))
                    .or_default()
                    .push(distances[third]);
            }
            1 => {
                let off: Vec<_> = triangle
                    .iter()
                    .copied()
                    .filter(|&v| distances[v] != 0.)
                    .collect();
                if distances[off[0]].is_sign_positive() != distances[off[1]].is_sign_positive() {
                    let edge = graph.crossing(off[0], off[1], mesh, &distances, frame)?;
                    graph.add(Node::Vertex(on[0]), edge, work)?;
                }
            }
            0 => {
                let mut nodes = Vec::with_capacity(2);
                for (u, v) in [
                    (triangle[0], triangle[1]),
                    (triangle[1], triangle[2]),
                    (triangle[2], triangle[0]),
                ] {
                    if distances[u].is_sign_positive() != distances[v].is_sign_positive() {
                        nodes.push(graph.crossing(u, v, mesh, &distances, frame)?);
                    }
                }
                if nodes.len() == 2 {
                    graph.add(nodes[0], nodes[1], work)?;
                }
            }
            _ => unreachable!(),
        }
    }
    for ((u, v), thirds) in plane_edges {
        work.tick()?;
        if thirds.len() != 2 || thirds[0].is_sign_positive() == thirds[1].is_sign_positive() {
            return Err(ambiguous("Le plan est tangent à une arête source."));
        }
        graph.add(Node::Vertex(u), Node::Vertex(v), work)?;
    }
    for &node in graph.points.keys() {
        work.tick()?;
        if graph
            .adjacent
            .get(&node)
            .is_none_or(|neighbors| neighbors.len() != 2)
        {
            return Err(ambiguous("Contour ouvert, tangent à un sommet ou ramifié."));
        }
    }
    let mut seen = BTreeSet::new();
    let mut polygons = Vec::new();
    for &start in graph.adjacent.keys() {
        work.tick()?;
        if seen.contains(&start) {
            continue;
        }
        work.loops += 1;
        if work.loops > MAX_LOOPS {
            return Err(budget());
        }
        let mut polygon = Vec::new();
        let mut previous = None;
        let mut current = start;
        loop {
            work.tick()?;
            if !seen.insert(current) {
                return Err(ambiguous("Contour de coupe non simple."));
            }
            polygon.push(graph.points[&current]);
            let neighbors = &graph.adjacent[&current];
            let next = if previous == Some(neighbors[0]) {
                neighbors[1]
            } else {
                neighbors[0]
            };
            if next == start {
                break;
            }
            previous = Some(current);
            current = next;
        }
        if polygon.len() < 3 {
            return Err(ambiguous("Contour de coupe dégénéré."));
        }
        polygons.push(polygon);
    }
    classify(polygons, work)
}

fn orientation(a: P2, b: P2, c: P2) -> Result<i8, FabError> {
    let (dx, ex) = two_diff(b[0], a[0]);
    let (dy, ey) = two_diff(b[1], a[1]);
    let (px, epx) = two_diff(c[0], a[0]);
    let (py, epy) = two_diff(c[1], a[1]);
    let (left, left_error) = predicate_product(dx, py)?;
    let (right, right_error) = predicate_product(dy, px)?;
    let (value, ev) = two_diff(left, right);
    let uncertainty = (left.abs() + right.abs()) * f64::EPSILON * 32.;
    if !value.is_finite() || !uncertainty.is_finite() {
        return Err(ambiguous("Orientation de contour hors plage numérique."));
    }
    if value.abs() > uncertainty {
        return Ok(if value > 0. { 1 } else { -1 });
    }
    let products_exact = left_error == 0.
        && right_error == 0.
        && (dx == 0. || py == 0. || left != 0.)
        && (dy == 0. || px == 0. || right != 0.);
    if value == 0. && [ex, ey, epx, epy, ev].iter().all(|v| *v == 0.) && products_exact {
        return Ok(0);
    }
    Err(ambiguous(
        "Orientation de contour numériquement indéterminée.",
    ))
}
fn on_segment(a: P2, b: P2, p: P2) -> bool {
    (0..2).all(|i| p[i] >= a[i].min(b[i]) && p[i] <= a[i].max(b[i]))
}
fn intersects(a: P2, b: P2, c: P2, d: P2) -> Result<bool, FabError> {
    let ab_c = orientation(a, b, c)?;
    let ab_d = orientation(a, b, d)?;
    let cd_a = orientation(c, d, a)?;
    let cd_b = orientation(c, d, b)?;
    Ok(
        (ab_c != 0 && ab_d != 0 && ab_c != ab_d && cd_a != 0 && cd_b != 0 && cd_a != cd_b)
            || (ab_c == 0 && on_segment(a, b, c))
            || (ab_d == 0 && on_segment(a, b, d))
            || (cd_a == 0 && on_segment(c, d, a))
            || (cd_b == 0 && on_segment(c, d, b)),
    )
}
#[derive(Clone, Copy)]
struct Edge {
    a: P2,
    b: P2,
    polygon: usize,
    index: usize,
}
impl Edge {
    fn min(&self, i: usize) -> f64 {
        self.a[i].min(self.b[i])
    }
    fn max(&self, i: usize) -> f64 {
        self.a[i].max(self.b[i])
    }
}
fn prove_edges(polygons: &[Vec<P2>], work: &mut Work<'_>) -> Result<(), FabError> {
    let mut edges = Vec::new();
    for (polygon, points) in polygons.iter().enumerate() {
        for (index, (&a, &b)) in points.iter().zip(points.iter().cycle().skip(1)).enumerate() {
            work.tick()?;
            edges.push(Edge {
                a,
                b,
                polygon,
                index,
            });
        }
    }
    edges.sort_by(|a, b| {
        a.min(0)
            .total_cmp(&b.min(0))
            .then(a.polygon.cmp(&b.polygon))
            .then(a.index.cmp(&b.index))
    });
    (work.check)()?;
    for (i, a) in edges.iter().enumerate() {
        for b in &edges[i + 1..] {
            work.tick()?;
            if b.min(0) > a.max(0) {
                break;
            }
            if b.max(1) < a.min(1) || b.min(1) > a.max(1) {
                continue;
            }
            let count = polygons[a.polygon].len();
            let adjacent = a.polygon == b.polygon
                && ((a.index + 1) % count == b.index || (b.index + 1) % count == a.index);
            if adjacent {
                let (previous, middle, next) = if a.b == b.a {
                    (a.a, a.b, b.b)
                } else {
                    (b.a, b.b, a.b)
                };
                if orientation(previous, middle, next).is_ok_and(|sign| sign != 0) {
                    continue;
                }
                let u = [previous[0] - middle[0], previous[1] - middle[1]];
                let v = [next[0] - middle[0], next[1] - middle[1]];
                let product = u[0] * v[0] + u[1] * v[1];
                if product.is_finite() && product < 0. {
                    continue;
                }
                return Err(ambiguous(
                    "Arêtes de coupe consécutives superposées ou indéterminées.",
                ));
            }
            if intersects(a.a, a.b, b.a, b.b)? {
                return Err(ambiguous(
                    "Contours de coupe en contact ou auto-intersectés.",
                ));
            }
        }
    }
    Ok(())
}
fn area(points: &[P2], work: &mut Work<'_>) -> Result<f64, FabError> {
    let mut sum = Sum::default();
    let mut absolute = Sum::default();
    let origin = points[0];
    for (&a, &b) in points.iter().zip(points.iter().cycle().skip(1)) {
        work.tick()?;
        let a = [a[0] - origin[0], a[1] - origin[1]];
        let b = [b[0] - origin[0], b[1] - origin[1]];
        let term = a[0] * b[1] - a[1] * b[0];
        sum.add(term);
        absolute.add(term.abs());
    }
    let value = sum.get() * 0.5;
    if !value.is_finite() || value.abs() <= absolute.get() * f64::EPSILON * 64. {
        return Err(ambiguous(
            "Aire de coupe nulle ou numériquement indéterminée.",
        ));
    }
    Ok(value)
}
fn inside(point: P2, polygon: &[P2], work: &mut Work<'_>) -> Result<bool, FabError> {
    let mut inside = false;
    for (&a, &b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        work.tick()?;
        if (a[1] > point[1]) != (b[1] > point[1]) {
            let side = orientation(a, b, point)?;
            if side == 0 {
                return Err(ambiguous("Contours de coupe en contact."));
            }
            if (side > 0) == (b[1] > a[1]) {
                inside = !inside;
            }
        }
    }
    Ok(inside)
}
fn classify(mut polygons: Vec<Vec<P2>>, work: &mut Work<'_>) -> Result<Vec<SectionLoop>, FabError> {
    prove_edges(&polygons, work)?;
    let areas: Vec<_> = polygons
        .iter()
        .map(|p| area(p, work))
        .collect::<Result<_, _>>()?;
    let mut bounds = Vec::with_capacity(polygons.len());
    for p in &polygons {
        let mut bbox = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
        for point in p {
            work.tick()?;
            for (i, coordinate) in point.iter().enumerate() {
                bbox[0][i] = bbox[0][i].min(*coordinate);
                bbox[1][i] = bbox[1][i].max(*coordinate);
            }
        }
        bounds.push(bbox);
    }
    let mut parents = vec![None; polygons.len()];
    for i in 0..polygons.len() {
        for j in 0..polygons.len() {
            work.tick()?;
            if i == j
                || (0..2).any(|axis| {
                    polygons[i][0][axis] < bounds[j][0][axis]
                        || polygons[i][0][axis] > bounds[j][1][axis]
                })
            {
                continue;
            }
            if inside(polygons[i][0], &polygons[j], work)? {
                if areas[j].abs() <= areas[i].abs() {
                    return Err(ambiguous("Inclusion des contours de coupe incohérente."));
                }
                if parents[i].is_none_or(|parent: usize| areas[j].abs() < areas[parent].abs()) {
                    parents[i] = Some(j);
                }
            }
        }
    }
    let mut depths = vec![0usize; polygons.len()];
    for i in 0..polygons.len() {
        let mut parent = parents[i];
        while let Some(j) = parent {
            work.tick()?;
            depths[i] += 1;
            if depths[i] >= polygons.len() {
                return Err(ambiguous("Inclusion cyclique des contours."));
            }
            parent = parents[j];
        }
    }
    let outers: Vec<_> = depths
        .iter()
        .enumerate()
        .filter_map(|(i, depth)| (depth % 2 == 0).then_some(i))
        .collect();
    let mut result = Vec::with_capacity(polygons.len());
    for (component_id, &outer) in outers.iter().enumerate() {
        for i in std::iter::once(outer)
            .chain((0..polygons.len()).filter(|&i| parents[i] == Some(outer) && depths[i] % 2 == 1))
        {
            work.tick()?;
            let is_outer = i == outer;
            if (areas[i] > 0.) != is_outer {
                polygons[i].reverse();
            }
            let segments = polygons[i]
                .iter()
                .zip(polygons[i].iter().cycle().skip(1))
                .map(|(&start_mm, &end_mm)| ProfileSegment::Line { start_mm, end_mm })
                .collect();
            result.push(SectionLoop {
                component_id,
                is_outer,
                segments,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        prepare_mesh_bodies_for_sections, prepare_mesh_bodies_for_sections_controlled,
        read_stl_mesh, InputUnit, Options,
    };

    fn cube() -> Mesh {
        Mesh {
            vertices: vec![
                [-1., -1., -1.],
                [1., -1., -1.],
                [1., 1., -1.],
                [-1., 1., -1.],
                [-1., -1., 1.],
                [1., -1., 1.],
                [1., 1., 1.],
                [-1., 1., 1.],
            ],
            triangles: vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ],
        }
    }
    fn request(origin: [f64; 3], normal: [f64; 3]) -> SectionRequest {
        let normal = normal.map(|v| v / norm(normal));
        let least = (0..3)
            .min_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
            .unwrap();
        let mut helper = [0.; 3];
        helper[least] = 1.;
        let x = cross(helper, normal);
        let x = x.map(|v| v / norm(x));
        SectionRequest {
            frame: ProfileFrame {
                origin_mm: origin,
                x_axis: x,
                y_axis: cross(normal, x),
                normal,
            },
            solid_ids: vec![0],
        }
    }
    fn profile_area(p: &SectionProfile) -> f64 {
        let mut total = 0.;
        for body in &p.bodies {
            for wire in &body.loops {
                for segment in &wire.segments {
                    let ProfileSegment::Line {
                        start_mm: a,
                        end_mm: b,
                    } = segment
                    else {
                        panic!("source curve must stay polygonal");
                    };
                    total += (a[0] * b[1] - a[1] * b[0]) * 0.5;
                }
            }
        }
        total
    }
    fn assert_closed(p: &SectionProfile) {
        validate_section_profile(p).unwrap();
        for body in &p.bodies {
            for wire in &body.loops {
                for (a, b) in wire
                    .segments
                    .iter()
                    .zip(wire.segments.iter().cycle().skip(1))
                {
                    let ProfileSegment::Line { end_mm, .. } = a else {
                        panic!();
                    };
                    let ProfileSegment::Line { start_mm, .. } = b else {
                        panic!();
                    };
                    assert_eq!(
                        end_mm, start_mm,
                        "closure must share the same computed node, with no snap"
                    );
                }
            }
        }
    }

    #[test]
    fn axis_negative_and_oblique_cube_sections_are_closed_and_exact() {
        let mesh = cube();
        for normal in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
            let p = section_meshes(&[&mesh], &request([0., 0., 0.], normal)).unwrap();
            assert_closed(&p);
            assert!((profile_area(&p) - 4.).abs() < 1e-12);
            assert_eq!(p.bodies[0].loops.len(), 1);
            assert_eq!(p.bodies[0].loops[0].segments.len(), 8); // source diagonals retained
        }
        let p = section_meshes(&[&mesh], &request([0., 0., -0.5], [0., 0., -1.])).unwrap();
        assert_closed(&p);
        assert!((profile_area(&p) - 4.).abs() < 1e-12);
        let p = section_meshes(&[&mesh], &request([0., 0., 0.], [0.2, 0.3, 1.])).unwrap();
        assert_closed(&p);
        assert!((profile_area(&p) - 4. * 1.13_f64.sqrt()).abs() < 1e-12);
        assert_eq!(
            format!("{p:?}"),
            format!(
                "{:?}",
                section_meshes(&[&mesh], &request([0., 0., 0.], [0.2, 0.3, 1.])).unwrap()
            )
        );
    }

    #[test]
    fn exact_transverse_source_vertices_and_edges_are_supported() {
        let mesh = cube();
        let p = section_meshes(&[&mesh], &request([0., 0., 1.], [1., 1., 1.])).unwrap();
        assert_closed(&p);
        assert!((profile_area(&p) - 2. * 3_f64.sqrt()).abs() < 1e-12);
        let p = section_meshes(&[&mesh], &request([0., 0., 0.], [1., -1., 0.])).unwrap();
        assert_closed(&p);
        assert!((profile_area(&p) - 4. * 2_f64.sqrt()).abs() < 1e-12);
        assert_eq!(p.bodies[0].loops[0].segments.len(), 4);
    }

    #[test]
    fn tangent_vertices_edges_and_coplanar_faces_refuse_the_whole_profile() {
        let mesh = cube();
        for plane in [
            request([1., 1., 1.], [1., 1., 1.]),
            request([1., 1., 0.], [1., 1., 0.]),
        ] {
            assert_eq!(
                section_meshes(&[&mesh], &plane).unwrap_err().code,
                "dxf.section_ambiguous"
            );
        }
        assert_eq!(
            section_meshes(&[&mesh], &request([0., 0., 1.], [0., 0., 1.]))
                .unwrap_err()
                .code,
            "dxf.section_coplanar"
        );
    }

    #[test]
    fn selected_empty_bodies_remain_and_unselected_invalid_bodies_block() {
        let a = cube();
        let mut b = cube();
        for p in &mut b.vertices {
            p[2] += 5.;
        }
        let mut r = request([0., 0., 0.], [0., 0., 1.]);
        r.solid_ids = vec![0, 1];
        let p = section_meshes(&[&a, &b], &r).unwrap();
        assert_eq!(p.bodies.len(), 2);
        assert_eq!(p.bodies[1].solid_id, 1);
        assert!(p.bodies[1].loops.is_empty());
        r.solid_ids = vec![1];
        assert!(section_meshes(&[&a, &b], &r).unwrap().bodies[0]
            .loops
            .is_empty());
        b.triangles.pop();
        r.solid_ids = vec![0];
        assert_eq!(section_meshes(&[&a, &b], &r).unwrap_err().code, "mesh.open");
        r.solid_ids = vec![2];
        assert_eq!(
            section_meshes(&[&a, &b], &r).unwrap_err().code,
            "dxf.invalid_profile"
        );
    }

    #[test]
    fn concave_and_holed_source_fixtures_preserve_polygonal_domains() {
        for (name, bytes, loops) in [
            (
                "concave",
                include_bytes!("../../../fixtures/concave_prism.stl").as_slice(),
                1,
            ),
            (
                "rectangular_hole",
                include_bytes!("../../../fixtures/rectangular_hole.stl").as_slice(),
                2,
            ),
            (
                "tube",
                include_bytes!("../../../fixtures/m2_tube.stl").as_slice(),
                2,
            ),
        ] {
            let mesh = read_stl_mesh(bytes, InputUnit::Mm).unwrap();
            let prepared = prepare_mesh_bodies_for_sections(&mesh, &Options::default()).unwrap();
            let body = &prepared[0];
            let center =
                std::array::from_fn(|i| (body.bounds_mm[0][i] + body.bounds_mm[1][i]) * 0.5);
            let p = section_meshes(&[&body.mesh], &request(center, [0., 0., 1.]))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_closed(&p);
            assert_eq!(p.bodies[0].loops.len(), loops, "{name}");
            assert!(p.bodies[0].loops[0].is_outer);
            assert!(p.bodies[0]
                .loops
                .iter()
                .skip(1)
                .all(|l| !l.is_outer && l.component_id == 0));
        }
    }

    #[test]
    fn containment_forest_keeps_disconnected_islands_and_holes() {
        let square = |r: f64, x: f64| vec![[x - r, -r], [x + r, -r], [x + r, r], [x - r, r]];
        let mut check = || Ok(());
        let mut work = Work {
            used: 0,
            segments: 0,
            loops: 0,
            check: &mut check,
        };
        let loops = classify(
            vec![
                square(5., 0.),
                square(4., 0.),
                square(3., 0.),
                square(1., 0.),
                square(2., 20.),
            ],
            &mut work,
        )
        .unwrap();
        assert_eq!(
            loops
                .iter()
                .map(|l| (l.component_id, l.is_outer))
                .collect::<Vec<_>>(),
            vec![(0, true), (0, false), (1, true), (1, false), (2, true)]
        );
    }

    #[test]
    fn crossings_touching_loops_and_numerical_ambiguity_are_refused() {
        let mut check = || Ok(());
        let mut work = Work {
            used: 0,
            segments: 0,
            loops: 0,
            check: &mut check,
        };
        assert!(classify(
            vec![vec![[0., 0.], [2., 2.], [0., 2.], [2., 0.]]],
            &mut work
        )
        .is_err());
        let mut work = Work {
            used: 0,
            segments: 0,
            loops: 0,
            check: &mut check,
        };
        assert!(classify(
            vec![
                vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
                vec![[0., 1.], [1., 1.], [1., 1.5]]
            ],
            &mut work
        )
        .is_err());
        let frame = request([0., 0., 0.], [0.1, 0.2, 1.]).frame;
        assert!(plane_distance([f64::MAX, f64::MAX, f64::MAX], &frame).is_err());
    }

    #[test]
    fn resource_limits_and_cancellation_are_global_and_propagated() {
        let mesh = cube();
        let request = request([0., 0., 0.], [0., 0., 1.]);
        assert_eq!(
            section_meshes(&vec![&mesh; 33], &request).unwrap_err().code,
            "dxf.too_complex"
        );
        let mut over = mesh.clone();
        over.triangles = vec![[0, 1, 2]; MAX_TRIANGLES + 1];
        assert_eq!(
            section_meshes(&[&over], &request).unwrap_err().code,
            "dxf.too_complex"
        );
        let mut check = || Err(FabError::new("engine.stale", "cancel"));
        assert_eq!(
            section_meshes_controlled(&[&mesh], &request, &mut check)
                .unwrap_err()
                .code,
            "engine.stale"
        );
        assert_eq!(
            prepare_mesh_bodies_for_sections_controlled(&mesh, &Options::default(), &mut check)
                .unwrap_err()
                .code,
            "engine.stale"
        );
        let mut check = || Ok(());
        let mut work = Work {
            used: MAX_WORK,
            segments: 0,
            loops: 0,
            check: &mut check,
        };
        assert_eq!(work.tick().unwrap_err().code, "dxf.too_complex");
        let mut work = Work {
            used: 511,
            segments: 0,
            loops: 0,
            check: &mut || Err(FabError::new("engine.stale", "cancel")),
        };
        assert_eq!(work.tick().unwrap_err().code, "engine.stale");
    }

    #[test]
    fn preparation_preserves_welding_orientation_diagnostics_and_mm_units() {
        let mut raw =
            read_stl_mesh(include_bytes!("../../../fixtures/cube.stl"), InputUnit::Mm).unwrap();
        raw.vertices[0][0] += 1e-8;
        for triangle in &mut raw.triangles {
            triangle.swap(1, 2);
        }
        let options = Options {
            input_unit: InputUnit::Inch,
            ..Options::default()
        };
        let p = prepare_mesh_bodies_for_sections(&raw, &options).unwrap();
        let a = crate::analyze_mesh_bodies(&raw, &options).unwrap();
        assert_eq!(p[0].mesh.vertices, a[0].mesh.vertices);
        assert_eq!(p[0].mesh.triangles, a[0].mesh.triangles);
        assert_eq!(p[0].volume_mm3, a[0].report.volume_mm3);
        assert_eq!(p[0].welded_vertices, a[0].report.welded_vertices);
        assert_eq!(p[0].bounds_mm, a[0].report.bounds_mm);
        assert!(p[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "mesh.vertices_moved"));
        assert!(p[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "mesh.orientation_reversed"));
        assert!(!p[0].diagnostics[0].message.contains("contours contrôlés"));
    }

    #[test]
    fn subnormal_and_tiny_normal_product_cancellation_never_becomes_on_plane() {
        for small in [f64::from_bits(1), 1e-300] {
            let frame = ProfileFrame {
                origin_mm: [0., 0., 0.],
                normal: [small, -small, 1.],
                x_axis: [1., 0., -small],
                y_axis: [0., 1., small],
            };
            crate::validate_profile_frame(&frame).unwrap();
            assert_eq!(
                plane_distance([0.6, 0.7, 0.], &frame).unwrap_err().code,
                "dxf.section_ambiguous"
            );
        }
        assert!(predicate_product(1e-150, 1e-150).is_err());
        assert!(orientation([0., 0.], [1e-150, 1e-150], [2e-150, 3e-150]).is_err());
    }

    #[test]
    fn torus_mesh_can_yield_two_disconnected_components_without_curve_fitting() {
        let raw = read_stl_mesh(
            include_bytes!("../../../fixtures/m5d_ring_torus.stl"),
            InputUnit::Mm,
        )
        .unwrap();
        let prepared = prepare_mesh_bodies_for_sections(&raw, &Options::default()).unwrap();
        let body = &prepared[0];
        let mut center =
            std::array::from_fn(|i| (body.bounds_mm[0][i] + body.bounds_mm[1][i]) * 0.5);
        center[0] += 0.123;
        let p = section_meshes(&[&body.mesh], &request(center, [1., 0., 0.])).unwrap();
        assert_closed(&p);
        assert_eq!(p.bodies[0].loops.len(), 2);
        assert!(p.bodies[0].loops.iter().all(|l| l.is_outer));
        assert_eq!(p.bodies[0].loops[1].component_id, 1);
    }

    #[test]
    fn saddle_vertex_is_not_repaired_into_arbitrary_closed_contours() {
        let mut mesh = Mesh {
            vertices: vec![
                [0., 0., 0.],
                [-1., -1., 1.],
                [1., -1., -1.],
                [1., 1., 1.],
                [-1., 1., -1.],
                [-1., -1., -3.],
                [1., -1., -3.],
                [1., 1., -3.],
                [-1., 1., -3.],
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
                [0, 3, 4],
                [0, 4, 1],
                [5, 7, 6],
                [5, 8, 7],
            ],
        };
        for (a, b) in [(1, 2), (2, 3), (3, 4), (4, 1)] {
            mesh.triangles.extend([[a, a + 4, b + 4], [a, b + 4, b]]);
        }
        crate::pipeline::validate_topology(&mesh).unwrap();
        assert_eq!(
            section_meshes(&[&mesh], &request([0., 0., 0.], [0., 0., 1.]))
                .unwrap_err()
                .code,
            "dxf.section_ambiguous"
        );
    }

    #[test]
    fn all_bodies_and_accumulated_output_limits_are_preserved() {
        let a = cube();
        let mut b = cube();
        for point in &mut b.vertices {
            point[0] += 5.;
        }
        let mut r = request([0., 0., 0.], [0., 0., 1.]);
        r.solid_ids = vec![0, 1];
        let p = section_meshes(&[&a, &b], &r).unwrap();
        assert_eq!(
            p.bodies.iter().map(|b| b.solid_id).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!((profile_area(&p) - 8.).abs() < 1e-12);
        let mut check = || Ok(());
        let mut work = Work {
            used: 0,
            segments: 0,
            loops: MAX_LOOPS,
            check: &mut check,
        };
        assert_eq!(
            extract(&a, &r.frame, &mut work).unwrap_err().code,
            "dxf.too_complex"
        );
        let mut work = Work {
            used: 0,
            segments: MAX_TRIANGLES,
            loops: 0,
            check: &mut check,
        };
        assert_eq!(
            extract(&a, &r.frame, &mut work).unwrap_err().code,
            "dxf.too_complex"
        );
        let mut calls = 0;
        let error =
            prepare_mesh_bodies_for_sections_controlled(&a, &Options::default(), &mut || {
                calls += 1;
                if calls >= 8 {
                    Err(FabError::new("engine.stale", "cancel during preparation"))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(error.code, "engine.stale");
        assert_eq!(calls, 8);
    }

    #[test]
    fn malformed_raw_meshes_fail_before_building_any_section() {
        for kind in [0, 1, 2] {
            let mut mesh = cube();
            match kind {
                0 => mesh.vertices[0][0] = f64::NAN,
                1 => mesh.triangles[0][0] = usize::MAX,
                _ => mesh.triangles[0][0] = mesh.triangles[0][1],
            }
            let error = section_meshes(&[&mesh], &request([0., 0., 0.], [0., 0., 1.])).unwrap_err();
            assert_eq!(
                error.code,
                [
                    "mesh.non_finite",
                    "mesh.invalid_index",
                    "mesh.degenerate_triangle"
                ][kind]
            );
        }
    }
}
