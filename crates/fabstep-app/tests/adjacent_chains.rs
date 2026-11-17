use fabstep_app::{convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};
use serde_json::Value;
use std::{fs, path::Path};

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/adjacent_chains")
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

#[test]
fn native_reimport_preserves_every_span_and_solidity() {
    let m = manifest();
    for mode in [
        ReconstructionMode::Primitives,
        ReconstructionMode::Surfaces,
        ReconstructionMode::Auto,
    ] {
        let o = options(mode);
        for (name, expected) in m["cases"].as_object().unwrap() {
            if mode != ReconstructionMode::Auto
                && (name.ends_with("mirror") || name.ends_with("rotated"))
            {
                continue;
            }
            let bytes = fixture(&format!("{name}.stl"));
            let converted =
                convert_stl(&bytes, &o).unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let r = &converted.report;
            let cad = &r.cad;
            let cones = expected["cones"].as_u64().unwrap() as usize;
            let cylinders = expected["cylinders"].as_u64().unwrap() as usize;
            assert_eq!(
                (cad.solids, cad.planar_faces, cad.faces),
                (1, 2, 2 + cones + cylinders)
            );
            assert_eq!(
                (cad.conical_faces, cad.cylindrical_faces),
                (cones, cylinders)
            );
            assert_eq!(cad.toroidal_faces + cad.spherical_faces, 0);
            let volume = expected["volume_mm3"].as_f64().unwrap();
            assert!(
                (cad.volume_mm3 - volume).abs() < volume * 1e-7,
                "{name}/{mode:?}: {cad:?}"
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
            assert_eq!(
                r.analysis.mesh.triangles.len(),
                expected["triangles"].as_u64().unwrap() as usize
            );
            let audit = r.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);
        }
    }
}

#[test]
fn mixed_chain_keeps_exact_source_ownership_after_transfer() {
    let m = manifest();
    let name = "mixed";
    let converted = convert_document_with_provenance(
        &fixture(&format!("{name}.stl")),
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    let faces = converted.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let expected = &m["cases"][name];
    let patches = expected["patches"].as_array().unwrap();
    assert_eq!(faces.source_correspondence.faces.len(), patches.len());
    let mut start = 0;
    for patch in patches {
        let family = match patch[0].as_str().unwrap() {
            "plane" => ResultFaceFamily::Plane,
            "cone" => ResultFaceFamily::Cone,
            "cylinder" => ResultFaceFamily::Cylinder,
            name => panic!("unexpected reference family {name}"),
        };
        let count = patch[1].as_u64().unwrap() as usize;
        let f = faces
            .source_correspondence
            .faces
            .iter()
            .find(|s| {
                s.source_triangle_ids
                    .iter()
                    .copied()
                    .eq(start..start + count)
            })
            .unwrap_or_else(|| panic!("missing source interval {}..{}", start, start + count));
        assert_eq!(faces.result.faces[f.face_id].family, family);
        start += count;
    }
    assert_eq!(start, expected["triangles"].as_u64().unwrap() as usize);
}

#[test]
fn an_unproved_split_or_excessive_chain_cannot_produce_an_explicit_step() {
    let m = manifest();
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let o = options(mode);
        let code = if mode == ReconstructionMode::Primitives {
            "recognition.conic_unproven"
        } else {
            "recognition.surface_unproven"
        };
        for name in m["negative_cases"].as_object().unwrap().keys() {
            let error = convert_stl(&fixture(&format!("{name}.stl")), &o)
                .err()
                .unwrap();
            assert_eq!(error.code, code, "{name}/{mode:?}: {error}");
        }
    }
}
