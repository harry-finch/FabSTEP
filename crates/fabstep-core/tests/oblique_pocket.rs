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
