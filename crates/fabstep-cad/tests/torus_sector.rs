use fabstep_cad::{build_rounded_step, build_rounded_step_with_token_groups};
use fabstep_core::{
    analyze_stl, recognize_rounded, MixedLoop, Options, ReconstructionMode, RoundedModel,
};

fn model() -> RoundedModel {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let stl = include_bytes!("../../../fixtures/torus_sectors/nominal.stl");
    let analysis = analyze_stl(stl, &options).unwrap();
    recognize_rounded(&analysis, &options).unwrap().model
}

#[test]
fn annular_sector_roundtrips_and_labels_all_seven_faces() {
    let m = model();
    let result = build_rounded_step_with_token_groups(
        &m,
        [
            &[81, 9, 3, 101],
            &[],
            &[12, 52],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[78],
        ],
    )
    .unwrap();
    assert_eq!(
        (
            result.report.solids,
            result.report.faces,
            result.report.toroidal_faces
        ),
        (1, 7, 1)
    );
    let mut tokens: Vec<_> = fabstep_cad::result_faces(&result.bytes)
        .unwrap()
        .faces
        .into_iter()
        .map(|face| face.source_face_token.unwrap())
        .collect();
    tokens.sort_unstable();
    assert_eq!(tokens, [3, 9, 12, 52, 78, 81, 101]);
    assert!((result.report.volume_mm3 - 264.31261025114605).abs() < 1e-4);
}

#[test]
fn inconsistent_sector_and_neighbor_metadata_cannot_build() {
    let reference = model();
    type Mutation = fn(&mut RoundedModel);
    for mutate in [
        (|m: &mut RoundedModel| m.torus_sectors[0].inner_radius_mm += 0.02) as Mutation,
        |m| m.torus_sectors[0].arc_uses[3].1 = false,
        |m| m.torus_sectors[0].axis[0] = f64::NAN,
        |m| m.torus_sectors[0].sweep_rad *= 0.5,
        |m| m.revolved.mixed.arcs[6].sweep_rad = f64::NAN,
        |m| m.revolved.mixed.arcs[5].vertices.swap(0, 1),
        |m| m.revolved.mixed.partial_cylinders[0].arcs.swap(0, 1),
        |m| {
            if let MixedLoop::Segments { segments } = &mut m.revolved.mixed.planes[0].loops[0] {
                segments.swap(0, 1)
            }
        },
        |m| {
            let id = m.revolved.mixed.arcs[0].vertices[0];
            m.revolved.mixed.vertices[id][0] += 0.01;
        },
        |m| m.torus_sectors.push(m.torus_sectors[0].clone()),
    ] {
        let mut m = reference.clone();
        mutate(&mut m);
        assert_eq!(
            build_rounded_step(&m).unwrap_err().code,
            "cad.invalid_model"
        );
    }
}

#[test]
fn altered_torus_support_in_serialized_step_is_refused() {
    let bytes = build_rounded_step(&model()).unwrap().bytes;
    let mut step = String::from_utf8(bytes).unwrap();
    let from = step.find("TOROIDAL_SURFACE(").unwrap();
    let to = from + step[from..].find(");").unwrap();
    let radius = from + step[from..to].rfind(',').unwrap() + 1;
    step.replace_range(radius..to, "0.51");
    assert!(fabstep_cad::tessellate_step(step.as_bytes(), 0.005).is_err());
}
