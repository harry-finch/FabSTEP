use fabstep_cad::{
    build_rounded_step, build_rounded_step_with_cross_drill_provenance,
    build_rounded_step_with_linear_blend_provenance, result_faces, ResultFaceFamily,
};
use fabstep_core::{
    analyze_stl, recognize_rounded, MixedLoop, Options, ReconstructionMode, RoundedModel,
};
use std::collections::BTreeSet;

fn from_source(stl: &[u8]) -> RoundedModel {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let a = analyze_stl(stl, &o).unwrap();
    recognize_rounded(&a, &o).unwrap().model
}

fn nominal() -> RoundedModel {
    from_source(include_bytes!(
        "../../../fixtures/cross_drill/f_nominal.stl"
    ))
}

#[test]
fn cross_drill_rejects_invalid_supports_cycles_and_nonexclusive_models() {
    let base = nominal();
    for mutate in [
        (|m: &mut RoundedModel| m.cross_drill.as_mut().unwrap().rims.swap(0, 1))
            as fn(&mut RoundedModel),
        |m| m.cross_drill.as_mut().unwrap().radii[1] = 3.,
        |m| m.cross_drill.as_mut().unwrap().axes.swap(0, 1),
        |m| m.cross_drill.as_mut().unwrap().half_lengths[1] *= 2.,
        |m| m.cross_drill.as_mut().unwrap().branches[0].reverse(),
        |m| {
            // Eight evenly spaced witnesses still traverse the whole valid branch.
            let cycle = &mut m.cross_drill.as_mut().unwrap().branches[0];
            *cycle = cycle.iter().step_by(8).copied().collect();
        },
        |m| {
            // Keep more than twelve witnesses but leave one unresolved angular gap.
            m.cross_drill.as_mut().unwrap().branches[0].drain(1..14);
        },
        |m| m.cross_drill.as_mut().unwrap().branches.swap(0, 1),
        |m| m.cross_drill.as_mut().unwrap().branches[1][0] = usize::MAX,
        |m| {
            let g = m.cross_drill.as_mut().unwrap();
            g.branches[0][1] = g.branches[0][0];
        },
        |m| {
            let v = m.cross_drill.as_ref().unwrap().branches[0][0];
            m.revolved.mixed.vertices[v][0] += 0.05;
        },
        |m| m.cross_drill.as_mut().unwrap().witness_tolerance_mm = f64::INFINITY,
        |m| {
            // Stock clearances alone must not admit witnesses farther than r/2.
            let f = m.cross_drill.as_mut().unwrap();
            f.radii[0] = 0.25;
            f.witness_tolerance_mm = 0.2;
            let f = f.clone();
            for slot in 0..2 {
                m.revolved.mixed.rings[f.rims[slot]].radius_mm = 0.25;
                for &v in &f.branches[slot] {
                    let old = m.revolved.mixed.vertices[v];
                    let local = f.axes.map(|axis| {
                        (0..3)
                            .map(|i| axis[i] * (old[i] - f.center[i]))
                            .sum::<f64>()
                    });
                    let phase = local[1].atan2(local[0]);
                    let y = 0.4 * phase.sin();
                    let q = [
                        0.4 * phase.cos(),
                        y,
                        (if slot == 0 { -1. } else { 1. }) * (f.radii[1].powi(2) - y * y).sqrt(),
                    ];
                    m.revolved.mixed.vertices[v] = std::array::from_fn(|i| {
                        f.center[i] + (0..3).map(|j| f.axes[j][i] * q[j]).sum::<f64>()
                    });
                }
            }
        },
        |m| {
            m.revolved.mixed.planes[0].loops.pop();
        },
        |m| {
            if let MixedLoop::Polygon { vertices } = &mut m.revolved.mixed.planes[0].loops[0] {
                vertices.reverse();
            }
        },
        |m| {
            m.revolved
                .mixed
                .cylinders
                .push(fabstep_core::MixedCylinder {
                    rings: [0, 1],
                    inner: true,
                });
        },
    ] {
        let mut wrong = base.clone();
        mutate(&mut wrong);
        let error = build_rounded_step(&wrong).unwrap_err();
        assert_eq!(error.code, "cad.invalid_model");
        assert_eq!(
            error.message,
            "Les limites et les supports des perçages sécants sont incohérents."
        );
    }
}

#[test]
fn cross_drill_tracks_only_complete_distinct_owner_groups() {
    let m = nominal();
    let planes = [4, 15, 27, 39, 42, 54];
    let with = |walls: &[usize]| {
        build_rounded_step_with_cross_drill_provenance(
            &m,
            &planes,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            walls,
        )
    };
    for bad in [&[][..], &[1, 2][..], &[1, 2, 4][..], &[1, 2, 100_001][..]] {
        assert_eq!(with(bad).unwrap_err().code, "cad.invalid_model");
    }
    assert_eq!(
        build_rounded_step_with_linear_blend_provenance(
            &m,
            &planes,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[]
        )
        .unwrap_err()
        .code,
        "cad.invalid_model"
    );
    let result = with(&[74, 82, 99]).unwrap();
    let faces = result_faces(&result.bytes).unwrap().faces;
    let all: BTreeSet<_> = faces.iter().map(|f| f.source_face_token.unwrap()).collect();
    assert_eq!(all, planes.into_iter().chain([74, 82, 99]).collect());
    for f in faces {
        if [74, 82, 99].contains(&f.source_face_token.unwrap()) {
            assert_eq!(f.family, ResultFaceFamily::Cylinder);
            assert_eq!(
                f.adjacent_face_ids.len(),
                if f.source_face_token == Some(99) {
                    4
                } else {
                    2
                }
            );
        } else {
            assert_eq!(f.family, ResultFaceFamily::Plane);
        }
    }
    let untracked = build_rounded_step(&m).unwrap();
    assert_eq!(untracked.report.faces, 9);
    assert!(result_faces(&untracked.bytes)
        .unwrap()
        .faces
        .iter()
        .all(|f| f.source_face_token.is_none()));
}

#[test]
fn wider_ratio_keeps_both_branches_on_reimport() {
    let model = from_source(include_bytes!(
        "../../../fixtures/cross_drill/f_ratio_near.stl"
    ));
    let d = model.cross_drill.as_ref().unwrap();
    assert!((0.665..(2. / 3.)).contains(&(d.radii[0] / d.radii[1])));
    let result = build_rounded_step(&model).unwrap();
    assert_eq!(
        (
            result.report.solids,
            result.report.planar_faces,
            result.report.cylindrical_faces
        ),
        (1, 6, 3)
    );
    // Independent integration in fixtures/cross_drill/manifest.json.
    assert!((result.report.volume_mm3 - 6837.46767288864).abs() < 1e-4);
    let inspection = result_faces(&result.bytes).unwrap();
    let mut neighbours: Vec<_> = inspection
        .faces
        .iter()
        .filter(|f| f.family == ResultFaceFamily::Cylinder)
        .map(|f| f.adjacent_face_ids.len())
        .collect();
    neighbours.sort_unstable();
    assert_eq!(neighbours, [2, 2, 4]);
}

#[test]
fn radius_uncertainty_must_clear_the_ratio_boundary() {
    let mut model = from_source(include_bytes!("../../../fixtures/cross_drill/f_ratio.stl"));
    let d = model.cross_drill.as_mut().unwrap();
    // These FOUR rims and both cycles still match the supports exactly.
    // The uncertainty interval alone now reaches beyond the 2/3 boundary.
    d.witness_tolerance_mm = (d.radii[1] - 1.5 * d.radii[0]) / 2.5;
    assert_eq!(
        build_rounded_step(&model).unwrap_err().code,
        "cad.invalid_model"
    );
}
