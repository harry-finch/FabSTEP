use std::{fs, path::Path, process::Command};

#[test]
fn complete_torus_reports_axis_and_no_boundaries_and_preserves_output_on_refusal() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("torus.step");
    let run = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(fixtures.join(name))
            .arg("-o")
            .arg(&output)
            .args(["--mode", "surfaces", "--force", "--json"])
            .output()
            .unwrap()
    };
    let valid = run("m5d_ring_torus.stl");
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(report["cad"]["toroidal_faces"], 1);
    assert_eq!(report["cad"]["faces"], 1);
    let torus = &report["bodies"][0]["rounded_recognition"]["model"]["tori"][0];
    assert!(torus.get("rings").is_some_and(serde_json::Value::is_null));
    assert_eq!(torus["axis"].as_array().unwrap().len(), 3);
    let before = fs::read(&output).unwrap();
    let invalid = run("m5d_torus_dent.stl");
    assert!(!invalid.status.success());
    let error: serde_json::Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(error["code"], "recognition.surface_unproven");
    assert_eq!(fs::read(&output).unwrap(), before);
}
