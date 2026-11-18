use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_rounded, Options, ReconstructionMode,
    RoundedFaceRef,
};
const MODELS: [(&str, &[u8], f64); 2] = [
    (
        "opposed",
        include_bytes!("../../../fixtures/two_linear_blends/opposed.stl"),
        3142.501177417038,
    ),
    (
        "offset",
        include_bytes!("../../../fixtures/two_linear_blends/offset.stl"),
        3156.637003044625,
    ),
];
#[test]
fn distinct_laws_and_all_eight_source_patches_are_preserved() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    for (name, raw, volume) in MODELS {
        let a = analyze_stl(raw, &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.cones, 2);
        assert_eq!(r.model.linear_blend.as_ref().unwrap().iter().count(), 2);
        assert!((r.report.expected_volume_mm3 - volume).abs() < 5e-5);
        let mut counts = [0; 8];
        for f in r.triangle_faces {
            match f {
                RoundedFaceRef::Plane { index } => counts[index] += 1,
                RoundedFaceRef::LinearBlend { index } => counts[6 + index] += 1,
                _ => panic!("{name}: {f:?}"),
            }
        }
        assert!(counts.into_iter().all(|x| x > 0));
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
    }
}
#[test]
fn a_nonlinear_second_radius_cannot_partially_authorize_a_pair() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/two_linear_blends/nonlinear_second.stl"),
        &o,
    )
    .unwrap();
    assert!(recognize_automatic(&a, &o).map_or(true, |r| r.model.linear_blend.is_none()));
}

#[test]
fn folding_the_second_cone_keeps_its_radii_but_invalidates_coverage() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut mesh = analyze_stl(MODELS[0].1, &o).unwrap().mesh;
    let mut ring: Vec<_> = mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            (p[2] - 4.).abs() < 1e-6 && p[0] > 16. && p[0] < 20. && p[1] > 12. && p[1] < 16.
        })
        .map(|(i, _)| i)
        .collect();
    ring.sort_by(|&a, &b| mesh.vertices[a][0].total_cmp(&mesh.vertices[b][0]));
    assert!(ring.len() > 20);
    mesh.vertices.swap(ring[8], ring[9]);
    let a = fabstep_core::analyze_mesh(&mesh, &o).unwrap();
    assert!(recognize_rounded(&a, &o).map_or(true, |r| r.model.linear_blend.is_none()));
}
