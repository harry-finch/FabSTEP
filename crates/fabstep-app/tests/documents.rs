use fabstep_app::{analyze_document, convert_document, preview_document, InputFormat, UnitOrigin};
use fabstep_core::{read_stl_mesh, InputUnit, Mesh, Options, ReconstructionMode};
use std::{fmt::Write, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(format!("{name}.3mf")),
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-6_f64.max(b.abs() * 1e-8), "{a} != {b}");
}

#[test]
fn independent_3mf_corpus_preserves_units_instances_and_composed_placements() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/m3_manifest.json")).unwrap();
    // A remembered STL-inch preference must never rescale a 3MF document.
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    for (name, expected) in manifest["cases"].as_object().unwrap() {
        if expected["valid"] != true {
            continue;
        }
        let result = convert_document(&fixture(name), InputFormat::ThreeMf, &options)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let n = expected["solids"].as_u64().unwrap() as usize;
        assert_eq!(result.report.cad.solids, n, "{name}");
        assert_eq!(result.report.bodies.len(), n, "{name}");
        assert_eq!(
            result.report.settings.reconstruction,
            ReconstructionMode::Planar
        );
        assert_eq!(
            result.report.settings.plane_tolerance_mm,
            options.plane_tolerance_mm
        );
        assert!(!result
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "precision.custom_tolerances"));
        assert_eq!(
            result.report.source.unit,
            expected["unit"].as_str().unwrap()
        );
        assert!(result.step_bytes.starts_with(b"ISO-10303-21;"));
        let mut sum = 0.;
        for (i, body) in result.report.bodies.iter().enumerate() {
            let cad = &body.conversion.cad;
            assert_eq!(cad.faces, 6, "{name}");
            let volume = expected["volume_each_mm3"][i].as_f64().unwrap();
            sum += volume;
            close(cad.volume_mm3, volume);
            for end in 0..2 {
                for axis in 0..3 {
                    close(
                        cad.bounds_mm[end][axis],
                        expected["body_bounds_mm"][i][end][axis].as_f64().unwrap(),
                    );
                }
            }
        }
        close(result.report.cad.volume_mm3, sum);
    }
}

#[test]
fn preview_is_available_for_open_mesh_but_never_a_partial_conversion() {
    let bytes = fixture("m3_open_mesh");
    let preview = preview_document(&bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    assert_eq!(preview.bodies.len(), 1);
    assert!(!preview.bodies[0].mesh.triangles.is_empty());
    let error = convert_document(&bytes, InputFormat::ThreeMf, &Options::default())
        .err()
        .unwrap();
    assert_eq!(error.code, "mesh.open");
    assert!(error.message.contains(&preview.bodies[0].id));
    let encoded = serde_json::to_value(&error).unwrap();
    assert_eq!(encoded["body"]["id"], preview.bodies[0].id);
    assert_eq!(encoded["body"]["name"], preview.bodies[0].name);
    let context = error.body.unwrap();
    assert_eq!(
        error.message,
        format!("{} [{}] : {}", context.name, context.id, context.detail)
    );
}

#[test]
fn contacts_overlaps_and_nested_solids_are_refused() {
    for name in ["m3_touching", "m3_overlap", "m3_nested_solids"] {
        let error = convert_document(&fixture(name), InputFormat::ThreeMf, &Options::default())
            .err()
            .unwrap();
        assert_eq!(error.code, "cad.bodies_not_separated", "{name}: {error}");
    }
}

#[test]
fn document_ids_are_unique_deterministic_and_keep_source_provenance() {
    let bytes = fixture("m3_instances");
    let a = analyze_document(&bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    let b = analyze_document(&bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    assert_ne!(a.bodies[0].id, a.bodies[1].id);
    assert_eq!(a.bodies[0].id, b.bodies[0].id);
    assert_eq!(
        a.bodies[0].provenance.object_id,
        a.bodies[1].provenance.object_id
    );
    assert_ne!(
        a.bodies[0].provenance.build_item,
        a.bodies[1].provenance.build_item
    );
    let default = preview_document(
        &fixture("m3_default_unit"),
        InputFormat::ThreeMf,
        &Options::default(),
    )
    .unwrap();
    assert!(matches!(default.source.unit_origin, UnitOrigin::Default));
}

#[test]
fn generated_names_are_explicit_and_never_inferred_from_user_text() {
    let bytes = fixture("m3_name_origins");
    let preview = preview_document(&bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    assert_eq!(preview.bodies.len(), 3);
    assert_eq!(preview.bodies[0].name, preview.bodies[1].name);
    assert!(preview.bodies[0].provenance.generated_name);
    assert!(!preview.bodies[1].provenance.generated_name);
    assert!(preview.bodies[2].provenance.generated_name);
    let document = analyze_document(&bytes, InputFormat::ThreeMf, &Options::default()).unwrap();
    for (before, after) in preview.bodies.iter().zip(&document.bodies) {
        assert_eq!(before.name, after.name);
        assert_eq!(
            before.provenance.generated_name,
            after.provenance.generated_name
        );
    }
}

fn stl(meshes: &[Mesh]) -> Vec<u8> {
    let mut s = String::from("solid document\n");
    for mesh in meshes {
        for t in &mesh.triangles {
            s.push_str("facet normal 0 0 0\nouter loop\n");
            for &v in t {
                let p = mesh.vertices[v];
                writeln!(s, "vertex {:.15} {:.15} {:.15}", p[0], p[1], p[2]).unwrap();
            }
            s.push_str("endloop\nendfacet\n");
        }
    }
    s.push_str("endsolid document\n");
    s.into_bytes()
}

#[test]
fn disconnected_stl_preserves_two_analytic_bodies_and_legacy_stays_strict() {
    let first = read_stl_mesh(
        include_bytes!("../../../fixtures/m2c_open_arc.stl"),
        InputUnit::Mm,
    )
    .unwrap();
    let mut second = read_stl_mesh(
        include_bytes!("../../../fixtures/m2b_blind.stl"),
        InputUnit::Mm,
    )
    .unwrap();
    for v in &mut second.vertices {
        v[0] += 200.;
    }
    let source = stl(&[first, second]);
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    assert_eq!(
        fabstep_app::convert_stl(&source, &options)
            .err()
            .unwrap()
            .code,
        "mesh.multiple_components"
    );
    let result = convert_document(&source, InputFormat::Stl, &options).unwrap();
    assert_eq!(result.report.cad.solids, 2);
    assert_eq!(result.report.cad.cylindrical_faces, 2);
    assert!(result
        .report
        .bodies
        .iter()
        .all(|b| b.conversion.fidelity.is_some()));
}

#[test]
fn nested_negative_stl_shell_is_not_silently_filled() {
    let outer = read_stl_mesh(include_bytes!("../../../fixtures/cube.stl"), InputUnit::Mm).unwrap();
    let mut inner = outer.clone();
    for v in &mut inner.vertices {
        for c in v {
            *c = *c * 0.25 + 0.25;
        }
    }
    for t in &mut inner.triangles {
        t.swap(1, 2);
    }
    let error = convert_document(&stl(&[outer, inner]), InputFormat::Stl, &Options::default())
        .err()
        .unwrap();
    assert_eq!(error.code, "cad.bodies_not_separated");
}

#[test]
fn stl_selected_units_are_applied_once() {
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    let result = convert_document(
        include_bytes!("../../../fixtures/cube.stl"),
        InputFormat::Stl,
        &options,
    )
    .unwrap();
    let bounds = result.report.cad.bounds_mm;
    close(bounds[1][0] - bounds[0][0], 25.4);
    assert!(matches!(
        result.report.source.unit_origin,
        UnitOrigin::Selected
    ));
}

#[test]
fn tolerance_snapshot_is_independent_of_later_options_and_stl_unit_preference() {
    let mut options = Options {
        input_unit: InputUnit::Inch,
        plane_tolerance_mm: 1e-7,
        ..Options::default()
    };
    let document =
        analyze_document(&fixture("m3_single_cube"), InputFormat::ThreeMf, &options).unwrap();
    options.plane_tolerance_mm = 0.01;
    assert_ne!(
        document.settings.plane_tolerance_mm,
        options.plane_tolerance_mm
    );
    assert_eq!(document.source.unit, "millimeter");
    assert_eq!(document.settings.plane_tolerance_mm, 1e-7);
    assert_eq!(
        document
            .diagnostics
            .iter()
            .filter(|d| d.code == "precision.custom_tolerances")
            .count(),
        1
    );
    let snapshot = serde_json::to_value(&document.settings).unwrap();
    assert!(snapshot.get("input_unit").is_none());
    // Les seuils cylindriques inactifs ne signalent pas un changement en Plans.
    let inactive = Options {
        cylinder_fit_tolerance_mm: 0.001,
        ..Options::default()
    };
    let default =
        analyze_document(&fixture("m3_single_cube"), InputFormat::ThreeMf, &inactive).unwrap();
    assert!(!default
        .diagnostics
        .iter()
        .any(|d| d.code == "precision.custom_tolerances"));
}
