use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{
    analyze_mesh, read_stl_mesh, recognize_automatic, InputUnit, Options, ReconstructionMode,
};
use std::f64::consts::PI;

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn toroidal_groove_keeps_both_sharp_shoulders_and_diameters() {
    for (bytes, sweep) in [
        (
            include_bytes!("../../../fixtures/grooved_pulley/semicircle-coarse.stl").as_slice(),
            PI,
        ),
        (
            include_bytes!("../../../fixtures/grooved_pulley/semicircle-fine.stl").as_slice(),
            PI,
        ),
        (
            include_bytes!("../../../fixtures/grooved_pulley/shallow-coarse.stl").as_slice(),
            2.0,
        ),
        (
            include_bytes!("../../../fixtures/grooved_pulley/shallow-fine.stl").as_slice(),
            2.0,
        ),
    ] {
        let o = options();
        let built = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
        let report = &built.report.bodies[0].conversion;
        let m = &report.rounded_recognition.as_ref().unwrap().model;
        let cad = &report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces,
                cad.toroidal_faces
            ),
            (1, 5, 2, 2, 1)
        );
        let torus = &m.tori[0];
        let land = 9.0;
        let minor = 1.5;
        let major = land + minor * (sweep * 0.5).cos();
        let h = minor * (sweep * 0.5).sin();
        assert!((torus.minor_radius_mm - minor).abs() < 1e-5);
        assert!((torus.major_radius_mm - major).abs() < 1e-5);
        let mut z = torus
            .rings
            .unwrap()
            .map(|i| m.revolved.mixed.rings[i].center[2]);
        z.sort_by(f64::total_cmp);
        assert!((z[0] - (5. - h)).abs() < 1e-5 && (z[1] - (5. + h)).abs() < 1e-5);
        let arc_volume = 2. * h * (major * major + minor * minor)
            - 2. * h.powi(3) / 3.
            - 2. * major * (h * minor * (sweep * 0.5).cos() + minor * minor * sweep * 0.5);
        let volume = PI * (land * land * (9. - 2. * h) + arc_volume);
        assert!((cad.volume_mm3 - volume).abs() < 1e-6 * volume);
        let fidelity = report.fidelity.as_ref().unwrap();
        assert!(
            fidelity.source_samples > 0
                && fidelity.result_samples > 0
                && fidelity.source_to_result_mm <= o.max_deviation_mm
                && fidelity.result_to_source_mm <= o.max_deviation_mm
        );
        let inspection = built.step.result_faces().unwrap();
        assert_eq!(inspection.source_correspondence.status, "exact");
        let f = inspection
            .result
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::Torus)
            .unwrap();
        assert_eq!(f.adjacent_face_ids.len(), 2);
        assert!(f
            .adjacent_face_ids
            .iter()
            .all(|&i| inspection.result.faces[i].family == ResultFaceFamily::Cylinder));
    }
}

#[test]
fn distorted_grooves_are_not_replaced_by_a_torus() {
    for bytes in [
        include_bytes!("../../../fixtures/grooved_pulley/eccentric.stl").as_slice(),
        include_bytes!("../../../fixtures/grooved_pulley/off_circle.stl").as_slice(),
    ] {
        let o = options();
        let mesh = read_stl_mesh(bytes, InputUnit::Mm).unwrap();
        let a = analyze_mesh(&mesh, &o).unwrap();
        assert!(recognize_automatic(&a, &o).unwrap().model.tori.is_empty());
    }
}
