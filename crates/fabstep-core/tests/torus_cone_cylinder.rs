use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_rounded, MixedLoop, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{f64::consts::FRAC_PI_2, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/torus_cone_cylinder")
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

fn delta(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn cylindrical(point: [f64; 3], origin: [f64; 3], axis: [f64; 3]) -> (f64, f64) {
    let d = delta(point, origin);
    let z = dot(d, axis);
    let radial: [f64; 3] = std::array::from_fn(|i| d[i] - z * axis[i]);
    (z, dot(radial, radial).sqrt())
}

#[test]
fn two_ring_straight_bands_join_the_measured_tangent_arc_and_own_every_triangle() {
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        for name in [
            "nominal",
            "mirrored",
            "rotated",
            "opposite_branch",
            "seam_noise",
        ] {
            let o = options(mode);
            let a = analyze_stl(&fixture(name), &o).unwrap();
            let r = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            let rev = &r.model.revolved;
            let mixed = &rev.mixed;
            let report = &r.report;
            assert_eq!(
                (report.tori, report.toroidal_fillets, report.spheres),
                (1, 1, 0),
                "{name} {mode:?}"
            );
            assert_eq!((report.cones, report.cylinders), (1, 1));
            assert_eq!((mixed.planes.len(), mixed.rings.len()), (2, 4));
            assert_eq!(mixed.vertices, a.mesh.vertices);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);

            let torus = &r.model.tori[0];
            let torus_ids = torus.rings.unwrap();
            let cone = &rev.cones[0];
            let cylinder = &mixed.cylinders[0];
            assert!(!cone.inner && !cylinder.inner);
            let cone_start = torus_ids
                .into_iter()
                .find(|id| cone.rings.contains(id))
                .unwrap();
            let torus_start = torus_ids.into_iter().find(|id| *id != cone_start).unwrap();
            let cylinder_start = cone.rings.into_iter().find(|id| *id != cone_start).unwrap();
            assert!(cylinder.rings.contains(&cylinder_start));
            let cylinder_end = cylinder
                .rings
                .into_iter()
                .find(|id| *id != cylinder_start)
                .unwrap();
            let mut plane_ids = mixed
                .planes
                .iter()
                .map(|plane| {
                    let [MixedLoop::Circle { ring }] = plane.loops.as_slice() else {
                        panic!("{name}: the terminal plane must use one source circle");
                    };
                    *ring
                })
                .collect::<Vec<_>>();
            plane_ids.sort_unstable();
            let mut expected_plane_ids = [torus_start, cylinder_end];
            expected_plane_ids.sort_unstable();
            assert_eq!(plane_ids, expected_plane_ids);

            let rings = &mixed.rings;
            let across = delta(rings[cylinder_end].center, rings[torus_start].center);
            let height = dot(across, across).sqrt();
            let direction = across.map(|x| x / height);
            let branch = if name == "opposite_branch" { 1.0 } else { -1.0 };
            let slope = branch / 15.;
            let sweep = FRAC_PI_2 - (1_f64 / 15.).atan();
            let tangent_height = 0.5 * (1. - sweep.cos());
            let tangent_radius = 10. + 0.5 * branch * sweep.sin();
            let cylinder_radius = tangent_radius + 2. * slope;
            let exact_volume = if branch > 0. {
                1759.2263317170618
            } else {
                1393.2379688651004
            };
            assert!(
                (report.expected_volume_mm3 - exact_volume).abs() < 0.005,
                "{name}"
            );
            assert!((torus.major_radius_mm - 10.).abs() < 1e-5, "{name}");
            assert!((torus.minor_radius_mm - 0.5).abs() < 1e-5, "{name}");
            assert!(
                (torus.meridian_rad[1] - torus.meridian_rad[0] - sweep).abs() < 1e-5,
                "{name}"
            );
            let expected_rings = [
                (0., 10.),
                (tangent_height, tangent_radius),
                (tangent_height + 2., cylinder_radius),
                (5., cylinder_radius),
            ];
            for (id, (z, radius)) in [torus_start, cone_start, cylinder_start, cylinder_end]
                .into_iter()
                .zip(expected_rings)
            {
                let ring = &rings[id];
                let (actual_z, center_offset) =
                    cylindrical(ring.center, rings[torus_start].center, direction);
                assert!(
                    (actual_z - z).abs() < 1e-5 && center_offset < 1e-5,
                    "{name}"
                );
                assert!((ring.radius_mm - radius).abs() < 1e-5, "{name}");
            }
            for (id, angle) in torus_ids.into_iter().zip(torus.meridian_rad) {
                let ring = &rings[id];
                let z = dot(delta(ring.center, torus.center), ring.axis);
                assert!(
                    (z - torus.minor_radius_mm * angle.sin()).abs() < 1e-9,
                    "{name}"
                );
                assert!(
                    (ring.radius_mm - torus.major_radius_mm - torus.minor_radius_mm * angle.cos())
                        .abs()
                        < 1e-9,
                    "{name}"
                );
                if id == torus_start {
                    assert!(angle.cos().abs() < 1e-9, "{name}");
                } else {
                    let cone_slope = (rings[cylinder_start].radius_mm - ring.radius_mm)
                        / dot(delta(rings[cylinder_start].center, ring.center), ring.axis);
                    assert!((cone_slope + angle.tan()).abs() < 1e-8, "{name}");
                }
            }

            let mut counts = [0; 5];
            for (&triangle, face) in a.mesh.triangles.iter().zip(&r.triangle_faces) {
                let (slot, span) = match *face {
                    RoundedFaceRef::Plane { index } => {
                        let [MixedLoop::Circle { ring }] = mixed.planes[index].loops.as_slice()
                        else {
                            unreachable!();
                        };
                        let z = if *ring == torus_start { 0. } else { 5. };
                        (index, [z, z])
                    }
                    RoundedFaceRef::Torus { index: 0 } => (2, [0., tangent_height]),
                    RoundedFaceRef::Cone { index: 0 } => (3, [tangent_height, tangent_height + 2.]),
                    RoundedFaceRef::Cylinder { index: 0 } => (4, [tangent_height + 2., 5.]),
                    _ => panic!("{name}: unexpected owner {face:?}"),
                };
                counts[slot] += 1;
                let points = triangle.map(|id| a.mesh.vertices[id]);
                for point in points {
                    let z = dot(delta(point, rings[torus_start].center), direction);
                    assert!(
                        (span[0] - 1e-5..=span[1] + 1e-5).contains(&z),
                        "{name}: {face:?} at {z}"
                    );
                }
                for w in [
                    [0.2, 0.3, 0.5],
                    [0.5, 0.5, 0.],
                    [0.5, 0., 0.5],
                    [0., 0.5, 0.5],
                ] {
                    let p = std::array::from_fn(|i| (0..3).map(|j| points[j][i] * w[j]).sum());
                    let (z, rho) = cylindrical(p, rings[torus_start].center, direction);
                    let distance = match slot {
                        0 | 1 => z - span[0],
                        2 => (rho - 10.).hypot(z - 0.5) - 0.5,
                        3 => {
                            (rho - tangent_radius - slope * (z - tangent_height)) / slope.hypot(1.)
                        }
                        _ => rho - cylinder_radius,
                    };
                    assert!(
                        distance.abs() <= report.max_deviation_bound_mm + 1e-6,
                        "{name}: {face:?} distance {distance}"
                    );
                }
            }
            assert_eq!(counts, [64, 64, 1536, 128, 128], "{name}");
            if name == "seam_noise" {
                assert!(
                    report
                        .max_radial_residual_mm
                        .max(report.max_wall_residual_mm)
                        >= 1e-6
                );
            }
        }
    }
}

#[test]
fn unsupported_joins_do_not_get_torus_attribution() {
    let explicit = options(ReconstructionMode::Surfaces);
    let auto = options(ReconstructionMode::Auto);
    for name in ["non_tangent", "missing_ring", "crossing_triangle", "warped"] {
        let a = analyze_stl(&fixture(name), &explicit).unwrap();
        let err = recognize_rounded(&a, &explicit).unwrap_err();
        assert_eq!(err.code, "recognition.surface_unproven", "{name}: {err}");
        let result = recognize_automatic(&a, &auto).unwrap();
        assert_eq!(result.report.tori, 0, "{name}");
        assert_eq!(result.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(result.triangle_faces.len(), a.mesh.triangles.len());
    }
}

#[test]
fn the_chord_budget_blocks_the_exact_vertex_fit() {
    let o = Options {
        max_deviation_mm: 0.001,
        ..options(ReconstructionMode::Surfaces)
    };
    let a = analyze_stl(&fixture("nominal"), &o).unwrap();
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}
