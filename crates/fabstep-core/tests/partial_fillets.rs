//! D1/E1 qualification of existing circular-wall contracts, not a claim that
//! every partial cylinder is a fillet or that arbitrary corners are supported.
use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_mixed, MixedFaceRef, MixedLoop, MixedModel,
    MixedSegment, Options, ReconstructionMode, RoundedFaceRef,
};
use std::{f64::consts::FRAC_PI_2, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/partial_fillets")
            .join(name),
    )
    .unwrap()
}

struct Nominal {
    name: &'static str,
    planes: usize,
    inner: usize,
    radii: &'static [f64],
    lengths: &'static [f64],
    volume: f64,
    triangles: usize,
}

fn nominal_cases() -> [Nominal; 6] {
    let cut = 1. - std::f64::consts::PI / 4.;
    let solid = |name, radii: &'static [f64], triangles| Nominal {
        name,
        planes: 6,
        inner: 0,
        radii,
        lengths: if radii.len() == 1 {
            &[12.]
        } else {
            &[12., 12.]
        },
        volume: (40. * 30. - cut * radii.iter().map(|r| r * r).sum::<f64>()) * 12.,
        triangles,
    };
    let tray = |name| Nominal {
        name,
        planes: 11,
        inner: 4,
        radii: &[2., 2., 2., 2., 4., 4., 4., 4.],
        lengths: &[9., 9., 9., 9., 12., 12., 12., 12.],
        volume: (40. * 30. - 4. * cut * 4_f64.powi(2)) * 12.
            - (34. * 24. - 4. * cut * 2_f64.powi(2)) * 9.,
        triangles: 816,
    };
    [
        solid("one_edge", &[4.], 120),
        solid("two_edges", &[2., 4.], 216),
        solid("two_edges_staggered", &[2., 4.], 216),
        solid("two_edges_rotated", &[2., 4.], 216),
        tray("open_tray"),
        tray("open_tray_rotated"),
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn assert_nominal_geometry(name: &str, model: &MixedModel, expected: &Nominal) {
    let partial_count = expected.radii.len();
    assert_eq!(model.planes.len(), expected.planes, "{name}");
    assert_eq!(model.partial_cylinders.len(), partial_count, "{name}");
    assert_eq!(model.cylinders.len(), 0, "{name}");
    assert_eq!(model.arcs.len(), 2 * partial_count, "{name}");
    assert_eq!(
        model.partial_cylinders.iter().filter(|c| c.inner).count(),
        expected.inner,
        "{name}"
    );
    let mut radii = Vec::new();
    let mut lengths = Vec::new();
    for cylinder in &model.partial_cylinders {
        let [lower_id, upper_id] = cylinder.arcs;
        let lower = &model.arcs[lower_id];
        let upper = &model.arcs[upper_id];
        radii.push(lower.radius_mm);
        lengths.push(dot(sub(upper.center, lower.center), lower.axis));
        assert!((lower.sweep_rad - FRAC_PI_2).abs() < 1e-9, "{name}");
        assert!((upper.sweep_rad - FRAC_PI_2).abs() < 1e-9, "{name}");
        assert!((upper.radius_mm - lower.radius_mm).abs() < 1e-9, "{name}");
        for arc_id in cylinder.arcs {
            let incident: Vec<_> = model
                .planes
                .iter()
                .filter(|plane| {
                    plane.loops.iter().any(|lp| match lp {
                        MixedLoop::Segments { segments } => segments
                            .iter()
                            .any(|s| matches!(s, MixedSegment::Arc { arc, .. } if *arc == arc_id)),
                        _ => false,
                    })
                })
                .collect();
            assert_eq!(incident.len(), 1, "{name}: actual planar stop for each arc");
            assert!((dot(incident[0].normal, lower.axis).abs() - 1.).abs() < 1e-9);
        }
        for end in 0..2 {
            let rail = [lower.vertices[end], upper.vertices[end]];
            let supports: Vec<_> = model
                .planes
                .iter()
                .filter(|plane| {
                    plane.loops.iter().any(|lp| match lp {
                        MixedLoop::Segments { segments } => segments.iter().any(|s| {
                            matches!(s, MixedSegment::Line { vertices }
                            if *vertices == rail || *vertices == [rail[1], rail[0]])
                        }),
                        _ => false,
                    })
                })
                .collect();
            assert_eq!(
                supports.len(),
                1,
                "{name}: source support of each straight rail"
            );
            let radial = sub(model.vertices[rail[0]], lower.center).map(|v| v / lower.radius_mm);
            let orientation = if cylinder.inner { -1. } else { 1. };
            // These nominal cases really are tangent. This assertion qualifies
            // the corpus; non-tangent circular walls remain valid cylinders.
            assert!(
                (dot(radial, supports[0].normal) - orientation).abs() < 1e-9,
                "{name}: tangent support"
            );
            assert!(dot(supports[0].normal, lower.axis).abs() < 1e-9, "{name}");
        }
    }
    radii.sort_by(f64::total_cmp);
    lengths.sort_by(f64::total_cmp);
    for (actual, nominal) in radii.iter().zip(expected.radii) {
        assert!((actual - nominal).abs() < 1e-9, "{name}: radius");
    }
    for (actual, nominal) in lengths.iter().zip(expected.lengths) {
        assert!((actual - nominal).abs() < 1e-9, "{name}: source stops");
    }
    if expected.inner > 0 {
        let rim = model
            .planes
            .iter()
            .filter(|p| {
                p.loops.len() == 2
                    && p.loops
                        .iter()
                        .all(|lp| matches!(lp, MixedLoop::Segments { .. }))
            })
            .count();
        assert_eq!(rim, 1, "{name}: actual annular rim keeps the cavity open");
    }
}

#[test]
fn disjoint_straight_edge_rounds_and_open_trays_use_proven_source_patches() {
    for expected in nominal_cases() {
        let name = expected.name;
        let options = Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        };
        let analysis = analyze_stl(&fixture(&format!("{name}.stl")), &options).unwrap();
        let proof = recognize_mixed(&analysis, &options).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_nominal_geometry(name, &proof.model, &expected);
        assert_eq!(proof.model.vertices, analysis.mesh.vertices, "{name}");
        assert_eq!(
            proof.triangle_faces.len(),
            analysis.mesh.triangles.len(),
            "{name}"
        );
        assert_eq!(analysis.mesh.triangles.len(), expected.triangles);
        let mut owners = vec![0; proof.model.partial_cylinders.len()];
        for face in &proof.triangle_faces {
            match *face {
                MixedFaceRef::Plane { index } => assert!(index < proof.model.planes.len()),
                MixedFaceRef::PartialCylinder { index } => owners[index] += 1,
                MixedFaceRef::Cylinder { .. } => panic!("{name}: no nominal complete cylinder"),
            }
        }
        assert!(owners.into_iter().all(|count| count > 0), "{name}");
        let volume = expected.volume;
        assert!(
            (proof.report.expected_volume_mm3 - volume).abs() < 1e-7,
            "{name}: analytic volume"
        );
        assert!(
            proof.report.max_deviation_bound_mm <= options.max_deviation_mm,
            "{name}"
        );
        assert!(
            proof.report.max_radial_residual_mm <= options.cylinder_fit_tolerance_mm,
            "{name}"
        );
    }
}

#[test]
fn automatic_reconstruction_preserves_the_same_independent_local_walls() {
    for expected in nominal_cases() {
        let name = expected.name;
        let options = Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        };
        let analysis = analyze_stl(&fixture(&format!("{name}.stl")), &options).unwrap();
        let proof =
            recognize_automatic(&analysis, &options).unwrap_or_else(|e| panic!("{name}: {e}"));
        let model = &proof.model.revolved.mixed;
        assert_nominal_geometry(name, model, &expected);
        assert_eq!(model.vertices, analysis.mesh.vertices, "{name}");
        assert_eq!(proof.triangle_faces.len(), analysis.mesh.triangles.len());
        assert!(proof.model.spheres.is_empty());
        assert!(proof.model.tori.is_empty());
        assert!(proof.model.partial_cones.is_empty());
        assert!(proof.model.revolved.cones.is_empty());
        let mut owners = vec![0; model.partial_cylinders.len()];
        for face in proof.triangle_faces {
            match face {
                RoundedFaceRef::Plane { index } => assert!(index < model.planes.len()),
                RoundedFaceRef::PartialCylinder { index } => owners[index] += 1,
                _ => panic!("{name}: unexpected curved family"),
            }
        }
        assert!(owners.into_iter().all(|count| count > 0), "{name}");
        assert!(
            (proof.report.expected_volume_mm3 - expected.volume).abs() < 1e-7,
            "{name}"
        );
    }
}

#[test]
fn missing_stops_or_floor_and_insufficient_deviation_budget_remain_refusals() {
    for name in ["one_edge_missing_end", "tray_missing_floor"] {
        let error = analyze_stl(&fixture(&format!("{name}.stl")), &Options::default()).unwrap_err();
        assert_eq!(error.code, "mesh.open", "{name}: {error}");
    }
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        max_deviation_mm: 0.0001,
        ..Options::default()
    };
    let analysis = analyze_stl(&fixture("one_edge.stl"), &options).unwrap();
    assert_eq!(
        recognize_mixed(&analysis, &options).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn closed_variable_sections_are_not_replaced_by_a_constant_cylinder() {
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    let analysis = analyze_stl(&fixture("one_edge_variable_radius.stl"), &options).unwrap();
    assert_eq!(
        recognize_mixed(&analysis, &options).unwrap_err().code,
        "recognition.partial_unproven"
    );
    let proof = recognize_automatic(
        &analysis,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..options
        },
    )
    .unwrap();
    assert!(proof.model.revolved.mixed.partial_cylinders.is_empty());
    assert!(proof.model.revolved.mixed.cylinders.is_empty());
    assert_eq!(proof.triangle_faces.len(), analysis.mesh.triangles.len());
    assert!(proof
        .triangle_faces
        .iter()
        .all(|owner| matches!(owner, RoundedFaceRef::Plane { .. })));
}
