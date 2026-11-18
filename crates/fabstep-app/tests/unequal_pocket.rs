use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{export_planar_face_dxf, Options, ProfileSegment, ReconstructionMode};
use serde::Deserialize;
use std::collections::BTreeMap;

const NOMINAL: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/nominal.stl");
const REFLECTED: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/reflected.stl");
const WARPED: &[u8] = include_bytes!("../../../fixtures/unequal_pocket/warped_seam.stl");

#[derive(Deserialize)]
struct Reference {
    volume_mm3: f64,
    bounds_mm: [[f64; 3]; 2],
    groups: BTreeMap<String, [usize; 2]>,
    continuation: Continuation,
}

#[derive(Deserialize)]
struct Continuation {
    plane: String,
    sweep_rad: f64,
}

#[test]
fn both_orientations_keep_thirteen_editable_faces_and_a_circular_continuation() {
    let manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/unequal_pocket/manifest.json"
    ))
    .unwrap();
    for (case, bytes, radii) in [
        ("nominal", NOMINAL, [2., 3.]),
        ("reflected", REFLECTED, [1.5, 3.5]),
    ] {
        let reference: Reference = serde_json::from_value(manifest["cases"][case].clone()).unwrap();
        for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
            let o = Options {
                reconstruction: mode,
                ..Options::default()
            };
            let doc = convert_document_with_provenance(bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{case}/{mode:?}: {e}"));
            let cad = &doc.report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.cylindrical_faces
                ),
                (1, 13, 11, 2)
            );
            assert!((cad.volume_mm3 - reference.volume_mm3).abs() < 0.0003);
            assert!(cad
                .bounds_mm
                .iter()
                .flatten()
                .zip(reference.bounds_mm.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 1e-5));
            let conversion = &doc.report.bodies[0].conversion;
            let recognition = conversion.rounded_recognition.as_ref().unwrap();
            let domain = recognition.model.unequal_pocket.as_ref().unwrap();
            assert!(domain
                .radii
                .into_iter()
                .zip(radii)
                .all(|(a, b)| (a - b).abs() < 1e-5));
            let report = &recognition.report;
            assert!((report.expected_volume_mm3 - reference.volume_mm3).abs() < 1e-5);
            assert!(report.max_deviation_bound_mm <= o.max_deviation_mm);
            let fidelity = conversion.fidelity.as_ref().unwrap();
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(
                fidelity.source_to_result_mm <= o.max_deviation_mm
                    && fidelity.result_to_source_mm <= o.max_deviation_mm
            );

            let inspected = doc.step.result_faces().unwrap();
            // Explicit Surfaces uses the untracked document path. Native faces
            // remain selectable there; triangle correspondence is provided in Auto.
            if mode == ReconstructionMode::Surfaces {
                assert_eq!(
                    inspected
                        .result
                        .faces
                        .iter()
                        .filter(|f| f.family == ResultFaceFamily::Cylinder && f.triangle_count > 0)
                        .count(),
                    2
                );
                assert_eq!(inspected.source_correspondence.status, "unavailable");
                continue;
            }
            assert_eq!(inspected.source_correspondence.status, "exact");
            let mut ids = BTreeMap::new();
            for (name, &[first, end]) in &reference.groups {
                let owner = inspected
                    .source_correspondence
                    .faces
                    .iter()
                    .find(|owner| owner.source_triangle_ids.iter().copied().eq(first..end))
                    .unwrap_or_else(|| panic!("{case}: source group {name}"));
                ids.insert(name.as_str(), owner.face_id);
            }
            let faces = &inspected.result.faces;
            let a = &faces[ids["fillet_x"]];
            let b = &faces[ids["fillet_y"]];
            assert_eq!(
                (&a.family, &b.family),
                (&ResultFaceFamily::Cylinder, &ResultFaceFamily::Cylinder)
            );
            assert!(a.adjacent_face_ids.contains(&ids["fillet_y"]));
            let mut neighbors = [a.adjacent_face_ids.len(), b.adjacent_face_ids.len()];
            neighbors.sort();
            assert_eq!(neighbors, [4, 5]);
            let profile = doc
                .step
                .planar_face_profile(ids[reference.continuation.plane.as_str()])
                .unwrap();
            assert_eq!(profile.loops.len(), 1);
            assert_eq!(profile.loops[0].segments.iter().filter(|s| matches!(s, ProfileSegment::Arc {radius_mm,sweep_angle_rad,..}
                if (radius_mm-radii[1]).abs()<1e-5 && (sweep_angle_rad.abs()-reference.continuation.sweep_rad).abs()<1e-5)).count(), 1);
            let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
            assert_eq!(dxf.matches("\n0\nARC\n").count(), 1);
        }
    }
}

#[test]
fn a_warped_seam_keeps_the_source_shape() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let doc = convert_document_with_provenance(WARPED, InputFormat::Stl, &o).unwrap();
    let cad = &doc.report.cad;
    assert_eq!((cad.solids, cad.faces), (1, cad.planar_faces));
    let source = &doc.report.bodies[0].conversion.analysis;
    assert!((cad.volume_mm3 - source.report.volume_mm3).abs() < 1e-5);
}
