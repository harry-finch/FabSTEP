use fabstep_core::{
    analyze_stl, automatic_region, recognize_automatic, recognize_automatic_guided,
    recognize_rounded, GuidanceFamily, Options, ReconstructionMode,
};

const NOMINAL: &[u8] = include_bytes!("../../../fixtures/adjacent_pocket/nominal.stl");

#[test]
fn each_wall_can_be_inspected_independently_and_coupled_edits_are_refused() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let source = analyze_stl(NOMINAL, &o).unwrap();
    for expected in [40..104, 240..304] {
        let wall = automatic_region(&source, &o, expected.start + 1).unwrap();
        assert!(wall.iter().copied().eq(expected));
        let err = recognize_automatic_guided(&source, &o, &[(GuidanceFamily::Cylinder, wall)])
            .unwrap_err();
        assert_eq!(err.code, "guidance.family_unproven");
    }
}

#[test]
fn a_tight_budget_blocks_both_modes_on_a_proven_seam() {
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = Options {
            reconstruction: mode,
            max_deviation_mm: 0.0001,
            ..Options::default()
        };
        let source = analyze_stl(NOMINAL, &o).unwrap();
        let result = if mode == ReconstructionMode::Surfaces {
            recognize_rounded(&source, &o)
        } else {
            recognize_automatic(&source, &o)
        };
        assert_eq!(result.unwrap_err().code, "recognition.deviation_exceeded");
    }
}

#[test]
fn a_rigid_mirror_keeps_the_seam_while_a_one_axis_stretch_rejects_it() {
    use fabstep_core::analyze_mesh;
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let mut mirrored = analyze_stl(NOMINAL, &o).unwrap().mesh;
    let mut stretched = mirrored.clone();
    let (sin, cos) = 0.42_f64.sin_cos();
    let (tilt_sin, tilt_cos) = 0.22_f64.sin_cos();
    for p in &mut mirrored.vertices {
        let [x, y, z] = *p;
        let across = -x * sin + y * cos;
        *p = [
            -x * cos - y * sin + 5.,
            across * tilt_cos - z * tilt_sin - 3.,
            across * tilt_sin + z * tilt_cos + 2.,
        ];
    }
    mirrored.triangles.reverse();
    for tri in &mut mirrored.triangles {
        tri.swap(1, 2);
    }
    let recovered = recognize_automatic(&analyze_mesh(&mirrored, &o).unwrap(), &o).unwrap();
    assert_eq!(recovered.model.revolved.mixed.partial_cylinders.len(), 2);
    assert!((recovered.report.expected_volume_mm3 - 5781.551628287691).abs() < 1e-5);
    for p in &mut stretched.vertices {
        p[0] *= 1.25;
    }
    let fallback = recognize_automatic(&analyze_mesh(&stretched, &o).unwrap(), &o).unwrap();
    assert!(fallback.model.revolved.mixed.elliptic_arcs.is_empty());
}

#[test]
fn unequal_adjacent_fillet_radii_never_receive_the_equal_radius_ellipse() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for mesh in [
        include_bytes!("../../../fixtures/unequal_pocket/nominal.stl").as_slice(),
        include_bytes!("../../../fixtures/unequal_pocket/reflected.stl").as_slice(),
        include_bytes!("../../../fixtures/unequal_pocket/warped_seam.stl").as_slice(),
    ] {
        let analysis = analyze_stl(mesh, &options).unwrap();
        let result = recognize_automatic(&analysis, &options).unwrap();
        let mixed = &result.model.revolved.mixed;
        let mut elliptic_ends = std::collections::BTreeSet::new();
        for wall in mixed.partial_cylinders.iter().filter(|c| c.elliptic_end) {
            assert!(
                elliptic_ends.insert(wall.arcs[1]),
                "Two different radii cannot share an elliptical stop"
            );
        }
    }
}
