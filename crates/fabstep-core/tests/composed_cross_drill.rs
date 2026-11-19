use fabstep_core::{analyze_stl, recognize_automatic, Options, ReconstructionMode, RoundedFaceRef};
const COARSE: &[u8] = include_bytes!("../../../fixtures/composed_cross_drill/disjoint-coarse.stl");
const FINE: &[u8] = include_bytes!("../../../fixtures/composed_cross_drill/disjoint-fine.stl");
const CONFLICT: &[u8] =
    include_bytes!("../../../fixtures/composed_cross_drill/common-bore-coarse.stl");

#[test]
fn fillet_and_bores_have_disjoint_source_faces_at_two_densities() {
    let opts = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (stl, count) in [(COARSE, 678), (FINE, 1290)] {
        let a = analyze_stl(stl, &opts).unwrap();
        let r = recognize_automatic(&a, &opts).unwrap();
        let rounded = r
            .model
            .cross_drill
            .as_deref()
            .expect("cylindres des perçages");
        let fillet = rounded.stock_fillet.unwrap();
        assert!((fillet.radius_mm - 1.5).abs() < 1e-5);
        assert!(
            (r.report.expected_volume_mm3 - 5733.46607568568).abs() < 0.001,
            "{}",
            r.report.expected_volume_mm3
        );
        let mut claimed = [0; 4];
        for owner in r.triangle_faces {
            match owner {
                RoundedFaceRef::CrossDrill { index } => claimed[index] += 1,
                RoundedFaceRef::Plane { index } => assert!(index < 6),
                _ => panic!("attribution inattendue"),
            }
        }
        assert!(claimed.iter().all(|&c| c > 0));
        assert_eq!(claimed.iter().sum::<usize>(), count);
    }
}
#[test]
fn shared_longitudinal_bore_keeps_its_existing_fallback() {
    let opts = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(CONFLICT, &opts).unwrap();
    let r = recognize_automatic(&a, &opts).unwrap();
    assert!(r.model.cross_drill.is_none());
}

#[test]
fn the_same_corner_in_a_placed_frame_and_a_false_radius_are_distinguished() {
    let opts = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let original = analyze_stl(COARSE, &opts).unwrap();
    let mut placed = original.mesh.clone();
    for p in &mut placed.vertices {
        let [x, y, z] = *p;
        *p = [z + 5., -y + 3., x - 2.];
    }
    let placed = fabstep_core::analyze_mesh(&placed, &opts).unwrap();
    let r = recognize_automatic(&placed, &opts).unwrap();
    let fillet = r.model.cross_drill.unwrap().stock_fillet.unwrap();
    assert!((fillet.radius_mm - 1.5).abs() < 1e-5);
    let mut flawed = original.mesh;
    let mut modified = 0;
    for p in &mut flawed.vertices {
        let [_, y, z] = *p;
        if (y + 8.5).hypot(z - 7.5) <= 1.50001 && y < -8.75 && z > 7.8 {
            p[1] += 0.1;
            modified += 1;
        }
    }
    assert!(modified > 0);
    let a = fabstep_core::analyze_mesh(&flawed, &opts).unwrap();
    let result = recognize_automatic(&a, &opts).unwrap();
    assert!(result.model.cross_drill.is_none());
}
