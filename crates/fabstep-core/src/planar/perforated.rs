//! A skinny seed can split a planar face across its hole after STL rounding.
//! Accept the union only if every source triangle fits one fixed plane and
//! every boundary of the complete, connected patch validates on that plane.
use super::*;

pub(super) const MAX_TRIANGLES: usize = 32_000;

pub(super) struct Search<'a> {
    pub mesh: &'a Mesh,
    pub adjacency: &'a [Vec<usize>],
    pub normals: &'a [V],
    pub assigned: &'a [bool],
    pub tolerance: f64,
}

type Patch = (BTreeSet<usize>, PlanarFace, bool);

impl Search<'_> {
    pub fn recover(
        &self,
        region: &BTreeSet<usize>,
        attempted: &mut Vec<bool>,
        checks: &mut usize,
    ) -> Result<Option<Patch>, FabError> {
        let seed = *region.first().expect("region includes its seed");
        if attempted.is_empty() {
            attempted.resize(self.mesh.triangles.len(), false);
        }
        if region.iter().any(|&t| attempted[t]) {
            return Ok(None);
        }

        // Each candidate triangle is visited at most once by recovery. When
        // a previous patch touched the same support, leave the seed alone.
        let mut patch = region.clone();
        let mut pending: VecDeque<_> = region.iter().copied().collect();
        for &t in region {
            attempted[t] = true;
        }
        while let Some(t) = pending.pop_front() {
            for &next in &self.adjacency[t] {
                if !aligned(self.normals[next], self.normals[seed]) || patch.contains(&next) {
                    continue;
                }
                if self.assigned[next] || attempted[next] {
                    return Ok(None);
                }
                attempted[next] = true;
                patch.insert(next);
                pending.push_back(next);
            }
        }
        let Ok(loops) = boundaries(self.mesh, &patch) else {
            return Ok(None);
        };
        if loops.len() < 2 {
            return Ok(None);
        }
        let Some(normal) = region_normal(self.mesh, &patch) else {
            return Ok(None);
        };
        let base = self.mesh.vertices[self.mesh.triangles[seed][0]];
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut extent: f64 = 0.;
        for &t in &patch {
            if !aligned(self.normals[t], normal) {
                return Ok(None);
            }
            for &v in &self.mesh.triangles[t] {
                let offset = sub(self.mesh.vertices[v], base);
                let h = dot(offset, normal);
                lo = lo.min(h);
                hi = hi.max(h);
                extent = extent.max(norm(offset));
            }
        }
        if hi - lo > 2. * self.tolerance {
            return Ok(None);
        }
        let mut loops = match validate_loops(self.mesh, loops, base, normal, checks) {
            Ok(loops) => loops,
            Err(e) if e.code == "planar.invalid_contour" => return Ok(None),
            Err(e) => return Err(e),
        };
        // The native plane starts at the first vertex of the outer contour.
        // Rotate it to the vertex minimizing the greatest point residual.
        let residual = |v: usize| {
            let h = dot(sub(self.mesh.vertices[v], base), normal);
            (hi - h).max(h - lo)
        };
        let (index, &v) = loops[0]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| residual(**a).total_cmp(&residual(**b)))
            .unwrap();
        let origin = self.mesh.vertices[v];
        // Check the actual anchored plane, including all interior vertices.
        let mut measured: f64 = 0.;
        for &t in &patch {
            for &v in &self.mesh.triangles[t] {
                measured = measured.max(dot(sub(self.mesh.vertices[v], origin), normal).abs());
            }
        }
        if measured > self.tolerance {
            return Ok(None);
        }
        loops[0].rotate_left(index);
        let approximate = measured > extent * f64::EPSILON * 128.;
        Ok(Some((patch, PlanarFace { loops, normal }, approximate)))
    }
}

/// A bounded alternative for F when at least two circular mouth loops already
/// exist. Large source triangles provide plane hypotheses. The entire mesh
/// must lie on their material side. Each assigned triangle is checked against
/// the fixed plane, then the full nine-domain proof checks its orientation,
/// adjacency, extent, trim cycles and distance to the reconstructed support.
pub(crate) fn cross_drill_supports(
    mesh: &Mesh,
    tolerance: f64,
    work: &mut crate::recognition::Work,
) -> Result<Option<Segmentation>, FabError> {
    let count = mesh.triangles.len();
    if count > 8192 - 6 || count == 0 {
        return Ok(None);
    }
    let normals = triangle_normals(mesh);
    let mut seeds: Vec<_> = mesh
        .triangles
        .iter()
        .enumerate()
        .map(|(id, t)| {
            let [p, q, r] = t.map(|v| mesh.vertices[v]);
            (norm(cross(sub(q, p), sub(r, p))), id)
        })
        .collect();
    work.spend(count * (count.ilog2() as usize + 3))?;
    seeds.sort_unstable_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut regions = vec![usize::MAX; count];
    let mut faces = Vec::<PlanarFace>::new();
    let mut checks = 0;
    for &(_, id) in seeds.iter().take(64) {
        if regions[id] != usize::MAX {
            continue;
        }
        let origin = mesh.vertices[mesh.triangles[id][0]];
        let n = normals[id];
        if faces.iter().any(|f| dot(f.normal, n) > 1. - 1e-8) {
            continue;
        }
        work.spend(mesh.vertices.len() + 4 * count)?;
        let distance = |v: V| dot(sub(v, origin), n);
        if mesh.vertices.iter().any(|&v| distance(v) > tolerance) {
            continue;
        }
        let region: BTreeSet<_> = mesh
            .triangles
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                dot(normals[*i], n) > 0.9
                    && t.iter()
                        .all(|&v| distance(mesh.vertices[v]).abs() <= tolerance)
            })
            .map(|(i, _)| i)
            .collect();
        if region.len() < 2 || region.iter().any(|i| regions[*i] != usize::MAX) {
            continue;
        }
        let loops = match boundaries(mesh, &region)
            .and_then(|lp| validate_loops(mesh, lp, origin, n, &mut checks))
        {
            Ok(lp) if (1..=2).contains(&lp.len()) => lp,
            Err(e) if e.code == "planar.too_complex" => return Ok(None),
            _ => continue,
        };
        for &i in &region {
            regions[i] = faces.len();
        }
        faces.push(PlanarFace { normal: n, loops });
        if faces.len() == 6 {
            break;
        }
    }
    work.spend(checks)?;
    if faces.len() != 6 || faces.iter().filter(|f| f.loops.len() == 2).count() != 4 {
        return Ok(None);
    }
    for (i, region) in regions.iter_mut().enumerate() {
        if *region == usize::MAX {
            *region = faces.len();
            faces.push(PlanarFace {
                normal: normals[i],
                loops: vec![mesh.triangles[i].to_vec()],
            });
        }
    }
    Ok(Some(Segmentation {
        model: PlanarModel {
            vertices: mesh.vertices.clone(),
            faces,
        },
        approximate_faces: 0,
        triangle_regions: regions,
        checks_used_before_retry: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A triangulated 5×5 grid with square holes. Keep the triangulation
    // inside the patch so that an interior displacement can be tested.
    fn grid(holes: &[(usize, usize)], lift: f64) -> Mesh {
        let id = |x, y| 6 * y + x;
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        for y in 0..=5 {
            for x in 0..=5 {
                let height = if (x, y) == (1, 1) { lift } else { 0. };
                mesh.vertices.push([x as f64, y as f64, height]);
            }
        }
        for y in 0..5 {
            for x in 0..5 {
                if !holes.contains(&(x, y)) {
                    let a = id(x, y);
                    mesh.triangles
                        .extend([[a, a + 1, a + 7], [a, a + 7, a + 6]]);
                }
            }
        }
        mesh
    }

    fn recover(mesh: &Mesh, checks: &mut usize) -> Result<Option<Patch>, FabError> {
        let mut normals = Vec::new();
        let mut adjacency = vec![Vec::new(); mesh.triangles.len()];
        let mut edges: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (t, &[a, b, c]) in mesh.triangles.iter().enumerate() {
            let n = cross(
                sub(mesh.vertices[b], mesh.vertices[a]),
                sub(mesh.vertices[c], mesh.vertices[a]),
            );
            normals.push(scale(n, 1. / norm(n)));
            for (u, v) in [(a, b), (b, c), (c, a)] {
                if let Some(prev) = edges.remove(&(v, u)) {
                    adjacency[t].push(prev);
                    adjacency[prev].push(t);
                } else {
                    edges.insert((u, v), t);
                }
            }
        }
        let ctx = Search {
            mesh,
            normals: &normals,
            adjacency: &adjacency,
            assigned: &vec![false; mesh.triangles.len()],
            tolerance: 1e-6,
        };
        ctx.recover(&BTreeSet::from([0]), &mut Vec::new(), checks)
    }

    #[test]
    fn complete_patch_requires_source_interior_and_separated_boundaries() {
        assert!(recover(&grid(&[(2, 2)], 0.), &mut 0).unwrap().is_some());
        assert!(recover(&grid(&[(2, 2)], 6e-6), &mut 0).unwrap().is_none());
        assert!(recover(&grid(&[(1, 2), (2, 3)], 0.), &mut 0)
            .unwrap()
            .is_none());
        let mut exhausted = MAX_SEGMENT_CHECKS;
        assert_eq!(
            recover(&grid(&[(2, 2)], 0.), &mut exhausted)
                .unwrap_err()
                .code,
            "planar.too_complex"
        );
    }
}
