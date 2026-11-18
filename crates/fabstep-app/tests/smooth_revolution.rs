use fabstep_app::{convert_document_with_provenance, selection_topology, InputFormat};
use fabstep_cad::{tessellate_step, CadReport, ResultFace, ResultFaceFamily};
use fabstep_core::{
    GuidanceFamily, Mesh, Options, ReconstructionMode, RoundedFaceRef, RoundedModel,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const BULGE: &[u8] = include_bytes!("../../../fixtures/smooth_revolution/bulge.stl");

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/smooth_revolution")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn options(reconstruction: ReconstructionMode) -> Options {
    Options {
        reconstruction,
        ..Options::default()
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn winding(mesh: &Mesh, point: [f64; 3]) -> f64 {
    let sum = mesh
        .triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] =
                triangle.map(|i| std::array::from_fn(|j| mesh.vertices[i][j] - point[j]));
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            let [la, lb, lc] = [a, b, c].map(|v| dot(v, v).sqrt());
            2. * dot(a, cross)
                .atan2(la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb)
        })
        .sum::<f64>();
    sum.abs() / (2. * std::f64::consts::TAU)
}

#[test]
fn cubic_walls_reimport_with_their_caps_volume_bounds_and_fidelity() {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../fixtures/smooth_revolution/manifest.json"
    ))
    .unwrap();
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        for (name, case) in manifest["cases"].as_object().unwrap() {
            let bytes = fixture(name);
            let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|error| panic!("{name}/{mode:?}: {error}"));
            let [body] = converted.report.bodies.as_slice() else {
                panic!("{name}/{mode:?}: expected one body");
            };
            let report = &body.conversion;
            let cad = &report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.revolution_faces
                ),
                (1, 3, 2, 1),
                "{name}/{mode:?}: {cad:?}"
            );
            let volume = case["parameters"]["volume_mm3"].as_f64().unwrap();
            assert!(
                (cad.volume_mm3 - volume).abs() < volume * 1e-6,
                "{name}/{mode:?}: {} vs {volume}",
                cad.volume_mm3
            );
            for (end, corner) in cad.bounds_mm.iter().enumerate() {
                for (axis, coordinate) in corner.iter().enumerate() {
                    let expected = case["analytic_bounds_mm"][end][axis].as_f64().unwrap();
                    assert!(
                        (coordinate - expected).abs() < 2e-4,
                        "{name}/{mode:?}: bound {end}/{axis} {coordinate} vs {expected}"
                    );
                }
            }

            let proof = report.rounded_recognition.as_ref().unwrap();
            assert!(proof.model.smooth_revolution.is_some());
            assert!((proof.report.expected_volume_mm3 - volume).abs() < volume * 1e-7);
            assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);
            let inspected = converted.step.result_faces().unwrap();
            let faces = &inspected.result.faces;
            assert_eq!(faces.len(), 3);
            let wall = faces
                .iter()
                .find(|face| face.family == ResultFaceFamily::Revolution)
                .unwrap();
            let caps: Vec<_> = faces
                .iter()
                .filter(|face| face.family == ResultFaceFamily::Plane)
                .map(|face| face.id)
                .collect();
            assert_eq!(wall.adjacent_face_ids, caps);
            assert!(caps
                .iter()
                .all(|&id| faces[id].adjacent_face_ids == [wall.id]));
            let step = std::str::from_utf8(converted.step.bytes()).unwrap();
            assert_eq!(step.matches("SURFACE_OF_REVOLUTION(").count(), 1);

            let topology = selection_topology(&bytes, InputFormat::Stl, &o).unwrap();
            let mapping = &topology.components[0].analysis_to_source_triangles;
            let mut owners = vec![None; proof.triangle_faces.len()];
            assert_eq!(owners.len(), mapping.len());
            for (&source, &owner) in mapping.iter().zip(&proof.triangle_faces) {
                assert!(owners[source].replace(owner).is_none());
            }
            assert_eq!(owners.len() as u64, case["triangles"].as_u64().unwrap());
            let correspondence = &inspected.source_correspondence;
            if mode == ReconstructionMode::Auto {
                assert_eq!(correspondence.status, "exact", "{name}");
                assert_eq!(correspondence.faces.len(), 3);
            }
            let mut patch_owners = Vec::new();
            for patch in case["patches"].as_array().unwrap() {
                let first = patch["first_source_triangle"].as_u64().unwrap() as usize;
                let count = patch["triangles"].as_u64().unwrap() as usize;
                let owner = owners[first].unwrap();
                assert!(owners[first..first + count]
                    .iter()
                    .all(|&actual| actual == Some(owner)));
                assert!(!patch_owners.contains(&owner));
                patch_owners.push(owner);
                let expected_family = if patch["owner"] == "wall" {
                    assert_eq!(owner, RoundedFaceRef::SmoothRevolution { index: 0 });
                    ResultFaceFamily::Revolution
                } else {
                    assert!(matches!(owner, RoundedFaceRef::Plane { .. }));
                    ResultFaceFamily::Plane
                };
                if mode == ReconstructionMode::Auto {
                    let row = correspondence
                        .faces
                        .iter()
                        .find(|row| {
                            row.source_triangle_ids
                                .iter()
                                .copied()
                                .eq(first..first + count)
                        })
                        .unwrap_or_else(|| {
                            panic!("{name}: missing native patch {}", patch["owner"])
                        });
                    assert_eq!(row.analysis_body_id, body.id);
                    let face: &ResultFace = &faces[row.face_id];
                    assert_eq!(face.family, expected_family);
                }
            }

            if mode == ReconstructionMode::Auto {
                if name == "bulge" || name == "two_span" {
                    let error = converted
                        .step
                        .guided_surface(&[wall.id], GuidanceFamily::Auto)
                        .err()
                        .expect("local mutation of the cubic wall accepted");
                    assert_eq!(error.code, "guidance.family_unproven");
                }
                let result = tessellate_step(converted.step.bytes(), 0.01).unwrap();
                for probe in case["material_probes"].as_array().unwrap() {
                    let point =
                        std::array::from_fn(|axis| probe["point_mm"][axis].as_f64().unwrap());
                    let expected = f64::from(u8::from(probe["material"].as_bool().unwrap()));
                    for mesh in [&report.analysis.mesh, &result] {
                        let actual = winding(mesh, point);
                        assert!(
                            (actual - expected).abs() < 1e-6,
                            "{name}: probe {point:?}, winding {actual} vs {expected}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_or_over_budget_sources_never_receive_a_cubic_wall() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let options = options(mode);
        let open =
            convert_document_with_provenance(&fixture("missing_cap"), InputFormat::Stl, &options)
                .err()
                .expect("open source accepted");
        assert_eq!(open.code, "mesh.open");
        for name in [
            "folded",
            "incomplete_level",
            "sparse",
            "quartic",
            "axial_tip",
            "bulge",
        ] {
            let mut o = options.clone();
            if name == "bulge" || name == "two_span" {
                o.max_deviation_mm = 1e-4;
            }
            match convert_document_with_provenance(&fixture(name), InputFormat::Stl, &o) {
                Ok(converted) => {
                    assert_ne!(name, "folded", "folded source accepted as a solid");
                    assert_eq!(converted.report.cad.solids, 1, "{name}/{mode:?}");
                    assert_eq!(converted.report.cad.revolution_faces, 0, "{name}/{mode:?}");
                    if let Some(proof) = &converted.report.bodies[0].conversion.rounded_recognition
                    {
                        assert!(proof.model.smooth_revolution.is_none());
                        assert!(!proof
                            .triangle_faces
                            .iter()
                            .any(|owner| matches!(owner, RoundedFaceRef::SmoothRevolution { .. })));
                    }
                }
                Err(error) => assert!(
                    matches!(
                        error.code.as_str(),
                        "recognition.surface_unproven"
                            | "recognition.unsupported"
                            | "recognition.partial_unproven"
                            | "recognition.conic_unproven"
                            | "recognition.deviation_exceeded"
                            | "cad.invalid_solid"
                    ),
                    "{name}/{mode:?}: {error}"
                ),
            }
        }
    }
}

#[test]
fn cubic_revolution_metadata_roundtrips_and_legacy_reports_still_read() {
    let old: RoundedModel = serde_json::from_value(json!({
        "revolved": {
            "mixed": {"vertices": [], "rings": [], "planes": [], "cylinders": []},
            "cones": []
        },
        "spheres": [], "tori": []
    }))
    .unwrap();
    assert!(old.smooth_revolution.is_none());
    assert!(serde_json::to_value(old)
        .unwrap()
        .get("smooth_revolution")
        .is_none());

    let o = options(ReconstructionMode::Surfaces);
    let converted = convert_document_with_provenance(BULGE, InputFormat::Stl, &o).unwrap();
    let mut native = serde_json::to_value(&converted.report.cad).unwrap();
    let missing = native.as_object_mut().unwrap().remove("revolution_faces");
    assert_eq!(missing, Some(json!(1)));
    let old_native: CadReport = serde_json::from_value(native).unwrap();
    assert_eq!(old_native.revolution_faces, 0);
    assert_eq!(
        serde_json::to_value(ResultFaceFamily::Revolution).unwrap(),
        "revolution"
    );

    let proof = converted.report.bodies[0]
        .conversion
        .rounded_recognition
        .as_ref()
        .unwrap();
    let encoded = serde_json::to_value(&proof.model).unwrap();
    assert_eq!(
        encoded["smooth_revolution"]["rings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let restored: RoundedModel = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), encoded);
    let owner = RoundedFaceRef::SmoothRevolution { index: 0 };
    let encoded = serde_json::to_value(owner).unwrap();
    assert_eq!(encoded, json!({"kind": "smooth_revolution", "index": 0}));
    assert_eq!(
        serde_json::from_value::<RoundedFaceRef>(encoded).unwrap(),
        owner
    );
}

#[test]
fn historical_cylinder_and_cone_keep_their_native_families() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        for (bytes, expected) in [
            (
                include_bytes!("../../../fixtures/m2_cylinder.stl").as_slice(),
                (1, 0),
            ),
            (
                include_bytes!("../../../fixtures/adjacent_chains/three.stl").as_slice(),
                (0, 3),
            ),
        ] {
            let converted =
                convert_document_with_provenance(bytes, InputFormat::Stl, &options(mode))
                    .unwrap_or_else(|error| panic!("{mode:?}/{expected:?}: {error}"));
            let cad = &converted.report.cad;
            assert_eq!(
                (cad.cylindrical_faces, cad.conical_faces),
                expected,
                "{mode:?}"
            );
            assert_eq!(cad.revolution_faces, 0);
        }
    }
}
