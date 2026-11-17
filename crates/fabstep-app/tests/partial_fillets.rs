use fabstep_app::{convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_cad::{tessellate_step, ResultFaceFamily};
use fabstep_core::{Mesh, MixedFaceRef, Options, ReconstructionMode, RoundedFaceRef};
use serde_json::Value;
use std::{collections::BTreeSet, f64::consts::TAU, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/partial_fillets")
            .join(name),
    )
    .unwrap()
}

fn manifest() -> Value {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn transformed(mut p: [f64; 3], rotated: bool) -> [f64; 3] {
    if rotated {
        let (sx, cx) = 0.37_f64.sin_cos();
        let (sy, cy) = (-0.51_f64).sin_cos();
        let y = cx * p[1] - sx * p[2];
        let z = sx * p[1] + cx * p[2];
        p = [cy * p[0] + sy * z + 4., y + 8., -sy * p[0] + cy * z + 12.];
    }
    p
}

fn winding_number(mesh: &Mesh, point: [f64; 3]) -> f64 {
    // Independent material/void probes, far from boundaries. The native STEP
    // tessellation is used only for these probes, not as a certified distance.
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] =
                triangle.map(|index| std::array::from_fn(|i| mesh.vertices[index][i] - point[i]));
            let [la, lb, lc] = [a, b, c].map(|v| dot(v, v).sqrt());
            2. * dot(a, cross(b, c))
                .atan2(la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb)
        })
        .sum::<f64>()
        / (2. * TAU)
}

#[test]
fn partial_rounds_and_trays_reimport_with_nominal_faces_volume_and_provenance() {
    for (name, expected) in manifest()["cases"].as_object().unwrap() {
        if expected.get("expected_refusal").is_some() {
            continue;
        }
        for mode in [ReconstructionMode::Cylinders, ReconstructionMode::Auto] {
            let options = Options {
                reconstruction: mode,
                ..Options::default()
            };
            let conversion = convert_stl(&fixture(&format!("{name}.stl")), &options)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let report = &conversion.report;
            let planes = expected["planes"].as_u64().unwrap() as usize;
            let cylinders = expected["partial_cylinders"].as_u64().unwrap() as usize;
            assert_eq!(report.cad.solids, 1, "{name}/{mode:?}");
            assert_eq!(report.cad.planar_faces, planes, "{name}/{mode:?}");
            assert_eq!(report.cad.cylindrical_faces, cylinders, "{name}/{mode:?}");
            assert_eq!(report.cad.faces, planes + cylinders, "{name}/{mode:?}");
            assert_eq!(
                report.cad.conical_faces + report.cad.spherical_faces + report.cad.toroidal_faces,
                0
            );
            assert!(
                (report.cad.volume_mm3 - expected["volume_mm3"].as_f64().unwrap()).abs() < 1e-6,
                "{name}/{mode:?}: volume {}",
                report.cad.volume_mm3
            );
            for end in 0..2 {
                for axis in 0..3 {
                    assert!(
                        (report.cad.bounds_mm[end][axis]
                            - expected["bounds_mm"][end][axis].as_f64().unwrap())
                        .abs()
                            < 2e-5,
                        "{name}/{mode:?}: nominal analytic bounds {:?}",
                        report.cad.bounds_mm
                    );
                }
            }
            let triangle_count = report.analysis.mesh.triangles.len();
            let mut owner_counts = vec![0; cylinders];
            let model = if mode == ReconstructionMode::Auto {
                let proof = report
                    .rounded_recognition
                    .as_ref()
                    .expect("Auto patch proof");
                assert_eq!(proof.triangle_faces.len(), triangle_count);
                for face in &proof.triangle_faces {
                    if let RoundedFaceRef::PartialCylinder { index } = *face {
                        owner_counts[index] += 1;
                    }
                }
                &proof.model.revolved.mixed
            } else {
                let proof = report
                    .mixed_recognition
                    .as_ref()
                    .expect("Mixed patch proof");
                assert_eq!(proof.triangle_faces.len(), triangle_count);
                for face in &proof.triangle_faces {
                    if let MixedFaceRef::PartialCylinder { index } = *face {
                        owner_counts[index] += 1;
                    }
                }
                &proof.model
            };
            assert!(
                owner_counts.into_iter().all(|count| count > 0),
                "{name}/{mode:?}"
            );
            assert_eq!(model.vertices, report.analysis.mesh.vertices);
            assert_eq!(model.partial_cylinders.len(), cylinders);
            assert_eq!(
                model.partial_cylinders.iter().filter(|c| c.inner).count(),
                expected["inner_cylinders"].as_u64().unwrap() as usize
            );
            let fidelity = report
                .fidelity
                .as_ref()
                .expect("native bidirectional audit");
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(
                fidelity.source_to_result_mm < options.max_deviation_mm,
                "{name}/{mode:?}"
            );
            assert!(
                fidelity.result_to_source_mm < options.max_deviation_mm,
                "{name}/{mode:?}"
            );
            assert_eq!(report.settings.reconstruction, mode);
            assert_eq!(
                report.settings.cylinder_fit_tolerance_mm,
                options.cylinder_fit_tolerance_mm
            );
            assert_eq!(
                std::str::from_utf8(&conversion.step_bytes)
                    .unwrap()
                    .matches("CYLINDRICAL_SURFACE(")
                    .count(),
                cylinders
            );
            if let Some(probes) = expected["probes"].as_array() {
                let result_mesh = tessellate_step(&conversion.step_bytes, 0.01).unwrap();
                for probe in probes {
                    let point = std::array::from_fn(|i| probe["point_mm"][i].as_f64().unwrap());
                    let point = transformed(point, expected["rotated"].as_bool().unwrap());
                    let material = probe["material"].as_bool().unwrap();
                    for (label, mesh) in [
                        ("source", &report.analysis.mesh),
                        ("reimported STEP", &result_mesh),
                    ] {
                        let winding = winding_number(mesh, point).abs();
                        let nominal = if material { 1. } else { 0. };
                        assert!(
                            (winding - nominal).abs() < 1e-6,
                            "{name}/{mode:?}/{label}: {} winding={winding}",
                            probe["role"]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn missing_material_boundaries_and_strict_rounding_budgets_never_export() {
    for mode in [ReconstructionMode::Cylinders, ReconstructionMode::Auto] {
        let options = Options {
            reconstruction: mode,
            ..Options::default()
        };
        for name in ["one_edge_missing_end", "tray_missing_floor"] {
            let error = convert_stl(&fixture(&format!("{name}.stl")), &options)
                .err()
                .unwrap();
            assert_eq!(error.code, "mesh.open", "{name}/{mode:?}: {error}");
        }
    }
    let error = convert_stl(
        &fixture("one_edge.stl"),
        &Options {
            reconstruction: ReconstructionMode::Cylinders,
            max_deviation_mm: 0.0001,
            ..Options::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "recognition.deviation_exceeded");
}

#[test]
fn closed_variable_sections_are_refused_or_kept_faceted_without_false_cylinders() {
    let bytes = fixture("one_edge_variable_radius.stl");
    let options = Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    };
    assert_eq!(
        convert_stl(&bytes, &options).err().unwrap().code,
        "recognition.partial_unproven"
    );
    let result = convert_stl(
        &bytes,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..options
        },
    )
    .unwrap();
    assert_eq!(result.report.cad.solids, 1);
    assert_eq!(result.report.cad.cylindrical_faces, 0);
    assert_eq!(result.report.cad.faces, result.report.cad.planar_faces);
    let nominal = manifest()["cases"]["one_edge_variable_radius"]["source_polyhedral_volume_mm3"]
        .as_f64()
        .unwrap();
    assert!((result.report.cad.volume_mm3 - nominal).abs() < 1e-6);
}

#[test]
fn automatic_native_faces_keep_exact_exhaustive_source_correspondence() {
    for (name, cylinders) in [("one_edge", 1), ("open_tray", 8)] {
        let converted = convert_document_with_provenance(
            &fixture(&format!("{name}.stl")),
            InputFormat::Stl,
            &Options {
                reconstruction: ReconstructionMode::Auto,
                ..Options::default()
            },
        )
        .unwrap();
        let inspected = converted.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact", "{name}");
        assert_eq!(
            inspected.source_correspondence.faces.len(),
            inspected.result.faces.len()
        );
        assert_eq!(
            inspected
                .result
                .faces
                .iter()
                .filter(|face| face.family == ResultFaceFamily::Cylinder)
                .count(),
            cylinders,
            "{name}"
        );
        let mut source_triangles = BTreeSet::new();
        for row in &inspected.source_correspondence.faces {
            assert!(!row.source_triangle_ids.is_empty(), "{name}");
            for &triangle in &row.source_triangle_ids {
                assert!(
                    source_triangles.insert(triangle),
                    "{name}: shared source owner"
                );
            }
        }
        assert_eq!(
            source_triangles.len(),
            converted.report.bodies[0]
                .conversion
                .analysis
                .mesh
                .triangles
                .len(),
            "{name}"
        );
    }
}
