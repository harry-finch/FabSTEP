use fabstep_core::{analyze_stl, recognize_height_patch, Options};

const BOSS: &[u8] = include_bytes!("../../../fixtures/height_patch/boss-1.0.stl");
const SADDLE: &[u8] = include_bytes!("../../../fixtures/height_patch/saddle-1.0.stl");

fn level(xy: [f64; 2], interior: [f64; 4]) -> f64 {
    let [u, v] = [(xy[0] + 6.) / 20., (xy[1] - 1.) / 16.];
    let weight = |t: f64| [3. * t * (1. - t).powi(2), 3. * t.powi(2) * (1. - t)];
    let [a, b] = weight(u);
    let [c, d] = weight(v);
    4. + a * (c * interior[0] + d * interior[1]) + b * (c * interior[2] + d * interior[3])
}

#[test]
fn independently_tessellated_reliefs_have_certified_fidelity_owners_and_volume() {
    for (src, interior, volume) in [
        (BOSS, [2., 3., 1., 2.5], 2090.),
        (SADDLE, [1.5, -2., -1.5, 2.], 1920.),
    ] {
        let o = Options::default();
        let a = analyze_stl(src, &o).unwrap();
        let r = recognize_height_patch(&a, &o).unwrap();
        assert_eq!(r.model.origin, [-6., 1., -2.]);
        assert_eq!(r.model.size, [20., 16.]);
        for (i, row) in r.model.heights.iter().enumerate() {
            for (j, &z) in row.iter().enumerate() {
                let offset = if (1..3).contains(&i) && (1..3).contains(&j) {
                    interior[2 * (i - 1) + j - 1]
                } else {
                    0.
                };
                assert!((z - 6. - offset).abs() < 2e-5);
            }
        }
        assert!((r.report.expected_volume_mm3 - volume).abs() < 0.001);
        let bound = r.report.max_deviation_bound_mm;
        assert!(bound <= o.max_deviation_mm && bound > r.report.max_wall_residual_mm);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        let mut count = [0; 6];
        let mut measured = 0_f64;
        for (&tri, &owner) in a.mesh.triangles.iter().zip(&r.triangle_faces) {
            count[owner] += 1;
            let pts = tri.map(|id| a.mesh.vertices[id]);
            if owner < 5 {
                assert!(pts
                    .iter()
                    .all(|p| p[owner / 2] == [-6., 14., 1., 17., -2.][owner]));
            } else {
                for w in [[0.5, 0.5, 0.], [1. / 3.; 3], [0.15, 0.6, 0.25]] {
                    let p: [f64; 3] =
                        std::array::from_fn(|a| (0..3).map(|i| w[i] * pts[i][a]).sum());
                    measured = measured.max((p[2] - level([p[0], p[1]], interior)).abs());
                }
            }
        }
        assert!(count.into_iter().all(|v| v > 0));
        assert!(
            measured > 1e-5 && measured < bound,
            "interior witness {measured}, bound {bound}"
        );
        let strict = Options {
            max_deviation_mm: measured / 2.,
            ..o
        };
        assert_eq!(
            recognize_height_patch(&a, &strict).unwrap_err().code,
            "recognition.deviation_exceeded"
        );
    }
}

#[test]
fn refuses_a_hole_reversed_facet_nonhorizontal_rim_and_incompatible_roof() {
    let o = Options::default();
    let source = analyze_stl(SADDLE, &o).unwrap();
    let mut open = source.clone();
    open.mesh.triangles.pop();
    assert_eq!(
        recognize_height_patch(&open, &o).unwrap_err().code,
        "recognition.height_patch_unproven"
    );
    let mut reversed = source.clone();
    reversed.mesh.triangles[0].swap(0, 1);
    assert_eq!(
        recognize_height_patch(&reversed, &o).unwrap_err().code,
        "recognition.height_patch_unproven"
    );
    for target in [0, 1] {
        let mut a = source.clone();
        let id = a
            .mesh
            .vertices
            .iter()
            .position(|&[x, y, z]| {
                if target == 0 {
                    z == 4. && x == -6.
                } else {
                    x > 2. && x < 6. && y > 7. && y < 11. && z > 3.
                }
            })
            .unwrap();
        a.mesh.vertices[id][2] += 0.125;
        a.model.vertices[id] = a.mesh.vertices[id];
        assert_eq!(
            recognize_height_patch(&a, &o).unwrap_err().code,
            "recognition.height_patch_unproven"
        );
    }
}

/// Four roof rectangles and a single interior roof vertex. Every source facet
/// closes, but several different sets of four interior poles fit its vertices.
fn underdetermined_roof() -> fabstep_core::Mesh {
    let mut vertices = Vec::new();
    for layer in 0..2 {
        for j in 0..3 {
            for i in 0..3 {
                let (x, y) = (i as f64, j as f64);
                let z = if layer == 0 {
                    0.
                } else if (i, j) == (1, 1) {
                    2.5
                } else {
                    2.
                };
                vertices.push([x, y, z]);
            }
        }
    }
    let mut triangles = Vec::new();
    for j in 0..2 {
        for i in 0..2 {
            let p = j * 3 + i;
            for [a, b, c] in [[p, p + 1, p + 4], [p, p + 4, p + 3]] {
                triangles.extend([[c, b, a], [a + 9, b + 9, c + 9]]);
            }
        }
    }
    for side in [0, 1, 2, 5, 8, 7, 6, 3, 0].windows(2) {
        let (a, b) = (side[0], side[1]);
        triangles.extend([[a, b, b + 9], [a, b + 9, a + 9]]);
    }
    fabstep_core::Mesh {
        vertices,
        triangles,
    }
}

#[test]
fn insufficient_interior_samples_and_overhangs_are_refused() {
    let o = Options::default();
    let sparse = fabstep_core::analyze_mesh(&underdetermined_roof(), &o).unwrap();
    assert_eq!(
        recognize_height_patch(&sparse, &o).unwrap_err().code,
        "recognition.height_patch_unproven"
    );
    let source = analyze_stl(SADDLE, &o).unwrap();
    let mut folded = source.clone();
    // Keep a closed, consistently wound abstract mesh and every rim vertex.
    // Move one interior roof vertex laterally past its immediate neighbors.
    let id = source
        .mesh
        .vertices
        .iter()
        .position(|&[x, y, z]| (2.0..6.0).contains(&x) && (7.0..11.0).contains(&y) && z > 3.)
        .unwrap();
    folded.mesh.vertices[id][0] += 5.;
    folded.model.vertices[id] = folded.mesh.vertices[id];
    assert_eq!(
        recognize_height_patch(&folded, &o).unwrap_err().code,
        "recognition.height_patch_unproven"
    );
    let mut tilted = source;
    for v in &mut tilted.mesh.vertices {
        let (sin, cos) = 0.15_f64.sin_cos();
        let [x, y, z] = *v;
        *v = [x, y * cos - z * sin, y * sin + z * cos];
    }
    tilted.model.vertices.clone_from(&tilted.mesh.vertices);
    assert_eq!(
        recognize_height_patch(&tilted, &o).unwrap_err().code,
        "recognition.height_patch_unproven"
    );
}
