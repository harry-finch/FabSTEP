use std::{fs, path::Path, process::Command};

#[test]
fn rounded_box_check_conversion_and_budget_preserve_validation_contract() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m6a_rounded_box_rotated.stl");
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("rounded-box.step");
    let check = Command::new(env!("CARGO_BIN_EXE_fabstep"))
        .arg("check")
        .arg(&source)
        .args(["--mode", "surfaces", "--json"])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(report["native_validated"], false);
    assert_eq!(report["rounded_box_recognition"]["report"]["cylinders"], 12);
    let run = |budget: &str| {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .args([
                "--mode",
                "surfaces",
                "--max-deviation",
                budget,
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
    let report: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(report["cad"]["faces"], 26);
    assert_eq!(report["cad"]["spherical_faces"], 8);
    assert_eq!(report["native_validated"], true);
    assert_eq!(report["rounded_box_recognition"]["report"]["spheres"], 8);
    assert_eq!(
        report["rounded_box_recognition"],
        report["bodies"][0]["rounded_box_recognition"]
    );
    let before = fs::read(&output).unwrap();
    let invalid = run("0.0001");
    assert!(!invalid.status.success());
    let error: serde_json::Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(error["code"], "recognition.deviation_exceeded");
    assert_eq!(fs::read(&output).unwrap(), before);
}
