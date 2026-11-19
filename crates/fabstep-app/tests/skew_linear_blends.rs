use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn each_skew_blend_keeps_the_correct_source_triangles_and_four_planar_neighbors() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (bytes, volume) in [
        (
            include_bytes!("../../../fixtures/skew_linear_blends/vertical-coarse.stl").as_slice(),
            3151.607063,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/vertical-fine.stl").as_slice(),
            3151.607063,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/swapped-coarse.stl").as_slice(),
            3152.958225,
        ),
        (
            include_bytes!("../../../fixtures/skew_linear_blends/swapped-fine.stl").as_slice(),
            3152.958225,
        ),
    ] {
        let doc = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (cad.solids, cad.faces, cad.planar_faces, cad.conical_faces),
            (1, 8, 6, 2)
        );
        assert!((cad.volume_mm3 - volume).abs() < 0.0001);
        let faces = doc.step.result_faces().unwrap();
        let corr = &faces.source_correspondence;
        assert_eq!(corr.status, "exact");
        let mut owners = Vec::new();
        let mut cone_side = [false; 2];
        for row in &corr.faces {
            let f = &faces.result.faces[row.face_id];
            let mut which = None;
            for &id in &row.source_triangle_ids {
                let start = 84 + id * 50 + 12;
                let points: [[f64; 3]; 3] = std::array::from_fn(|v| {
                    std::array::from_fn(|j| {
                        let a = start + v * 12 + j * 4;
                        f32::from_le_bytes(bytes[a..a + 4].try_into().unwrap()) as f64
                    })
                });
                // Independent planar oracle: only a triangle on a stock
                // bounding plane has all three vertices close to that plane.
                let plane = [20., 16., 10.].iter().enumerate().any(|(k, &h)| {
                    [0., h]
                        .into_iter()
                        .any(|s| points.iter().all(|p| (p[k] - s).abs() < 1e-5))
                });
                assert_eq!(plane, f.family == ResultFaceFamily::Plane);
                if !plane {
                    let high = points[0][0] > 10.;
                    assert!(points.iter().all(|p| (p[0] > 10.) == high));
                    assert!(which.replace(high).is_none_or(|previous| previous == high));
                }
                owners.push(id);
            }
            if f.family == ResultFaceFamily::Cone {
                let side = usize::from(which.unwrap());
                assert!(!cone_side[side]);
                cone_side[side] = true;
                assert_eq!(f.adjacent_face_ids.len(), 4);
                assert!(f
                    .adjacent_face_ids
                    .iter()
                    .all(|&id| faces.result.faces[id].family == ResultFaceFamily::Plane));
            }
        }
        assert_eq!(cone_side, [true; 2]);
        let conversion = &doc.report.bodies[0].conversion;
        owners.sort_unstable();
        assert!(owners
            .into_iter()
            .eq(0..conversion.analysis.mesh.triangles.len()));
        let audit = conversion.fidelity.as_ref().unwrap();
        assert!(audit.source_samples > 0 && audit.result_samples > 0);
        assert!(
            audit.source_to_result_mm <= o.max_deviation_mm
                && audit.result_to_source_mm <= o.max_deviation_mm
        );
    }
}
