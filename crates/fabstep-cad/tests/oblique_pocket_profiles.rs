use fabstep_cad::{planar_face_profile, result_faces, ResultFaceFamily};
use fabstep_core::{export_planar_face_dxf, ProfileSegment};
use std::f64::consts::FRAC_PI_2;

// The independent FreeCAD BRep separates profile support from STL recognition.
#[test]
fn the_two_end_walls_share_a_circle_and_an_ellipse_with_the_same_cylinder() {
    let step = include_bytes!("../../../fixtures/oblique_pocket/freecad_reference.step");
    let result = result_faces(step).unwrap();
    assert_eq!(result.faces.len(), 12);
    let mut cylinders = result
        .faces
        .iter()
        .filter(|f| f.family == ResultFaceFamily::Cylinder);
    let cylinder = cylinders.next().unwrap();
    assert!(cylinders.next().is_none());
    assert_eq!(cylinder.adjacent_face_ids.len(), 4);
    let (mut circles, mut ellipses) = (0, 0);
    for &id in &cylinder.adjacent_face_ids {
        assert_eq!(result.faces[id].family, ResultFaceFamily::Plane);
        let profile = planar_face_profile(step, id).unwrap();
        assert_eq!(profile.loops.len(), 1);
        let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
        for segment in &profile.loops[0].segments {
            match segment {
                ProfileSegment::Arc {
                    radius_mm,
                    sweep_angle_rad,
                    ..
                } => {
                    assert!((radius_mm - 2.5).abs() < 1e-7);
                    assert!((sweep_angle_rad.abs() - FRAC_PI_2).abs() < 1e-7);
                    assert_eq!(dxf.matches("\n0\nARC\n").count(), 1);
                    circles += 1;
                }
                ProfileSegment::Ellipse {
                    major_axis_mm,
                    ratio,
                    sweep_param_rad,
                    ..
                } => {
                    assert!((major_axis_mm[0].hypot(major_axis_mm[1]) - 3.125).abs() < 1e-7);
                    assert!((ratio - 0.8).abs() < 1e-7);
                    assert!((sweep_param_rad.abs() - FRAC_PI_2).abs() < 1e-7);
                    assert_eq!(dxf.matches("\n0\nELLIPSE\n").count(), 1);
                    ellipses += 1;
                }
                ProfileSegment::Line { .. } => {}
                other => panic!("unexpected profile segment {other:?}"),
            }
        }
    }
    assert_eq!((circles, ellipses), (1, 1));
}
