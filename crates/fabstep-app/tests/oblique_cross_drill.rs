//! Both axes tilt relative to one another. The four mouths are circles in
//! four skew stock planes that are normal to their own bore.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ProfileSegment, ReconstructionMode};

const ACUTE: &[u8] = include_bytes!("../../../fixtures/oblique_cross_drill/angle-60-coarse.stl");
const STEEP: &[u8] = include_bytes!("../../../fixtures/oblique_cross_drill/angle-75-coarse.stl");

#[test]
fn inclined_bores_have_exact_mouths_bidirectional_fidelity_and_source_ownership() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (bytes, angle, volume) in [
        (ACUTE, 60_f64, 23856.175867449572),
        (STEEP, 75., 21089.937824591914),
    ] {
        let doc = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces
            ),
            (1, 9, 6, 3)
        );
        assert!((cad.volume_mm3 - volume).abs() < 0.001);
        let body = &doc.report.bodies[0].conversion;
        let recognized = body.rounded_recognition.as_ref().unwrap();
        let (s, c) = angle.to_radians().sin_cos();
        let model = recognized.model.cross_drill.as_ref().unwrap();
        assert!((model.large_axis_cosine.abs() - c).abs() < 2e-6);
        assert!(recognized.report.max_deviation_bound_mm <= options.max_deviation_mm);
        let fidelity = body.fidelity.as_ref().unwrap();
        assert!(
            fidelity.source_to_result_mm <= options.max_deviation_mm
                && fidelity.result_to_source_mm <= options.max_deviation_mm
        );
        let f = doc.step.result_faces().unwrap();
        assert_eq!(f.source_correspondence.status, "exact");
        let mut owned = vec![false; (bytes.len() - 84) / 50];
        let mut circular = [0; 2];
        let mut walls = [false; 3];
        for row in &f.source_correspondence.faces {
            let face = &f.result.faces[row.face_id];
            assert!(!row.source_triangle_ids.is_empty());
            let mut candidates = [true; 3];
            for &tri in &row.source_triangle_ids {
                assert!(!std::mem::replace(&mut owned[tri], true));
                if face.family != ResultFaceFamily::Cylinder {
                    continue;
                }
                let off = 84 + 50 * tri + 12;
                for p in bytes[off..off + 36].chunks_exact(12) {
                    let [x, y, z] = std::array::from_fn(|i| {
                        f32::from_le_bytes(p[4 * i..4 * i + 4].try_into().unwrap()) as f64
                    });
                    let transverse = s * x - c * z;
                    let on_small = (transverse.hypot(y) - 2.).abs() < 2e-5;
                    candidates[0] &= on_small && z < 0.;
                    candidates[1] &= on_small && z > 0.;
                    candidates[2] &= (y.hypot(z) - 5.).abs() < 2e-5;
                }
            }
            if face.family == ResultFaceFamily::Cylinder {
                let id = candidates
                    .into_iter()
                    .position(|ok| ok)
                    .expect("a single actual bore wall must own all facets");
                assert!(!std::mem::replace(&mut walls[id], true));
            } else {
                let profile = doc.step.planar_face_profile(face.id).unwrap();
                for seg in profile.loops.iter().flat_map(|l| &l.segments) {
                    if let ProfileSegment::Circle { radius_mm, .. } = seg {
                        let i = usize::from(*radius_mm > 3.);
                        assert!((radius_mm - [2., 5.][i]).abs() < 2e-5);
                        circular[i] += 1;
                    }
                }
            }
        }
        assert!(owned.into_iter().all(|b| b) && walls.into_iter().all(|b| b));
        assert_eq!(circular, [2, 2]);
    }
}
