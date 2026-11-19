//! Recheck shared source IDs on the outer wire of every support plane.
use super::*;
use fabstep_core::{CrossDrill, StockFillet};
use std::collections::BTreeSet;

pub(super) fn shared_outline(
    model: &MixedModel,
    f: &CrossDrill,
    corner: StockFillet,
    tolerance: f64,
) -> bool {
    let mut pending = BTreeMap::new();
    let mut matched = BTreeSet::new();
    for (face, plane) in model.planes.iter().enumerate() {
        let Some(MixedLoop::Polygon { vertices }) = plane.loops.first() else {
            return false;
        };
        if !(3..=4096).contains(&vertices.len()) {
            return false;
        }
        for (&a, &b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
            if a == b || a >= model.vertices.len() || b >= model.vertices.len() {
                return false;
            }
            let key = (a.min(b), a.max(b));
            if matched.contains(&key) {
                return false;
            }
            if let Some((prior, first, second)) = pending.remove(&key) {
                if face == prior || a != second || b != first {
                    return false;
                }
                matched.insert(key);
            } else {
                pending.insert(key, (face, a, b));
            }
        }
    }
    let (sy, sz) = (corner.y_sign as f64, corner.z_sign as f64);
    let (cy, cz) = (
        sy * (f.half_lengths[1] - corner.radius_mm),
        sz * (f.half_lengths[2] - corner.radius_mm),
    );
    let mut next = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for (_, (face, a, b)) in pending {
        let tangent =
            (face == if sy < 0. { 1 } else { 4 }) || (face == if sz < 0. { 2 } else { 5 });
        if !(face == 0 || face == 3 || tangent)
            || next.insert(a, b).is_some()
            || !incoming.insert(b)
        {
            return false;
        }
        for id in [a, b] {
            let p = model.vertices[id];
            let coord = |axis: usize| {
                (0..3)
                    .map(|i| (p[i] - f.center[i]) * f.axes[axis][i])
                    .sum::<f64>()
            };
            let (y, z) = (coord(1) - cy, coord(2) - cz);
            if (y.hypot(z) - corner.radius_mm).abs() > tolerance
                || y * sy < -tolerance
                || z * sz < -tolerance
            {
                return false;
            }
        }
    }
    let Some((&start, _)) = next.first_key_value() else {
        return false;
    };
    let mut id = start;
    while let Some(to) = next.remove(&id) {
        id = to;
    }
    id == start && next.is_empty()
}
