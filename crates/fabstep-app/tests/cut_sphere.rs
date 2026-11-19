use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ProfileSegment, ReconstructionMode, RoundedFaceRef};
use std::collections::BTreeSet;

// An independently tessellated R5 octant. A triangular barycentric grid is
// projected radially, and three fans close its boundary. The frame is rotated
// by a 3-4-5 rotation about Z and translated by [7,4,3].
const OCTANT: &[u8] = include_bytes!("../../../fixtures/r117-cut-sphere.stl");

#[test]
fn truncated_sphere_reimports_with_three_shared_quarter_circles_and_exact_source_owners() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let options = Options {
            reconstruction: mode,
            ..Options::default()
        };
        let result = convert_document_with_provenance(OCTANT, InputFormat::Stl, &options).unwrap();
        let body = &result.report.bodies[0].conversion;
        let recognition = body
            .rounded_recognition
            .as_ref()
            .expect("spherical reconstruction");
        assert_eq!(
            (
                body.cad.solids,
                body.cad.planar_faces,
                body.cad.spherical_faces
            ),
            (1, 3, 1)
        );
        assert_eq!(body.cad.faces, 4);
        assert!((body.cad.volume_mm3 - std::f64::consts::PI * 125. / 6.).abs() < 1e-7);
        assert_eq!(
            recognition
                .triangle_faces
                .iter()
                .filter(|f| matches!(f, RoundedFaceRef::SphericalOctant { .. }))
                .count(),
            324
        );
        assert!(recognition.report.max_deviation_bound_mm <= options.max_deviation_mm);
        let fid = body.fidelity.as_ref().unwrap();
        assert!(
            fid.source_to_result_mm <= options.max_deviation_mm
                && fid.result_to_source_mm <= options.max_deviation_mm
        );
        let faces = result.step.result_faces().unwrap();
        if mode == ReconstructionMode::Surfaces {
            // The historical explicit mode reports core triangle ownership.
            // Only Auto publishes the native STEP/source sidecar.
            assert_eq!(faces.source_correspondence.status, "unavailable");
            continue;
        }
        assert_eq!(faces.source_correspondence.status, "exact", "{mode:?}");
        let mut seen = BTreeSet::new();
        for face in faces.source_correspondence.faces {
            let step_face = &faces.result.faces[face.face_id];
            assert_eq!(step_face.adjacent_face_ids.len(), 3);
            if step_face.family == fabstep_cad::ResultFaceFamily::Sphere {
                assert_eq!(face.source_triangle_ids.len(), 324);
                assert!(face.source_triangle_ids.iter().all(|&t| t < 324));
            } else {
                assert_eq!(face.source_triangle_ids.len(), 18);
                assert!(face.source_triangle_ids.iter().all(|&t| t >= 324));
                let profile = result.step.planar_face_profile(face.face_id).unwrap();
                let [outer] = profile.loops.as_slice() else {
                    panic!("multiple planar contours")
                };
                assert_eq!(outer.segments.len(), 3);
                assert_eq!(outer.segments.iter().filter(|s| matches!(s,ProfileSegment::Arc {radius_mm,sweep_angle_rad,..}
                    if (*radius_mm-5.).abs()<1e-8 && (sweep_angle_rad.abs()-std::f64::consts::FRAC_PI_2).abs()<1e-8)).count(),1);
            }
            for t in face.source_triangle_ids {
                assert!(seen.insert(t));
            }
        }
        assert_eq!(seen.len(), 378);
    }
}

#[test]
fn altered_octant_members_cannot_be_exported() {
    let opts = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let a = fabstep_core::analyze_stl(OCTANT, &opts).unwrap();
    let model = fabstep_core::recognize_rounded(&a, &opts).unwrap().model;
    let mut wrong = model.clone();
    wrong.revolved.mixed.arcs[1].radius_mm += 0.01;
    assert_eq!(
        fabstep_cad::build_rounded_step(&wrong).err().unwrap().code,
        "cad.invalid_model"
    );
    let mut wrong = model;
    wrong.revolved.mixed.planes[0].loops.clear();
    assert_eq!(
        fabstep_cad::build_rounded_step(&wrong).err().unwrap().code,
        "cad.invalid_model"
    );
}

#[test]
fn tighter_deviation_keeps_the_curved_patch_faceted_in_auto() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        max_deviation_mm: 0.001,
        ..Options::default()
    };
    let result = convert_document_with_provenance(OCTANT, InputFormat::Stl, &options).unwrap();
    assert_eq!(result.report.cad.spherical_faces, 0);
    assert_eq!(result.report.cad.faces, result.report.cad.planar_faces);
}

#[test]
fn separated_octants_keep_each_center_and_its_corners_in_the_combined_step() {
    let mut data = OCTANT.to_vec();
    let mut second = data[84..].to_vec();
    for triangle in second.chunks_exact_mut(50) {
        for p in triangle[12..48].chunks_exact_mut(12) {
            // Rotate the second copy exactly in binary32. This isolates
            // STEP placement from a second quantization of the source mesh.
            for coordinate in p[..8].chunks_exact_mut(4) {
                let x = f32::from_le_bytes(coordinate.try_into().unwrap());
                coordinate.copy_from_slice(&(-x).to_le_bytes());
            }
        }
    }
    data[80..84].copy_from_slice(&(2 * 378_u32).to_le_bytes());
    data.extend(second);
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let result = convert_document_with_provenance(&data, InputFormat::Stl, &options).unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.spherical_faces, 2);
    let bounds: Vec<_> = result
        .report
        .bodies
        .iter()
        .map(|b| b.conversion.cad.bounds_mm)
        .collect();
    assert_eq!(bounds.len(), 2);
    for k in 0..2 {
        assert!((bounds[0][0][k] + bounds[1][1][k]).abs() < 1e-6);
        assert!((bounds[0][1][k] + bounds[1][0][k]).abs() < 1e-6);
    }
}
