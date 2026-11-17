//! Opt-in before/after measurement of the real search API, with exact report comparison.
//! FABSTEP_PERF_OUTPUT and optional FABSTEP_PERF_BASELINE are absolute JSON paths.
//! Optional FABSTEP_PROFILE_PRIVATE_STL adds a local, untracked source fixture.
use fabstep_app::{search_parameters, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use serde_json::{json, Value};
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "native performance measurement, run explicitly"]
fn measure_real_search() {
    let destination =
        PathBuf::from(std::env::var("FABSTEP_PERF_OUTPUT").expect("output JSON path"));
    let mut cases = vec![
        (
            "mixed_stl",
            include_bytes!("../../../fixtures/m2b_boss_blind.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "two_instances_3mf",
            include_bytes!("../../../fixtures/guided_cylinder_instances.3mf").to_vec(),
            InputFormat::ThreeMf,
        ),
    ];
    if let Ok(path) = std::env::var("FABSTEP_PROFILE_PRIVATE_STL") {
        cases.push((
            "private_sock_clip_stl",
            std::fs::read(path).unwrap(),
            InputFormat::Stl,
        ));
    }
    let baseline: Option<Value> = std::env::var("FABSTEP_PERF_BASELINE")
        .ok()
        .map(|path| serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap());
    if let Some(expected) = &baseline {
        assert_eq!(expected["complete"], true, "baseline must be complete");
        assert_eq!(expected["cases"].as_array().unwrap().len(), cases.len());
    }
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut results = Vec::new();
    for (index, (name, bytes, format)) in cases.into_iter().enumerate() {
        let mut runs = Vec::new();
        let mut first = None;
        for repetition in 0..3 {
            let start = Instant::now();
            let report = search_parameters(&bytes, format, &options, || Ok(())).unwrap();
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let report = serde_json::to_value(report).unwrap();
            if let Some(expected) = &first {
                assert_eq!(&report, expected);
            } else {
                first = Some(report.clone());
            }
            if let Some(expected) = &baseline {
                assert_eq!(expected["cases"][index]["name"], name);
                // Compare both sides after the same JSON parse. serde_json's
                // default float parser need not round-trip every final bit.
                // Keep the original report in the output for external exact
                // comparison with a correctly rounded JSON parser.
                let parsed_report: Value =
                    serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
                assert_eq!(
                    expected["cases"][index]["report"], parsed_report,
                    "before/after report differs"
                );
            }
            eprintln!("{name} run {}: {elapsed_ms:.3} ms", repetition + 1);
            runs.push(elapsed_ms);
        }
        runs.sort_by(f64::total_cmp);
        results.push(json!({"name":name,"median_ms":runs[1],"min_ms":runs[0],"max_ms":runs[2],"sorted_runs_ms":runs,"report":first}));
        std::fs::write(
            &destination,
            serde_json::to_vec_pretty(&json!({"complete":false,"cases":results})).unwrap(),
        )
        .unwrap();
    }
    std::fs::write(destination, serde_json::to_vec_pretty(&json!({"complete":true,"profile":"cargo test development, native OCCT, three sequential repetitions, no warmup discarded","cases":results})).unwrap()).unwrap();
}
