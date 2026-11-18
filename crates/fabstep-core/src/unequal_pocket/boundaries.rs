//! Check every straight edge and its two incidences in the closed source.
use super::*;

pub(super) fn verify(
    part: &Partition,
    d: &UnequalPocket,
    coords: &[V],
    tol: f64,
) -> Result<(f64, usize), FabError> {
    let mut seen = BTreeSet::new();
    let mut vertices = BTreeMap::<usize, V>::new();
    let mut residual = 0_f64;
    let mut line = |owners: Edge, start: V, end: V| -> Result<(), FabError> {
        let key = pair(owners.0, owners.1);
        checked(seen.insert(key))?;
        let path = part.cuts.get(&key).ok_or_else(unsupported)?;
        let axis = unit(sub(end, start)).ok_or_else(unsupported)?;
        let length = norm(sub(end, start));
        let forwards = dot(sub(coords[*path.last().unwrap()], coords[path[0]]), axis) > 0.;
        let mut last = f64::NEG_INFINITY;
        for i in 0..path.len() {
            let v = path[if forwards { i } else { path.len() - 1 - i }];
            let a = if i == 0 {
                0.
            } else if i + 1 == path.len() {
                length
            } else {
                dot(sub(coords[v], start), axis)
            };
            checked(a > last && a <= length)?;
            last = a;
            let target = if i == 0 {
                start
            } else if i + 1 == path.len() {
                end
            } else {
                add(start, scale(axis, a))
            };
            if i == 0 || i + 1 == path.len() {
                if let Some(p) = vertices.insert(v, target) {
                    checked(norm(sub(p, target)) < 1e-10)?;
                }
            }
            residual = residual.max(norm(sub(coords[v], target)));
        }
        checked(residual <= tol)
    };
    let outer = [d.outer_min, d.outer_max];
    for i in 0..3 {
        for j in i + 1..3 {
            let k = 3 - i - j;
            for (ni, p) in outer.iter().enumerate() {
                for (nj, q) in outer.iter().enumerate() {
                    let mut a = d.outer_min;
                    a[i] = p[i];
                    a[j] = q[j];
                    let mut b = a;
                    b[k] = d.outer_max[k];
                    line((2 * i + ni, 2 * j + nj), a, b)?;
                }
            }
        }
    }
    let [x, y] = d.pocket_spans;
    let [small, large] = d.radii;
    let top = d.outer_max[2];
    for (i, span) in d.pocket_spans.into_iter().enumerate() {
        for (sign, base) in [0., span].into_iter().enumerate() {
            let mut a = [0., 0., top];
            a[i] = base;
            let mut b = a;
            b[1 - i] = d.pocket_spans[1 - i];
            line((5, 6 + 2 * i + sign), a, b)?;
        }
    }
    for (i, ax) in [0., x].into_iter().enumerate() {
        for (j, ay) in [0., y].into_iter().enumerate() {
            let h = if i == 0 {
                large
            } else if j == 0 {
                small
            } else {
                0.
            };
            line((6 + i, 8 + j), [ax, ay, h], [ax, ay, top])?;
        }
    }
    line((FLOOR, 7), [x, small, 0.], [x, y, 0.])?;
    line((FLOOR, 9), [large, y, 0.], [x, y, 0.])?;
    line((FLOOR, SMALL), [large, small, 0.], [x, small, 0.])?;
    line((FLOOR, LARGE), [large, small, 0.], [large, y, 0.])?;
    line(
        (8, SMALL),
        [inset_at_angle(small, large, FRAC_PI_2), 0., small],
        [x, 0., small],
    )?;
    line((6, LARGE), [0., 0., large], [0., y, large])?;
    // The wall chart proves these four curved paths and their endpoints.
    for edge in [(7, SMALL), (9, LARGE), (SMALL, LARGE), (8, LARGE)] {
        checked(part.cuts.contains_key(&edge) && seen.insert(edge))?;
    }
    checked(seen.len() == part.cuts.len())?;
    let adjusted = vertices
        .iter()
        .filter(|(v, target)| coords[**v] != **target)
        .count();
    Ok((residual, adjusted))
}
