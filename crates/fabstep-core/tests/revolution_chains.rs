use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_rounded, MixedLoop, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/revolution_chains")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

fn axial(point: [f64; 3], origin: [f64; 3], axis: [f64; 3]) -> f64 {
    (0..3).map(|i| (point[i] - origin[i]) * axis[i]).sum()
}

#[test]
fn reconstructed_rings_and_triangle_owners_match_every_profile_interval() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        for name in [
            "stepped",
            "stepped_mirror",
            "arc100",
            "arc100_mirror",
            "arc100_inward",
            "arc100_inward_mirror",
            "arc100_rotated",
            "arc100_mixed_azimuths",
        ] {
            let stepped = name.starts_with("stepped");
            let inward = name.contains("inward");
            let branch = if inward { -1. } else { 1. };
            let minor = if stepped { 2. } else { 0.3 };
            let terminal_angle = if stepped { 0. } else { 10_f64.to_radians() };
            let tangent_z = minor * (1. + terminal_angle.sin());
            let tangent_r = 10. + branch * minor * terminal_angle.cos();
            let rings = if stepped {
                vec![
                    (0., 10.),
                    (2., 12.),
                    (5., 12.),
                    (7., 14.),
                    (10., 14.),
                    (14., 11.),
                ]
            } else {
                vec![
                    (0., 10.),
                    (tangent_z, tangent_r),
                    (
                        tangent_z + 4.,
                        tangent_r - 4. * branch * terminal_angle.tan(),
                    ),
                ]
            };
            let volume = if stepped {
                7083.763184690613
            } else if inward {
                1376.8445267283162
            } else {
                1358.9234656895578
            };
            let end = rings.last().unwrap().0;
            let (origin, direction) = if name.ends_with("mirror") {
                ([0., 0., end], [0., 0., -1.])
            } else if name.ends_with("rotated") {
                ([7., -4., 2.], [0., -5. / 13., 12. / 13.])
            } else {
                ([0.; 3], [0., 0., 1.])
            };
            let expected_counts: &[_] = if stepped {
                &[64, 2560, 128, 128, 128, 128, 64]
            } else if name == "arc100_mixed_azimuths" {
                &[360, 14400, 722, 362]
            } else {
                &[64, 2560, 128, 64]
            };

            let o = options(mode);
            let a = analyze_stl(&fixture(name), &o).unwrap_or_else(|e| panic!("{name}: {e}"));
            let r = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let rev = &r.model.revolved;
            let mixed = &rev.mixed;
            assert_eq!(mixed.vertices, a.mesh.vertices);
            assert_eq!(mixed.rings.len(), rings.len(), "{name}/{mode:?}");
            assert_eq!(mixed.planes.len(), 2);
            assert_eq!(mixed.cylinders.len(), if stepped { 2 } else { 0 });
            assert_eq!(rev.cones.len(), if stepped { 2 } else { 1 });
            assert!(r.model.spheres.is_empty() && r.model.partial_cones.is_empty());
            assert!(mixed.partial_cylinders.is_empty());
            assert_eq!((r.report.tori, r.report.toroidal_fillets), (1, 1));
            assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!(
                (r.report.expected_volume_mm3 - volume).abs() < 0.005,
                "{name}"
            );

            let ring_position = |index: usize| {
                let ring = &mixed.rings[index];
                let z = axial(ring.center, origin, direction);
                let centered =
                    (0..3).all(|i| (ring.center[i] - origin[i] - z * direction[i]).abs() < 1e-5);
                assert!(centered, "{name}: off-axis ring {index}");
                rings
                    .iter()
                    .position(|&(expected_z, radius)| {
                        (z - expected_z).abs() < 1e-5 && (ring.radius_mm - radius).abs() < 1e-5
                    })
                    .unwrap_or_else(|| {
                        panic!("{name}: unexpected ring z={z} rho={}", ring.radius_mm)
                    })
            };
            let interval = |ends: [usize; 2]| {
                let mut positions = ends.map(ring_position);
                positions.sort_unstable();
                assert_eq!(positions[1], positions[0] + 1, "{name}: skipped interval");
                positions[1]
            };
            let last = rings.len() - 1;
            let [torus] = r.model.tori.as_slice() else {
                panic!("{name}: expected exactly one torus");
            };
            assert_eq!(interval(torus.rings.unwrap()), 1);
            assert!((torus.major_radius_mm - 10.).abs() < 1e-5);
            assert!((torus.minor_radius_mm - minor).abs() < 1e-5);
            assert!(
                (torus.meridian_rad[1]
                    - torus.meridian_rad[0]
                    - (terminal_angle + std::f64::consts::FRAC_PI_2))
                    .abs()
                    < 1e-5,
                "{name}: wrong meridian domain"
            );

            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            let mut counts = vec![0; expected_counts.len()];
            for (&triangle, &owner) in a.mesh.triangles.iter().zip(&r.triangle_faces) {
                let (patch, [lo, hi]) = match owner {
                    RoundedFaceRef::Plane { index } => {
                        let [MixedLoop::Circle { ring }] = mixed.planes[index].loops.as_slice()
                        else {
                            panic!("{name}: plane did not keep a circular source boundary");
                        };
                        match ring_position(*ring) {
                            0 => (0, [0., 0.]),
                            position if position == last => (last + 1, [end, end]),
                            _ => panic!("{name}: interior cap"),
                        }
                    }
                    RoundedFaceRef::Torus { index: 0 } => (1, [0., tangent_z]),
                    RoundedFaceRef::Cone { index } => {
                        let cone = &rev.cones[index];
                        let position = interval(cone.rings);
                        assert!(!cone.inner);
                        assert!(if stepped {
                            matches!(position, 3 | 5)
                        } else {
                            position == 2
                        });
                        (position, [rings[position - 1].0, rings[position].0])
                    }
                    RoundedFaceRef::Cylinder { index } => {
                        let cylinder = &mixed.cylinders[index];
                        let position = interval(cylinder.rings);
                        assert!(!cylinder.inner);
                        assert!(stepped && matches!(position, 2 | 4));
                        (position, [rings[position - 1].0, rings[position].0])
                    }
                    _ => panic!("{name}: unexpected source owner {owner:?}"),
                };
                counts[patch] += 1;
                for vertex in triangle {
                    let z = axial(a.mesh.vertices[vertex], origin, direction);
                    assert!(
                        (lo - 1e-5..=hi + 1e-5).contains(&z),
                        "{name}: {owner:?} at {z}"
                    );
                }
            }
            assert_eq!(counts, expected_counts, "{name}/{mode:?}");
        }
    }
}

#[test]
fn incomplete_or_unresolved_profile_evidence_is_not_promoted() {
    let explicit = options(ReconstructionMode::Surfaces);
    let auto = options(ReconstructionMode::Auto);
    for name in [
        "missing_ring",
        "crossing_triangle",
        "bad_fit",
        "non_tangent",
        "unresolved_end",
    ] {
        let a = analyze_stl(&fixture(name), &explicit).unwrap_or_else(|e| panic!("{name}: {e}"));
        let error = recognize_rounded(&a, &explicit).unwrap_err();
        assert_eq!(
            error.code, "recognition.surface_unproven",
            "{name}: {error}"
        );
        let r = recognize_automatic(&a, &auto).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(r.model.tori.is_empty(), "{name}: unproved torus promoted");
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    }
}

#[test]
fn exact_vertex_fit_does_not_override_the_triangle_deviation_budget() {
    let o = Options {
        max_deviation_mm: 0.001,
        ..options(ReconstructionMode::Surfaces)
    };
    for name in ["stepped", "arc100"] {
        let a = analyze_stl(&fixture(name), &o).unwrap();
        let error = recognize_rounded(&a, &o).unwrap_err();
        assert_eq!(
            error.code, "recognition.deviation_exceeded",
            "{name}: {error}"
        );
    }
}
