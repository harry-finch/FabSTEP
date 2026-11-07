use crate::math::*;
use crate::{Analysis, Diagnostic, FabError, InputUnit, Mesh, Options, Report, Severity};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// Cells are twice the weld radius. Keep their indices far below the precision
// limit of f64 so even after rounding, points within one radius occupy the
// same or adjacent cells. This also bounds the packing density of each cell.
const MAX_GRID_INDEX: f64 = (1_u64 << 48) as f64;

#[derive(Default)]
struct WeldStats {
    reused: usize,
    moved: usize,
    max_displacement: f64,
}

/// Analyzes one closed, consistently oriented STL component in millimetres.
/// No hole filling, triangle removal, or per-face vertex projection is performed.
/// Global inversion is corrected. Spatial self-intersection remains the CAD
/// backend's responsibility: this is a topology and planar-contour analysis.
pub fn analyze_stl(bytes: &[u8], options: &Options) -> Result<Analysis, FabError> {
    let raw = read_stl_mm(bytes, options)?;
    analyze_raw(raw, options)
}

/// Bounded STL preview in millimetres; no welding or topology validation.
pub fn read_stl_mesh(bytes: &[u8], input_unit: InputUnit) -> Result<Mesh, FabError> {
    let options = Options {
        input_unit,
        ..Options::default()
    };
    let raw = read_stl_mm(bytes, &options)?;
    validate_raw(&raw, &options)?;
    let triangles = (0..raw.len())
        .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
        .collect();
    Ok(Mesh {
        vertices: raw.into_iter().flatten().collect(),
        triangles,
    })
}

/// Analyze an indexed mesh already expressed in millimetres. `input_unit` is
/// deliberately ignored; all geometric tolerances remain active.
pub fn analyze_mesh(mesh: &Mesh, options: &Options) -> Result<Analysis, FabError> {
    analyze_raw(mesh_triangles(mesh)?, options)
}

/// Split a millimetre mesh into closed edge-connected components. Components
/// may still touch, intersect or nest: the native document validator MUST reject
/// those cases before export. Never combine separate 3MF instances here.
pub fn analyze_mesh_bodies(mesh: &Mesh, options: &Options) -> Result<Vec<Analysis>, FabError> {
    analyze_bodies(mesh_triangles(mesh)?, options)
}

/// STL counterpart retaining the explicit STL input unit, up to 32 bodies.
pub fn analyze_stl_bodies(bytes: &[u8], options: &Options) -> Result<Vec<Analysis>, FabError> {
    analyze_bodies(read_stl_mm(bytes, options)?, options)
}

fn read_stl_mm(bytes: &[u8], options: &Options) -> Result<Vec<[V; 3]>, FabError> {
    let mut raw = crate::stl::read(bytes)?;
    let factor = match options.input_unit {
        InputUnit::Mm => 1.,
        InputUnit::Inch => 25.4,
    };
    for coord in raw.iter_mut().flatten().flatten() {
        *coord *= factor;
    }
    Ok(raw)
}

fn mesh_triangles(mesh: &Mesh) -> Result<Vec<[V; 3]>, FabError> {
    if mesh.triangles.len() > crate::stl::MAX_TRIANGLES || mesh.vertices.len() > 300_000 {
        return Err(FabError::new(
            "input.too_many_triangles",
            "Maillage limité à 100 000 triangles et 300 000 sommets.",
        ));
    }
    if mesh.vertices.iter().flatten().any(|x| !x.is_finite()) {
        return Err(FabError::new("mesh.non_finite", "Coordonnée non finie."));
    }
    mesh.triangles
        .iter()
        .map(|t| {
            if t.iter().any(|&v| v >= mesh.vertices.len()) {
                Err(FabError::new(
                    "mesh.invalid_index",
                    "Indice de sommet hors du maillage.",
                ))
            } else {
                Ok(t.map(|v| mesh.vertices[v]))
            }
        })
        .collect()
}

fn validate_raw(raw: &[[V; 3]], options: &Options) -> Result<(), FabError> {
    for tolerance in [options.weld_tolerance_mm, options.plane_tolerance_mm] {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    if raw.is_empty() {
        return Err(FabError::new(
            "mesh.empty",
            "Le maillage ne contient aucun triangle.",
        ));
    }
    if raw.len() > crate::stl::MAX_TRIANGLES {
        return Err(FabError::new(
            "input.too_many_triangles",
            "Maillage limité à 100 000 triangles.",
        ));
    }
    if raw.iter().flatten().flatten().any(|c| !c.is_finite()) {
        return Err(FabError::new(
            "mesh.non_finite",
            "Coordonnée non finie ou débordement de conversion d'unité.",
        ));
    }
    Ok(())
}

fn analyze_bodies(raw: Vec<[V; 3]>, options: &Options) -> Result<Vec<Analysis>, FabError> {
    validate_raw(&raw, options)?;
    let (mesh, _) = weld(raw.clone(), options.weld_tolerance_mm)?;
    let adjacency = validate_topology_impl(&mesh, false)?;
    let mut seen = vec![false; raw.len()];
    let mut result = Vec::new();
    for start in 0..raw.len() {
        if seen[start] {
            continue;
        }
        if result.len() >= 32 {
            return Err(FabError::new(
                "input.too_many_bodies",
                "Document limité à 32 corps.",
            ));
        }
        let mut pending = VecDeque::from([start]);
        let mut ids = Vec::new();
        seen[start] = true;
        while let Some(i) = pending.pop_front() {
            ids.push(i);
            for &j in &adjacency[i] {
                if !seen[j] {
                    seen[j] = true;
                    pending.push_back(j);
                }
            }
        }
        ids.sort_unstable();
        // Reanalyze original coordinates, not welded representatives, so each
        // body's displacement diagnostics describe the actual source.
        result.push(analyze_raw(ids.iter().map(|&i| raw[i]).collect(), options)?);
    }
    Ok(result)
}

fn analyze_raw(raw: Vec<[V; 3]>, options: &Options) -> Result<Analysis, FabError> {
    validate_raw(&raw, options)?;
    let input_triangles = raw.len();
    let (mut mesh, welded) = weld(raw, options.weld_tolerance_mm)?;
    let adjacency = validate_topology(&mesh)?;
    let bbox = bounds(&mesh.vertices);
    let reference = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let mut volume = Sum::default();
    let mut absolute = Sum::default();
    for tri in &mesh.triangles {
        let term = dot(
            sub(mesh.vertices[tri[0]], reference),
            cross(
                sub(mesh.vertices[tri[1]], reference),
                sub(mesh.vertices[tri[2]], reference),
            ),
        ) / 6.0;
        volume.add(term);
        absolute.add(term.abs());
    }
    let signed_volume = volume.get();
    if !signed_volume.is_finite() || signed_volume.abs() <= absolute.get() * f64::EPSILON * 64.0 {
        return Err(FabError::new(
            "mesh.invalid_volume",
            "Volume nul, ambigu ou hors plage numérique.",
        ));
    }
    let mut diagnostics = vec![Diagnostic {
        code: "validation.native_required".into(), severity: Severity::Info,
        message: "Topologie et contours contrôlés ; la validité géométrique 3D doit être confirmée par le noyau CAO.".into(), count: 1,
    }];
    if welded.moved > 0 {
        diagnostics.push(Diagnostic {
            code: "mesh.vertices_moved".into(),
            severity: Severity::Warning,
            message: format!(
                "Le soudage a déplacé des sommets. Déplacement maximal : {:.6e} mm.",
                welded.max_displacement
            ),
            count: welded.moved,
        });
    }
    if signed_volume < 0.0 {
        for tri in &mut mesh.triangles {
            tri.swap(1, 2);
        }
        diagnostics.push(Diagnostic {
            code: "mesh.orientation_reversed".into(),
            severity: Severity::Info,
            message: "Orientation globale inversée pour obtenir un volume positif.".into(),
            count: input_triangles,
        });
    }
    let (model, approximate_faces) =
        crate::planar::build(&mesh, &adjacency, options.plane_tolerance_mm)?;
    if approximate_faces > 0 {
        diagnostics.push(Diagnostic { code: "planar.approximate".into(), severity: Severity::Warning,
            message: "Des faces ont un résidu de planéité non nul. Sommets conservés ; validation CAO nécessaire.".into(), count: approximate_faces });
    }
    let report = Report {
        input_triangles,
        vertices: mesh.vertices.len(),
        planar_faces: model.faces.len(),
        welded_vertices: welded.reused,
        volume_mm3: signed_volume.abs(),
        bounds_mm: bbox,
        diagnostics,
    };
    Ok(Analysis {
        mesh,
        model,
        report,
    })
}

fn weld(raw: Vec<[[f64; 3]; 3]>, tolerance: f64) -> Result<(Mesh, WeldStats), FabError> {
    let origin = raw.iter().flatten().fold([f64::INFINITY; 3], |mut a, p| {
        for i in 0..3 {
            a[i] = a[i].min(p[i]);
        }
        a
    });
    let mut vertices: Vec<V> = Vec::new();
    let mut triangles = Vec::with_capacity(raw.len());
    let mut cells: BTreeMap<[i64; 3], Vec<usize>> = BTreeMap::new();
    let mut unique = BTreeSet::new();
    let mut welded = WeldStats::default();
    for triangle in raw {
        let mut indices = [0; 3];
        for (i, point) in triangle.into_iter().enumerate() {
            let mut key = [0; 3];
            for j in 0..3 {
                let q = ((point[j] - origin[j]) / tolerance) * 0.5;
                if !q.is_finite() || q >= MAX_GRID_INDEX {
                    return Err(FabError::new(
                        "mesh.coordinate_range",
                        "Étendue des coordonnées trop grande pour la tolérance de soudage.",
                    ));
                }
                key[j] = q.floor() as i64;
            }
            let mut found: Option<(usize, f64)> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(candidates) =
                            cells.get(&[key[0] + dx, key[1] + dy, key[2] + dz])
                        {
                            for &candidate in candidates {
                                let distance = norm(sub(vertices[candidate], point));
                                if distance <= tolerance
                                    && found.is_none_or(|(old, _)| candidate < old)
                                {
                                    found = Some((candidate, distance));
                                }
                            }
                        }
                    }
                }
            }
            indices[i] = match found {
                Some((idx, distance)) => {
                    welded.reused += 1;
                    if distance > 0.0 {
                        welded.moved += 1;
                        welded.max_displacement = welded.max_displacement.max(distance);
                    }
                    idx
                }
                None => {
                    let idx = vertices.len();
                    vertices.push(point);
                    cells.entry(key).or_default().push(idx);
                    idx
                }
            };
        }
        let [a, b, c] = indices;
        let ab = sub(vertices[b], vertices[a]);
        let ac = sub(vertices[c], vertices[a]);
        let area2 = norm(cross(ab, ac));
        if a == b
            || a == c
            || b == c
            || !area2.is_finite()
            || area2 <= norm(ab) * norm(ac) * f64::EPSILON * 32.0
        {
            return Err(FabError::new("mesh.degenerate_triangle", "Triangle dégénéré, initialement ou après soudage. Aucune face supprimée silencieusement."));
        }
        let mut sorted = indices;
        sorted.sort_unstable();
        if !unique.insert(sorted) {
            return Err(FabError::new(
                "mesh.duplicate_triangle",
                "Triangle dupliqué, même avec une orientation différente.",
            ));
        }
        triangles.push(indices);
    }
    Ok((
        Mesh {
            vertices,
            triangles,
        },
        welded,
    ))
}

pub(crate) fn validate_topology(mesh: &Mesh) -> Result<Vec<Vec<usize>>, FabError> {
    validate_topology_impl(mesh, true)
}
fn validate_topology_impl(
    mesh: &Mesh,
    require_connected: bool,
) -> Result<Vec<Vec<usize>>, FabError> {
    let mut edges: BTreeMap<(usize, usize), Vec<(usize, bool)>> = BTreeMap::new();
    let mut stars = vec![Vec::new(); mesh.vertices.len()];
    for (index, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            edges
                .entry((u.min(v), u.max(v)))
                .or_default()
                .push((index, u < v));
        }
        for (v, a, b) in [(a, b, c), (b, c, a), (c, a, b)] {
            stars[v].push((a, b));
        }
    }
    if edges.values().any(|e| e.len() > 2) {
        return Err(FabError::new(
            "mesh.non_manifold_edge",
            "Une arête appartient à plus de deux triangles.",
        ));
    }
    if edges.values().any(|e| e.len() == 1) {
        return Err(FabError::new(
            "mesh.open",
            "Le maillage possède des arêtes ouvertes. Aucune fermeture automatique.",
        ));
    }
    let mut adjacency = vec![Vec::new(); mesh.triangles.len()];
    for edge in edges.values() {
        if edge[0].1 == edge[1].1 {
            return Err(FabError::new(
                "mesh.inconsistent_orientation",
                "Deux triangles voisins parcourent une arête dans le même sens.",
            ));
        }
        adjacency[edge[0].0].push(edge[1].0);
        adjacency[edge[1].0].push(edge[0].0);
    }
    let mut seen = vec![false; mesh.triangles.len()];
    let mut pending = VecDeque::from([0]);
    seen[0] = true;
    while let Some(face) = pending.pop_front() {
        for &next in &adjacency[face] {
            if !seen[next] {
                seen[next] = true;
                pending.push_back(next);
            }
        }
    }
    if require_connected && seen.iter().any(|s| !s) {
        return Err(FabError::new(
            "mesh.multiple_components",
            "Ce jalon accepte un seul composant fermé connecté par ses arêtes.",
        ));
    }
    // Edge manifoldness alone does not exclude a pinched (bow-tie) vertex.
    for star in stars {
        let mut link: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (a, b) in star {
            link.entry(a).or_default().push(b);
            link.entry(b).or_default().push(a);
        }
        if link.values().any(|n| n.len() != 2) {
            return Err(vertex_error());
        }
        let start = *link.keys().next().ok_or_else(vertex_error)?;
        let mut visited = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(v) = pending.pop() {
            if visited.insert(v) {
                pending.extend(&link[&v]);
            }
        }
        if visited.len() != link.len() {
            return Err(vertex_error());
        }
    }
    Ok(adjacency)
}
fn vertex_error() -> FabError {
    FabError::new(
        "mesh.non_manifold_vertex",
        "Le voisinage d'un sommet ne forme pas un disque topologique.",
    )
}
