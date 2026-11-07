use std::{fs, process::Command};

#[test]
fn conversion_writes_parseable_report_and_protects_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("cube.stl");
    let output = dir.path().join("cube.step");
    fs::write(&source, include_bytes!("../../../fixtures/cube.stl")).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .arg("--json")
            .output()
            .unwrap()
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(json["cad"]["solids"], 1);
    assert_eq!(json["cad"]["faces"], 6);
    assert_eq!(json["settings"]["reconstruction"], "planar");
    assert_eq!(json["settings"]["weld_tolerance_mm"], 1e-6);
    fs::write(&output, "original output").unwrap();
    assert!(!run().status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "original output");
    fs::write(&source, "not an STL").unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("convert")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .arg("--force")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "original output");
}

#[test]
fn three_mf_uses_declared_units_and_protects_output_on_document_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("instances.3mf");
    let target = dir.path().join("instances.step");
    fs::write(
        &source,
        include_bytes!("../../../fixtures/m3_instances.3mf"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("check")
        .arg(&source)
        .args(["--unit", "inch", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(json["code"], "import.unit_override");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(&source)
            .arg("-o")
            .arg(&target)
            .args(["--force", "--json"])
            .output()
            .unwrap()
    };
    let success = run();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(report["cad"]["solids"], 2);
    assert_eq!(report["bodies"].as_array().unwrap().len(), 2);
    assert!(report["mesh"].is_null());
    let before = fs::read(&target).unwrap();
    fs::write(&source, include_bytes!("../../../fixtures/m3_overlap.3mf")).unwrap();
    let refused = run();
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["code"], "cad.bodies_not_separated");
    assert_eq!(fs::read(&target).unwrap(), before);
}

#[test]
fn check_does_not_claim_to_validate_body_separation() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m3_overlap.3mf");
    let output = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("check")
        .arg(source)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native_validated"], false);
    assert_eq!(report["bodies"].as_array().unwrap().len(), 2);
    assert!(report["cad"].is_null());
}

#[test]
fn mixed_cli_report_has_coverage_and_bidirectional_audit() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("blind.stl");
    let output = dir.path().join("blind.step");
    fs::write(&source, include_bytes!("../../../fixtures/m2b_blind.stl")).unwrap();
    let valid = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("convert")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .args(["--mode", "cylinders", "--json"])
        .output()
        .unwrap();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert!(report["recognition"].is_null());
    assert_eq!(
        report["mixed_recognition"]["model"]["cylinders"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["mixed_recognition"]["triangle_faces"]
            .as_array()
            .unwrap()
            .len(),
        report["mesh"]["input_triangles"].as_u64().unwrap() as usize
    );
    assert!(report["fidelity"]["source_to_result_mm"].as_f64().unwrap() < 0.05);
    assert!(report["fidelity"]["result_to_source_mm"].as_f64().unwrap() < 0.05);
}

#[test]
fn cylinder_options_and_refusals_preserve_the_output() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("cylinder.stl");
    let output = dir.path().join("cylinder.step");
    fs::write(&source, include_bytes!("../../../fixtures/m2_cylinder.stl")).unwrap();
    let run = |deviation: &str| {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .args([
                "--mode",
                "cylinders",
                "--fit",
                "0.00001",
                "--max-deviation",
                deviation,
                "--force",
                "--json",
            ])
            .output()
            .unwrap()
    };
    let valid = run("0.05");
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(json["cad"]["cylindrical_faces"], 1);
    assert_eq!(json["recognition"]["report"]["cylinders"], 1);
    assert_eq!(json["settings"]["reconstruction"], "cylinders");
    assert_eq!(json["settings"]["cylinder_fit_tolerance_mm"], 1e-5);
    assert_eq!(json["settings"]["max_deviation_mm"], 0.05);
    let previous = fs::read(&output).unwrap();
    let strict = run("0.001");
    assert!(!strict.status.success());
    let error: serde_json::Value = serde_json::from_slice(&strict.stderr).unwrap();
    assert_eq!(error["code"], "recognition.deviation_exceeded");
    assert_eq!(fs::read(&output).unwrap(), previous);
    fs::write(
        &source,
        include_bytes!("../../../fixtures/m2_conical_frustum.stl"),
    )
    .unwrap();
    let unsupported = run("0.05");
    assert!(!unsupported.status.success());
    assert_eq!(fs::read(&output).unwrap(), previous);
}

#[test]
fn check_reports_explicit_tolerances_without_claiming_native_validation() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m3_single_cube.3mf");
    let output = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("check")
        .arg(source)
        .args(["--plane", "0.0000001", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["settings"]["plane_tolerance_mm"], 1e-7);
    assert_eq!(report["source"]["unit"], "millimeter");
    assert_eq!(report["native_validated"], false);
    assert_eq!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == "precision.custom_tolerances")
            .count(),
        1
    );
}

#[test]
fn conical_mode_reports_native_surfaces_and_protects_output_on_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("chamfer.stl");
    let target = dir.path().join("chamfer.step");
    fs::write(
        &source,
        include_bytes!("../../../fixtures/m4_chamfered_cylinder.stl"),
    )
    .unwrap();
    let run = |command: &str, mode: &str, budget: &str| {
        let mut cli = Command::new(env!("CARGO_BIN_EXE_fabstep"));
        cli.arg(command)
            .arg(&source)
            .args(["--mode", mode, "--max-deviation", budget, "--json"]);
        if command == "convert" {
            cli.arg("-o").arg(&target).arg("--force");
        }
        cli.output().unwrap()
    };
    let check = run("check", "primitives", "0.05");
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let checked: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(checked["native_validated"], false);
    assert_eq!(checked["settings"]["reconstruction"], "primitives");
    assert_eq!(checked["revolved_recognition"]["report"]["cones"], 1);
    assert_eq!(checked["revolved_recognition"]["report"]["cylinders"], 1);
    let converted = run("convert", "primitives", "0.05");
    assert!(
        converted.status.success(),
        "{}",
        String::from_utf8_lossy(&converted.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&converted.stdout).unwrap();
    assert_eq!(report["native_validated"], true);
    assert_eq!(report["cad"]["conical_faces"], 1);
    assert_eq!(report["cad"]["cylindrical_faces"], 1);
    assert_eq!(report["cad"]["faces"], 4);
    assert_eq!(
        report["revolved_recognition"]["triangle_faces"]
            .as_array()
            .unwrap()
            .len(),
        192
    );
    assert!(report["fidelity"]["source_to_result_mm"].as_f64().unwrap() < 0.05);
    let previous = fs::read(&target).unwrap();
    for (mode, budget) in [("cylinders", "0.05"), ("primitives", "0.001")] {
        let refused = run("convert", mode, budget);
        assert!(!refused.status.success());
        let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
        if mode == "primitives" {
            assert_eq!(error["code"], "recognition.deviation_exceeded");
        }
        assert_eq!(fs::read(&target).unwrap(), previous);
    }
    fs::write(
        &source,
        include_bytes!("../../../fixtures/m4_warped_frustum.stl"),
    )
    .unwrap();
    let refused = run("convert", "primitives", "0.05");
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["code"], "recognition.conic_unproven");
    assert_eq!(fs::read(&target).unwrap(), previous);
}

#[test]
fn rounded_mode_reports_spheres_tori_and_keeps_previous_export_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("rounded.stl");
    let target = dir.path().join("rounded.step");
    let run = |command: &str, mode: &str, budget: &str| {
        let mut cli = Command::new(env!("CARGO_BIN_EXE_fabstep"));
        cli.arg(command)
            .arg(&source)
            .args(["--mode", mode, "--max-deviation", budget, "--json"]);
        if command == "convert" {
            cli.arg("-o").arg(&target).arg("--force");
        }
        cli.output().unwrap()
    };
    for (bytes, spheres, tori, fillets) in [
        (
            include_bytes!("../../../fixtures/m5_hemisphere.stl").as_slice(),
            1,
            0,
            0,
        ),
        (
            include_bytes!("../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
            0,
            1,
            1,
        ),
    ] {
        fs::write(&source, bytes).unwrap();
        let checked = run("check", "surfaces", "0.05");
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
        let checked: serde_json::Value = serde_json::from_slice(&checked.stdout).unwrap();
        assert_eq!(checked["native_validated"], false);
        assert_eq!(checked["settings"]["reconstruction"], "surfaces");
        assert_eq!(checked["rounded_recognition"]["report"]["spheres"], spheres);
        assert_eq!(checked["rounded_recognition"]["report"]["tori"], tori);
        assert_eq!(
            checked["rounded_recognition"]["report"]["toroidal_fillets"],
            fillets
        );
        let converted = run("convert", "surfaces", "0.05");
        assert!(
            converted.status.success(),
            "{}",
            String::from_utf8_lossy(&converted.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&converted.stdout).unwrap();
        assert_eq!(report["native_validated"], true);
        assert_eq!(report["cad"]["spherical_faces"], spheres);
        assert_eq!(report["cad"]["toroidal_faces"], tori);
        assert_eq!(
            report["rounded_recognition"]["triangle_faces"]
                .as_array()
                .unwrap()
                .len() as u64,
            report["mesh"]["input_triangles"].as_u64().unwrap()
        );
        assert!(report["fidelity"]["source_to_result_mm"].as_f64().unwrap() < 0.05);
        let previous = fs::read(&target).unwrap();
        for (mode, budget) in [("primitives", "0.05"), ("surfaces", "0.001")] {
            let refused = run("convert", mode, budget);
            assert!(!refused.status.success());
            let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
            if mode == "surfaces" {
                assert_eq!(error["code"], "recognition.deviation_exceeded");
            }
            assert_eq!(fs::read(&target).unwrap(), previous);
        }
    }
    let previous = fs::read(&target).unwrap();
    fs::write(
        &source,
        include_bytes!("../../../fixtures/m5_warped_torus.stl"),
    )
    .unwrap();
    let refused = run("convert", "surfaces", "0.05");
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert!(error["code"].as_str().unwrap().starts_with("recognition."));
    assert_eq!(fs::read(&target).unwrap(), previous);
}
