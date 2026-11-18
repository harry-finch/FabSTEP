//! The reference dimensions and independent volume are from fixtures/unequal_pocket/nominal.
use fabstep_cad::{
    build_mixed_step, build_rounded_step, build_rounded_step_with_all_provenance,
    combine_steps_with_provenance, planar_face_profile, result_faces, ResultFaceFamily, StepPart,
};
use fabstep_core::{
    export_planar_face_dxf, MixedLoop, MixedModel, MixedPartialCylinder, MixedPlane, MixedSphere,
    ProfileSegment, RevolvedModel, RoundedModel, UnequalPocket,
};
use std::collections::BTreeSet;

fn pocket(reflected: bool) -> RoundedModel {
    let geometry = UnequalPocket {
        origin: if reflected {
            [4., 60., -2.]
        } else {
            [-8., -5., 3.]
        },
        axes: if reflected {
            // Reflected orthonormal frame. None of its axes is a world axis.
            [
                [2. / 3., -1. / 3., 2. / 3.],
                [-2. / 3., 2. / 15., 11. / 15.],
                [1. / 3., 14. / 15., 2. / 15.],
            ]
        } else {
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        },
        radii: [2., 3.],
        outer_min: [-7., -5., -3.],
        outer_max: [23., 15., 9.],
        pocket_spans: [16., 10.],
    };
    let [x, y, z] = geometry.axes;
    let neg = |v: [f64; 3]| v.map(|a| -a);
    let normals = [neg(x), x, neg(y), y, neg(z), z, x, neg(x), y, neg(y), z];
    let planes = normals
        .into_iter()
        .map(|normal| MixedPlane {
            normal,
            loops: vec![],
        })
        .collect();
    RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                planes,
                vertices: vec![],
                partial_cylinders: vec![],
                elliptic_arcs: vec![],
                arcs: vec![],
                rings: vec![],
                cylinders: vec![],
            },
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        torus_sectors: vec![],
        partial_cones: vec![],
        octants: vec![],
        linear_blend: None,
        cross_drill: None,
        smooth_revolution: None,
        unequal_pocket: Some(Box::new(geometry)),
    }
}

#[test]
fn both_frames_preserve_the_exact_faces_and_extra_arc_in_a_document() {
    let steps = [false, true].map(|reflected| {
        let base = if reflected { 101 } else { 1 };
        let planes = (base..base + 11).collect::<Vec<_>>();
        let step = build_rounded_step_with_all_provenance(
            &pocket(reflected),
            &planes,
            &[],
            &[base + 11, base + 12],
            &[],
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            (
                step.report.solids,
                step.report.planar_faces,
                step.report.cylindrical_faces
            ),
            (1, 11, 2)
        );
        assert!((step.report.volume_mm3 - 5791.67320308558).abs() < 0.0003);
        step
    });
    let combined = combine_steps_with_provenance(&[
        StepPart {
            bytes: &steps[0].bytes,
            name: "Poche",
        },
        StepPart {
            bytes: &steps[1].bytes,
            name: "Miroir",
        },
    ])
    .unwrap();
    assert_eq!((combined.report.solids, combined.report.faces), (2, 26));
    let result = result_faces(&combined.bytes).unwrap();
    for (body, base) in [1, 101].into_iter().enumerate() {
        let faces = &result.faces;
        let face_for = |token| {
            faces
                .iter()
                .find(|f| f.source_face_token == Some(token) && f.solid_id == body)
                .unwrap()
        };
        assert_eq!(
            faces
                .iter()
                .filter(|f| f.solid_id == body && f.triangle_count > 0)
                .filter_map(|f| f.source_face_token)
                .collect::<BTreeSet<_>>(),
            (base..base + 13).collect()
        );
        for (index, expected) in [(11, vec![7, 8, 10, 12]), (12, vec![6, 8, 9, 10, 11])] {
            let wall = face_for(base + index);
            assert_eq!(wall.family, ResultFaceFamily::Cylinder);
            let mut peers: Vec<_> = wall
                .adjacent_face_ids
                .iter()
                .map(|&i| {
                    assert_eq!(faces[i].solid_id, body);
                    faces[i].source_face_token.unwrap() - base
                })
                .collect();
            peers.sort_unstable();
            assert_eq!(peers, expected);
        }
        let profile = planar_face_profile(&combined.bytes, face_for(base + 8).id).unwrap();
        let arcs: Vec<_> = profile
            .loops
            .iter()
            .flat_map(|l| &l.segments)
            .filter_map(|s| match s {
                ProfileSegment::Arc {
                    radius_mm,
                    sweep_angle_rad,
                    ..
                } => Some((*radius_mm, sweep_angle_rad.abs())),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 1);
        assert!((arcs[0].0 - 3.).abs() < 1e-7);
        assert!((arcs[0].1 - (1_f64 / 3.).asin()).abs() < 1e-7);
        let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
        assert_eq!(dxf.matches("\n0\nARC\n").count(), 1);
    }
}

#[test]
fn incompatible_dimensions_and_forged_owners_are_refused() {
    let bad: &[fn(&mut UnequalPocket)] = &[
        |p| p.origin[1] = f64::NAN,
        |p| p.axes[1] = p.axes[0],
        |p| p.axes[2][2] = 0.5,
        |p| p.radii.swap(0, 1),
        |p| p.radii[0] = p.radii[1],
        |p| p.radii[0] = 0.,
        |p| p.outer_min[0] = 0.,
        |p| p.outer_min[2] = 0.,
        |p| p.outer_max[1] = f64::INFINITY,
        |p| p.outer_max[2] = p.radii[1],
        |p| p.outer_max[0] = p.pocket_spans[0],
        |p| p.pocket_spans[1] = p.radii[0],
    ];
    for (i, mutate) in bad.iter().enumerate() {
        let mut model = pocket(false);
        mutate(model.unequal_pocket.as_mut().unwrap());
        assert_eq!(
            build_rounded_step(&model).unwrap_err().code,
            "cad.invalid_model",
            "dimension {i}"
        );
    }
    let bad: &[fn(&mut RoundedModel)] = &[
        |m| m.revolved.mixed.planes.swap(0, 2),
        |m| {
            m.revolved.mixed.planes[2].loops.push(MixedLoop::Polygon {
                vertices: vec![0, 1, 2],
            })
        },
        |m| m.revolved.mixed.vertices.push([0.; 3]),
        |m| {
            m.revolved
                .mixed
                .partial_cylinders
                .push(MixedPartialCylinder {
                    arcs: [0, 0],
                    inner: true,
                    elliptic_end: false,
                })
        },
        |m| {
            m.revolved.mixed.planes.pop();
        },
        |m| {
            m.spheres.push(MixedSphere {
                rings: None,
                inner: false,
                center: [0.; 3],
                radius_mm: 2.,
            })
        },
    ];
    for mutate in bad {
        let mut model = pocket(false);
        mutate(&mut model);
        assert_eq!(
            build_rounded_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
    }
    let m = pocket(false);
    assert!(build_mixed_step(&m.revolved.mixed).is_err());
    let planes = (1..12).collect::<Vec<_>>();
    for wall_tokens in [&[][..], &[12][..], &[10, 12][..]] {
        assert_eq!(
            build_rounded_step_with_all_provenance(&m, &planes, &[], wall_tokens, &[], &[], &[])
                .unwrap_err()
                .code,
            "cad.invalid_model"
        );
    }
}

#[test]
fn reimport_refuses_a_quartic_with_the_right_degree_and_wrong_geometry() {
    let step = build_rounded_step(&pocket(false)).unwrap();
    let mut text = String::from_utf8(step.bytes).unwrap();
    let quartic = text.find("B_SPLINE_CURVE(4,").unwrap();
    let marker = "RATIONAL_B_SPLINE_CURVE((";
    let start = quartic + text[quartic..].find(marker).unwrap() + marker.len();
    let end = start + text[start..].find("))").unwrap();
    let mut weights: Vec<_> = text[start..end].split(',').map(str::trim).collect();
    assert_eq!(weights.len(), 5);
    weights[2] = "2.5";
    let changed = weights.join(",");
    text.replace_range(start..end, &changed);
    assert!(result_faces(text.as_bytes()).is_err());
    assert!(combine_steps_with_provenance(&[StepPart {
        bytes: text.as_bytes(),
        name: "Quartique altérée"
    }])
    .is_err());
}
