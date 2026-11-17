use fabstep_app::{convert_document_with_provenance, convert_stl, InputFormat};
use fabstep_cad::{tessellate_step, ResultFaceFamily};
use fabstep_core::{Mesh, Options, ReconstructionMode, RoundedFaceRef};
use serde_json::Value;
use std::{collections::BTreeSet, f64::consts::TAU, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/pierced_tray")
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

fn transformed(p: [f64; 3], rotated: bool) -> [f64; 3] {
    if rotated {
        [p[1] + 10., p[2] - 4., p[0] + 5.]
    } else {
        p
    }
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
fn pierced_tray_keeps_floor_void_and_all_native_faces() {
    let reference = manifest();
    for (name, rotated) in [("nominal", false), ("rotated", true)] {
        let options = Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        };
        let converted = convert_document_with_provenance(
            &fixture(&format!("{name}.stl")),
            InputFormat::Stl,
            &options,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let report = &converted.report.bodies[0].conversion;
        assert_eq!(report.cad.solids, 1);
        assert_eq!(report.cad.planar_faces, 11);
        assert_eq!(report.cad.cylindrical_faces, 9);
        assert_eq!(report.cad.faces, 20);
        assert!((report.cad.volume_mm3 - reference["volume_mm3"].as_f64().unwrap()).abs() < 1e-6);
        for (end, p) in [[-20., -15., 0.], [20., 15., 12.]].into_iter().enumerate() {
            let p = transformed(p, rotated);
            for (axis, expected) in p.into_iter().enumerate() {
                assert!((report.cad.bounds_mm[end][axis] - expected).abs() < 2e-5);
            }
        }
        let proof = report.rounded_recognition.as_ref().unwrap();
        let model = &proof.model.revolved.mixed;
        assert_eq!(model.partial_cylinders.len(), 8);
        assert_eq!(model.cylinders.len(), 1);
        assert!(model.cylinders[0].inner);
        let centers: Vec<_> = model.cylinders[0]
            .rings
            .iter()
            .map(|&i| {
                let ring = &model.rings[i];
                assert!((ring.radius_mm - 2.).abs() < 1e-8);
                ring.center
            })
            .collect();
        for z in [0., 3.] {
            let expected = transformed([0., 0., z], rotated);
            assert!(centers
                .iter()
                .any(|c| c.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-8)));
        }
        assert_eq!(
            proof.triangle_faces.len(),
            report.analysis.mesh.triangles.len()
        );
        assert!(proof
            .triangle_faces
            .iter()
            .any(|f| matches!(f, RoundedFaceRef::Cylinder { index: 0 })));
        let inspected = converted.step.result_faces().unwrap();
        assert_eq!(inspected.source_correspondence.status, "exact");
        assert_eq!(
            inspected
                .result
                .faces
                .iter()
                .filter(|f| f.family == ResultFaceFamily::Cylinder)
                .count(),
            9
        );
        assert_eq!(inspected.source_correspondence.faces.len(), 20);
        let mut ids = BTreeSet::new();
        for row in &inspected.source_correspondence.faces {
            assert!(!row.source_triangle_ids.is_empty());
            for &id in &row.source_triangle_ids {
                assert!(ids.insert(id));
            }
        }
        assert_eq!(ids.len(), report.analysis.mesh.triangles.len());
        let mesh = tessellate_step(converted.step.bytes(), 0.01).unwrap();
        for (point, material) in [
            ([0.1, 0.2, 1.6], false),
            ([5., 0.2, 1.6], true),
            ([5., 0.2, 7.6], false),
            ([18.6, 0.2, 7.6], true),
        ] {
            for source in [&report.analysis.mesh, &mesh] {
                let expected = if material { 1. } else { 0. };
                assert!(
                    (winding_number(source, transformed(point, rotated)).abs() - expected).abs()
                        < 1e-6,
                    "{name}: {point:?}"
                );
            }
        }
        let audit = report.fidelity.as_ref().unwrap();
        assert!(audit.source_to_result_mm < options.max_deviation_mm);
        assert!(audit.result_to_source_mm < options.max_deviation_mm);
    }
}

#[test]
fn missing_floor_refuses_and_deformed_hole_is_not_a_false_cylinder() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    assert_eq!(
        convert_stl(&fixture("missing_floor.stl"), &options)
            .err()
            .unwrap()
            .code,
        "mesh.open"
    );
    let warped = convert_stl(&fixture("warped_bore.stl"), &options).unwrap();
    assert_eq!(warped.report.cad.solids, 1);
    let model = &warped
        .report
        .rounded_recognition
        .as_ref()
        .unwrap()
        .model
        .revolved
        .mixed;
    assert!(model.cylinders.is_empty());
    assert_eq!(model.partial_cylinders.len(), 8);
}
