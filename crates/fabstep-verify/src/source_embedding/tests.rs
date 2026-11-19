use super::*;
fn pair(vertices: &[Point], a: [usize; 3], b: [usize; 3]) -> Result<(), FabError> {
    certify_source_embedding(&Mesh {
        vertices: vertices.to_vec(),
        triangles: vec![a, b],
    })
}

#[test]
fn unrelated_faces_require_disjoint_continuous_interiors_and_clearance() {
    let common = [[0., 0., 0.], [4., 0., 0.], [0., 4., 0.]];
    for (other, accept) in [
        ([[1., 1., -1.], [1., 1., 1.], [1., 2., 0.]], false), // pierces the interior
        ([[1., 1., 0.], [2., 1., 0.], [1., 2., 0.]], false),  // coplanar containment
        ([[2., 2., 0.], [4., 3., 0.], [3., 4., 0.]], false),  // touches only an edge
        ([[0., 0., 0.], [-2., 0., 1.], [-1., 1., 0.]], false), // unregistered shared position
        ([[3., 3., 0.], [4., 3., 0.], [3., 4., 0.]], true), // overlapping boxes, disjoint triangles
        (
            [
                [1., 1., 0.5 * CLEARANCE],
                [2., 1., 0.5 * CLEARANCE],
                [1., 2., 0.5 * CLEARANCE],
            ],
            false,
        ),
        (
            [
                [1., 1., 3. * CLEARANCE],
                [2., 1., 3. * CLEARANCE],
                [1., 2., 3. * CLEARANCE],
            ],
            true,
        ),
    ] {
        let vertices = [common, other].concat();
        assert_eq!(pair(&vertices, [0, 1, 2], [3, 4, 5]).is_ok(), accept);
        assert_eq!(pair(&vertices, [3, 5, 4], [2, 1, 0]).is_ok(), accept);
    }
}

#[test]
fn a_common_vertex_never_excuses_a_second_contact_or_crossing() {
    let base = [[0., 0., 0.], [4., 0., 0.], [0., 4., 0.]];
    for (others, accept) in [
        ([[-2., -1., 0.], [-1., -2., 0.]], true),
        ([[2., 0., 0.], [0., 0., 1.]], false), // shared half-edge with distinct endpoint IDs
        ([[2., 1., 1.], [1., 2., -1.]], false),
        ([[2., 1., 0.], [1., 2., 0.]], false),
        ([[0., 0., 1.], [1., -2., 0.]], true),
    ] {
        let mut vertices = base.to_vec();
        vertices.extend(others);
        assert_eq!(pair(&vertices, [0, 1, 2], [0, 3, 4]).is_ok(), accept);
    }
}

#[test]
fn common_edge_checks_both_sides_even_if_coplanar_or_nearly_folded() {
    for (third, accept) in [
        ([0.5, -1., 0.], true),
        ([0.5, 0.3, 0.], false),
        ([0.5, 0.3, 0.01 * CLEARANCE], false),
        ([0.5, 0., 1.], true),
        ([0.5, -0.2, -2.], true),
    ] {
        let verts = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], third];
        assert_eq!(pair(&verts, [0, 1, 2], [1, 0, 3]).is_ok(), accept);
    }
    assert!(pair(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        [0, 1, 2],
        [2, 1, 0]
    )
    .is_err());
}

fn octahedron(top: Point) -> Mesh {
    Mesh {
        vertices: vec![
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
            top,
            [0., 0., -1.],
        ],
        triangles: (0..4)
            .flat_map(|i| {
                let j = (i + 1) % 4;
                [[i, j, 4], [j, i, 5]]
            })
            .collect(),
    }
}
#[test]
fn core_topology_can_pass_on_a_self_crossing_closed_octahedron() {
    for (top, valid) in [([0., 0., 1.], true), ([1.2, 0.3, -0.5], false)] {
        let a = fabstep_core::analyze_mesh(&octahedron(top), &Default::default()).unwrap();
        assert_eq!(certify_source_embedding(&a.mesh).is_ok(), valid);
    }
}

#[test]
fn bvh_matches_all_pairs_on_reordered_thin_sheets_and_exhausts_both_limits() {
    let make = |bad: bool| {
        let mut m = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        for i in 0..161 {
            let k = (i * 67) % 161;
            let x = (k % 11) as f64 * 0.71;
            let y = (k / 11) as f64 * 0.63;
            let z = if bad && k == 5 { 4e-3 } else { k as f64 * 1e-3 };
            let j = m.vertices.len();
            m.vertices
                .extend([[x, y, z], [x + 2., y, z], [x, y + 2., z]]);
            m.triangles.push([j, j + 1, j + 2]);
        }
        m
    };
    for bad in [false, true] {
        let mesh = make(bad);
        let exhaustive = (0..mesh.triangles.len())
            .all(|i| (i + 1..mesh.triangles.len()).all(|j| separation::certified(&mesh, i, j)));
        assert_eq!(exhaustive, !bad);
        assert_eq!(certify_source_embedding(&mesh).is_ok(), exhaustive);
    }
    let mesh = octahedron([0., 0., 1.]);
    for (visits, pairs) in [(0, 100), (100, 0)] {
        assert_eq!(
            certify(&mesh, visits, pairs).unwrap_err().code,
            "validation.source_embedding_budget"
        );
    }
}

#[test]
fn unsupported_numeric_inputs_fail_closed() {
    for bad in [f64::NAN, f64::INFINITY, 1e100] {
        let mut m = octahedron([0., 0., 1.]);
        m.vertices[1][1] = bad;
        assert!(certify_source_embedding(&m).is_err());
    }
    assert!(pair(
        &[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        [0, 1, 2],
        [0, 1, 100]
    )
    .is_err());
    assert!(pair(
        &[[0., 0., 0.], [2., 0., 0.], [1., 1e-12, 0.], [0., 1., 0.]],
        [0, 1, 2],
        [1, 0, 3]
    )
    .is_err());
}

#[test]
fn coplanar_predicate_matches_integer_edge_intersections_at_large_offsets() {
    type P = [i64; 2];
    fn orient(a: P, b: P, c: P) -> i64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    let inside = |p: P, t: [P; 3]| {
        let o = std::array::from_fn::<_, 3, _>(|i| orient(t[i], t[(i + 1) % 3], p));
        o.iter().all(|&v| v >= 0) || o.iter().all(|&v| v <= 0)
    };
    let hit = |a: P, b: P, c: P, d: P| {
        let overlap =
            (0..2).all(|k| a[k].max(b[k]) >= c[k].min(d[k]) && c[k].max(d[k]) >= a[k].min(b[k]));
        overlap && orient(a, b, c) * orient(a, b, d) <= 0 && orient(c, d, a) * orient(c, d, b) <= 0
    };
    let mut state = 93721u64;
    let mut coord = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 32) % 41) as i64 - 20
    };
    for _ in 0..2000 {
        let a = std::array::from_fn::<_, 3, _>(|_| [coord(), coord()]);
        let b = std::array::from_fn::<_, 3, _>(|_| [coord(), coord()]);
        if orient(a[0], a[1], a[2]) == 0 || orient(b[0], b[1], b[2]) == 0 {
            continue;
        }
        let intersects = inside(a[0], b)
            || inside(b[0], a)
            || (0..3).any(|i| (0..3).any(|j| hit(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3])));
        let pts = [a, b]
            .concat()
            .into_iter()
            .map(|[x, y]| [x as f64 + 1e10, y as f64 - 1e11, 7.])
            .collect::<Vec<_>>();
        assert_eq!(
            pair(&pts, [0, 1, 2], [3, 4, 5]).is_ok(),
            !intersects,
            "{a:?} {b:?}"
        );
    }
}

#[test]
#[ignore = "Set FABSTEP_LARGE_SOURCE_STL to a local STL; run in release with --nocapture"]
fn measure_local_source_embedding() {
    let path = std::env::var_os("FABSTEP_LARGE_SOURCE_STL").expect("local input path");
    let bytes = std::fs::read(path).unwrap();
    let a = fabstep_core::analyze_stl(&bytes, &Default::default()).unwrap();
    let start = std::time::Instant::now();
    let remaining = certify(&a.mesh, MAX_VISITS, MAX_CANDIDATES).unwrap();
    eprintln!(
        "{} triangles, {} tree/box visits, {} geometric pairs, {:.3} s",
        a.mesh.triangles.len(),
        MAX_VISITS - remaining.visits,
        MAX_CANDIDATES - remaining.pairs,
        start.elapsed().as_secs_f64()
    );
}
