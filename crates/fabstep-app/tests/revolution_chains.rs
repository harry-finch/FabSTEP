use fabstep_app::{convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use serde_json::Value;
use std::{fs, path::Path};

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/revolution_chains")
            .join(file),
    )
    .unwrap()
}

fn manifest() -> Value {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

fn family(name: &str) -> ResultFaceFamily {
    match name {
        "plane" => ResultFaceFamily::Plane,
        "cylinder" => ResultFaceFamily::Cylinder,
        "cone" => ResultFaceFamily::Cone,
        "torus" => ResultFaceFamily::Torus,
        _ => panic!("Unknown reference family {name}"),
    }
}

#[test]
fn reimport_preserves_the_analytic_faces_dimensions_and_bidirectional_audit() {
    let m = manifest();
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        for name in [
            "stepped",
            "arc100",
            "arc100_inward_mirror",
            "arc100_rotated",
        ] {
            let expected = &m["cases"][name];
            let patches = expected["patches"].as_array().unwrap();
            let converted = convert_stl(&fixture(&format!("{name}.stl")), &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let report = &converted.report;
            let cad = &report.cad;
            let stepped = name == "stepped";
            assert_eq!(
                (cad.solids, cad.faces, cad.planar_faces),
                (1, patches.len(), 2)
            );
            assert_eq!(cad.toroidal_faces, 1);
            assert_eq!(cad.conical_faces, if stepped { 2 } else { 1 });
            assert_eq!(cad.cylindrical_faces, if stepped { 2 } else { 0 });
            assert_eq!(cad.spherical_faces, 0);
            assert!(
                (cad.volume_mm3 - expected["volume_mm3"].as_f64().unwrap()).abs() < 0.005,
                "{name}/{mode:?}: wrong reimported volume {}",
                cad.volume_mm3
            );
            for (end, corner) in cad.bounds_mm.iter().enumerate() {
                for (axis, coordinate) in corner.iter().enumerate() {
                    let bound = expected["bounds_mm"][end][axis].as_f64().unwrap();
                    assert!(
                        (coordinate - bound).abs() < 2e-5,
                        "{name}/{mode:?}: {cad:?}"
                    );
                }
            }
            let triangles = expected["triangles"].as_u64().unwrap() as usize;
            assert_eq!(report.analysis.mesh.triangles.len(), triangles);
            let recognized = report.rounded_recognition.as_ref().unwrap();
            assert_eq!(recognized.triangle_faces.len(), triangles);
            assert!(recognized.report.max_deviation_bound_mm <= o.max_deviation_mm);
            let fidelity = report.fidelity.as_ref().unwrap();
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(fidelity.source_to_result_mm <= o.max_deviation_mm);
            assert!(fidelity.result_to_source_mm <= o.max_deviation_mm);

            let step = std::str::from_utf8(&converted.step_bytes).unwrap();
            assert_eq!(step.matches("TOROIDAL_SURFACE(").count(), 1);
            assert_eq!(step.matches("CONICAL_SURFACE(").count(), cad.conical_faces);
            assert_eq!(
                step.matches("CYLINDRICAL_SURFACE(").count(),
                cad.cylindrical_faces
            );
        }
    }
}

#[test]
fn native_adjacencies_and_source_partition_survive_step_transfer() {
    let m = manifest();
    let o = options(ReconstructionMode::Auto);
    for name in ["stepped", "arc100_mixed_azimuths"] {
        let expected = &m["cases"][name];
        let patches = expected["patches"].as_array().unwrap();
        let bytes = fixture(&format!("{name}.stl"));
        let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let faces = converted.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        assert_eq!(faces.source_correspondence.faces.len(), patches.len());
        assert_eq!(faces.result.faces.len(), patches.len());
        let mut chain = Vec::with_capacity(patches.len());
        let mut start = 0;
        for patch in patches {
            let count = patch["triangles"].as_u64().unwrap() as usize;
            let mut candidates = faces.source_correspondence.faces.iter().filter(|source| {
                source.source_triangle_ids.len() == count
                    && source
                        .source_triangle_ids
                        .iter()
                        .copied()
                        .eq(start..start + count)
            });
            let matched = candidates.next().unwrap_or_else(|| {
                panic!(
                    "{name}: native source interval {}..{} missing",
                    start,
                    start + count
                )
            });
            assert!(candidates.next().is_none());
            let face = &faces.result.faces[matched.face_id];
            assert_eq!(face.family, family(patch["family"].as_str().unwrap()));
            chain.push(face);
            start += count;
        }
        assert_eq!(start, expected["triangles"].as_u64().unwrap() as usize);
        for (index, face) in chain.iter().enumerate() {
            let neighbors = [
                index.checked_sub(1),
                (index + 1 < chain.len()).then_some(index + 1),
            ];
            let mut expected_neighbors = neighbors
                .into_iter()
                .flatten()
                .map(|i| chain[i].id)
                .collect::<Vec<_>>();
            let mut actual_neighbors = face.adjacent_face_ids.clone();
            expected_neighbors.sort_unstable();
            actual_neighbors.sort_unstable();
            assert_eq!(actual_neighbors, expected_neighbors, "{name}: face {index}");
        }
    }
}

#[test]
fn explicit_surface_conversion_refuses_missing_evidence_and_insufficient_deviation() {
    let m = manifest();
    let o = options(ReconstructionMode::Surfaces);
    for name in m["negative_cases"].as_object().unwrap().keys() {
        let bytes = fixture(&format!("{name}.stl"));
        let error = convert_stl(&bytes, &o)
            .err()
            .expect("unproved analytic conversion");
        assert_eq!(
            error.code, "recognition.surface_unproven",
            "{name}: {error}"
        );
    }
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    let error = convert_stl(&fixture("arc100.stl"), &strict)
        .err()
        .expect("deviation exceeds the requested budget");
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
