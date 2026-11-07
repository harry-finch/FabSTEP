//! Audit de distance bidirectionnel entre deux maillages, sans dépendance CAO.
//! Les sommets, milieux d'arêtes et barycentres sont tous examinés. Ces maxima
//! échantillonnés ne constituent pas une borne de Hausdorff des surfaces continues.
use fabstep_core::{FabError, Mesh};
use serde::Serialize;
use std::collections::BTreeSet;

type Point = [f64; 3];
const MAX_TRIANGLES: usize = 100_000;
const MAX_VERTICES: usize = 300_000;
const MAX_WORK: usize = 50_000_000;

#[derive(Debug, Clone, Serialize)]
pub struct FidelityReport {
    pub source_to_result_mm: f64,
    pub result_to_source_mm: f64,
    pub source_samples: usize,
    pub result_samples: usize,
    /// Paramètre demandé au mailleur CAO ; ce n'est pas une erreur certifiée.
    pub tessellation_deflection_mm: f64,
}

/// Audit déterministe sans sous-échantillonnage silencieux. Un budget dépassé
/// est une erreur explicite, aucun rapport partiel n'est rendu.
pub fn compare_meshes(
    source: &Mesh,
    result: &Mesh,
    tessellation_deflection_mm: f64,
) -> Result<FidelityReport, FabError> {
    if !tessellation_deflection_mm.is_finite() || tessellation_deflection_mm <= 0.0 {
        return Err(invalid(
            "La déflexion de tessellation doit être positive et finie.",
        ));
    }
    let source_tree = Tree::new(source)?;
    let result_tree = Tree::new(result)?;
    let mut work = Work(0);
    let (source_to_result_mm, source_samples) = sample(source, &result_tree, &mut work)?;
    let (result_to_source_mm, result_samples) = sample(result, &source_tree, &mut work)?;
    Ok(FidelityReport {
        source_to_result_mm,
        result_to_source_mm,
        source_samples,
        result_samples,
        tessellation_deflection_mm,
    })
}

fn invalid(message: &str) -> FabError {
    FabError::new("validation.distance_invalid_mesh", message)
}

struct Work(usize);
impl Work {
    fn spend(&mut self) -> Result<(), FabError> {
        self.0 += 1;
        if self.0 > MAX_WORK {
            Err(FabError::new(
                "validation.distance_budget",
                "Budget de comparaison géométrique dépassé ; aucun audit partiel n'est accepté.",
            ))
        } else {
            Ok(())
        }
    }
}

fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn squared(a: Point) -> f64 {
    dot(a, a)
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Point,
    max: Point,
}
impl Bounds {
    fn of(points: impl IntoIterator<Item = Point>) -> Self {
        let mut bounds = Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        };
        for p in points {
            for (i, value) in p.into_iter().enumerate() {
                bounds.min[i] = bounds.min[i].min(value);
                bounds.max[i] = bounds.max[i].max(value);
            }
        }
        bounds
    }
    fn union(self, other: Self) -> Self {
        Self {
            min: std::array::from_fn(|i| self.min[i].min(other.min[i])),
            max: std::array::from_fn(|i| self.max[i].max(other.max[i])),
        }
    }
    fn distance_squared(&self, point: Point) -> f64 {
        squared(std::array::from_fn(|i| {
            (self.min[i] - point[i])
                .max(point[i] - self.max[i])
                .max(0.0)
        }))
    }
}

#[derive(Clone)]
struct Triangle {
    points: [Point; 3],
    bounds: Bounds,
    center: Point,
}
struct Node {
    bounds: Bounds,
    range: std::ops::Range<usize>,
    children: Option<(usize, usize)>,
}
struct Tree {
    triangles: Vec<Triangle>,
    nodes: Vec<Node>,
}
impl Tree {
    fn new(mesh: &Mesh) -> Result<Self, FabError> {
        if mesh.triangles.is_empty()
            || mesh.vertices.is_empty()
            || mesh.triangles.len() > MAX_TRIANGLES
            || mesh.vertices.len() > MAX_VERTICES
        {
            return Err(invalid(
                "Le maillage de comparaison est vide ou dépasse les limites de taille.",
            ));
        }
        if mesh.vertices.iter().flatten().any(|v| !v.is_finite()) {
            return Err(invalid(
                "Le maillage de comparaison contient une coordonnée non finie.",
            ));
        }
        let mut triangles = Vec::with_capacity(mesh.triangles.len());
        let mut used = vec![false; mesh.vertices.len()];
        for indices in &mesh.triangles {
            if indices.iter().any(|&i| i >= mesh.vertices.len()) {
                return Err(invalid(
                    "Un triangle de comparaison référence un sommet absent.",
                ));
            }
            for &i in indices {
                used[i] = true;
            }
            let points = indices.map(|i| mesh.vertices[i]);
            let ab = sub(points[1], points[0]);
            let ac = sub(points[2], points[0]);
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let area = squared(cross);
            if !area.is_finite() || area <= f64::MIN_POSITIVE {
                return Err(invalid("Le maillage de comparaison contient un triangle dégénéré ou hors plage numérique."));
            }
            triangles.push(Triangle {
                bounds: Bounds::of(points),
                center: std::array::from_fn(|i| points[0][i] + ab[i] / 3.0 + ac[i] / 3.0),
                points,
            });
        }
        if used.contains(&false) {
            return Err(invalid(
                "Le maillage de comparaison contient des sommets non référencés.",
            ));
        }
        let mut nodes = Vec::with_capacity(triangles.len() * 2);
        Self::build(&mut triangles, 0, &mut nodes);
        Ok(Self { triangles, nodes })
    }
    fn build(triangles: &mut [Triangle], start: usize, nodes: &mut Vec<Node>) -> usize {
        let bounds = triangles
            .iter()
            .skip(1)
            .fold(triangles[0].bounds, |b, t| b.union(t.bounds));
        let id = nodes.len();
        nodes.push(Node {
            bounds,
            range: start..start + triangles.len(),
            children: None,
        });
        if triangles.len() > 8 {
            // Une facette plane allongée peut dominer la boîte du nœud sans
            // renseigner sur la répartition des triangles à séparer.
            let centers = Bounds::of(triangles.iter().map(|triangle| triangle.center));
            let axis = (0..3)
                .max_by(|&a, &b| {
                    (centers.max[a] - centers.min[a]).total_cmp(&(centers.max[b] - centers.min[b]))
                })
                .unwrap();
            triangles.sort_by(|a, b| a.center[axis].total_cmp(&b.center[axis]));
            let middle = triangles.len() / 2;
            let (a, b) = triangles.split_at_mut(middle);
            let left = Self::build(a, start, nodes);
            let right = Self::build(b, start + middle, nodes);
            nodes[id].children = Some((left, right));
        }
        id
    }
    fn distance(
        &self,
        point: Point,
        work: &mut Work,
        stack: &mut Vec<usize>,
    ) -> Result<f64, FabError> {
        let mut best = f64::INFINITY;
        stack.clear();
        stack.push(0);
        while let Some(id) = stack.pop() {
            work.spend()?;
            let node = &self.nodes[id];
            if node.bounds.distance_squared(point) > best {
                continue;
            }
            if let Some((a, b)) = node.children {
                let da = self.nodes[a].bounds.distance_squared(point);
                let db = self.nodes[b].bounds.distance_squared(point);
                let (near, far) = if da <= db { (a, b) } else { (b, a) };
                stack.push(far);
                stack.push(near);
            } else {
                for tri in &self.triangles[node.range.clone()] {
                    work.spend()?;
                    // La distance à la boîte minore celle au triangle.
                    // Même une boîte écartée consomme son unité de travail.
                    if tri.bounds.distance_squared(point) > best {
                        continue;
                    }
                    let d = triangle_distance_squared(point, tri.points);
                    if !d.is_finite() || d < 0.0 {
                        return Err(invalid(
                            "Le calcul de distance dépasse la précision numérique prise en charge.",
                        ));
                    }
                    best = best.min(d);
                }
            }
        }
        if !best.is_finite() {
            return Err(invalid("Aucune distance finie n'a pu être calculée."));
        }
        Ok(best.sqrt())
    }
}

fn segment_distance_squared(p: Point, a: Point, b: Point) -> f64 {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let t = (dot(ap, ab) / squared(ab)).clamp(0.0, 1.0);
    squared(std::array::from_fn(|i| ap[i] - t * ab[i]))
}

// Projection dans le plan avec tests orientés. Le déterminant de Gram
// soustrait presque deux valeurs égales sur les triangles très fins ; les
// produits vectoriels conservent leur petite hauteur sans cette annulation.
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn triangle_distance_squared(p: Point, [a, b, c]: [Point; 3]) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let n = cross(ab, ac);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let unit = n.map(|x| x / length);
    let distance = dot(ap, unit);
    let bp = sub(p, b);
    let cp = sub(p, c);
    let sides = [
        dot(cross(ab, ap), unit),
        dot(cross(sub(c, b), bp), unit),
        dot(cross(sub(a, c), cp), unit),
    ];
    if sides.iter().any(|x| !x.is_finite()) {
        return f64::NAN;
    }
    if sides.into_iter().all(|x| x >= 0.0) {
        return distance * distance;
    }
    segment_distance_squared(p, a, b)
        .min(segment_distance_squared(p, b, c))
        .min(segment_distance_squared(p, c, a))
}

fn sample(mesh: &Mesh, target: &Tree, work: &mut Work) -> Result<(f64, usize), FabError> {
    let mut stack = Vec::with_capacity(32);
    let mut maximum = 0.0_f64;
    let mut visit = |p| -> Result<(), FabError> {
        maximum = maximum.max(target.distance(p, work, &mut stack)?);
        Ok(())
    };
    for &vertex in &mesh.vertices {
        visit(vertex)?;
    }
    // Le cache est borné à trois entrées par triangle. Une arête parcourue en
    // sens inverse peut donner un arrondi différent : seules les coordonnées
    // identiques bit à bit réutilisent la distance déjà prise en compte.
    let mut midpoints = BTreeSet::new();
    for triangle in &mesh.triangles {
        let [a, b, c] = triangle.map(|i| mesh.vertices[i]);
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let midpoint = std::array::from_fn(|i| p[i] + (q[i] - p[i]) * 0.5);
            if midpoints.insert(midpoint.map(f64::to_bits)) {
                visit(midpoint)?;
            }
        }
        visit(std::array::from_fn(|i| {
            a[i] + (b[i] - a[i]) / 3.0 + (c[i] - a[i]) / 3.0
        }))?;
    }
    // Le rapport compte toujours les mêmes échantillons, doublons compris.
    Ok((maximum, mesh.vertices.len() + 4 * mesh.triangles.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square(z: f64) -> Mesh {
        Mesh {
            vertices: vec![[-2., -2., z], [2., -2., z], [2., 2., z], [-2., 2., z]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }
    }
    #[test]
    fn distances_are_to_triangle_interiors_and_work_after_translation() {
        let mut a = square(0.);
        let mut b = square(2.);
        b.triangles = vec![[0, 1, 3], [1, 2, 3]];
        for mesh in [&mut a, &mut b] {
            for p in &mut mesh.vertices {
                for v in p {
                    *v += 1e8;
                }
            }
        }
        let r = compare_meshes(&a, &b, 0.001).unwrap();
        assert!((r.source_to_result_mm - 2.).abs() < 1e-10);
        assert!((r.result_to_source_mm - 2.).abs() < 1e-10);
        assert_eq!(r.source_samples, 12);
    }
    #[test]
    fn reverse_direction_detects_missing_material_with_coincident_outer_vertices() {
        let full = square(0.);
        let ring = Mesh {
            vertices: vec![
                [-2., -2., 0.],
                [2., -2., 0.],
                [2., 2., 0.],
                [-2., 2., 0.],
                [-1., -1., 0.],
                [1., -1., 0.],
                [1., 1., 0.],
                [-1., 1., 0.],
            ],
            triangles: vec![
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ],
        };
        let r = compare_meshes(&ring, &full, 0.001).unwrap();
        assert!(r.source_to_result_mm < 1e-12);
        assert!((r.result_to_source_mm - 1.).abs() < 1e-12);
    }
    #[test]
    fn thin_triangles_keep_interior_distance_instead_of_distance_to_edges() {
        let source = Mesh {
            vertices: vec![
                [0.75, 2.5e-10, 1e-12],
                [0.750000000001, 2.5e-10, 1e-12],
                [0.75, 2.51e-10, 1e-12],
            ],
            triangles: vec![[0, 1, 2]],
        };
        let target = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1e-9, 0.]],
            triangles: vec![[0, 1, 2]],
        };
        let r = compare_meshes(&source, &target, 1e-6).unwrap();
        assert!((r.source_to_result_mm - 1e-12).abs() < 1e-24);
    }

    #[test]
    fn pruned_search_agrees_with_exhaustive_search_on_translated_thin_triangles() {
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        let origin = 1e8;
        for index in 0..18 {
            let start = mesh.vertices.len();
            let y = origin + f64::from(index) * 0.05;
            let z = origin + f64::from(index % 3) * 0.01;
            mesh.vertices.extend([
                [origin, y, z],
                [origin + 1.0, y + 0.1, z],
                [origin + 1.0, y + 0.100001, z],
            ]);
            mesh.triangles.push([start, start + 1, start + 2]);
        }
        let tree = Tree::new(&mesh).unwrap();
        let mut work = Work(0);
        let mut stack = Vec::new();
        for index in 0..18 {
            let y = origin + f64::from(index) * 0.05;
            let z = origin + f64::from(index % 3) * 0.01;
            for point in [
                [origin + 0.75, y + 0.07500025, z + 0.000002],
                [origin + 1.000001, y + 0.1, z],
                [origin - 0.2, y + 0.1, z + 0.02],
            ] {
                let exhaustive = tree
                    .triangles
                    .iter()
                    .map(|triangle| triangle_distance_squared(point, triangle.points))
                    .fold(f64::INFINITY, f64::min)
                    .sqrt();
                let actual = tree.distance(point, &mut work, &mut stack).unwrap();
                assert_eq!(actual.to_bits(), exhaustive.to_bits(), "{point:?}");
            }
        }
    }

    #[test]
    fn shared_midpoints_reuse_distances_only_when_coordinates_are_identical() {
        let mut source = square(0.);
        let target = Tree::new(&square(2.)).unwrap();
        let mut work = Work(0);
        assert_eq!(sample(&source, &target, &mut work).unwrap(), (2., 12));
        // One leaf and two triangle bounds per query. Only the exact shared
        // diagonal midpoint avoids a query.
        assert_eq!(work.0, 3 * 11);

        source.vertices[0][0] = 0.1;
        source.vertices[2][0] = 0.7;
        let mut work = Work(0);
        assert_eq!(sample(&source, &target, &mut work).unwrap(), (2., 12));
        // The diagonal's two directions round to different x coordinates.
        assert_eq!(work.0, 3 * 12);
    }

    #[test]
    fn exhausted_distance_budget_returns_no_partial_sample_result() {
        let source = square(0.);
        let target = Tree::new(&square(2.)).unwrap();
        let mut work = Work(MAX_WORK - 1);
        // The first tree visit is allowed; its triangle examination exhausts
        // the shared budget. Sampling must return an error, never a maximum
        // computed from an incomplete subset of points.
        let error = sample(&source, &target, &mut work).unwrap_err();
        assert_eq!(error.code, "validation.distance_budget");
        assert!(work.0 > MAX_WORK);
    }

    #[test]
    fn invalid_meshes_fail_before_sampling() {
        let valid = square(0.);
        let mut invalid_mesh = valid.clone();
        invalid_mesh.triangles[0][0] = 999;
        assert!(compare_meshes(&invalid_mesh, &valid, 0.001).is_err());
        invalid_mesh = valid.clone();
        invalid_mesh.vertices[0][0] = f64::NAN;
        assert!(compare_meshes(&valid, &invalid_mesh, 0.001).is_err());
        invalid_mesh = valid.clone();
        invalid_mesh.vertices.push([100., 100., 100.]);
        assert!(compare_meshes(&valid, &invalid_mesh, 0.001).is_err());
        invalid_mesh = valid.clone();
        invalid_mesh.vertices[1] = invalid_mesh.vertices[0];
        assert!(compare_meshes(&valid, &invalid_mesh, 0.001).is_err());
    }
}
