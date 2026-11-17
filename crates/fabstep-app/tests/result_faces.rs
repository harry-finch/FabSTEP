use fabstep_app::{convert_document, result_faces, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use std::{fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
    .unwrap()
}

#[test]
fn final_step_inspection_covers_families_and_poles_without_claiming_source_provenance() {
    for name in [
        "m4_chamfered_cylinder.stl",
        "m5b_concave_cap.stl",
        "m5c_icosphere.stl",
        "m5d_ring_torus.stl",
        "m6a_rounded_box.stl",
    ] {
        let options = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        };
        let conversion = convert_document(&fixture(name), InputFormat::Stl, &options).unwrap();
        let before = conversion.step_bytes.clone();
        let inspected =
            result_faces(&conversion.step_bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(conversion.step_bytes, before);
        assert_eq!(inspected.source_correspondence.status, "unavailable");
        assert_eq!(
            inspected.source_correspondence.reason,
            "step.provenance_unavailable"
        );
        let cad = &conversion.report.cad;
        let result = &inspected.result;
        assert_eq!(result.deflection_mm, 0.01);
        assert_eq!(result.faces.len(), cad.faces, "{name}");
        for (family, count) in [
            (ResultFaceFamily::Plane, cad.planar_faces),
            (ResultFaceFamily::Cylinder, cad.cylindrical_faces),
            (ResultFaceFamily::Cone, cad.conical_faces),
            (ResultFaceFamily::Sphere, cad.spherical_faces),
            (ResultFaceFamily::Torus, cad.toroidal_faces),
        ] {
            assert_eq!(
                result.faces.iter().filter(|f| f.family == family).count(),
                count,
                "{name}: {family:?}"
            );
        }
        let mut covered = 0;
        for (id, face) in result.faces.iter().enumerate() {
            assert_eq!(face.id, id);
            assert_eq!(face.solid_id, 0);
            assert_eq!(face.triangle_start, covered);
            assert!(face.triangle_count > 0);
            covered += face.triangle_count;
            for &neighbor in &face.adjacent_face_ids {
                assert_ne!(neighbor, id);
                assert!(result.faces[neighbor].adjacent_face_ids.contains(&id));
            }
        }
        assert_eq!(covered, result.mesh.triangles.len());
        assert!(covered <= 100_000);
        assert!(result.mesh.vertices.len() <= 300_000);
        for point in &result.mesh.vertices {
            for (axis, coordinate) in point.iter().enumerate() {
                assert!(coordinate.is_finite());
                assert!(*coordinate >= cad.bounds_mm[0][axis] - 1e-6);
                assert!(*coordinate <= cad.bounds_mm[1][axis] + 1e-6);
            }
        }
        let json = serde_json::to_value(&inspected).unwrap();
        assert!(json.get("mesh").is_some());
        assert!(json.get("faces").is_some());
        assert!(json.get("result").is_none(), "IPC uses flattened fields");
    }
}

#[test]
fn placed_3mf_result_keeps_bodies_and_every_face_without_source_id_guessing() {
    for name in ["m3_instances.3mf", "m3_mirror.3mf"] {
        let conversion =
            convert_document(&fixture(name), InputFormat::ThreeMf, &Options::default()).unwrap();
        let inspected = result_faces(&conversion.step_bytes).unwrap();
        assert_eq!(inspected.result.faces.len(), conversion.report.cad.faces);
        let solids = inspected
            .result
            .faces
            .iter()
            .map(|f| f.solid_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(solids.len(), conversion.report.cad.solids);
        for face in &inspected.result.faces {
            for &other in &face.adjacent_face_ids {
                assert_eq!(inspected.result.faces[other].solid_id, face.solid_id);
            }
        }
        let json = serde_json::to_value(&inspected).unwrap();
        assert!(json["faces"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f.get("analysis_body_id").is_none() && f.get("preview_body_id").is_none()));
    }
}
