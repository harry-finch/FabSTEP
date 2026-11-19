//! A last proposal for fragmented planes. Smoothed coordinates exist only
//! while ranking the original triangle seeds. Every output plane is built and checked
//! on the unchanged source before the partition can replace its predecessor.
use super::*;

/// Pin creases and apply one positive/negative Taubin pair. In a closed,
/// consistently wound mesh each directed edge occurs once, so outgoing edges
/// give the uniform one-ring neighbours without a per-vertex set.
fn proposal_normals(mesh: &Mesh, normals: &[V]) -> Vec<V> {
    let mut incident = vec![None; mesh.vertices.len()];
    let mut pinned = vec![false; mesh.vertices.len()];
    let mut degree = vec![0usize; mesh.vertices.len()];
    for (t, &n) in mesh.triangles.iter().zip(normals) {
        for &v in t {
            let first = *incident[v].get_or_insert(n);
            pinned[v] |= !aligned(n, first);
            degree[v] += 1;
        }
    }
    let mut points = mesh.vertices.clone();
    let mut deltas = vec![[0.; 3]; points.len()];
    for factor in [0.5, -0.53] {
        deltas.fill([0.; 3]);
        for &[a, b, c] in &mesh.triangles {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                if !pinned[u] {
                    let offset = sub(points[v], points[u]);
                    for (d, value) in deltas[u].iter_mut().zip(offset) {
                        *d += value;
                    }
                }
            }
        }
        for (v, p) in points.iter_mut().enumerate() {
            if degree[v] > 0 && !pinned[v] {
                let weight = factor / degree[v] as f64;
                for (coord, delta) in p.iter_mut().zip(deltas[v]) {
                    *coord += weight * delta;
                }
            }
        }
    }
    mesh.triangles
        .iter()
        .zip(normals)
        .map(|(t, &source)| {
            let [a, b, c] = t.map(|v| points[v]);
            let n = cross(sub(b, a), sub(c, a));
            let length = norm(n);
            if length.is_finite() && length > 0. && dot(n, source) > 0. {
                scale(n, 1. / length)
            } else {
                source
            }
        })
        .collect()
}

/// Prefer triangles whose normals are least affected by local smoothing,
/// breaking ties by source area and then index. The chosen plane is ALWAYS
/// the normal and first vertex of the original source triangle.
pub(super) fn order_by_stability(mesh: &Mesh, normals: &[V], areas: &[f64], order: &mut [usize]) {
    let scores: Vec<_> = proposal_normals(mesh, normals)
        .into_iter()
        .zip(normals)
        .map(|(smooth, &original)| {
            let s = cross(smooth, original);
            dot(s, s)
        })
        .collect();
    order.sort_unstable_by(|&a, &b| {
        scores[a]
            .total_cmp(&scores[b])
            .then_with(|| areas[b].total_cmp(&areas[a]))
            .then(a.cmp(&b))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Closed grid with an exactly planar rim. Only interior points on the
    // upper face carry sub-micrometre noise. Quantize after tilting the mesh.
    fn noisy_block(amplitude: f64, rounded: bool, phase: f64) -> Mesh {
        let n = 8;
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let id = |i: usize, j: usize, top: usize| top * (n + 1) * (n + 1) + j * (n + 1) + i;
        for top in 0..2 {
            for j in 0..=n {
                for i in 0..=n {
                    let mut z = 0.04 * top as f64;
                    if top == 1 && i > 0 && i < n && j > 0 && j < n {
                        z += amplitude * ((i * 11 + j * 17) as f64 + phase).sin();
                    }
                    let x = i as f64 * 0.06;
                    let y = j as f64 * 0.04;
                    let (s, c) = 0.31f64.sin_cos();
                    let p = [x * c - z * s, y, x * s + z * c];
                    vertices.push(if rounded {
                        p.map(|v| v as f32 as f64)
                    } else {
                        p
                    });
                }
            }
        }
        let mut quad = |a, b, c, d| triangles.extend([[a, b, c], [a, c, d]]);
        for i in 0..n {
            for j in 0..n {
                quad(
                    id(i, j, 1),
                    id(i + 1, j, 1),
                    id(i + 1, j + 1, 1),
                    id(i, j + 1, 1),
                );
                quad(
                    id(i, j, 0),
                    id(i, j + 1, 0),
                    id(i + 1, j + 1, 0),
                    id(i + 1, j, 0),
                );
            }
            quad(id(i, 0, 0), id(i + 1, 0, 0), id(i + 1, 0, 1), id(i, 0, 1));
            quad(id(i, n, 1), id(i + 1, n, 1), id(i + 1, n, 0), id(i, n, 0));
            quad(id(0, i, 1), id(0, i + 1, 1), id(0, i + 1, 0), id(0, i, 0));
            quad(id(n, i, 0), id(n, i + 1, 0), id(n, i + 1, 1), id(n, i, 1));
        }
        Mesh {
            vertices,
            triangles,
        }
    }

    #[test]
    fn stable_seeds_recover_quantized_and_noisy_planes_without_moving_source() {
        for (rounded, amplitude, phase, expected) in
            [(true, 1.75e-7, 2.12, 8), (false, 1.5e-7, 2.12, 7)]
        {
            let mesh = noisy_block(amplitude, rounded, phase);
            let adj = crate::pipeline::validate_topology(&mesh).unwrap();
            let normals = triangle_normals(&mesh);
            let mut checks = 0;
            let first = initial_partition(&mesh, &adj, &normals, 1e-6, &mut checks).unwrap();
            assert_eq!(first.model.faces.len(), expected);
            let (result, recovered) = build_for_output(&mesh, &adj, 1e-6).unwrap();
            let diagnostic = recovered.expect("both input and area order fragment this roof");
            assert_eq!(diagnostic.code, "planar.taubin_seeds_recovered");
            assert_eq!(diagnostic.count, expected - 6);
            assert_eq!(result.model.faces.len(), 6);
            assert_eq!(result.model.vertices, mesh.vertices);
            let owners = &result.triangle_regions;
            let top = owners[0];
            let bottom = owners[2];
            let n = 8;
            // Top and bottom quads are interleaved; four side quads follow
            // each column. No source triangle may cross the nominal edges.
            for column in owners.chunks_exact(n * 4 + 8) {
                for faces in column[..n * 4].chunks_exact(4) {
                    assert_eq!(faces, [top, top, bottom, bottom]);
                }
                assert_eq!(&column[n * 4..], &owners[n * 4..n * 4 + 8]);
            }
            for (t, &owner) in mesh.triangles.iter().zip(owners) {
                let face = &result.model.faces[owner];
                let origin = mesh.vertices[face.loops[0][0]];
                assert!(t
                    .iter()
                    .all(|&v| dot(sub(mesh.vertices[v], origin), face.normal).abs() <= 1e-6));
            }
            assert_eq!(
                proposal_normals(&mesh, &normals),
                proposal_normals(&mesh, &normals)
            );
            let (again, _) = build_for_output(&mesh, &adj, 1e-6).unwrap();
            assert_eq!(result.triangle_regions, again.triangle_regions);
        }
    }

    #[test]
    fn out_of_tolerance_roof_cannot_become_one_plane() {
        let mesh = noisy_block(2e-5, true, 5.3);
        let adj = crate::pipeline::validate_topology(&mesh).unwrap();
        match build_for_output(&mesh, &adj, 1e-6) {
            Ok((result, _)) => assert!(result.model.faces.len() > 6),
            Err(error) => assert_eq!(error.code, "planar.invalid_contour"),
        }
    }
}
