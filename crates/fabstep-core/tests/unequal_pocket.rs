use fabstep_core::{
    analyze_mesh, analyze_stl, automatic_region, recognize_automatic, recognize_automatic_guided,
    recognize_rounded, GuidanceFamily, Options, ReconstructionMode, RoundedFaceRef,
};
const NOMINAL: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/nominal.stl");
const REFLECTED: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/reflected.stl");
const WARPED: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/warped_seam.stl");

#[test]
fn recovers_distinct_radii_volume_and_source_owners_in_both_modes() {
    for (bytes, radii, volume, small, large) in [
        (NOMINAL, [2., 3.], 5791.67320308558, 40..112, 296..384),
        (REFLECTED, [1.5, 3.5], 5308.068388489955, 684..828, 60..264),
    ] {
        for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = Options {
                reconstruction: mode,
                ..Options::default()
            };
            let a = analyze_stl(bytes, &o).unwrap();
            let result = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap();
            let d = result.model.unequal_pocket.unwrap();
            assert!(d.radii.iter().zip(radii).all(|(a, b)| (a - b).abs() < 1e-5));
            assert!((result.report.expected_volume_mm3 - volume).abs() < 1e-5);
            assert!(result.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert_eq!(result.triangle_faces.len(), a.mesh.triangles.len());
            for (i, f) in result.triangle_faces.iter().enumerate() {
                match f {
                    RoundedFaceRef::Plane { index } => {
                        assert!(*index < 11 && !small.contains(&i) && !large.contains(&i))
                    }
                    RoundedFaceRef::PartialCylinder { index: 0 } => assert!(small.contains(&i)),
                    RoundedFaceRef::PartialCylinder { index: 1 } => assert!(large.contains(&i)),
                    _ => panic!("unexpected face"),
                }
            }
        }
    }
}

#[test]
fn deformed_common_edge_and_strict_deviation_budget_do_not_receive_the_domain() {
    let mut o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    assert!(recognize_automatic(&analyze_stl(WARPED, &o).unwrap(), &o)
        .unwrap()
        .model
        .unequal_pocket
        .is_none());
    o.max_deviation_mm = 0.0001;
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        o.reconstruction = mode;
        let a = analyze_stl(NOMINAL, &o).unwrap();
        let err = if mode == ReconstructionMode::Auto {
            recognize_automatic(&a, &o)
        } else {
            recognize_rounded(&a, &o)
        }
        .unwrap_err();
        assert_eq!(err.code, "recognition.deviation_exceeded");
    }
}

#[test]
fn tilted_mirror_and_triangle_reordering_preserve_the_pocket() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let mut mesh = analyze_stl(NOMINAL, &o).unwrap().mesh;
    let (s, c) = 0.37_f64.sin_cos();
    let (t, d) = 0.68_f64.sin_cos();
    for p in &mut mesh.vertices {
        let [x, y, z] = *p;
        let v = y * c - x * s;
        *p = [9. - x * c - y * s, v * d - z * t - 8., v * t + z * d + 5.];
    }
    mesh.triangles.reverse();
    for tri in &mut mesh.triangles {
        tri.swap(1, 2);
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert!(r.model.unequal_pocket.is_some());
    assert!((r.report.expected_volume_mm3 - 5791.67320308558).abs() < 1e-5);
}

#[test]
fn a_nonrectangular_envelope_preserves_the_source_instead_of_inventing_a_box() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let mut mesh = analyze_stl(NOMINAL, &o).unwrap().mesh;
    for p in &mut mesh.vertices {
        if p[2] == 0. {
            p[0] += 0.75;
        }
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert!(r.model.unequal_pocket.is_none());
}

#[test]
fn rotated_binary_stl_meets_the_original_plane_tolerance() {
    let (sz, cz) = 0.31_f64.sin_cos();
    let (sx, cx) = 0.52_f64.sin_cos();
    let mut stl = NOMINAL.to_vec();
    for record in stl[84..].chunks_exact_mut(50) {
        for point in record[12..48].chunks_exact_mut(12) {
            let [x, y, z]: [f64; 3] = std::array::from_fn(|i| {
                f32::from_le_bytes(point[4 * i..4 * i + 4].try_into().unwrap()) as f64
            });
            let v = x * sz + y * cz;
            let world = [
                x * cz - y * sz + 1.,
                v * cx - z * sx - 2.,
                v * sx + z * cx + 2.,
            ];
            for (bytes, value) in point.chunks_exact_mut(4).zip(world) {
                bytes.copy_from_slice(&(value as f32).to_le_bytes());
            }
        }
    }
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = Options {
            reconstruction: mode,
            ..Options::default()
        };
        let a = analyze_stl(&stl, &o).unwrap();
        let r = if mode == ReconstructionMode::Auto {
            recognize_automatic(&a, &o)
        } else {
            recognize_rounded(&a, &o)
        }
        .unwrap();
        let d = r.model.unequal_pocket.unwrap();
        assert!(d
            .radii
            .iter()
            .zip([2., 3.])
            .all(|(a, b)| (a - b).abs() < 2e-6));
        assert!((r.report.expected_volume_mm3 - 5791.67320308558).abs() < 0.001);
        assert!(r.report.max_deviation_bound_mm < 0.007);
    }
}

#[test]
fn plane_discovery_does_not_relax_final_flatness() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let mut mesh = analyze_stl(NOMINAL, &o).unwrap().mesh;
    // Offset the bottom face’s interior vertex between the two tolerances.
    let center = mesh
        .vertices
        .iter_mut()
        .find(|p| **p == [0., 0., 0.])
        .unwrap();
    center[2] = 5e-6;
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert!(recognize_automatic(&a, &o)
        .unwrap()
        .model
        .unequal_pocket
        .is_none());
}

#[test]
fn selecting_one_rounded_wall_stops_at_the_shared_seam() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(NOMINAL, &options).unwrap();
    for expected in [40..112, 296..384] {
        let selected = automatic_region(&a, &options, expected.start).unwrap();
        assert!(selected.iter().copied().eq(expected));
        let error =
            recognize_automatic_guided(&a, &options, &[(GuidanceFamily::Cylinder, selected)])
                .unwrap_err();
        assert_eq!(error.code, "guidance.family_unproven");
    }
}
