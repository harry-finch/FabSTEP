use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode};

#[test]
fn two_turning_meridians_are_recovered_at_both_resolutions() {
    for (name, bytes) in [
        (
            "flared-coarse",
            include_bytes!("../../../fixtures/returning_meridian/flared-coarse.stl").as_slice(),
        ),
        (
            "flared-fine",
            include_bytes!("../../../fixtures/returning_meridian/flared-fine.stl").as_slice(),
        ),
        (
            "inverted-coarse",
            include_bytes!("../../../fixtures/returning_meridian/inverted-coarse.stl").as_slice(),
        ),
        (
            "inverted-fine",
            include_bytes!("../../../fixtures/returning_meridian/inverted-fine.stl").as_slice(),
        ),
    ] {
        let options = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        };
        let a = analyze_stl(bytes, &options).unwrap();
        let r = recognize_rounded(&a, &options).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(
            r.model
                .smooth_revolution
                .unwrap()
                .axial_controls_mm
                .is_some(),
            "{name}"
        );
    }
}

#[test]
fn off_profile_vertices_and_folded_cells_do_not_claim_a_returning_wall() {
    let raw = include_bytes!("../../../fixtures/returning_meridian/flared-coarse.stl");
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let original = analyze_stl(raw, &o).unwrap().mesh;
    let a = original
        .vertices
        .iter()
        .position(|p| (5.5..7.).contains(&p[0].hypot(p[1])))
        .unwrap();
    let b = original
        .triangles
        .iter()
        .find(|t| t.contains(&a))
        .unwrap()
        .iter()
        .copied()
        .find(|v| *v != a)
        .unwrap();
    for folded in [false, true] {
        let mut mesh = original.clone();
        if folded {
            mesh.vertices.swap(a, b);
        } else {
            mesh.vertices[a][2] += 0.008;
        }
        let damaged = fabstep_core::analyze_mesh(&mesh, &o).unwrap();
        assert!(
            recognize_rounded(&damaged, &o).map_or(true, |r| r.model.smooth_revolution.is_none())
        );
    }
}

#[test]
fn an_undercut_cannot_spend_more_than_the_requested_deviation() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        max_deviation_mm: 0.001,
        ..Options::default()
    };
    let bytes = include_bytes!("../../../fixtures/returning_meridian/flared-coarse.stl");
    let source = analyze_stl(bytes, &options).unwrap();
    assert_eq!(
        recognize_rounded(&source, &options).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}
