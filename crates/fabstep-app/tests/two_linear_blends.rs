use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::{build_rounded_step, ResultFaceFamily};
use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/two_linear_blends")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

#[test]
fn source_and_two_independent_cad_drawings_reimport_with_both_cones_and_all_owners() {
    let table: Value = serde_json::from_str(include_str!(
        "../../../fixtures/two_linear_blends/manifest.json"
    ))
    .unwrap();
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for name in ["opposed", "offset"] {
        let reference = &table[name];
        let volume = reference["parameters"]["volume_mm3"].as_f64().unwrap();
        for variant in [
            name.to_owned(),
            format!("freecad/{name}-coarse"),
            format!("freecad/{name}-fine"),
        ] {
            let raw = fixture(&variant);
            let converted = convert_document_with_provenance(&raw, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{variant}: {e}"));
            let cad = &converted.report.cad;
            assert_eq!(
                (cad.solids, cad.faces, cad.conical_faces, cad.planar_faces),
                (1, 8, 2, 6)
            );
            assert!((cad.volume_mm3 - volume).abs() < volume * 1e-8);
            let faces = converted.step.result_faces().unwrap();
            let corr = &faces.source_correspondence;
            assert_eq!(corr.status, "exact");
            assert_eq!(corr.faces.len(), 8);
            let mut seen = BTreeSet::new();
            for row in &corr.faces {
                let f = &faces.result.faces[row.face_id];
                if f.family == ResultFaceFamily::Cone {
                    assert_eq!(f.adjacent_face_ids.len(), 4);
                    assert!(f
                        .adjacent_face_ids
                        .iter()
                        .all(|&id| faces.result.faces[id].family == ResultFaceFamily::Plane));
                }
                for id in &row.source_triangle_ids {
                    assert!(seen.insert(*id));
                }
            }
            let report = &converted.report.bodies[0].conversion;
            assert!(seen
                .iter()
                .copied()
                .eq(0..report.analysis.mesh.triangles.len()));
            if variant == name {
                for patch in reference["patches"].as_array().unwrap() {
                    let start = patch["first_source_triangle"].as_u64().unwrap() as usize;
                    let end = start + patch["triangles"].as_u64().unwrap() as usize;
                    let row = corr
                        .faces
                        .iter()
                        .find(|row| row.source_triangle_ids.iter().copied().eq(start..end))
                        .expect("source patch not recovered");
                    let expected = if patch["owner"].as_str().unwrap().starts_with("cone") {
                        ResultFaceFamily::Cone
                    } else {
                        ResultFaceFamily::Plane
                    };
                    assert_eq!(faces.result.faces[row.face_id].family, expected);
                }
            }
            let audit = report.fidelity.as_ref().unwrap();
            assert!(
                audit.source_to_result_mm <= o.max_deviation_mm
                    && audit.result_to_source_mm <= o.max_deviation_mm
            );
        }
    }
}

#[test]
fn invalid_second_domain_and_thin_budget_are_rejected() {
    let mut o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let a = analyze_stl(&fixture("opposed"), &o).unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    for change in [
        (|b: &mut fabstep_core::MixedLinearBlend| b.arcs = [0, 1])
            as fn(&mut fabstep_core::MixedLinearBlend),
        |b| b.rails[1].reverse(),
        |b| b.apex[0] += 0.1,
    ] {
        let mut m = r.model.clone();
        change(m.linear_blend.as_mut().unwrap().second.as_mut().unwrap());
        assert_eq!(
            build_rounded_step(&m).unwrap_err().code,
            "cad.invalid_model"
        );
    }
    o.max_deviation_mm = 0.00001;
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}
