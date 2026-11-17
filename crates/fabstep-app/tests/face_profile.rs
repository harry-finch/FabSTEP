use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{export_planar_face_dxf, Options, ReconstructionMode};

#[test]
fn immutable_document_profiles_use_the_inspected_face_identity_and_preserve_step() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    let before = document.step.bytes().to_vec();
    let faces = document.step.result_faces().unwrap();
    for face in &faces.result.faces {
        let profile = document.step.planar_face_profile(face.id).unwrap();
        assert_eq!(
            (profile.face_id, profile.solid_id),
            (face.id, face.solid_id)
        );
        assert_eq!(profile.loops.len(), 1);
        let dxf = export_planar_face_dxf(&profile).unwrap();
        assert!(std::str::from_utf8(&dxf)
            .unwrap()
            .contains("$INSUNITS\n70\n4\n"));
    }
    assert_eq!(
        document
            .step
            .planar_face_profile(usize::MAX)
            .unwrap_err()
            .code,
        "dxf.face_not_found"
    );
    assert_eq!(document.step.bytes(), before);
}

#[test]
#[ignore = "emit native profiles for an independent DXF reader; requires absolute FABSTEP_DXF_CORPUS_DIR"]
fn emit_native_dxf_corpus() {
    let directory = std::path::PathBuf::from(std::env::var("FABSTEP_DXF_CORPUS_DIR").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let cases: Vec<(&str, &[u8], InputFormat, ReconstructionMode)> = vec![
        (
            "cube",
            include_bytes!("../../../fixtures/cube.stl"),
            InputFormat::Stl,
            ReconstructionMode::Planar,
        ),
        (
            "boss_blind",
            include_bytes!("../../../fixtures/m2b_boss_blind.stl"),
            InputFormat::Stl,
            ReconstructionMode::Surfaces,
        ),
        (
            "open_arc_rotated",
            include_bytes!("../../../fixtures/m2c_open_arc_rotated.stl"),
            InputFormat::Stl,
            ReconstructionMode::Cylinders,
        ),
        (
            "short_arcs",
            include_bytes!("../../../fixtures/short_arcs/convex.stl"),
            InputFormat::Stl,
            ReconstructionMode::Surfaces,
        ),
        (
            "instances",
            include_bytes!("../../../fixtures/guided_cylinder_instances.3mf"),
            InputFormat::ThreeMf,
            ReconstructionMode::Surfaces,
        ),
    ];
    let mut manifest = Vec::new();
    for (name, bytes, format, reconstruction) in cases {
        let document = convert_document_with_provenance(
            bytes,
            format,
            &Options {
                reconstruction,
                ..Options::default()
            },
        )
        .unwrap();
        let inspected = document.step.result_faces().unwrap();
        for face in &inspected.result.faces {
            if !matches!(face.family, fabstep_cad::ResultFaceFamily::Plane) {
                continue;
            }
            let profile = document.step.planar_face_profile(face.id).unwrap();
            let filename = format!("{name}-{}.dxf", face.id);
            std::fs::write(
                directory.join(&filename),
                export_planar_face_dxf(&profile).unwrap(),
            )
            .unwrap();
            manifest.push(serde_json::json!({"file":filename,"profile":profile}));
        }
    }
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}
