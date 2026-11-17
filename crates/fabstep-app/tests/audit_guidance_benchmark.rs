//! Manual paired benchmark: run before/after with --ignored --nocapture and
//! --test-threads=1, on the same build profile and native SDK.
use fabstep_app::{
    analyze_document, convert_document_with_provenance, selection_topology, InputFormat,
    ProcessingSettings,
};
use fabstep_core::{recognize_mixed, MixedFaceRef, Options, ReconstructionMode};
use std::time::Instant;

#[test]
#[ignore = "explicit INT-AUDIT-01 paired benchmark; performs six complete guided conversions"]
fn public_guidance_after_result_inspection() {
    let options = Options::default();
    for (fixture, format, bytes) in [
        (
            "m2b_boss_blind.stl",
            InputFormat::Stl,
            include_bytes!("../../../fixtures/m2b_boss_blind.stl").as_slice(),
        ),
        (
            "guided_cylinder_instances.3mf",
            InputFormat::ThreeMf,
            include_bytes!("../../../fixtures/guided_cylinder_instances.3mf").as_slice(),
        ),
    ] {
        let document = analyze_document(bytes, format, &options).unwrap();
        let body = &document.bodies[0];
        let topology = selection_topology(bytes, format, &options).unwrap();
        let component = topology
            .components
            .iter()
            .find(|c| c.analysis_body_id == body.id)
            .unwrap();
        let recognized = recognize_mixed(
            &body.analysis,
            &Options {
                reconstruction: ReconstructionMode::Cylinders,
                ..options.clone()
            },
        )
        .unwrap();
        let triangle = recognized
            .triangle_faces
            .iter()
            .position(|face| matches!(face, MixedFaceRef::Cylinder { index: 0 }))
            .unwrap();
        let source_triangle = component.analysis_to_source_triangles[triangle];
        let mut guided_times = Vec::new();
        for repetition in 1..=3 {
            let start = Instant::now();
            let planar = convert_document_with_provenance(bytes, format, &options).unwrap();
            let convert_ms = start.elapsed().as_secs_f64() * 1000.;
            let start = Instant::now();
            let inspected = planar.step.result_faces().unwrap();
            let inspection_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(inspected.source_correspondence.status, "exact");
            let seed = inspected
                .source_correspondence
                .faces
                .iter()
                .find(|face| {
                    face.analysis_body_id == body.id
                        && face.source_triangle_ids.contains(&source_triangle)
                })
                .unwrap()
                .face_id;
            let start = Instant::now();
            let guided = planar.step.guided_cylinder(&[seed]).unwrap();
            let guided_ms = start.elapsed().as_secs_f64() * 1000.;
            guided_times.push(guided_ms);
            assert_eq!(guided.report.cad.solids, document.bodies.len());
            assert_eq!(guided.report.cad.cylindrical_faces, 1);
            println!(
                "INT_AUDIT_BENCH {}",
                serde_json::json!({
                    "fixture": fixture, "repetition": repetition,
                    "settings": ProcessingSettings::from(&options),
                    "conversion_ms": convert_ms, "inspection_ms": inspection_ms,
                    "guided_ms": guided_ms, "inspected_faces": inspected.result.faces.len(),
                    "cad": guided.report.cad,
                    "fidelity": guided.report.bodies.iter().map(|b| &b.conversion.fidelity).collect::<Vec<_>>(),
                })
            );
        }
        guided_times.sort_by(f64::total_cmp);
        println!(
            "INT_AUDIT_MEDIAN {}",
            serde_json::json!({"fixture":fixture, "guided_ms":guided_times[1], "repetitions":3})
        );
    }
}
