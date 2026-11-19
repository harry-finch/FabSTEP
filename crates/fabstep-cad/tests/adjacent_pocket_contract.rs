use fabstep_cad::{
    build_mixed_step, build_rounded_step_with_all_provenance, result_faces, ResultFaceFamily,
};
use fabstep_core::{
    CircularArc, EllipticArc, MixedLoop, MixedModel, MixedPartialCylinder, MixedPlane,
    MixedSegment, RevolvedModel, RoundedModel,
};
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI, SQRT_2};

fn polygon(normal: [f64; 3], indices: &[usize]) -> MixedPlane {
    MixedPlane {
        normal,
        loops: vec![MixedLoop::Polygon {
            vertices: indices.to_vec(),
        }],
    }
}
fn stop(normal: [f64; 3], arc: usize, rest: &[usize]) -> MixedPlane {
    let mut segments = vec![MixedSegment::Arc {
        arc,
        reversed: false,
    }];
    segments.extend(rest.windows(2).map(|p| MixedSegment::Line {
        vertices: [p[0], p[1]],
    }));
    MixedPlane {
        normal,
        loops: vec![MixedLoop::Segments { segments }],
    }
}

// Independent of triangulation and recognition: a 20x16x8 block with a
// 10x8x6 pocket and two equal R2 fillets meeting at the x-min/y-min corner.
fn pocket() -> MixedModel {
    let mut vertices = vec![];
    for z in [-2., 6.] {
        vertices.extend([[-10., -8., z], [10., -8., z], [10., 8., z], [-10., 8., z]]);
    }
    vertices.extend([
        [-5., -4., 6.],
        [5., -4., 6.],
        [5., 4., 6.],
        [-5., 4., 6.],
        [-5., -4., 2.],
        [5., -4., 2.],
        [-5., 4., 2.],
        [-3., -2., 0.],
        [5., -2., 0.],
        [-3., 4., 0.],
        [5., 4., 0.],
    ]);
    let mut planes = vec![
        polygon([0., 0., -1.], &[0, 3, 2, 1]),
        polygon([0., -1., 0.], &[0, 1, 5, 4]),
        polygon([1., 0., 0.], &[1, 2, 6, 5]),
        polygon([0., 1., 0.], &[2, 3, 7, 6]),
        polygon([-1., 0., 0.], &[3, 0, 4, 7]),
        polygon([0., 0., 1.], &[4, 5, 6, 7]),
        polygon([0., 1., 0.], &[12, 8, 9, 13]),
        polygon([1., 0., 0.], &[12, 14, 11, 8]),
        polygon([0., 0., 1.], &[15, 16, 18, 17]),
        stop([-1., 0., 0.], 0, &[13, 9, 10, 18, 16]),
        stop([0., -1., 0.], 1, &[17, 18, 10, 11, 14]),
    ];
    planes[5].loops.push(MixedLoop::Polygon {
        vertices: vec![8, 11, 10, 9],
    });
    MixedModel {
        vertices,
        planes,
        rings: vec![],
        cylinders: vec![],
        arcs: vec![
            CircularArc {
                center: [5., -2., 2.],
                axis: [-1., 0., 0.],
                vertices: [16, 13],
                radius_mm: 2.,
                sweep_rad: FRAC_PI_2,
            },
            CircularArc {
                center: [-3., 4., 2.],
                axis: [0., -1., 0.],
                vertices: [14, 17],
                radius_mm: 2.,
                sweep_rad: FRAC_PI_2,
            },
        ],
        elliptic_arcs: vec![EllipticArc {
            center: [-3., -2., 2.],
            axis: [FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0.],
            major_direction: [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.],
            major_mm: 2. * SQRT_2,
            minor_mm: 2.,
            start_rad: PI,
            sweep_rad: FRAC_PI_2,
            vertices: [12, 15],
        }],
        partial_cylinders: (0..2)
            .map(|i| MixedPartialCylinder {
                arcs: [i, 0],
                elliptic_start: false,
                elliptic_end: true,
                inner: true,
            })
            .collect(),
    }
}

#[test]
fn both_ellipse_orientations_and_wall_orders_preserve_geometry_and_provenance() {
    for reverse in [false, true] {
        let mut geometry = pocket();
        if reverse {
            let e = &mut geometry.elliptic_arcs[0];
            e.axis = e.axis.map(|v| -v);
            e.vertices.swap(0, 1);
            e.start_rad = -(e.start_rad + e.sweep_rad);
            geometry.partial_cylinders.swap(0, 1);
        }
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
        let plane_tokens: Vec<_> = (31..42).collect();
        // Source tokens 90 and 91 track the X and Y cylinder, independently
        // of their storage order in the mixed model.
        let wall_tokens = if reverse { [91, 90] } else { [90, 91] };
        let step = build_rounded_step_with_all_provenance(
            &model,
            &plane_tokens,
            &[],
            &wall_tokens,
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
        let volume =
            20. * 16. * 8. - 10. * 8. * 6. + 18. * 4. * (1. - PI / 4.) - 8. * (5. / 3. - PI / 2.);
        assert!((step.report.volume_mm3 - volume).abs() < 1e-5);
        let faces = result_faces(&step.bytes).unwrap().faces;
        for (token, peers) in [(90, vec![37, 39, 40, 91]), (91, vec![38, 39, 41, 90])] {
            let wall = faces
                .iter()
                .find(|f| f.source_face_token == Some(token))
                .unwrap();
            assert_eq!(wall.family, ResultFaceFamily::Cylinder);
            let mut actual: Vec<_> = wall
                .adjacent_face_ids
                .iter()
                .map(|&id| faces[id].source_face_token.unwrap())
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, peers);
        }
    }
}

#[test]
fn inconsistent_neighbor_curves_or_ownership_are_refused() {
    let bad: &[fn(&mut MixedModel)] = &[
        |m| m.partial_cylinders[0].elliptic_end = false,
        |m| m.partial_cylinders[1].inner = false,
        |m| m.partial_cylinders[1].arcs[0] = 0,
        |m| m.partial_cylinders[0].arcs[1] = 1,
        |m| m.arcs[0].axis = [0., -1., 0.],
        |m| m.arcs[0].radius_mm += 0.001,
        |m| m.elliptic_arcs[0].vertices.swap(0, 1),
        |m| m.elliptic_arcs[0].start_rad += 0.1,
        |m| m.elliptic_arcs[0].sweep_rad -= 0.01,
        |m| m.elliptic_arcs[0].major_mm += 0.01,
        |m| m.arcs[1].vertices[0] = usize::MAX,
        |m| m.elliptic_arcs[0].center[0] += 0.01,
        |m| m.vertices[12][2] += 0.01,
        |m| m.partial_cylinders[0].elliptic_start = true,
        |m| m.planes[5].loops.truncate(1),
        |m| m.planes[8].normal = [0., 1., 0.],
        |m| {
            if let MixedLoop::Segments { segments } = &mut m.planes[9].loops[0] {
                segments[0] = MixedSegment::EllipticArc {
                    arc: 0,
                    reversed: true,
                }
            }
        },
        |m| {
            if let MixedLoop::Segments { segments } = &mut m.planes[10].loops[0] {
                segments[0] = MixedSegment::Arc {
                    arc: 0,
                    reversed: false,
                }
            }
        },
    ];
    for (i, mutate) in bad.iter().enumerate() {
        let mut model = pocket();
        mutate(&mut model);
        assert_eq!(
            build_mixed_step(&model).unwrap_err().code,
            "cad.invalid_model",
            "mutation {i}"
        );
    }
}
