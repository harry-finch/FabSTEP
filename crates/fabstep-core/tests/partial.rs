use fabstep_core::{
    analyze_stl, recognize_mixed, MixedFaceRef, MixedLoop, MixedSegment, Options,
    ReconstructionMode,
};
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}
fn recognize(bytes: &[u8]) -> fabstep_core::MixedRecognition {
    let o = options();
    let a = analyze_stl(bytes, &o).unwrap();
    recognize_mixed(&a, &o).unwrap()
}

#[test]
fn independent_open_arcs_preserve_endpoints_and_replace_complete_walls() {
    for source in [
        include_bytes!("../../../fixtures/m2c_open_arc.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_subdivided.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_rotated.stl").as_slice(),
    ] {
        let r = recognize(source);
        assert_eq!(r.model.partial_cylinders.len(), 1);
        assert_eq!(r.model.planes.len(), 6);
        assert_eq!(r.model.arcs.len(), 2);
        assert_eq!(r.report.cylinders, 1);
        assert!(r.model.partial_cylinders[0].inner);
        assert!((r.report.expected_volume_mm3 - 53857.34658341286).abs() < 1e-6);
        assert!((r.model.arcs[0].radius_mm - 18.).abs() < 1e-9);
        assert!((r.model.arcs[0].sweep_rad - 2.524170533697723).abs() < 1e-9);
        assert!((r.report.max_chord_error_mm - 0.011060389475308519).abs() < 1e-9);
        assert!(r
            .triangle_faces
            .iter()
            .any(|f| matches!(f, MixedFaceRef::PartialCylinder { index: 0 })));
        let mut uses = [0; 2];
        for plane in &r.model.planes {
            for lp in &plane.loops {
                if let MixedLoop::Segments { segments } = lp {
                    for s in segments {
                        if let MixedSegment::Arc { arc, .. } = s {
                            uses[*arc] += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(uses, [1, 1]);
        for arc in &r.model.arcs {
            for &v in &arc.vertices {
                let p = r.model.vertices[v];
                let d = std::array::from_fn::<_, 3, _>(|i| p[i] - arc.center[i]);
                assert!((d[0].hypot(d[1]).hypot(d[2]) - arc.radius_mm).abs() < 1e-9);
                assert!(
                    d.iter()
                        .zip(arc.axis)
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                        .abs()
                        < 1e-9
                );
            }
        }
    }
}

#[test]
fn partial_and_complete_cylinders_coexist_in_mixed_solids() {
    for source in [
        include_bytes!("../../../fixtures/m2c_open_arc_mixed.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_mixed_rotated.stl").as_slice(),
    ] {
        let r = recognize(source);
        assert_eq!(r.model.partial_cylinders.len(), 1);
        assert_eq!(r.model.cylinders.len(), 2);
        assert_eq!(r.model.planes.len(), 7);
        assert_eq!(r.report.cylinders, 3);
        assert!((r.report.expected_volume_mm3 - 53345.26698087772).abs() < 1e-6);
    }
    let r = recognize(include_bytes!("../../../fixtures/m2c_open_arc_hole.stl"));
    assert_eq!(r.report.cylinders, 2);
    assert!((r.report.expected_volume_mm3 - 52952.567899179).abs() < 1e-6);
    let r = recognize(include_bytes!("../../../fixtures/m2c_arc_other_angle.stl"));
    assert!((r.model.arcs[0].radius_mm - 21.).abs() < 1e-9);
    assert!((r.report.expected_volume_mm3 - 51282.20548677901).abs() < 1e-6);
}

#[test]
fn defective_complete_candidates_are_not_retried_as_shorter_arcs() {
    let o = options();
    for source in [
        include_bytes!("../../../fixtures/m2c_arc_conical.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_arc_bulge.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_arc_mismatch.stl").as_slice(),
    ] {
        let a = analyze_stl(source, &o).unwrap();
        assert_eq!(
            recognize_mixed(&a, &o).unwrap_err().code,
            "recognition.partial_unproven"
        );
    }
    let o = Options {
        max_deviation_mm: 1.,
        ..options()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m2c_arc_few_segments.stl"),
        &o,
    )
    .unwrap();
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.unsupported"
    );
    let o = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    let a = analyze_stl(include_bytes!("../../../fixtures/m2c_open_arc.stl"), &o).unwrap();
    assert_eq!(
        recognize_mixed(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

// Independent convex polygon prism writer: no recognition code constructs it.
fn polygon_prism(profile: &[[f64; 2]], cap_center: [f64; 2]) -> Vec<u8> {
    use std::fmt::Write;
    let mut stl = String::from("solid regression\n");
    let mut triangle = |p: [[f64; 3]; 3]| {
        stl.push_str("facet normal 0 0 0\nouter loop\n");
        for v in p {
            writeln!(stl, "vertex {:.17} {:.17} {:.17}", v[0], v[1], v[2]).unwrap();
        }
        stl.push_str("endloop\nendfacet\n");
    };
    for i in 0..profile.len() {
        let a = profile[i];
        let b = profile[(i + 1) % profile.len()];
        let a0 = [a[0], a[1], 0.];
        let b0 = [b[0], b[1], 0.];
        let a1 = [a[0], a[1], 10.];
        let b1 = [b[0], b[1], 10.];
        triangle([[cap_center[0], cap_center[1], 0.], b0, a0]);
        triangle([[cap_center[0], cap_center[1], 10.], a1, b1]);
        triangle([a0, b0, b1]);
        triangle([a0, b1, a1]);
    }
    stl.push_str("endsolid regression\n");
    stl.into_bytes()
}

#[test]
fn sparse_complete_circle_cannot_be_reinterpreted_as_a_partial_wall() {
    let o = options();
    let mut profile: Vec<_> = (0..=8)
        .map(|i| {
            let angle = (i as f64 * 260. / 8.).to_radians();
            [angle.cos(), angle.sin()]
        })
        .collect();
    profile.push([310_f64.to_radians().cos(), 310_f64.to_radians().sin()]);
    // Rotate the boundary enumeration to exercise internal and closing gaps.
    for _ in 0..profile.len() {
        let a = analyze_stl(&polygon_prism(&profile, [0., 0.]), &o).unwrap();
        assert_eq!(
            recognize_mixed(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
        profile.rotate_left(1);
    }
}

#[test]
fn d_profile_is_independent_of_collinear_chord_subdivision() {
    let mut profile: Vec<_> = (0..=16)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::PI / 16.;
            [2. * angle.cos(), 2. * angle.sin()]
        })
        .collect();
    for split in [false, true] {
        if split {
            profile.push([0., 0.]);
        }
        let r = recognize(&polygon_prism(&profile, [0., 0.5]));
        assert_eq!(r.model.partial_cylinders.len(), 1);
        assert_eq!(r.model.planes.len(), 3);
        assert!((r.report.expected_volume_mm3 - 20. * std::f64::consts::PI).abs() < 1e-9);
    }
}
