use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;
const CASES: [(&str, &[u8], f64); 4] = [
    (
        "flared-coarse",
        include_bytes!("../../../fixtures/returning_meridian/flared-coarse.stl"),
        1445.5253197330035,
    ),
    (
        "flared-fine",
        include_bytes!("../../../fixtures/returning_meridian/flared-fine.stl"),
        1445.5253197330035,
    ),
    (
        "inverted-coarse",
        include_bytes!("../../../fixtures/returning_meridian/inverted-coarse.stl"),
        2224.5617580069325,
    ),
    (
        "inverted-fine",
        include_bytes!("../../../fixtures/returning_meridian/inverted-fine.stl"),
        2224.5617580069325,
    ),
];
#[test]
fn undercut_reimports_with_three_faces_both_directions_and_all_source_owners() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (name, raw, reference) in CASES {
        let converted = convert_document_with_provenance(raw, InputFormat::Stl, &o)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let cad = &converted.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.planar_faces,
                cad.revolution_faces,
                cad.faces
            ),
            (1, 2, 1, 3)
        );
        assert!((cad.volume_mm3 - reference).abs() < reference * 1e-7);
        let r = &converted.report.bodies[0].conversion;
        let proof = r.rounded_recognition.as_ref().unwrap();
        let curve = proof.model.smooth_revolution.as_ref().unwrap();
        assert!(curve.axial_controls_mm.is_some());
        assert!(curve.second_span.is_none());
        assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
        let f = r.fidelity.as_ref().unwrap();
        assert!(
            f.source_to_result_mm <= o.max_deviation_mm
                && f.result_to_source_mm <= o.max_deviation_mm
        );
        let result = converted.step.result_faces().unwrap();
        let wall = result
            .result
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::Revolution)
            .unwrap();
        assert_eq!(wall.adjacent_face_ids.len(), 2);
        let map = &result.source_correspondence;
        assert_eq!(map.status, "exact");
        assert_eq!(map.faces.len(), 3);
        let mut source = BTreeSet::new();
        for row in &map.faces {
            for &id in &row.source_triangle_ids {
                assert!(source.insert(id));
                let xyz = r.analysis.mesh.triangles[id].map(|i| r.analysis.mesh.vertices[i]);
                let (axis, low, high) = if name.starts_with("flared") {
                    (2, 0., 8.)
                } else {
                    (1, -15., -5.)
                };
                let cap = [low, high]
                    .into_iter()
                    .any(|z| xyz.iter().all(|p| (p[axis] - z).abs() < 1e-6));
                assert_eq!(
                    result.result.faces[row.face_id].family,
                    if cap {
                        ResultFaceFamily::Plane
                    } else {
                        ResultFaceFamily::Revolution
                    }
                );
            }
        }
        assert!(source.into_iter().eq(0..r.analysis.mesh.triangles.len()));
    }
}

#[test]
fn existing_two_disk_reports_keep_the_original_axial_parameter() {
    let previous = serde_json::json!({
        "origin":[0.,0.,0.], "axis":[0.,0.,1.], "height_mm":8.,
        "radii_mm":[3.,5.,4.,7.], "rings":[0,1], "witness_tolerance_mm":0.00001
    });
    let domain: fabstep_core::SmoothRevolution = serde_json::from_value(previous.clone()).unwrap();
    assert!(domain.axial_controls_mm.is_none());
    assert_eq!(serde_json::to_value(domain).unwrap(), previous);
}
