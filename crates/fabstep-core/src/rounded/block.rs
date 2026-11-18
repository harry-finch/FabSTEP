//! Shared bounded six-plane seed and source-disk checks for local blends.
use super::*;

pub(super) fn candidate(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let p = &a.mesh.vertices;
    // Fixed upper bounds apply only to attempting this additional hypothesis.
    if !(10..=8192).contains(&s.model.faces.len()) || a.mesh.triangles.len() > 32_000 {
        return Ok(None);
    }
    let mut top = Vec::<(f64, usize)>::new();
    for (i, plane) in s.model.faces.iter().enumerate() {
        if plane.loops.len() != 1 {
            continue;
        }
        let cycle = &plane.loops[0];
        work.spend(cycle.len() + 1)?;
        let base = p[cycle[0]];
        let area = (0..cycle.len())
            .map(|j| {
                dot(
                    cross(
                        sub(p[cycle[j]], base),
                        sub(p[cycle[(j + 1) % cycle.len()]], base),
                    ),
                    plane.normal,
                )
            })
            .sum::<f64>()
            .abs()
            / 2.;
        top.push((area, i));
        top.sort_by(|x, y| y.0.total_cmp(&x.0).then_with(|| x.1.cmp(&y.1)));
        top.truncate(7);
    }
    if top.len() < 7 || top[5].0 <= 4. * top[6].0 || !top[0].0.is_finite() {
        return Ok(None);
    }
    let mut map = vec![None; s.model.faces.len()];
    for (i, &(_, r)) in top[..6].iter().enumerate() {
        map[r] = Some(i);
    }
    let mut neighbors = [0u8; 6];
    for (t, list) in adj.iter().enumerate() {
        work.spend(1)?;
        let Some(i) = map[s.triangle_regions[t]] else {
            continue;
        };
        for &other in list {
            if let Some(j) = map[s.triangle_regions[other]] {
                if i != j {
                    neighbors[i] |= 1 << j;
                }
            }
        }
    }
    if neighbors.iter().filter(|n| n.count_ones() == 3).count() == 2 {
        linear_blend::candidate(&top, neighbors, a, s, adj, edges, o, work)
    } else {
        octant::candidate(&top, neighbors, a, s, adj, edges, o, work)
    }
}

pub(crate) fn ordered_chain(edges: &[Edge]) -> Result<Vec<usize>, FabError> {
    let mut next = BTreeMap::<usize, Vec<usize>>::new();
    for &(a, b) in edges {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    let ends: Vec<_> = next
        .iter()
        .filter(|(_, v)| v.len() == 1)
        .map(|(&k, _)| k)
        .collect();
    if ends.len() != 2 || next.values().any(|v| v.len() > 2) {
        return Err(unproven("frontière ouverte, fermée ou ramifiée du raccord"));
    }
    let mut chain = vec![ends[0]];
    let mut previous = usize::MAX;
    loop {
        let current = *chain.last().unwrap();
        let Some(&neighbor) = next[&current].iter().find(|&&v| v != previous) else {
            break;
        };
        chain.push(neighbor);
        previous = current;
        if chain.len() > next.len() {
            return Err(unproven("contour de raccord multiple"));
        }
    }
    if chain.len() != edges.len() + 1 || chain.last() != Some(&ends[1]) {
        return Err(unproven("contour de raccord disjoint"));
    }
    Ok(chain)
}

// Check that each face really is a single source disk with one simple
// boundary cycle. Closed source incidence has already been validated. This
// prevents isolated extra islands or pinched seams from contributing chart area.
pub(super) fn disks<const N: usize>(
    a: &Analysis,
    owners: &[usize],
    adj: &[Vec<usize>],
    cuts: &BTreeMap<(usize, usize), Vec<Edge>>,
    work: &mut Work,
) -> Result<(), FabError> {
    let mut tags = vec![false; owners.len()];
    let mut seen = [false; N];
    let mut degrees: [BTreeMap<usize, Vec<usize>>; N] = std::array::from_fn(|_| BTreeMap::new());
    for (&(a, b), edges) in cuts {
        for &face in &[a, b] {
            work.spend(edges.len() * 2)?;
            for &(x, y) in edges {
                degrees[face].entry(x).or_default().push(y);
                degrees[face].entry(y).or_default().push(x);
            }
        }
    }
    for t in 0..owners.len() {
        if tags[t] {
            continue;
        }
        let face = owners[t];
        if std::mem::replace(&mut seen[face], true) {
            return Err(unproven("face source en plusieurs composantes"));
        }
        let boundary = &degrees[face];
        let Some((&start, _)) = boundary.first_key_value() else {
            return Err(unproven("face du raccord sans contour"));
        };
        work.spend(boundary.len() * 2)?;
        if boundary.values().any(|n| n.len() != 2) {
            return Err(unproven("contour source pincé"));
        }
        let mut previous = usize::MAX;
        let mut vertex = start;
        let mut n = 0;
        loop {
            let pair = &boundary[&vertex];
            let next = if pair[0] == previous {
                pair[1]
            } else {
                pair[0]
            };
            previous = vertex;
            vertex = next;
            n += 1;
            if vertex == start {
                break;
            }
            if n >= boundary.len() {
                return Err(unproven("contour source non simple"));
            }
        }
        if n != boundary.len() {
            return Err(unproven("plusieurs contours de face"));
        }
        let mut vertices = BTreeSet::new();
        let mut stack = vec![t];
        tags[t] = true;
        let mut triangles = 0isize;
        while let Some(t) = stack.pop() {
            work.spend(7)?;
            triangles += 1;
            vertices.extend(a.mesh.triangles[t]);
            for &neighbor in &adj[t] {
                if owners[neighbor] == face && !tags[neighbor] {
                    tags[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        let twice_edges = 3 * triangles + n as isize;
        if 2 * vertices.len() as isize - twice_edges + 2 * triangles != 2 {
            return Err(unproven("topologie de face différente d’un disque"));
        }
    }
    if seen.contains(&false) {
        return Err(unproven("face du raccord absente"));
    }
    Ok(())
}

pub(super) fn plane_segments(
    lp: &[usize],
    i: usize,
    expected: usize,
    edges: &BTreeMap<Edge, Vec<usize>>,
    owners: &[usize],
    work: &mut Work,
    make: impl Fn([usize; 2], usize) -> Result<crate::MixedSegment, FabError>,
) -> Result<Vec<crate::MixedSegment>, FabError> {
    work.spend(lp.len() * 3)?;
    let adjacent: Vec<_> = (0..lp.len())
        .map(|k| {
            edges
                .get(&edge(lp[k], lp[(k + 1) % lp.len()]))
                .and_then(|incidence| {
                    incidence
                        .iter()
                        .map(|&t| owners[t])
                        .find(|&other| other != i)
                })
                .ok_or_else(|| unproven("bord plan sans face incidente"))
        })
        .collect::<Result<_, _>>()?;
    let breaks: Vec<_> = (0..lp.len())
        .filter(|&k| adjacent[k] != adjacent[(k + lp.len() - 1) % lp.len()])
        .collect();
    if breaks.len() != expected {
        return Err(unproven("extrémité plane sans le contour attendu"));
    }
    let mut segments = Vec::new();
    for (k, &from) in breaks.iter().enumerate() {
        let to = breaks[(k + 1) % breaks.len()];
        segments.push(make([lp[from], lp[to]], adjacent[from])?);
    }
    Ok(segments)
}
