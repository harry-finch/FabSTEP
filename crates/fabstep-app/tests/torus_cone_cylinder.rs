use fabstep_app::{
    convert_document_with_provenance, convert_stl, selection_topology, ConversionReport,
    DocumentResultFaces, InputFormat,
};
use fabstep_cad::{CadReport, ResultFaceFamily};
use fabstep_core::{GuidanceFamily, Options, ReconstructionMode};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/torus_cone_cylinder")
            .join(file),
    )
    .unwrap()
}

fn manifest() -> Value {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

fn assert_families(cad: &CadReport, bodies: usize) {
    assert_eq!(cad.solids, bodies);
    assert_eq!(cad.faces, 5 * bodies);
    assert_eq!(cad.planar_faces, 2 * bodies);
    assert_eq!(cad.cylindrical_faces, bodies);
    assert_eq!(cad.conical_faces, bodies);
    assert_eq!(cad.toroidal_faces, bodies);
    assert_eq!(cad.spherical_faces, 0);
}

fn assert_bounds(cad: &CadReport, bounds: &Value) {
    for (end, corner) in cad.bounds_mm.iter().enumerate() {
        for (axis, coordinate) in corner.iter().enumerate() {
            let expected = bounds[end][axis].as_f64().unwrap();
            assert!(
                (coordinate - expected).abs() < 2e-5,
                "{:?} != {bounds}",
                cad.bounds_mm
            );
        }
    }
}

fn assert_body(report: &ConversionReport, o: &Options, triangles: usize, expected_volume: f64) {
    assert_families(&report.cad, 1);
    assert_analytic_audit(report, o, triangles, expected_volume);
}

fn assert_analytic_audit(
    report: &ConversionReport,
    o: &Options,
    triangles: usize,
    expected_volume: f64,
) {
    assert!((report.cad.volume_mm3 - expected_volume).abs() < 0.005);
    assert_eq!(report.analysis.mesh.triangles.len(), triangles);
    let recognition = report.rounded_recognition.as_ref().unwrap();
    assert_eq!(recognition.triangle_faces.len(), triangles);
    assert_eq!(
        recognition.model.revolved.mixed.vertices,
        report.analysis.mesh.vertices
    );
    assert!(recognition.report.max_deviation_bound_mm <= o.max_deviation_mm);
    let fidelity = report.fidelity.as_ref().unwrap();
    assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
    assert!(fidelity.source_to_result_mm <= o.max_deviation_mm);
    assert!(fidelity.result_to_source_mm <= o.max_deviation_mm);
}

#[test]
fn both_modes_reimport_complete_solids_with_analytic_faces_and_two_way_fidelity() {
    let m = manifest();
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = options(mode);
        for name in [
            "nominal",
            "mirrored",
            "rotated",
            "opposite_branch",
            "seam_noise",
        ] {
            let expected = &m["cases"][name];
            let converted = convert_stl(&fixture(&format!("{name}.stl")), &o)
                .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
            assert_body(
                &converted.report,
                &o,
                expected["triangles"].as_u64().unwrap() as usize,
                expected["volume_mm3"].as_f64().unwrap(),
            );
            assert_bounds(&converted.report.cad, &expected["bounds_mm"]);
            let step = std::str::from_utf8(&converted.step_bytes).unwrap();
            for entity in [
                "TOROIDAL_SURFACE(",
                "CONICAL_SURFACE(",
                "CYLINDRICAL_SURFACE(",
            ] {
                assert_eq!(step.matches(entity).count(), 1, "{name}: {entity}");
            }
        }
    }
}

fn assert_provenance(
    bytes: &[u8],
    format: InputFormat,
    faces: &DocumentResultFaces,
    o: &Options,
    plane_triangles: usize,
) {
    let topology = selection_topology(bytes, format, o).unwrap();
    let expected: BTreeSet<_> = topology
        .components
        .iter()
        .flat_map(|body| {
            body.analysis_to_source_triangles
                .iter()
                .map(|&id| (body.analysis_body_id.as_str(), id))
        })
        .collect();
    let correspondence = &faces.source_correspondence;
    assert_eq!(correspondence.status, "exact");
    assert_eq!(correspondence.faces.len(), faces.result.faces.len());
    let mut actual = BTreeSet::new();
    for source in &correspondence.faces {
        let face = faces
            .result
            .faces
            .iter()
            .find(|f| f.id == source.face_id)
            .unwrap();
        let triangle_count = match face.family {
            ResultFaceFamily::Plane => plane_triangles,
            ResultFaceFamily::Torus => 1536,
            ResultFaceFamily::Cone | ResultFaceFamily::Cylinder => 128,
            unexpected => panic!("unexpected native surface {unexpected:?}"),
        };
        assert_eq!(source.source_triangle_ids.len(), triangle_count);
        for &id in &source.source_triangle_ids {
            assert!(actual.insert((source.analysis_body_id.as_str(), id)));
        }
    }
    assert_eq!(actual, expected);
}

#[test]
fn automatic_source_partition_survives_step_transfer_and_torus_guidance_keeps_its_scope() {
    let o = options(ReconstructionMode::Auto);
    let bytes = fixture("nominal.stl");
    let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o).unwrap();
    let faces = converted.step.result_faces().unwrap();
    assert_provenance(&bytes, InputFormat::Stl, &faces, &o, 64);
    let torus = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Torus)
        .unwrap();
    let cone = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Cone)
        .unwrap();
    let cylinder = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Cylinder)
        .unwrap();
    assert!(torus.adjacent_face_ids.contains(&cone.id));
    assert!(cone.adjacent_face_ids.contains(&cylinder.id));
    assert_eq!(
        converted
            .step
            .guided_surface(&[torus.id], GuidanceFamily::Torus)
            .err()
            .unwrap()
            .code,
        "guidance.family_unproven",
    );
}

#[test]
fn mirrored_3mf_occurrence_preserves_two_whole_solids_and_their_placements() {
    let m = manifest();
    let expected = &m["cases"]["instances"];
    let bytes = fixture("instances.3mf");
    let volume = expected["volume_mm3"].as_f64().unwrap();
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = options(mode);
        let converted = convert_document_with_provenance(&bytes, InputFormat::ThreeMf, &o).unwrap();
        assert_families(&converted.report.cad, 2);
        assert_eq!(converted.report.bodies.len(), 2);
        assert!((converted.report.cad.volume_mm3 - volume).abs() < 0.005);
        for (i, body) in converted.report.bodies.iter().enumerate() {
            assert_body(&body.conversion, &o, 1920, volume / 2.0);
            assert_bounds(&body.conversion.cad, &expected["body_bounds_mm"][i]);
        }
        if mode == ReconstructionMode::Auto {
            let faces = converted.step.result_faces().unwrap();
            assert_provenance(&bytes, InputFormat::ThreeMf, &faces, &o, 64);
        }
    }
}

#[test]
fn oblique_3mf_rotation_reimports_the_full_chain_at_its_placed_dimensions() {
    let m = manifest();
    let name = "rotated_oblique";
    let expected = &m["cases"][name];
    let bytes = fixture(&format!("{name}.3mf"));
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = options(mode);
        let converted = convert_document_with_provenance(&bytes, InputFormat::ThreeMf, &o)
            .unwrap_or_else(|e| panic!("{name} {mode:?}: {e}"));
        assert_families(&converted.report.cad, 1);
        let [body] = converted.report.bodies.as_slice() else {
            panic!("{name}: expected one body");
        };
        assert_body(
            &body.conversion,
            &o,
            expected["triangles"].as_u64().unwrap() as usize,
            expected["volume_mm3"].as_f64().unwrap(),
        );
        assert_bounds(&converted.report.cad, &expected["bounds_mm"]);
        if mode == ReconstructionMode::Auto {
            let faces = converted.step.result_faces().unwrap();
            assert_provenance(&bytes, InputFormat::ThreeMf, &faces, &o, 64);
        }
    }
}

#[test]
fn through_bore_reimports_with_the_cavity_orientation_and_complete_face_partition() {
    let m = manifest();
    let expected = &m["cases"]["bore"];
    let bytes = fixture("bore.stl");
    for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
        let o = options(mode);
        let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
            .unwrap_or_else(|e| panic!("bore {mode:?}: {e}"));
        let [body] = converted.report.bodies.as_slice() else {
            panic!("bore: expected one body");
        };
        let cad = &converted.report.cad;
        assert_eq!((cad.solids, cad.faces, cad.planar_faces), (1, 6, 2));
        assert_eq!(
            (
                cad.cylindrical_faces,
                cad.conical_faces,
                cad.toroidal_faces,
                cad.spherical_faces
            ),
            (2, 1, 1, 0),
        );
        assert_analytic_audit(
            &body.conversion,
            &o,
            expected["triangles"].as_u64().unwrap() as usize,
            expected["volume_mm3"].as_f64().unwrap(),
        );
        assert_bounds(cad, &expected["bounds_mm"]);
        let rounded = body.conversion.rounded_recognition.as_ref().unwrap();
        let revolved = &rounded.model.revolved;
        let [toroidal_bore] = rounded.model.tori.as_slice() else {
            panic!("bore: expected one torus");
        };
        assert!(!toroidal_bore.inner);
        let [conical_bore] = revolved.cones.as_slice() else {
            panic!("bore: expected one cone");
        };
        assert!(conical_bore.inner);
        assert_eq!(
            revolved
                .mixed
                .cylinders
                .iter()
                .filter(|face| face.inner)
                .count(),
            1,
        );
        if mode == ReconstructionMode::Auto {
            let faces = converted.step.result_faces().unwrap();
            assert_eq!(faces.result.faces.len(), 6);
            assert_provenance(&bytes, InputFormat::Stl, &faces, &o, 128);
        }
    }
}

#[test]
fn oblique_binary_stl_keeps_the_native_coplanarity_refusal() {
    let m = manifest();
    let name = "rotated_f32_nonplanar";
    let bytes = fixture(&format!("{name}.stl"));
    let o = options(ReconstructionMode::Surfaces);
    let error = convert_stl(&bytes, &o).err().unwrap();
    assert_eq!(
        error.code,
        m["limitations"][name]["surfaces_error"].as_str().unwrap(),
        "{name}: {error}"
    );
}

#[test]
fn no_surfaces_export_with_an_unproved_transition_or_an_insufficient_distance_budget() {
    let m = manifest();
    let o = options(ReconstructionMode::Surfaces);
    for name in m["negative_cases"].as_object().unwrap().keys() {
        let bytes = fixture(&format!("{name}.stl"));
        let error = convert_stl(&bytes, &o).err().unwrap();
        assert_eq!(
            error.code, "recognition.surface_unproven",
            "{name}: {error}"
        );
    }
    let strict = Options {
        max_deviation_mm: m["insufficient_deviation_mm"].as_f64().unwrap(),
        ..o
    };
    let error = convert_stl(&fixture("nominal.stl"), &strict).err().unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");
}
