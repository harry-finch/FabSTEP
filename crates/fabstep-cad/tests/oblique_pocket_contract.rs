use fabstep_cad::{
    build_mixed_step, build_rounded_step_with_all_provenance, result_faces, ResultFaceFamily,
};
use fabstep_core::{
    CircularArc, EllipticArc, MixedLoop, MixedModel, MixedPartialCylinder, MixedPlane,
    MixedSegment, RevolvedModel, RoundedModel,
};
use std::f64::consts::{FRAC_PI_2, PI};

fn polygon(normal: [f64; 3], ids: &[usize]) -> MixedPlane {
    MixedPlane {
        normal,
        loops: vec![MixedLoop::Polygon {
            vertices: ids.to_vec(),
        }],
    }
}

// The solid is specified independently of the recognizer. The end stop
// x=5+y/2 clips the right end of a pocket in an otherwise rectangular block.
fn pocket() -> MixedModel {
    let mut vertices = vec![];
    for z in [-2., 6.] {
        vertices.extend([[-10., -8., z], [10., -8., z], [10., 8., z], [-10., 8., z]]);
    }
    for far in [false, true] {
        for [y, z] in [[-4., 6.], [-4., 2.], [-2., 0.], [4., 0.], [4., 6.]] {
            vertices.push([if far { 5. + y / 2. } else { -5. }, y, z]);
        }
    }
    let mut planes = vec![
        polygon([0., 0., -1.], &[0, 3, 2, 1]),
        polygon([0., -1., 0.], &[0, 1, 5, 4]),
        polygon([1., 0., 0.], &[1, 2, 6, 5]),
        polygon([0., 1., 0.], &[2, 3, 7, 6]),
        polygon([-1., 0., 0.], &[3, 0, 4, 7]),
        polygon([0., 0., 1.], &[4, 5, 6, 7]),
        polygon([0., 1., 0.], &[9, 8, 13, 14]),
        polygon([0., 0., 1.], &[10, 15, 16, 11]),
        polygon([0., -1., 0.], &[11, 16, 17, 12]),
    ];
    planes[5].loops.push(MixedLoop::Polygon {
        vertices: vec![8, 12, 17, 13],
    });
    let stretch = 1.25_f64.sqrt();
    let axis = [1., 0., 0.];
    let oblique = [1. / stretch, -0.5 / stretch, 0.];
    let line = |from, to| MixedSegment::Line {
        vertices: [from, to],
    };
    planes.push(MixedPlane {
        normal: axis,
        loops: vec![MixedLoop::Segments {
            segments: vec![
                line(8, 9),
                MixedSegment::Arc {
                    arc: 0,
                    reversed: false,
                },
                line(10, 11),
                line(11, 12),
                line(12, 8),
            ],
        }],
    });
    planes.push(MixedPlane {
        normal: oblique.map(|v| -v),
        loops: vec![MixedLoop::Segments {
            segments: vec![
                line(13, 17),
                line(17, 16),
                line(16, 15),
                MixedSegment::EllipticArc {
                    arc: 0,
                    reversed: true,
                },
                line(14, 13),
            ],
        }],
    });
    MixedModel {
        vertices,
        planes,
        rings: vec![],
        cylinders: vec![],
        arcs: vec![CircularArc {
            center: [-5., -2., 2.],
            axis,
            radius_mm: 2.,
            sweep_rad: FRAC_PI_2,
            vertices: [9, 10],
        }],
        elliptic_arcs: vec![EllipticArc {
            center: [4., -2., 2.],
            axis: oblique,
            major_direction: [0.5 / stretch, 1. / stretch, 0.],
            major_mm: 2. * stretch,
            minor_mm: 2.,
            start_rad: PI,
            sweep_rad: FRAC_PI_2,
            vertices: [14, 15],
        }],
        partial_cylinders: vec![MixedPartialCylinder {
            arcs: [0, 0],
            elliptic_start: false,
            elliptic_end: true,
            inner: true,
        }],
    }
}

#[test]
fn one_circle_and_one_ellipse_reimport_as_a_single_concave_fillet() {
    let geometry = pocket();
    let step = build_mixed_step(&geometry).unwrap();
    assert_eq!(
        (
            step.report.solids,
            step.report.planar_faces,
            step.report.cylindrical_faces
        ),
        (1, 11, 1)
    );
    let volume = 20. * 16. * 8. - 8. * 6. * 10. + 9. * 4. * (1. - PI / 4.) - 0.5 * 8. / 6.;
    assert!((step.report.volume_mm3 - volume).abs() < 1e-5);
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed: geometry,
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        torus_sectors: vec![],
        partial_cones: vec![],
        octants: vec![],
        linear_blend: None,
        drilled_shaft: None,
        cross_drill: None,
        smooth_revolution: None,
        unequal_pocket: None,
    };
    let tokens: Vec<_> = (21..=31).rev().collect();
    let tracked = |wall: &[usize]| {
        build_rounded_step_with_all_provenance(&model, &tokens, &[], wall, &[], &[], &[])
    };
    assert!(tracked(&[21]).is_err());
    let faces = result_faces(&tracked(&[63]).unwrap().bytes).unwrap().faces;
    let wall = faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Cylinder)
        .unwrap();
    assert_eq!(wall.source_face_token, Some(63));
    let mut neighbor_tokens: Vec<_> = wall
        .adjacent_face_ids
        .iter()
        .map(|&id| faces[id].source_face_token.unwrap())
        .collect();
    neighbor_tokens.sort_unstable();
    assert_eq!(neighbor_tokens, [21, 22, 24, 25]);
}

#[test]
fn cylinder_and_neighbors_must_agree_before_transfer() {
    let mutate: &[fn(&mut MixedModel)] = &[
        |m| m.partial_cylinders[0].inner = false,
        |m| m.partial_cylinders[0].elliptic_end = false,
        |m| m.partial_cylinders[0].arcs[1] = 1,
        |m| m.elliptic_arcs[0].major_mm += 0.01,
        |m| m.elliptic_arcs[0].center[0] -= 20.,
        |m| m.elliptic_arcs[0].axis = m.elliptic_arcs[0].axis.map(|v| -v),
        |m| m.elliptic_arcs[0].major_direction = [0., 0., 1.],
        |m| m.elliptic_arcs[0].vertices.reverse(),
        |m| m.arcs[0].vertices[1] = usize::MAX,
        |m| m.vertices[15][0] += 0.005,
        |m| m.planes[9].normal[0] = -1.,
        |m| m.planes[5].loops.truncate(1),
        |m| {
            if let MixedLoop::Segments { segments } = &mut m.planes[10].loops[0] {
                segments[3] = MixedSegment::Arc {
                    arc: 0,
                    reversed: true,
                };
            }
        },
    ];
    for (i, change) in mutate.iter().enumerate() {
        let mut model = pocket();
        change(&mut model);
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model",
            "mutation {i}"
        );
    }
}

#[test]
fn double_oblique_native_contract_rejects_malformed_and_nontransverse_stops() {
    let o = fabstep_core::Options {
        reconstruction: fabstep_core::ReconstructionMode::Auto,
        ..Default::default()
    };
    let a = fabstep_core::analyze_stl(
        include_bytes!("../../../fixtures/double_oblique_pocket/opposed.stl"),
        &o,
    )
    .unwrap();
    let model = fabstep_core::recognize_automatic(&a, &o)
        .unwrap()
        .model
        .revolved
        .mixed;
    assert!(model.partial_cylinders[0].elliptic_start);
    for mutate in [
        (|m: &mut MixedModel| m.partial_cylinders[0].elliptic_end = false) as fn(&mut MixedModel),
        |m| m.partial_cylinders[0].elliptic_start = false,
        |m| m.elliptic_arcs[0].axis = [1., 0., 0.],
        |m| m.elliptic_arcs[1].center = m.elliptic_arcs[0].center,
        |m| m.elliptic_arcs[0].major_mm += 0.01,
        |m| m.elliptic_arcs[1].start_rad += 0.01,
        |m| m.elliptic_arcs[1].vertices.reverse(),
        |m| m.partial_cylinders[0].arcs = [1, 0],
    ] {
        let mut bad = model.clone();
        mutate(&mut bad);
        assert_eq!(
            build_mixed_step(&bad).unwrap_err().code,
            "cad.invalid_model"
        );
    }
}
