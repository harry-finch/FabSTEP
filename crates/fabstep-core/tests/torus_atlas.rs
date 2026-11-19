use fabstep_core::torus_atlas::recognize;
use fabstep_core::Mesh;
use std::collections::BTreeMap;

fn ramp(u_steps: usize, v_steps: usize) -> Mesh {
    let mut vertices = Vec::new();
    for base in [false, true] {
        for i in 0..=u_steps {
            let u = 0.2 + 1.9 * i as f64 / u_steps as f64;
            for j in 0..=v_steps {
                let v = 0.12 + 1.23 * j as f64 / v_steps as f64;
                let r = 20. + 1.7 * v.cos();
                vertices.push([
                    if base { -0.8 } else { 1.7 * v.sin() },
                    r * u.cos(),
                    r * u.sin(),
                ]);
            }
        }
    }
    let n = vertices.len() / 2;
    let mut top = Vec::new();
    let mut edges = BTreeMap::new();
    for i in 0..u_steps {
        for j in 0..v_steps {
            if (u_steps / 3..u_steps / 2).contains(&i) && j >= v_steps / 2
                || (2 * u_steps / 3..3 * u_steps / 4).contains(&i) && j < v_steps / 4
            {
                continue;
            }
            let a = i * (v_steps + 1) + j;
            let b = a + v_steps + 1;
            top.extend([[a, b, b + 1], [a, b + 1, a + 1]]);
        }
    }
    for t in &top {
        for j in 0..3 {
            let (a, b) = (t[j], t[(j + 1) % 3]);
            if edges.remove(&(b, a)).is_none() {
                edges.insert((a, b), ());
            }
        }
    }
    let mut triangles = top.clone();
    triangles.extend(top.iter().map(|&[a, b, c]| [c + n, b + n, a + n]));
    for ((a, b), _) in edges {
        triangles.extend([[a, b + n, b], [a, a + n, b + n]]);
    }
    // Source orientation is globally consistent; recognition preserves it.
    Mesh {
        vertices,
        triangles,
    }
}

#[test]
fn notched_torus_is_discovered_from_the_closed_source_at_two_resolutions() {
    for (u, v, angle) in [(60, 36, 0_f64), (72, 40, 0.43_f64)] {
        let mut mesh = ramp(u, v);
        for q in &mut mesh.vertices {
            let (s, c) = angle.sin_cos();
            *q = [
                5. + c * q[0] - s * q[1],
                -3. + s * q[0] + c * q[1],
                q[2] + 8.,
            ];
        }
        let atlas = recognize(&mesh, 0.05).unwrap().unwrap();
        // A rotated working frame can split the same support at the angular seam.
        assert_eq!(atlas.charts.len(), if angle == 0. { 1 } else { 2 });
        let mut owners = std::collections::BTreeSet::new();
        for chart in &atlas.charts {
            assert!((chart.major_radius_mm - 20.).abs() < 1e-4);
            assert!((chart.minor_radius_mm - 1.7).abs() < 1e-4);
            assert!(chart.bound_mm < 0.05);
            assert_eq!(chart.loops.len(), 1);
            assert!(chart.loops[0].len() > 50);
            assert!(chart.triangles.windows(2).all(|q| q[0] < q[1]));
            assert!(chart.triangles.iter().all(|i| owners.insert(*i)));
        }
        assert!(owners.len() > mesh.triangles.len() / 3);
        assert!(recognize(&mesh, 1e-5).unwrap().is_none());
    }
}

#[test]
fn missing_or_duplicate_facets_cannot_create_an_atlas() {
    let mut mesh = ramp(40, 28);
    let missing = mesh.triangles.pop().unwrap();
    assert!(recognize(&mesh, 0.05).unwrap().is_none());
    mesh.triangles.push(missing);
    mesh.triangles.push(missing);
    assert!(recognize(&mesh, 0.05).unwrap().is_none());
}

#[test]
#[ignore = "optional private measured STL, not distributed"]
fn characterize_large_welded_mesh() {
    let path = std::env::var("FABSTEP_LARGE_STL").expect("path to the private Flansch STL");
    let bytes = std::fs::read(path).unwrap();
    let analysis = fabstep_core::analyze_stl(&bytes, &fabstep_core::Options::default()).unwrap();
    assert_eq!(analysis.mesh.triangles.len(), 540_170);
    let atlas = recognize(&analysis.mesh, 0.05).unwrap().unwrap();
    let claimed: usize = atlas.charts.iter().map(|chart| chart.triangles.len()).sum();
    let faces = analysis.mesh.triangles.len() - claimed + atlas.charts.len();
    eprintln!(
        "charts={} claimed={claimed} result_faces={faces}",
        atlas.charts.len()
    );
    assert!(claimed > 440_000 && faces <= 100_000);
}
