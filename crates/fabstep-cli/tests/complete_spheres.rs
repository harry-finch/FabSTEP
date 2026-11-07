use std::{fs, path::Path, process::Command};

#[test]
fn complete_sphere_reports_null_boundaries_and_refusal_preserves_existing_step() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("sphere.step");
    let run = |name: &str| {
        Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(fixtures.join(name))
            .arg("-o")
            .arg(&output)
            .args(["--mode", "surfaces", "--unit", "mm", "--force", "--json"])
            .output()
            .unwrap()
    };
    let valid = run("m5c_icosphere.stl");
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(report["cad"]["spherical_faces"], 1);
    assert_eq!(report["cad"]["faces"], 1);
    let sphere = &report["bodies"][0]["rounded_recognition"]["model"]["spheres"][0];
    assert!(sphere.get("rings").is_some_and(serde_json::Value::is_null));
    let before = fs::read(&output).unwrap();
    let refused = run("m5c_sphere_dent.stl");
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["code"], "recognition.surface_unproven");
    assert_eq!(fs::read(&output).unwrap(), before);
}
