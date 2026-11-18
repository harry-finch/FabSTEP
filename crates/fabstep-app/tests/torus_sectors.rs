use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode, RoundedFaceRef};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/torus_sectors")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

// The generator writes a contiguous block of source triangles for each
// physical face. Use those identities independently of the recognizer.
fn patches(case: &Value) -> Vec<(ResultFaceFamily, std::ops::Range<usize>)> {
    use ResultFaceFamily::{Cylinder, Plane, Torus};
    let mut start = 0;
    [
        ("bottom", Plane),
        ("outer_cylinder", Cylinder),
        ("torus", Torus),
        ("top", Plane),
        ("inner_cylinder", Cylinder),
        ("start", Plane),
        ("end", Plane),
    ]
    .into_iter()
    .map(|(name, family)| {
        let end = start + case["triangles_per_family"][name].as_u64().unwrap() as usize;
        let indices = start..end;
        start = end;
        (family, indices)
    })
    .collect()
}

#[test]
fn bounded_sectors_preserve_the_source_partition_volume_and_fidelity_after_step() {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../fixtures/torus_sectors/manifest.json"
    ))
    .unwrap();
    let volume = manifest["reference_volume_mm3"].as_f64().unwrap();
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = options(mode);
        for name in manifest["positives"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
        {
            let converted = convert_document_with_provenance(&fixture(name), InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let cad = &converted.report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.toroidal_faces,
                    cad.planar_faces,
                    cad.cylindrical_faces
                ),
                (1, 7, 1, 4, 2),
                "{name}/{mode:?}"
            );
            assert!(
                (cad.volume_mm3 - volume).abs() < volume * 1e-6,
                "{name}/{mode:?}: {cad:?}"
            );
            let body = &converted.report.bodies[0];
            let proof = body.conversion.rounded_recognition.as_ref().unwrap();
            assert_eq!(
                (
                    proof.model.torus_sectors.len(),
                    proof.model.tori.len(),
                    proof.report.tori
                ),
                (1, 0, 1)
            );
            assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!(proof
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "recognition.toric_sector_fitted"));
            assert!(proof
                .triangle_faces
                .contains(&RoundedFaceRef::TorusSector { index: 0 }));
            let audit = body.conversion.fidelity.as_ref().unwrap();
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);
            let inspected = converted.step.result_faces().unwrap();
            let sector = inspected
                .result
                .faces
                .iter()
                .find(|f| f.family == ResultFaceFamily::Torus)
                .unwrap();
            assert_eq!(sector.adjacent_face_ids.len(), 4);
            if mode == ReconstructionMode::Auto {
                let sources = &inspected.source_correspondence;
                assert_eq!(sources.status, "exact", "{name}/{mode:?}");
                assert_eq!(sources.faces.len(), 7);
                for (family, indices) in patches(&manifest["cases"][name]) {
                    let entry = sources
                        .faces
                        .iter()
                        .find(|row| row.source_triangle_ids.iter().copied().eq(indices.clone()))
                        .expect("missing exact face patch");
                    assert_eq!(
                        inspected.result.faces[entry.face_id].family, family,
                        "{name}"
                    );
                    assert_eq!(entry.analysis_body_id, body.id);
                }
            }
        }
    }
}

#[test]
fn open_folded_and_dented_inputs_cannot_export_a_torus_sector() {
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        for name in ["open", "folded", "dented"] {
            match convert_document_with_provenance(&fixture(name), InputFormat::Stl, &options(mode))
            {
                Ok(converted) => {
                    assert_eq!(name, "dented", "{mode:?}: invalid solid accepted");
                    assert_eq!(converted.report.cad.toroidal_faces, 0);
                }
                Err(e) if name == "open" => assert_eq!(e.code, "mesh.open"),
                Err(e) => assert!(
                    e.code.starts_with("recognition.")
                        || e.code == "cad.invalid_solid"
                        || (name == "dented" && e.code == "cad.invalid_model"),
                    "{name}/{mode:?}: {e}"
                ),
            }
        }
    }
}

#[test]
fn interleaved_sectors_keep_separate_roots_and_face_tokens() {
    let nominal = fixture("nominal");
    let mirrored = fixture("mirrored");
    let mut bytes = nominal[..80].to_vec();
    let n = u32::from_le_bytes(nominal[80..84].try_into().unwrap());
    bytes.extend((2 * n).to_le_bytes());
    for (a, b) in nominal[84..]
        .chunks_exact(50)
        .zip(mirrored[84..].chunks_exact(50))
    {
        bytes.extend_from_slice(a);
        let mut other = b.to_vec();
        for i in [20, 32, 44] {
            let z = f32::from_le_bytes(other[i..i + 4].try_into().unwrap());
            other[i..i + 4].copy_from_slice(&(z + 10.).to_le_bytes());
        }
        bytes.extend(other);
    }
    let converted = convert_document_with_provenance(
        &bytes,
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    assert_eq!(
        (
            converted.report.cad.solids,
            converted.report.cad.faces,
            converted.report.cad.toroidal_faces
        ),
        (2, 14, 2)
    );
    let inspected = converted.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let mut roots = BTreeSet::new();
    for parity in 0..2 {
        let mut solids = BTreeSet::new();
        let mut covered = BTreeSet::new();
        let body = converted
            .report
            .bodies
            .iter()
            .find(|body| (body.conversion.cad.bounds_mm[0][2] - parity as f64 * 10.).abs() < 2e-4)
            .unwrap();
        for row in &inspected.source_correspondence.faces {
            if row.analysis_body_id != body.id {
                continue;
            }
            let face = &inspected.result.faces[row.face_id];
            solids.insert(face.solid_id);
            for &i in &row.source_triangle_ids {
                assert_eq!(i % 2, parity);
                assert!(covered.insert(i / 2));
            }
            if face.family == ResultFaceFamily::Torus {
                assert!(row.source_triangle_ids.iter().map(|i| i / 2).eq(192..1728));
            }
            assert!(face
                .adjacent_face_ids
                .iter()
                .all(|&id| inspected.result.faces[id].solid_id == face.solid_id));
        }
        assert!(covered.into_iter().eq(0..n as usize));
        assert_eq!(solids.len(), 1);
        assert!(roots.insert(*solids.first().unwrap()));
    }
}
