use fabstep_core::{
    analyze_mesh, recognize_automatic, recognize_revolved, recognize_rounded, Mesh, Options,
    ReconstructionMode,
};
use std::f64::consts::{PI, TAU};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    }
}

// Independent triangulation of an annular, piecewise linear axial profile.
// No fitted parameters, generated STEP, or production recognizer is used.
fn annular_profile(profile: &[(f64, f64)]) -> Mesh {
    varied_profile(profile, &vec![96; profile.len() + 2])
}
fn sampled(profile: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for w in profile.windows(2) {
        for i in 0..3 {
            let t = i as f64 / 3.;
            out.push((
                w[0].0 + t * (w[1].0 - w[0].0),
                w[0].1 + t * (w[1].1 - w[0].1),
            ));
        }
    }
    out.push(*profile.last().unwrap());
    out
}
fn volume(profile: &[(f64, f64)]) -> f64 {
    PI * profile
        .windows(2)
        .map(|w| {
            (w[1].0 - w[0].0) * ((w[0].1 * w[0].1 + w[0].1 * w[1].1 + w[1].1 * w[1].1) / 3. - 25.)
        })
        .sum::<f64>()
}

fn varied_profile(profile: &[(f64, f64)], counts: &[usize]) -> Mesh {
    let mut vertices = Vec::new();
    for (level, &(z, radius)) in profile.iter().enumerate() {
        let segments = counts[level];
        for i in 0..segments {
            let u = TAU * i as f64 / segments as f64;
            vertices.push([radius * u.cos(), radius * u.sin(), z]);
        }
    }
    // Close the profile through the actual bore, never via invented caps.
    for (end, z) in [profile.last().unwrap().0, profile[0].0]
        .into_iter()
        .enumerate()
    {
        let segments = counts[profile.len() + end];
        for i in 0..segments {
            let u = TAU * i as f64 / segments as f64;
            vertices.push([5. * u.cos(), 5. * u.sin(), z]);
        }
    }
    let levels = profile.len() + 2;
    let mut triangles = Vec::new();
    let mut offsets = vec![0];
    for &count in counts {
        offsets.push(offsets.last().unwrap() + count);
    }
    for level in 0..levels {
        let next = (level + 1) % levels;
        let (lower, upper) = (offsets[level], offsets[next]);
        let (n, m) = (counts[level], counts[next]);
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            let low = lower + i % n;
            let up = upper + j % m;
            if (i + 1) * m <= (j + 1) * n {
                triangles.push([low, lower + (i + 1) % n, up]);
                i += 1;
            } else {
                triangles.push([low, upper + (j + 1) % m, up]);
                j += 1;
            }
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

#[test]
fn adjacent_cones_require_unique_source_breaks_and_preserve_the_whole_model() {
    for nominal in [
        vec![(0., 10.), (3., 13.), (9., 16.)],
        vec![(0., 20.), (3., 17.), (9., 15.)],
        vec![(0., 10.), (3., 15.), (9., 12.)],
        vec![(0., 10.), (3., 13.), (6., 14.), (9., 17.)],
    ] {
        for transformed in [false, true] {
            let mut mesh = annular_profile(&sampled(&nominal));
            if transformed {
                for p in &mut mesh.vertices {
                    let [x, y, z] = *p;
                    *p = [0.6 * x - 0.8 * z + 27., -y + 14., 0.8 * x + 0.6 * z - 9.];
                }
            }
            for mode in [
                ReconstructionMode::Primitives,
                ReconstructionMode::Surfaces,
                ReconstructionMode::Auto,
            ] {
                let o = Options {
                    reconstruction: mode,
                    ..options()
                };
                let a = analyze_mesh(&mesh, &o).unwrap();
                let (report, vertices, provenance) = if mode == ReconstructionMode::Primitives {
                    let r = recognize_revolved(&a, &o).unwrap();
                    (r.report, r.model.mixed.vertices, r.triangle_faces.len())
                } else {
                    let r = if mode == ReconstructionMode::Auto {
                        recognize_automatic(&a, &o)
                    } else {
                        recognize_rounded(&a, &o)
                    }
                    .unwrap();
                    (
                        r.report,
                        r.model.revolved.mixed.vertices,
                        r.triangle_faces.len(),
                    )
                };
                assert_eq!(report.cones, nominal.len() - 1);
                assert_eq!(report.cylinders, 1);
                assert_eq!(vertices, a.mesh.vertices);
                assert_eq!(provenance, a.mesh.triangles.len());
                assert!((report.expected_volume_mm3 - volume(&nominal)).abs() < 1e-8);
                assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);
            }
        }
    }
}

fn refused(profile: &[(f64, f64)]) {
    for mode in [
        ReconstructionMode::Primitives,
        ReconstructionMode::Surfaces,
        ReconstructionMode::Auto,
    ] {
        let o = Options {
            reconstruction: mode,
            ..options()
        };
        let a = analyze_mesh(&annular_profile(profile), &o).unwrap();
        if mode == ReconstructionMode::Auto {
            let r = recognize_automatic(&a, &o).unwrap();
            assert_eq!(r.report.cones, 0);
            assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        } else {
            let error = if mode == ReconstructionMode::Primitives {
                recognize_revolved(&a, &o).unwrap_err()
            } else {
                recognize_rounded(&a, &o).unwrap_err()
            };
            assert_eq!(
                error.code,
                if mode == ReconstructionMode::Primitives {
                    "recognition.conic_unproven"
                } else {
                    "recognition.surface_unproven"
                }
            );
        }
    }
}

#[test]
fn adjacent_cones_refuse_sparse_curved_or_ambiguous_profiles() {
    let nominal = [(0., 10.), (3., 13.), (9., 16.)];
    refused(&nominal);
    refused(&[(0., 10.), (1.5, 11.5), (3., 13.), (6., 14.5), (9., 16.)]);
    let mut missing = sampled(&nominal);
    missing.remove(missing.len() - 2);
    refused(&missing);
    let mut warped = sampled(&nominal);
    warped[4].1 += 0.05;
    refused(&warped);
    let curved: Vec<_> = (0..10)
        .map(|i| {
            let z = i as f64;
            (z, 10. + 0.01 * z * z)
        })
        .collect();
    refused(&curved);
    // Two distinct source rings can both serve as a breakpoint within fit
    // tolerance. Do not select one by order or by its slightly lower residual.
    refused(&[
        (0., 10.),
        (1., 10.5),
        (2., 11.),
        (3., 11.5),
        (3.00006, 11.500036),
        (4., 12.1),
        (5., 12.7),
        (6., 13.3),
    ]);
    // Resolved cones must have slope intervals separated well beyond noise.
    refused(&sampled(&[(0., 10.), (3., 11.5), (6., 13.00009)]));
}

#[test]
fn adjacent_cones_keep_deviation_budget_and_one_affine_law() {
    let profile = sampled(&[(0., 10.), (3., 13.), (9., 16.)]);
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            max_deviation_mm: 0.001,
            ..options()
        };
        let a = analyze_mesh(&annular_profile(&profile), &o).unwrap();
        let error = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &o).unwrap_err()
        } else {
            recognize_rounded(&a, &o).unwrap_err()
        };
        assert_eq!(error.code, "recognition.deviation_exceeded");
    }
    let profile = sampled(&[(0., 10.), (3., 13.), (6., 16.)]);
    let o = options();
    let a = analyze_mesh(&annular_profile(&profile), &o).unwrap();
    assert_eq!(recognize_revolved(&a, &o).unwrap().report.cones, 1);
}

#[test]
fn adjacent_surfaces_keep_independent_ring_sampling() {
    let profile = sampled(&[(0., 10.), (3., 13.), (9., 16.)]);
    let counts: Vec<_> = (0..profile.len() + 2)
        .map(|i| if i % 3 == 1 { 80 } else { 96 })
        .collect();
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..options()
    };
    let a = analyze_mesh(&varied_profile(&profile, &counts), &o).unwrap();
    assert_eq!(recognize_rounded(&a, &o).unwrap().report.cones, 2);
}

#[test]
fn adjacent_cones_require_a_complete_shared_ring_and_bound_triangle_interiors() {
    let profile = sampled(&[(0., 10.), (3., 13.), (9., 16.)]);
    let o = options();
    let mesh = annular_profile(&profile);
    let a = analyze_mesh(&mesh, &o).unwrap();
    let report = recognize_revolved(&a, &o).unwrap().report;
    for triangle in &mesh.triangles {
        let p = triangle.map(|v| mesh.vertices[v]);
        if p.iter().any(|v| v[0].hypot(v[1]) < 9.) {
            continue; // Source caps/bore have their own unchanged proofs.
        }
        for weights in [[1. / 3.; 3], [0.1, 0.2, 0.7], [0.45, 0.45, 0.1]] {
            let q: [f64; 3] = std::array::from_fn(|i| (0..3).map(|j| weights[j] * p[j][i]).sum());
            let r = if q[2] <= 3. {
                10. + q[2]
            } else {
                13. + (q[2] - 3.) * 0.5
            };
            assert!((q[0].hypot(q[1]) - r).abs() <= report.max_deviation_bound_mm);
        }
    }
    // Still closed and manifold, but the purported shared circular boundary
    // is broken by a real axial vertex displacement. No inferred ring allowed.
    let mut broken = mesh;
    broken.vertices[3 * 96][2] += 0.01;
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..options()
        };
        let a = analyze_mesh(&broken, &o).unwrap();
        let error = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &o).unwrap_err()
        } else {
            recognize_rounded(&a, &o).unwrap_err()
        };
        assert_eq!(
            error.code,
            if mode == ReconstructionMode::Primitives {
                "recognition.conic_unproven"
            } else {
                "recognition.surface_unproven"
            }
        );
    }
}

#[test]
fn adjacent_cones_preserve_concave_wall_orientation() {
    let nominal = [(0., 10.), (3., 13.), (9., 16.)];
    let profile = sampled(&nominal);
    let mut mesh = annular_profile(&profile);
    // Swap the constant R5 bore for a real R25 outer wall; the sampled
    // profile becomes the cavity. Reverse every face to preserve its material side.
    for p in &mut mesh.vertices[profile.len() * 96..] {
        p[0] *= 5.;
        p[1] *= 5.;
    }
    for t in &mut mesh.triangles {
        t.swap(0, 2);
    }
    let expected = PI * 600. * 9. - volume(&nominal);
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..options()
        };
        let a = analyze_mesh(&mesh, &o).unwrap();
        if mode == ReconstructionMode::Primitives {
            let r = recognize_revolved(&a, &o).unwrap();
            assert!(r.model.cones.iter().all(|c| c.inner));
            assert_eq!(r.report.cones, 2);
            assert!((r.report.expected_volume_mm3 - expected).abs() < 1e-8);
        } else {
            let r = recognize_rounded(&a, &o).unwrap();
            assert!(r.model.revolved.cones.iter().all(|c| c.inner));
            assert_eq!(r.report.cones, 2);
            assert!((r.report.expected_volume_mm3 - expected).abs() < 1e-8);
        }
    }
}
