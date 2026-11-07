use fabstep_core::{analyze_mesh, analyze_mesh_bodies, InputUnit, Options};
use fabstep_import::{import_3mf, UnitOrigin};

#[test]
fn independent_core_packages_preserve_units_instances_and_transform_order() {
    for (bytes, volumes, bounds, unit) in [
        (
            include_bytes!("../../../fixtures/m3_single_cube.3mf").as_slice(),
            vec![1000.],
            vec![[[0., 0., 0.], [10., 10., 10.]]],
            "millimeter",
        ),
        (
            include_bytes!("../../../fixtures/m3_instances.3mf").as_slice(),
            vec![1000., 1000.],
            vec![
                [[0., 0., 0.], [10., 10., 10.]],
                [[30., 5., 0.], [40., 15., 10.]],
            ],
            "millimeter",
        ),
        (
            include_bytes!("../../../fixtures/m3_nested_components.3mf").as_slice(),
            vec![1000.],
            vec![[[10., 12., 0.], [20., 22., 10.]]],
            "millimeter",
        ),
        (
            include_bytes!("../../../fixtures/m3_inches.3mf").as_slice(),
            vec![25.4_f64.powi(3)],
            vec![[[50.8, 0., 0.], [76.2, 25.4, 25.4]]],
            "inch",
        ),
        (
            include_bytes!("../../../fixtures/m3_mirror.3mf").as_slice(),
            vec![1000.],
            vec![[[10., 0., 0.], [20., 10., 10.]]],
            "millimeter",
        ),
    ] {
        let d = import_3mf(bytes).unwrap();
        assert_eq!(d.bodies.len(), volumes.len());
        assert_eq!(d.unit, unit);
        assert_eq!(d.unit_origin, UnitOrigin::Declared);
        for (i, b) in d.bodies.iter().enumerate() {
            let a = analyze_mesh(
                &b.mesh,
                &Options {
                    input_unit: InputUnit::Inch,
                    ..Options::default()
                },
            )
            .unwrap();
            assert!((a.report.volume_mm3 - volumes[i]).abs() < 1e-7);
            for j in 0..2 {
                for k in 0..3 {
                    assert!((a.report.bounds_mm[j][k] - bounds[i][j][k]).abs() < 1e-9);
                }
            }
            assert!(!a
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "mesh.orientation_reversed"));
        }
    }
    let d = import_3mf(include_bytes!("../../../fixtures/m3_default_unit.3mf")).unwrap();
    assert_eq!(d.unit_origin, UnitOrigin::Default);
    assert_eq!(d.unit, "millimeter");
    let nested = import_3mf(include_bytes!("../../../fixtures/m3_nested_components.3mf")).unwrap();
    assert_eq!(nested.bodies[0].source.instance_path, vec![0, 0]);
    assert_eq!(nested.bodies[0].source.object_id, 1);
}

#[test]
fn one_instance_can_contain_separate_closed_components() {
    let d = import_3mf(include_bytes!("../../../fixtures/m3_disconnected_mesh.3mf")).unwrap();
    assert_eq!(d.bodies.len(), 1);
    let a = analyze_mesh_bodies(&d.bodies[0].mesh, &Options::default()).unwrap();
    assert_eq!(a.len(), 2);
    assert!(a.iter().all(|a| (a.report.volume_mm3 - 1000.).abs() < 1e-9));
    assert_eq!(
        analyze_mesh(&d.bodies[0].mesh, &Options::default())
            .unwrap_err()
            .code,
        "mesh.multiple_components"
    );
}

#[test]
fn independent_invalid_packages_fail_explicitly() {
    for (source, code) in [
        (
            include_bytes!("../../../fixtures/m3_bad_indices.3mf").as_slice(),
            "threemf.invalid_index",
        ),
        (
            include_bytes!("../../../fixtures/m3_cycle.3mf").as_slice(),
            "threemf.invalid_reference",
        ),
        (
            include_bytes!("../../../fixtures/m3_required_extension.3mf").as_slice(),
            "threemf.unsupported_extension",
        ),
        (
            include_bytes!("../../../fixtures/m3_zip_budget.3mf").as_slice(),
            "threemf.archive_limit",
        ),
    ] {
        assert_eq!(import_3mf(source).unwrap_err().code, code);
    }
    let d = import_3mf(include_bytes!("../../../fixtures/m3_open_mesh.3mf")).unwrap();
    assert_eq!(
        analyze_mesh(&d.bodies[0].mesh, &Options::default())
            .unwrap_err()
            .code,
        "mesh.open"
    );
}

fn repack(model: &str, target: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let rel = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="{target}"/></Relationships>"#
    );
    let types = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
    for (name, data) in [
        ("_rels/.rels", rel.as_bytes()),
        ("[Content_Types].xml", types.as_bytes()),
        ("custom/part.model", model.as_bytes()),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    {
        w.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}
fn source_model() -> String {
    use std::io::{Cursor, Read};
    let mut z = zip::ZipArchive::new(Cursor::new(include_bytes!(
        "../../../fixtures/m3_single_cube.3mf"
    )))
    .unwrap();
    let mut s = String::new();
    z.by_name("3D/3dmodel.model")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    s
}
#[test]
fn opc_root_is_resolved_and_names_crc_and_xml_are_checked() {
    let s = source_model();
    let bytes = repack(&s, "/custom/part.model", &[]);
    assert_eq!(
        import_3mf(&bytes).unwrap().bodies[0].source.model_path,
        "custom/part.model"
    );
    assert_eq!(
        import_3mf(&repack(&s, "../custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.invalid_path"
    );
    assert_eq!(
        import_3mf(&repack(&s, "/missing.model", &[]))
            .unwrap_err()
            .code,
        "threemf.missing_part"
    );
    assert_eq!(
        import_3mf(&repack(
            &s,
            "/custom/part.model",
            &[("CUSTOM/PART.model", b"other")]
        ))
        .unwrap_err()
        .code,
        "threemf.duplicate_part"
    );
    let dtd = s.replace(
        "<model",
        "<!DOCTYPE model [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><model",
    );
    assert_eq!(
        import_3mf(&repack(&dtd, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.invalid_xml"
    );
    let mut corrupted = bytes.clone();
    let start = corrupted.windows(7).position(|x| x == b"<model ").unwrap();
    corrupted[start + 1] = b'n';
    assert_eq!(
        import_3mf(&corrupted).unwrap_err().code,
        "threemf.invalid_archive"
    );
    for cut in [0, 4, bytes.len() - 1] {
        assert!(import_3mf(&bytes[..cut]).is_err());
    }
}
#[test]
fn extensions_singular_transforms_and_instance_explosion_are_rejected() {
    let s = source_model();
    let production = s
        .replace(
            "<model ",
            "<model xmlns:p=\"http://schemas.microsoft.com/3dmanufacturing/production/2015/06\" ",
        )
        .replace("<item ", "<item p:path=\"/other.model\" ");
    assert_eq!(
        import_3mf(&repack(&production, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.unsupported_extension"
    );
    for transform in [
        "0 0 0 0 0 0 0 0 0 0 0 0",
        "1 0 0 0 1 0 0 0 1 NaN 0 0",
        "1 0",
    ] {
        let bad = s.replace("<item ", &format!("<item transform=\"{transform}\" "));
        assert!(import_3mf(&repack(&bad, "/custom/part.model", &[])).is_err());
    }
    let repeated = s.replace(
        "<item objectid=\"1\"/>",
        &"<item objectid=\"1\"/>".repeat(33),
    );
    assert_eq!(
        import_3mf(&repack(&repeated, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.complexity_limit"
    );
}
#[test]
fn all_six_units_and_optional_metadata_have_explicit_semantics() {
    let s = source_model();
    for (unit, factor) in [
        ("micron", 0.001),
        ("millimeter", 1.),
        ("centimeter", 10.),
        ("inch", 25.4),
        ("foot", 304.8),
        ("meter", 1000.),
    ] {
        let xml = s.replace("unit=\"millimeter\"", &format!("unit=\"{unit}\""));
        let d = import_3mf(&repack(&xml, "/custom/part.model", &[])).unwrap();
        assert_eq!(d.bodies[0].mesh.vertices[1][0], 10. * factor);
    }
    let meta = s.replace(
        "<resources>",
        "<metadata name=\"Title\">A &amp; B</metadata><resources>",
    );
    let d = import_3mf(&repack(&meta, "/custom/part.model", &[])).unwrap();
    assert!(d
        .diagnostics
        .iter()
        .any(|d| d.code == "threemf.metadata_ignored"));
}

#[test]
fn xml_nesting_and_attribute_allocations_are_bounded_before_dom_parsing() {
    let s = source_model();
    let deep = s.replace(
        "<resources>",
        &format!("{}{}<resources>", "<x>".repeat(4000), "</x>".repeat(4000)),
    );
    assert_eq!(
        import_3mf(&repack(&deep, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.xml_limit"
    );
    let name = s.replace("name=\"Cube\"", &format!("name=\"{}\"", "x".repeat(1025)));
    assert_eq!(
        import_3mf(&repack(&name, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.xml_limit"
    );
    let attrs = (0..65).map(|i| format!("a{i}=\"0\" ")).collect::<String>();
    let many = s.replace("<model ", &format!("<model {attrs}"));
    assert_eq!(
        import_3mf(&repack(&many, "/custom/part.model", &[]))
            .unwrap_err()
            .code,
        "threemf.xml_limit"
    );
}

#[test]
fn malformed_core_containers_do_not_silently_change_coordinate_semantics() {
    let s = source_model();
    for bad in [
        s.replace("<vertices>", "<vertices>error"),
        s.replace("<vertices>", "<vertices unit=\"inch\">"),
        s.replace("<triangles>", "<triangles scale=\"25.4\">"),
        s.replace("<resources>", "<resources>unexpected"),
        s.replace("<build>", "<build bogus=\"1\">"),
    ] {
        assert_eq!(
            import_3mf(&repack(&bad, "/custom/part.model", &[]))
                .unwrap_err()
                .code,
            "threemf.invalid_xml"
        );
    }
    // Nonempty metadata is legal and remains available as ignored metadata.
    let metadata = s.replace(
        "<resources>",
        "<metadata name=\"Description\">real text</metadata><resources>",
    );
    assert!(import_3mf(&repack(&metadata, "/custom/part.model", &[])).is_ok());
}
