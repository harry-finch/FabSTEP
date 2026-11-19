use super::{Mesh, Point, CLEARANCE};
use crate::{dot, sub};

type Tri = [Point; 3];
fn cross([a, b, c]: Point, [x, y, z]: Point) -> Point {
    [b * z - c * y, c * x - a * z, a * y - b * x]
}
fn length(a: Point) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: Point) -> Point {
    let m = a.into_iter().map(f64::abs).fold(0., f64::max);
    if m > 0. && m.is_finite() {
        a.map(|x| x / m)
    } else {
        [0.; 3]
    }
}
fn normalized(a: Point) -> Point {
    let q = unit(a);
    let len = length(q);
    if len > 0. {
        q.map(|v| v / len)
    } else {
        q
    }
}
fn neg(a: Point) -> Point {
    a.map(|v| -v)
}
fn add(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] + b[i])
}
const ROUND: f64 = 128. * f64::EPSILON;

// The cross product components are differences of two products. The sum
// of absolute products bounds cancellation, including the rounded input
// differences. 128 epsilon exceeds the gamma_n bound for these <16 operations.
fn magnitude_products(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        (a[j] * b[k]).abs() + (a[k] * b[j]).abs()
    })
}
pub(super) fn regular(t: Tri) -> bool {
    let a = sub(t[1], t[0]);
    let b = sub(t[2], t[0]);
    let longest = length(a).max(length(b)).max(length(sub(t[2], t[1])));
    let error = ROUND * magnitude_products(a, b).iter().sum::<f64>() + 1e-200;
    length(cross(a, b)) > 2. * CLEARANCE * longest + error
}

// These axes are only proposed witnesses. Irrespective of how a witness
// was computed, a strict interval gap along that actual axis proves a 3D
// gap. Deltas are rounded differences from a common original vertex.
// The error includes those subtractions, dot products and endpoint bounds.
// 2 exceeds the maximum length sqrt(3) of a max-component-normalized axis.
fn interval_dir(a: Point, d: Point) -> (f64, f64) {
    let p = dot(a, d);
    let err = ROUND * (a[0].abs() + a[1].abs() + a[2].abs()) + 1e-200;
    (p - err, p + err)
}
fn axis_gap(a: &[Point], b: &[Point], axis: Point) -> bool {
    let axis = unit(axis);
    if axis == [0.; 3] {
        return false;
    }
    let interval = |p: &[Point]| {
        p.iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &q| {
                let (l, h) = interval_dir(q, axis);
                (lo.min(l), hi.max(h))
            })
    };
    let (al, ah) = interval(a);
    let (bl, bh) = interval(b);
    al - bh > 2. * CLEARANCE || bl - ah > 2. * CLEARANCE
}
fn edges(t: Tri) -> Tri {
    [sub(t[1], t[0]), sub(t[2], t[1]), sub(t[0], t[2])]
}

fn disjoint(a: Tri, b: Tri) -> bool {
    // Compute candidate axes using original coordinates. All comparisons
    // below use freshly translated input points, never chained differences.
    let ea = edges(a);
    let eb = edges(b);
    let (na, nb) = (cross(ea[0], ea[1]), cross(eb[0], eb[1]));
    let origin = a[0];
    let a = a.map(|p| sub(p, origin));
    let b = b.map(|p| sub(p, origin));
    let gap = |axis| axis_gap(&a, &b, axis);
    if gap(na) || gap(nb) {
        return true;
    }
    // In-plane edge normals cover coplanar separation. Cross products of
    // edges supply the remaining SAT directions for noncoplanar triangles.
    for k in 0..3 {
        if gap(cross(na, ea[k])) || gap(cross(nb, eb[k])) {
            return true;
        }
        if eb.iter().any(|&e| gap(cross(ea[k], e))) {
            return true;
        }
    }
    false
}

fn one_vertex(p: Point, a: [Point; 2], b: [Point; 2]) -> bool {
    let rays = [
        sub(a[0], p),
        sub(a[1], p),
        neg(sub(b[0], p)),
        neg(sub(b[1], p)),
    ];
    let strict = |v: Point| {
        let d = unit(v);
        rays.iter().all(|&r| interval_dir(r, d).0 > 2. * CLEARANCE)
    };
    let q = rays.map(normalized);
    if strict(q.into_iter().fold([0.; 3], add)) {
        return true;
    }
    // A strict separating plane through p exists if the four signed rays
    // lie in an open hemisphere. The closest point of their convex hull
    // to the origin lies on a vertex, edge or triangular face. Try its
    // normal directions, always confirming with the strict interval test.
    for i in 0..4 {
        if strict(q[i]) {
            return true;
        }
        for j in i + 1..4 {
            if strict(add(q[i], q[j])) {
                return true;
            }
            for k in j + 1..4 {
                let n = cross(sub(q[j], q[i]), sub(q[k], q[i]));
                if strict(n) || strict(neg(n)) {
                    return true;
                }
            }
        }
    }
    false
}

// Two disjoint segments can be separated along their common normal, a
// normal from an endpoint to a line, or a line joining two endpoints.
fn segment_gap(a: [Point; 2], b: [Point; 2]) -> bool {
    let ea = sub(a[1], a[0]);
    let eb = sub(b[1], b[0]);
    let origin = a[0];
    let a = a.map(|p| sub(p, origin));
    let b = b.map(|p| sub(p, origin));
    let gap = |d| axis_gap(&a, &b, d);
    if gap(ea) || gap(eb) || gap(cross(ea, eb)) {
        return true;
    }
    for x in a {
        for y in b {
            let delta = sub(x, y);
            if gap(delta) || gap(cross(ea, cross(delta, ea))) || gap(cross(eb, cross(delta, eb))) {
                return true;
            }
        }
    }
    false
}
fn shared_edge(p: Point, q: Point, a: Point, b: Point) -> bool {
    let e = sub(q, p);
    let u = sub(a, p);
    let v = sub(b, p);
    let x = normalized(e);
    let radial = |v| normalized(cross(x, cross(v, x)));
    let witness = unit(cross(sub(radial(u), radial(v)), x));
    // The true plane contains the *exact* edge vector crossed with this
    // chosen witness. Both common vertices are on it even if the float
    // calculation of the normal is not exactly perpendicular to the edge.
    let n = cross(e, witness);
    let perm = magnitude_products(e, witness);
    let side = |v: Point| {
        let h = dot(v, n);
        let err = ROUND * dot(v.map(f64::abs), perm) + 1e-200;
        (h - err, h + err)
    };
    let (lo, _) = side(u);
    let (_, hi) = side(v);
    let margin = 2. * CLEARANCE * length(e) * length(witness);
    lo > margin && hi < -margin && segment_gap([p, a], [q, b]) && segment_gap([q, a], [p, b])
}

pub(super) fn certified(mesh: &Mesh, i: usize, j: usize) -> bool {
    let (a, b) = (mesh.triangles[i], mesh.triangles[j]);
    let shared = a.map(|v| b.contains(&v));
    let p = |id: usize| mesh.vertices[id];
    match shared.into_iter().filter(|&s| s).count() {
        0 => disjoint(a.map(p), b.map(p)),
        1 => {
            let k = shared.iter().position(|&v| v).unwrap();
            let l = b.iter().position(|&id| id == a[k]).unwrap();
            one_vertex(
                p(a[k]),
                [p(a[(k + 1) % 3]), p(a[(k + 2) % 3])],
                [p(b[(l + 1) % 3]), p(b[(l + 2) % 3])],
            )
        }
        2 => {
            let k = shared.iter().position(|&v| !v).unwrap();
            let l = b.iter().position(|id| !a.contains(id)).unwrap();
            shared_edge(p(a[(k + 1) % 3]), p(a[(k + 2) % 3]), p(a[k]), p(b[l]))
        }
        _ => false,
    }
}
