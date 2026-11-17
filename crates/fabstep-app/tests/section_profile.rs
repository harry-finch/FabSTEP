use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{export_section_dxf, Options, ProfileFrame, ReconstructionMode, SectionRequest};

fn xy(z: f64) -> ProfileFrame {
    ProfileFrame {
        origin_mm: [0., 0., z],
        x_axis: [1., 0., 0.],
        y_axis: [0., 1., 0.],
        normal: [0., 0., 1.],
    }
}

#[test]
fn document_sections_preserve_validated_step_and_refuse_empty_export() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options::default(),
    )
    .unwrap();
    let before = document.step.bytes().to_vec();
    let section = document
        .step
        .section_profile(&SectionRequest {
            frame: xy(0.5),
            solid_ids: vec![0],
        })
        .unwrap();
    assert_eq!(section.bodies.len(), 1);
    assert_eq!(section.bodies[0].loops.len(), 1);
    assert_eq!(section.bodies[0].loops[0].segments.len(), 4);
    assert!(!export_section_dxf(&section).unwrap().is_empty());
    let empty = document
        .step
        .section_profile(&SectionRequest {
            frame: xy(2.),
            solid_ids: vec![0],
        })
        .unwrap();
    assert!(empty.bodies[0].loops.is_empty());
    assert_eq!(
        export_section_dxf(&empty).unwrap_err().code,
        "dxf.section_empty"
    );
    assert!(document
        .step
        .section_profile(&SectionRequest {
            frame: xy(0.),
            solid_ids: vec![0]
        })
        .is_err());
    assert_eq!(document.step.bytes(), before);
    let face = document.step.result_faces().unwrap().result.faces[0].id;
    let frame = document.step.planar_face_frame(face).unwrap();
    let normal_length: f64 = frame.normal.iter().map(|v| v * v).sum();
    assert!((normal_length - 1.).abs() < 1e-12);
}

#[test]
fn stl_inches_section_is_exported_in_actual_millimetres() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options {
            input_unit: fabstep_core::InputUnit::Inch,
            ..Options::default()
        },
    )
    .unwrap();
    let profile = document
        .step
        .section_profile(&SectionRequest {
            frame: xy(12.7),
            solid_ids: vec![0],
        })
        .unwrap();
    for segment in &profile.bodies[0].loops[0].segments {
        let fabstep_core::ProfileSegment::Line { start_mm, end_mm } = segment else {
            panic!("cube section is linear")
        };
        assert!(((end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]) - 25.4).abs() < 1e-9);
    }
    let bytes = export_section_dxf(&profile).unwrap();
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("$INSUNITS\n70\n4\n"));
}

#[test]
#[ignore = "native section corpus for independent DXF reader; requires absolute FABSTEP_SECTION_CORPUS_DIR"]
fn emit_native_section_corpus() {
    let directory = std::path::PathBuf::from(std::env::var("FABSTEP_SECTION_CORPUS_DIR").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let cases: Vec<(&str, &[u8], InputFormat, ReconstructionMode)> = vec![
        (
            "box",
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
            "short_arcs",
            include_bytes!("../../../fixtures/short_arcs/convex.stl"),
            InputFormat::Stl,
            ReconstructionMode::Surfaces,
        ),
        (
            "instances",
            include_bytes!("../../../fixtures/m3_instances.3mf"),
            InputFormat::ThreeMf,
            ReconstructionMode::Planar,
        ),
        (
            "torus",
            include_bytes!("../../../fixtures/m5d_ring_torus.stl"),
            InputFormat::Stl,
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
        let bounds = document.report.cad.bounds_mm;
        let z = bounds[0][2] + (bounds[1][2] - bounds[0][2]) * 0.6;
        let ids = (0..document.report.cad.solids).collect::<Vec<_>>();
        let mut frames = vec![("xy", xy(z))];
        if name == "box" {
            let n = 3f64.sqrt().recip();
            let x = 2f64.sqrt().recip();
            let y = 6f64.sqrt().recip();
            frames.push((
                "oblique",
                ProfileFrame {
                    origin_mm: [0.5; 3],
                    x_axis: [x, -x, 0.],
                    y_axis: [y, y, -2. * y],
                    normal: [n; 3],
                },
            ));
        }
        if name == "torus" {
            frames.push((
                "yz",
                ProfileFrame {
                    origin_mm: [0.; 3],
                    x_axis: [0., 1., 0.],
                    y_axis: [0., 0., 1.],
                    normal: [1., 0., 0.],
                },
            ));
        }
        for (label, frame) in frames {
            let request = SectionRequest {
                frame,
                solid_ids: ids.clone(),
            };
            let profile = document.step.section_profile(&request).unwrap();
            assert_eq!(
                profile
                    .bodies
                    .iter()
                    .map(|b| b.solid_id)
                    .collect::<Vec<_>>(),
                ids
            );
            let filename = format!("{name}-{label}.dxf");
            std::fs::write(
                directory.join(&filename),
                export_section_dxf(&profile).unwrap(),
            )
            .unwrap();
            manifest.push(
                serde_json::json!({"file":filename,"profile":profile,"selected_solid_ids":ids}),
            );
        }
    }
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn tilted_frame(center: [f64; 3], degrees: f64, around_x: bool) -> ProfileFrame {
    let (s, c) = degrees.to_radians().sin_cos();
    ProfileFrame {
        origin_mm: center,
        x_axis: if around_x { [1., 0., 0.] } else { [c, 0., -s] },
        y_axis: if around_x { [0., c, s] } else { [0., 1., 0.] },
        normal: if around_x { [0., -s, c] } else { [s, 0., c] },
    }
}

#[test]
fn oblique_cylinder_profiles_export_exact_ellipses() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/m2_cylinder.stl"),
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            ..Options::default()
        },
    )
    .unwrap();
    let bounds = document.report.cad.bounds_mm;
    let center = std::array::from_fn(|i| (bounds[0][i] + bounds[1][i]) / 2.);
    let original = document.step.bytes().to_vec();
    for around_x in [false, true] {
        let section = document
            .step
            .section_profile(&SectionRequest {
                frame: tilted_frame(center, 1., around_x),
                solid_ids: vec![0],
            })
            .unwrap();
        assert_eq!(section.bodies[0].loops.len(), 1);
        let segments = &section.bodies[0].loops[0].segments;
        assert!(segments
            .iter()
            .any(|segment| matches!(segment, fabstep_core::ProfileSegment::Ellipse { .. })));
        for segment in segments {
            if let fabstep_core::ProfileSegment::Ellipse { ratio, .. } = segment {
                assert!((ratio - 1f64.to_radians().cos()).abs() < 1e-8);
            }
        }
        let bytes = export_section_dxf(&section).unwrap();
        assert!(std::str::from_utf8(&bytes).unwrap().contains("\nELLIPSE\n"));
        assert_eq!(document.step.bytes(), original);
    }
}

#[test]
#[ignore = "private source qualification; requires FABSTEP_SECTION_PRIVATE_STL and absolute FABSTEP_SECTION_CORPUS_DIR"]
fn emit_private_inclined_section_corpus() {
    let source = std::fs::read(std::env::var("FABSTEP_SECTION_PRIVATE_STL").unwrap()).unwrap();
    let directory = std::path::PathBuf::from(std::env::var("FABSTEP_SECTION_CORPUS_DIR").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    // Explicit qualification setting previously established for the private fixture;
    // this does not alter application defaults or the user's saved settings.
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        cylinder_fit_tolerance_mm: 0.000012,
        ..Options::default()
    };
    let document = convert_document_with_provenance(&source, InputFormat::Stl, &options).unwrap();
    let bounds = document.report.cad.bounds_mm;
    let center = std::array::from_fn(|i| (bounds[0][i] + bounds[1][i]) / 2.);
    let ids = (0..document.report.cad.solids).collect::<Vec<_>>();
    let mut manifest = Vec::new();
    let mut outcomes = Vec::new();
    let mut slight_passes = 0;
    for around_x in [false, true] {
        for degrees in [0., 1., 5., 15., 30., 60., 89.] {
            let axis = if around_x { "x" } else { "y" };
            let request = SectionRequest {
                frame: tilted_frame(center, degrees, around_x),
                solid_ids: ids.clone(),
            };
            match document.step.section_profile(&request) {
                Ok(profile) => match export_section_dxf(&profile) {
                    Ok(bytes) => {
                        let filename = format!("private-{axis}-{degrees}.dxf");
                        std::fs::write(directory.join(&filename), bytes).unwrap();
                        if degrees == 1. { slight_passes += 1; }
                        outcomes.push(serde_json::json!({"axis":axis,"degrees":degrees,"status":"PASS"}));
                        manifest.push(serde_json::json!({"file":filename,"profile":profile,"selected_solid_ids":ids}));
                    },
                    Err(error) => outcomes.push(serde_json::json!({"axis":axis,"degrees":degrees,"error":error.code})),
                },
                Err(error) => outcomes.push(serde_json::json!({"axis":axis,"degrees":degrees,"error":error.code,"detail":error.message})),
            }
        }
    }
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("outcomes.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"options":options,"bounds_mm":bounds,"outcomes":outcomes}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        slight_passes, 2,
        "Both one-degree cuts must export; see outcomes.json"
    );
}
