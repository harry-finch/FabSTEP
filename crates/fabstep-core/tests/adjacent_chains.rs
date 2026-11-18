use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_revolved, recognize_rounded, MixedLoop, Options,
    ReconstructionMode, RevolvedFaceRef, RoundedFaceRef,
};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/adjacent_chains")
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

#[test]
fn complete_source_rings_define_every_span_and_triangle_owner() {
    for (name, starts, cones, cylinders) in [
        ("three", vec![0., 3., 6., 10.], 3, 0),
        ("three_mirror", vec![0., 4., 7., 10.], 3, 0),
        ("eight", (0..=8).map(|i| 3. * f64::from(i)).collect(), 8, 0),
        ("mixed", vec![0., 3., 6., 9., 12., 15., 19.], 3, 3),
        ("mixed_rotated", vec![1., 4., 7., 10., 13., 16., 20.], 3, 3),
    ] {
        for mode in [
            ReconstructionMode::Primitives,
            ReconstructionMode::Surfaces,
            ReconstructionMode::Auto,
        ] {
            let o = options(mode);
            let a = analyze_stl(&fixture(name), &o).unwrap();
            let (model, report, owners) = if mode == ReconstructionMode::Primitives {
                let r =
                    recognize_revolved(&a, &o).unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
                let owners = r
                    .triangle_faces
                    .into_iter()
                    .map(|f| match f {
                        RevolvedFaceRef::Plane { index } => RoundedFaceRef::Plane { index },
                        RevolvedFaceRef::Cone { index } => RoundedFaceRef::Cone { index },
                        RevolvedFaceRef::Cylinder { index } => RoundedFaceRef::Cylinder { index },
                    })
                    .collect();
                (r.model, r.report, owners)
            } else {
                let r = if mode == ReconstructionMode::Auto {
                    recognize_automatic(&a, &o)
                } else {
                    recognize_rounded(&a, &o)
                }
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
                (r.model.revolved, r.report, r.triangle_faces)
            };
            assert_eq!(model.mixed.vertices, a.mesh.vertices);
            assert_eq!(
                (report.cones, report.cylinders),
                (cones, cylinders),
                "{name}/{mode:?}"
            );
            assert_eq!(
                (model.mixed.planes.len(), model.mixed.rings.len()),
                (2, starts.len())
            );
            assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert_eq!(owners.len(), a.mesh.triangles.len());

            let axis = if name.ends_with("rotated") { 0 } else { 2 };
            let position = |index: usize| {
                let coordinate = model.mixed.rings[index].center[axis];
                starts
                    .iter()
                    .position(|z| (z - coordinate).abs() < 1e-5)
                    .unwrap_or_else(|| panic!("{name}: unexpected ring at {coordinate}"))
            };
            let mut counts = vec![0; starts.len() + 1];
            for (&triangle, owner) in a.mesh.triangles.iter().zip(owners) {
                let bounds = match owner {
                    RoundedFaceRef::Plane { index } => {
                        let [MixedLoop::Circle { ring }] =
                            model.mixed.planes[index].loops.as_slice()
                        else {
                            panic!("{name}: plane has no circular boundary");
                        };
                        let rim = position(*ring);
                        assert!(rim == 0 || rim == starts.len() - 1);
                        [rim, rim]
                    }
                    RoundedFaceRef::Cone { index } => model.cones[index].rings.map(position),
                    RoundedFaceRef::Cylinder { index } => {
                        model.mixed.cylinders[index].rings.map(position)
                    }
                    _ => panic!("{name}: unexpected owner {owner:?}"),
                };
                let low = bounds[0].min(bounds[1]);
                let high = bounds[0].max(bounds[1]);
                assert!(high - low <= 1);
                let slot = if low == high && low != 0 {
                    high + 1
                } else {
                    high
                };
                counts[slot] += 1;
                for id in triangle {
                    let z = a.mesh.vertices[id][axis];
                    assert!((starts[low] - 1e-5..=starts[high] + 1e-5).contains(&z));
                }
            }
            let mut expected = vec![384; counts.len()];
            expected[0] = 64;
            *expected.last_mut().unwrap() = 64;
            assert_eq!(counts, expected, "{name}/{mode:?}");
        }
    }
}

#[test]
fn unresolved_and_ambiguous_profiles_cannot_gain_a_conical_decomposition() {
    let o = options(ReconstructionMode::Primitives);
    let surfaces = options(ReconstructionMode::Surfaces);
    let auto = options(ReconstructionMode::Auto);
    for name in [
        "ambiguous",
        "unresolved_slope",
        "missing_witness",
        "crossed",
        "smooth",
        "nine",
    ] {
        let a = analyze_stl(&fixture(name), &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        let revolved = recognize_revolved(&a, &o).unwrap_err();
        let rounded = recognize_rounded(&a, &surfaces);
        assert_eq!(
            revolved.code, "recognition.conic_unproven",
            "{name}/{revolved}"
        );
        if name == "smooth" {
            // The historical negative is exactly r(z) = 10 + 0.04 z².
            // Its complete source levels now prove a C2 fit; no cone is gained.
            let fitted = rounded.unwrap();
            assert_eq!(fitted.report.revolutions, 1);
            assert!(fitted.model.revolved.cones.is_empty());
        } else {
            let error = rounded.unwrap_err();
            assert_eq!(error.code, "recognition.surface_unproven", "{name}/{error}");
        }
        let r = recognize_automatic(&a, &auto).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(r.model.revolved.cones.is_empty(), "{name}");
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    }
}
