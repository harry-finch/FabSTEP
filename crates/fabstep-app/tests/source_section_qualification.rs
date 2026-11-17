//! End-to-end qualification of trusted source sections and independent DXF corpora.
use fabstep_app::{prepare_source_section, InputFormat};
use fabstep_core::{export_section_dxf, Options, ProfileFrame, ProfileSegment, SectionRequest};

fn frame(bounds: [[f64; 3]; 2], angle: f64) -> ProfileFrame {
    let (s, c) = angle.to_radians().sin_cos();
    ProfileFrame {
        origin_mm: [
            (bounds[0][0] + bounds[1][0]) / 2.,
            (bounds[0][1] + bounds[1][1]) / 2.,
            bounds[0][2] + 0.37 * (bounds[1][2] - bounds[0][2]),
        ],
        x_axis: [c, 0., -s],
        y_axis: [0., 1., 0.],
        normal: [s, 0., c],
    }
}

#[test]
fn source_sections_do_not_depend_on_successful_curve_recognition() {
    let bytes = include_bytes!("../../../fixtures/m5c_spheroid.stl");
    let options = Options {
        reconstruction: fabstep_core::ReconstructionMode::Surfaces,
        ..Options::default()
    };
    assert!(
        fabstep_app::convert_document_with_provenance(bytes, InputFormat::Stl, &options).is_err()
    );
    let source = prepare_source_section(bytes, InputFormat::Stl, &options).unwrap();
    let section = source
        .section_profile(&SectionRequest {
            frame: frame(source.metadata().bounds_mm, 13.),
            solid_ids: vec![0],
        })
        .unwrap();
    assert!(!section.bodies[0].loops.is_empty());
    assert!(section.bodies[0]
        .loops
        .iter()
        .flat_map(|loop_| &loop_.segments)
        .all(|segment| matches!(segment, ProfileSegment::Line { .. })));
    assert!(!export_section_dxf(&section).unwrap().is_empty());
}

#[test]
#[ignore = "independent source DXF qualification requires absolute FABSTEP_SOURCE_SECTION_CORPUS_DIR; private STL optional"]
fn emit_source_section_corpus() {
    let directory =
        std::path::PathBuf::from(std::env::var("FABSTEP_SOURCE_SECTION_CORPUS_DIR").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let mut cases: Vec<(&str, Vec<u8>, InputFormat)> = vec![
        (
            "cube",
            include_bytes!("../../../fixtures/cube.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "concave",
            include_bytes!("../../../fixtures/concave_prism.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "tube",
            include_bytes!("../../../fixtures/m2_tube.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "spheroid",
            include_bytes!("../../../fixtures/m5c_spheroid.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "torus",
            include_bytes!("../../../fixtures/m5d_ring_torus.stl").to_vec(),
            InputFormat::Stl,
        ),
        (
            "instances",
            include_bytes!("../../../fixtures/m3_instances.3mf").to_vec(),
            InputFormat::ThreeMf,
        ),
    ];
    if let Ok(path) = std::env::var("FABSTEP_SOURCE_SECTION_PRIVATE_STL") {
        cases.push(("private", std::fs::read(path).unwrap(), InputFormat::Stl));
    }
    let mut manifest = Vec::new();
    for (name, bytes, format) in cases {
        let source = prepare_source_section(&bytes, format, &Options::default()).unwrap();
        let ids = source
            .metadata()
            .bodies
            .iter()
            .map(|body| body.solid_id)
            .collect::<Vec<_>>();
        let bounds = source.metadata().bodies[0].bounds_mm;
        for angle in [0., 13.] {
            let profile = source
                .section_profile(&SectionRequest {
                    frame: frame(bounds, angle),
                    solid_ids: ids.clone(),
                })
                .unwrap();
            assert!(profile
                .bodies
                .iter()
                .flat_map(|body| &body.loops)
                .flat_map(|loop_| &loop_.segments)
                .all(|segment| matches!(segment, ProfileSegment::Line { .. })));
            let filename = format!("source-{name}-{angle}.dxf");
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

#[test]
fn inch_source_section_keeps_physical_dimensions_and_dxf_millimetres() {
    let source = prepare_source_section(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &Options {
            input_unit: fabstep_core::InputUnit::Inch,
            ..Options::default()
        },
    )
    .unwrap();
    let profile = source
        .section_profile(&SectionRequest {
            frame: frame(source.metadata().bounds_mm, 0.),
            solid_ids: vec![0],
        })
        .unwrap();
    let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    for segment in &profile.bodies[0].loops[0].segments {
        let ProfileSegment::Line { start_mm, end_mm } = segment else {
            panic!("source contours must remain polygonal")
        };
        for point in [start_mm, end_mm] {
            for (axis, &coordinate) in point.iter().enumerate() {
                bounds[0][axis] = bounds[0][axis].min(coordinate);
                bounds[1][axis] = bounds[1][axis].max(coordinate);
            }
        }
    }
    for axis in 0..2 {
        assert!((bounds[1][axis] - bounds[0][axis] - 25.4).abs() < 1e-9);
    }
    let bytes = export_section_dxf(&profile).unwrap();
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("$INSUNITS\n70\n4\n"));
}
