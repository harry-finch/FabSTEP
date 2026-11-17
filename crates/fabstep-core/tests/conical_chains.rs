use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_revolved, recognize_rounded, Mesh,
    Options, ReconstructionMode,
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
const THREE: [(f64, f64); 6] = [
    (0., 12.),
    (3., 15.),
    (9., 15.),
    (12., 11.),
    (18., 11.),
    (21., 14.),
];
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
fn extended_chains_preserve_volume_provenance_and_explicit_intent_in_both_modes() {
    for terminal in [false, true] {
        let mut nominal = THREE.to_vec();
        if terminal {
            nominal.insert(0, (-3., 12.));
            nominal.push((24., 14.));
        }
        for transformed in [false, true] {
            let mut mesh = annular_profile(&sampled(&nominal));
            if transformed {
                for p in &mut mesh.vertices {
                    let [x, y, z] = *p;
                    *p = [0.6 * x - 0.8 * z + 27., -y + 14., 0.8 * x + 0.6 * z - 9.];
                }
            }
            let o = options();
            let a = analyze_mesh(&mesh, &o).unwrap();
            let r = recognize_revolved(&a, &o).unwrap();
            assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            let o = Options {
                reconstruction: ReconstructionMode::Surfaces,
                ..o
            };
            let rounded = recognize_rounded(&a, &o).unwrap();
            assert_eq!(rounded.model.revolved.mixed.vertices, a.mesh.vertices);
            assert_eq!(rounded.triangle_faces.len(), a.mesh.triangles.len());
            for report in [&r.report, &rounded.report] {
                assert_eq!(report.cones, 3);
                assert_eq!(report.cylinders, if terminal { 5 } else { 3 });
                assert_eq!(report.spheres + report.tori, 0);
                assert!((report.expected_volume_mm3 - volume(&nominal)).abs() < 1e-8);
                assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);
                assert!(report
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "recognition.conic_intent_assumed" && d.count == 3));
            }
        }
    }
}

#[test]
fn surfaces_proves_independently_sampled_source_rings() {
    for nominal in [
        THREE.to_vec(),
        vec![(0., 10.), (3., 13.), (9., 13.), (12., 16.)],
    ] {
        let profile = sampled(&nominal);
        let counts: Vec<_> = (0..profile.len() + 2)
            .map(|i| if i % 3 == 1 { 80 } else { 96 })
            .collect();
        let mesh = varied_profile(&profile, &counts);
        let o = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..options()
        };
        let a = analyze_mesh(&mesh, &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap();
        assert_eq!(r.report.cones, nominal.len().div_ceil(2));
        assert!((r.report.expected_volume_mm3 - volume(&nominal)).abs() < 1e-8);
    }
}

fn refused(profile: &[(f64, f64)]) {
    let mesh = annular_profile(profile);
    let o = options();
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert_eq!(
        recognize_revolved(&a, &o).unwrap_err().code,
        "recognition.conic_unproven"
    );
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..o
    };
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.surface_unproven"
    );
}

#[test]
fn missing_interior_evidence_and_late_unresolved_breaks_are_blocking() {
    refused(&THREE); // Endpoint-only tessellation cannot witness each run.
    let mut one_midpoint = Vec::new();
    for w in THREE.windows(2) {
        one_midpoint.push(w[0]);
        one_midpoint.push(((w[0].0 + w[1].0) / 2., (w[0].1 + w[1].1) / 2.));
    }
    one_midpoint.push(THREE[5]);
    refused(&one_midpoint); // One midpoint must not count as two witnesses.
    let mut missing_last_witness = sampled(&THREE);
    missing_last_witness.remove(missing_last_witness.len() - 2);
    refused(&missing_last_witness);
    let mut terminal = THREE.to_vec();
    terminal.push((24., 14.));
    let mut terminal_missing_witness = sampled(&terminal);
    terminal_missing_witness.remove(terminal_missing_witness.len() - 2);
    refused(&terminal_missing_witness);
    let mut non_affine = sampled(&THREE);
    non_affine[7].1 += 0.05;
    refused(&non_affine);
    // Four cones: failure on the final land must not be hidden by earlier pairs.
    let four = [
        (0., 12.),
        (3., 15.),
        (9., 15.),
        (12., 11.),
        (18., 11.),
        (21., 14.),
        (21.00015, 14.),
        (24., 10.),
    ];
    // This land clears source-ring spacing, but fails the slope margin
    // against the last shallow cone.
    let mut shallow = four;
    shallow[7].1 = 13.5;
    refused(&sampled(&shallow));
    let mut smooth = sampled(&THREE);
    for p in &mut smooth {
        p.1 = 12. + 0.01 * p.0 * p.0;
    }
    refused(&smooth);
}

#[test]
fn witnessed_same_sign_chains_preserve_source_volume_and_orientation() {
    for nominal in [
        vec![(0., 10.), (3., 13.), (9., 13.), (12., 16.)],
        vec![(0., 20.), (3., 17.), (9., 17.), (12., 14.)],
        vec![
            (0., 12.),
            (3., 15.),
            (9., 15.),
            (12., 18.),
            (18., 18.),
            (21., 14.),
        ],
    ] {
        let cones = nominal.windows(2).filter(|w| w[0].1 != w[1].1).count();
        for terminals in [false, true] {
            let mut profile = nominal.clone();
            if terminals {
                profile.insert(0, (-3., profile[0].1));
                let last = *profile.last().unwrap();
                profile.push((last.0 + 3., last.1));
            }
            for transformed in [false, true] {
                let mut mesh = annular_profile(&sampled(&profile));
                if transformed {
                    for p in &mut mesh.vertices {
                        let [x, y, z] = *p;
                        // Rotation, reflection and translation; the decreasing case
                        // also exercises the opposite axial slope signs.
                        *p = [0.6 * x - 0.8 * z + 27., -y + 14., 0.8 * x + 0.6 * z - 9.];
                    }
                }
                let o = options();
                let a = analyze_mesh(&mesh, &o).unwrap();
                let revolved = recognize_revolved(&a, &o).unwrap();
                let rounded = recognize_rounded(
                    &a,
                    &Options {
                        reconstruction: ReconstructionMode::Surfaces,
                        ..o.clone()
                    },
                )
                .unwrap();
                let automatic = recognize_automatic(
                    &a,
                    &Options {
                        reconstruction: ReconstructionMode::Auto,
                        ..o.clone()
                    },
                )
                .unwrap();
                assert_eq!(revolved.model.mixed.vertices, a.mesh.vertices);
                assert_eq!(rounded.model.revolved.mixed.vertices, a.mesh.vertices);
                assert_eq!(automatic.model.revolved.mixed.vertices, a.mesh.vertices);
                assert_eq!(revolved.triangle_faces.len(), a.mesh.triangles.len());
                assert_eq!(rounded.triangle_faces.len(), a.mesh.triangles.len());
                assert_eq!(automatic.triangle_faces.len(), a.mesh.triangles.len());
                for report in [&revolved.report, &rounded.report, &automatic.report] {
                    assert_eq!(report.cones, cones);
                    assert_eq!(report.cylinders, cones + usize::from(terminals) * 2);
                    assert!((report.expected_volume_mm3 - volume(&profile)).abs() < 1e-8);
                    assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);
                }
            }
        }
    }
}

#[test]
fn same_sign_chain_requires_witnesses_and_keeps_adjacent_cones_unproven() {
    let nominal = [(0., 10.), (3., 13.), (9., 13.), (12., 16.)];
    refused(&nominal);
    let mut midpoint_only = Vec::new();
    for w in nominal.windows(2) {
        midpoint_only.push(w[0]);
        midpoint_only.push(((w[0].0 + w[1].0) / 2., (w[0].1 + w[1].1) / 2.));
    }
    midpoint_only.push(nominal[3]);
    refused(&midpoint_only);
    let mut missing = sampled(&nominal);
    missing.remove(missing.len() - 2);
    refused(&missing);
    let mut non_affine = sampled(&nominal);
    non_affine[7].1 += 0.05;
    refused(&non_affine);
    // No source land exists at the breakpoint: never split this meridian
    // opportunistically into cones, even with interior affine samples.
    refused(&sampled(&[(0., 10.), (3., 13.), (9., 16.)]));
    let mut smooth = sampled(&nominal);
    for p in &mut smooth {
        p.1 = 10. + 0.01 * p.0 * p.0;
    }
    refused(&smooth);
    // The last land exists but does not distinguish the shallow adjacent
    // cone from tolerance-scale radial variation. Earlier valid lands must
    // not hide this late ambiguity.
    let late = [
        (0., 10.),
        (3., 13.),
        (9., 13.),
        (12., 16.),
        (12.00015, 16.),
        (15., 16.5),
    ];
    refused(&sampled(&late));

    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..options()
    };
    for profile in [&nominal[..], &non_affine[..], &smooth[..]] {
        let a = analyze_mesh(&annular_profile(profile), &o).unwrap();
        let r = recognize_automatic(&a, &o).unwrap();
        assert_eq!(r.report.cones, 0);
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    }
}

#[test]
fn historical_opposed_pair_keeps_its_endpoint_only_source_contract() {
    let nominal = [(0., 10.), (3., 13.), (9., 13.), (12., 10.)];
    let o = options();
    let a = analyze_mesh(&annular_profile(&nominal), &o).unwrap();
    assert_eq!(recognize_revolved(&a, &o).unwrap().report.cones, 2);
    assert_eq!(
        recognize_rounded(
            &a,
            &Options {
                reconstruction: ReconstructionMode::Surfaces,
                ..o
            }
        )
        .unwrap()
        .report
        .cones,
        2
    );
}

#[test]
fn same_sign_chain_cap_and_deviation_budget_are_unchanged() {
    let mut profile = vec![(0., 10.)];
    for i in 0..9 {
        let (z, r) = *profile.last().unwrap();
        profile.push((z + 3., r + 1.));
        if i < 8 {
            profile.push((z + 9., r + 1.));
        }
    }
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..options()
        };
        for (nominal, expected) in [
            (&profile[..], Some("recognition.too_complex")),
            (&profile[..profile.len() - 2], None),
        ] {
            let a = analyze_mesh(&annular_profile(&sampled(nominal)), &o).unwrap();
            let r = if mode == ReconstructionMode::Primitives {
                recognize_revolved(&a, &o).map(|r| r.report)
            } else {
                recognize_rounded(&a, &o).map(|r| r.report)
            };
            if let Some(code) = expected {
                assert_eq!(r.unwrap_err().code, code);
            } else {
                assert_eq!(r.unwrap().cones, 8);
            }
        }
        let strict = Options {
            max_deviation_mm: 0.001,
            ..o
        };
        let a = analyze_mesh(&annular_profile(&sampled(&profile[..4])), &strict).unwrap();
        let error = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &strict).unwrap_err()
        } else {
            recognize_rounded(&a, &strict).unwrap_err()
        };
        assert_eq!(error.code, "recognition.deviation_exceeded");
    }
}

#[test]
fn cone_cap_deviation_and_old_false_positives_remain_blocking() {
    let mut profile = vec![(0., 12.)];
    for i in 0..9 {
        let (z, _) = *profile.last().unwrap();
        let next = if i % 2 == 0 { 15. } else { 12. };
        profile.push((z + 3., next));
        if i < 8 {
            profile.push((z + 9., next));
        }
    }
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..options()
        };
        let a = analyze_mesh(&annular_profile(&sampled(&profile)), &o).unwrap();
        let error = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &o).unwrap_err()
        } else {
            recognize_rounded(&a, &o).unwrap_err()
        };
        assert_eq!(error.code, "recognition.too_complex");
        let eight = &profile[..profile.len() - 2];
        let a = analyze_mesh(&annular_profile(&sampled(eight)), &o).unwrap();
        let report = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &o).unwrap().report
        } else {
            recognize_rounded(&a, &o).unwrap().report
        };
        assert_eq!(report.cones, 8);
        assert!((report.expected_volume_mm3 - volume(eight)).abs() < 1e-7);

        let strict = Options {
            max_deviation_mm: 0.001,
            ..o.clone()
        };
        let a = analyze_mesh(&annular_profile(&sampled(&THREE)), &strict).unwrap();
        let error = if mode == ReconstructionMode::Primitives {
            recognize_revolved(&a, &strict).unwrap_err()
        } else {
            recognize_rounded(&a, &strict).unwrap_err()
        };
        assert_eq!(error.code, "recognition.deviation_exceeded");
        for bytes in [
            include_bytes!("../../../fixtures/m4_alternating_taper.stl").as_slice(),
            include_bytes!("../../../fixtures/m4_piecewise_taper.stl").as_slice(),
        ] {
            let a = analyze_stl(bytes, &o).unwrap();
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
