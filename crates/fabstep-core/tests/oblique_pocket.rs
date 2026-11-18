use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_rounded, Options, ReconstructionMode,
    RoundedFaceRef,
};
use std::{fs, path::Path};

fn input(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/oblique_pocket")
            .join(name),
    )
    .unwrap()
}

#[test]
fn auto_and_surfaces_certify_the_entire_oblique_fillet_from_source() {
    let cases = [
        ("nominal", 34, 98, 5771.780239856281),
        ("reverse", 46, 166, 5775.688795228845),
        ("staggered", 22, 61, 5771.780239856281),
        ("rotated", 34, 98, 5771.780239856281),
        ("compound", 46, 184, 5828.363573189615),
    ];
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = Options {
            reconstruction: mode,
            ..Options::default()
        };
        for (name, start, end, volume) in cases {
            let analysis = analyze_stl(&input(&format!("{name}.stl")), &o).unwrap();
            let result = if mode == ReconstructionMode::Auto {
                recognize_automatic(&analysis, &o)
            } else {
                recognize_rounded(&analysis, &o)
            }
            .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            let mixed = &result.model.revolved.mixed;
            assert_eq!(mixed.planes.len(), 11, "{name} {mode:?}");
            assert!(mixed.partial_cylinders[0].elliptic_end);
            assert_eq!(result.triangle_faces.len(), analysis.mesh.triangles.len());
            assert_eq!(
                result
                    .triangle_faces
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| matches!(
                        label,
                        RoundedFaceRef::PartialCylinder { index: 0 }
                    )
                    .then_some(i))
                    .collect::<Vec<_>>(),
                (start..end).collect::<Vec<_>>()
            );
            assert!(
                (result.report.expected_volume_mm3 - volume).abs() < 1e-4,
                "{name}: {}",
                result.report.expected_volume_mm3
            );
            assert!(result.report.max_deviation_bound_mm <= o.max_deviation_mm);
        }
    }
}

#[test]
fn a_proven_pocket_still_respects_the_deviation_budget() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        max_deviation_mm: 0.0001,
        ..Options::default()
    };
    let analysis = analyze_stl(&input("nominal.stl"), &o).unwrap();
    assert_eq!(
        recognize_rounded(&analysis, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn inspecting_the_oblique_wall_selects_its_triangles_and_keeps_guidance_scoped() {
    use fabstep_core::{automatic_region, recognize_automatic_guided, GuidanceFamily};

    let auto = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let source = analyze_stl(&input("nominal.stl"), &auto).unwrap();
    let wall = automatic_region(&source, &auto, 35).unwrap();
    assert!(wall.iter().copied().eq(34..98));
    let result = recognize_automatic_guided(&source, &auto, &[(GuidanceFamily::Cylinder, wall)]);
    assert_eq!(result.unwrap_err().code, "guidance.family_unproven");
}

#[test]
fn both_oblique_stops_require_two_complete_planar_ends_and_budget() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let raw = include_bytes!("../../../fixtures/double_oblique_pocket/opposed.stl");
    let a = analyze_stl(raw, &o).unwrap();
    let p = recognize_automatic(&a, &o).unwrap();
    let m = &p.model.revolved.mixed;
    assert!(m.partial_cylinders[0].elliptic_start && m.partial_cylinders[0].elliptic_end);
    assert!(m.arcs.is_empty());
    assert_eq!(m.elliptic_arcs.len(), 2);
    assert_eq!(m.partial_cylinders[0].arcs, [0, 1]);
    assert_eq!(p.triangle_faces.len(), a.mesh.triangles.len());
    for (face, triangle) in p.triangle_faces.iter().zip(&a.mesh.triangles) {
        let on_wall = triangle.iter().all(|&i| {
            let [x, _, z] = a.mesh.vertices[i];
            ((x - 7.5).hypot(z - 6.5) - 2.5).abs() < 2e-6
        });
        // A stop triangulation can use three points of its elliptic arc.
        // Such a triangle lies entirely on one of the two known planes.
        let on_wall = on_wall
            && [(6., -0.25), (29., 0.5)]
                .into_iter()
                .all(|(station, slope)| {
                    triangle.iter().any(|&i| {
                        let [x, y, _] = a.mesh.vertices[i];
                        (y - station - slope * (x - 7.5)).abs() > 2e-6
                    })
                });
        assert_eq!(
            matches!(face, RoundedFaceRef::PartialCylinder { index: 0 }),
            on_wall
        );
    }

    let strict = Options {
        max_deviation_mm: p.report.max_chord_error_mm / 2.,
        ..o.clone()
    };
    assert!(recognize_rounded(&a, &strict).is_err());
    let fallback = recognize_automatic(&a, &strict).unwrap();
    assert!(!fallback
        .model
        .revolved
        .mixed
        .partial_cylinders
        .iter()
        .any(|w| w.elliptic_start));
    for (station, slope) in [(6., -0.25), (29., 0.5)] {
        let mut mesh = a.mesh.clone();
        let point = mesh
            .vertices
            .iter_mut()
            .find(|p| {
                let [x, y, z] = **p;
                (y - station - slope * (x - 7.5)).abs() < 1e-5
                    && x > 8.
                    && x < 9.8
                    && (z - 6.5).abs() > 0.1
            })
            .unwrap();
        point[1] += 0.03;
        let bad = fabstep_core::analyze_mesh(&mesh, &o).unwrap();
        let r = recognize_automatic(&bad, &o).unwrap();
        assert!(!r
            .model
            .revolved
            .mixed
            .partial_cylinders
            .iter()
            .any(|w| w.elliptic_start));
    }
}
