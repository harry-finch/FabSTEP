use std::{fs, process::Command};

#[test]
fn source_intersection_refusal_preserves_output_even_with_force() {
    for (extension, bytes) in [
        (
            "stl",
            include_bytes!("../../../fixtures/v1_intersecting_plate.stl").as_slice(),
        ),
        (
            "3mf",
            include_bytes!("../../../fixtures/v1_intersecting_plate.3mf").as_slice(),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join(format!("source.{extension}"));
        let target = dir.path().join("preserved.step");
        fs::write(&source, bytes).unwrap();
        let sentinel = b"existing STEP must remain unchanged\n";
        fs::write(&target, sentinel).unwrap();
        let run = Command::new(env!("CARGO_BIN_EXE_fabstep"))
            .arg("convert")
            .arg(&source)
            .arg("-o")
            .arg(&target)
            .args(["--mode", "planar", "--plane", "0.01", "--force", "--json"])
            .output()
            .unwrap();
        assert!(!run.status.success());
        let error: serde_json::Value = serde_json::from_slice(&run.stderr).unwrap();
        assert_eq!(error["code"], "cad.invalid_solid");
        assert_eq!(fs::read(&target).unwrap(), sentinel);
        assert!(run.stdout.is_empty());
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            2,
            "no temporary output left behind"
        );
    }
}
