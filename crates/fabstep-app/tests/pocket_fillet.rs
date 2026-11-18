use fabstep_app::{
    convert_document_with_provenance, DocumentStep, InputFormat, ProvenancedDocumentConversion,
};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Mesh, Options, ProfileSegment, ReconstructionMode};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, PI};
use std::fs;
use std::path::Path;

type Point = [f64; 3];

#[derive(Deserialize)]
struct Pocket {
    groups: BTreeMap<String, [usize; 2]>,
    bounds_mm: [Point; 2],
    axis_centers_mm: [Point; 2],
    radius_mm: f64,
    volume_mm3: f64,
    inside_void_mm: Point,
    inside_material_mm: Point,
    triangles: usize,
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/pocket_fillet")
            .join(name),
    )
    .unwrap()
}

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(v: Point) -> f64 {
    dot(v, v).sqrt()
}

// Solid angle of the oriented STEP tessellation at a material or cavity witness.
fn winding(mesh: &Mesh, point: Point) -> f64 {
    mesh.triangles
        .iter()
        .map(|tri| {
            let [a, b, c] = tri.map(|i| sub(mesh.vertices[i], point));
            let denominator = norm(a) * norm(b) * norm(c)
                + dot(a, b) * norm(c)
                + dot(b, c) * norm(a)
                + dot(c, a) * norm(b);
            2. * dot(a, cross(b, c)).atan2(denominator)
        })
        .sum::<f64>()
        / (4. * PI)
}

fn check_stopping_arc(
    step: &DocumentStep,
    wall_id: usize,
    expected: Point,
    radius: f64,
    tangents: [Point; 2],
) {
    let profile = step.planar_face_profile(wall_id).unwrap();
    assert_eq!(profile.loops.len(), 1);
    let mut arcs = profile.loops[0]
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ProfileSegment::Arc {
                center_mm,
                radius_mm,
                sweep_angle_rad,
                start_angle_rad,
            } => Some((center_mm, radius_mm, start_angle_rad, sweep_angle_rad)),
            ProfileSegment::Line { .. } => None,
            other => panic!("unexpected end-wall trim: {other:?}"),
        });
    let (center, r, start, sweep) = arcs.next().expect("missing circular end of the fillet");
    assert!(arcs.next().is_none());
    let frame = profile.frame;
    let world = std::array::from_fn(|i| {
        frame.origin_mm[i] + center[0] * frame.x_axis[i] + center[1] * frame.y_axis[i]
    });
    assert!(
        norm(sub(world, expected)) < 1e-4,
        "{world:?} / {expected:?}"
    );
    assert!((r - radius).abs() < 1e-4 && (sweep.abs() - FRAC_PI_2).abs() < 1e-5);
    // At the floor and the side wall, the inward cylindrical normal must
    // agree with the adjacent plane normal. Both arc ends are checked.
    let inward = [*start, start + sweep]
        .map(|a| std::array::from_fn(|i| -(a.cos() * frame.x_axis[i] + a.sin() * frame.y_axis[i])));
    for tangent in tangents {
        assert!(
            inward
                .iter()
                .any(|&direction| norm(sub(direction, tangent)) < 1e-5),
            "end arc is not tangent to its neighboring plane"
        );
    }
}

#[test]
fn auto_reimports_one_concave_fillet_stopped_by_the_pocket_end_walls() {
    let manifest: serde_json::Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for name in manifest["positives"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        let expected: Pocket = serde_json::from_value(manifest["cases"][name].clone()).unwrap();
        let options = options();
        let document = convert_document_with_provenance(
            &fixture(&format!("{name}.stl")),
            InputFormat::Stl,
            &options,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let cad = &document.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces
            ),
            (1, 12, 11, 1),
            "{name}"
        );
        assert!(
            (cad.volume_mm3 - expected.volume_mm3).abs() < 1e-6 * expected.volume_mm3,
            "{name}: volume {}",
            cad.volume_mm3
        );
        for (actual, nominal) in cad.bounds_mm.into_iter().zip(expected.bounds_mm) {
            assert!(
                norm(sub(actual, nominal)) < 1e-4,
                "{name}: {actual:?} / {nominal:?}"
            );
        }
        let body = &document.report.bodies[0];
        let fidelity = body.conversion.fidelity.as_ref().unwrap();
        assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
        assert!(
            fidelity.source_to_result_mm <= options.max_deviation_mm
                && fidelity.result_to_source_mm <= options.max_deviation_mm
        );
        let inspected = document.step.result_faces().unwrap();
        let correspondence = &inspected.source_correspondence;
        assert_eq!(correspondence.status, "exact");
        assert_eq!(correspondence.faces.len(), 12);
        let mut face_ids = BTreeMap::new();
        let mut covered = BTreeSet::new();
        for (group, &[start, end]) in &expected.groups {
            let row = correspondence
                .faces
                .iter()
                .find(|r| r.source_triangle_ids.iter().copied().eq(start..end))
                .unwrap_or_else(|| panic!("{name}: no STEP face matches {group} ({start}..{end})"));
            assert_eq!(row.analysis_body_id, body.id);
            assert!(row.source_triangle_ids.iter().all(|&i| covered.insert(i)));
            face_ids.insert(group.as_str(), row.face_id);
            let family = if group == "fillet" {
                ResultFaceFamily::Cylinder
            } else {
                ResultFaceFamily::Plane
            };
            assert_eq!(
                inspected.result.faces[row.face_id].family, family,
                "{name}: {group}"
            );
        }
        assert!(covered.into_iter().eq(0..expected.triangles));
        let fillet = &inspected.result.faces[face_ids["fillet"]];
        let neighbors =
            ["floor", "tangent_wall", "stop_min", "stop_max"].map(|name| face_ids[name]);
        assert_eq!(
            fillet
                .adjacent_face_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(neighbors)
        );
        let tangents = ["floor", "tangent_wall"].map(|name| {
            document
                .step
                .planar_face_frame(face_ids[name])
                .unwrap()
                .normal
        });
        for (name, center) in ["stop_min", "stop_max"]
            .into_iter()
            .zip(expected.axis_centers_mm)
        {
            check_stopping_arc(
                &document.step,
                face_ids[name],
                center,
                expected.radius_mm,
                tangents,
            );
        }
        let mesh = &inspected.result.mesh;
        let [near, far] = expected.axis_centers_mm;
        let axis = sub(far, near);
        let axis = axis.map(|x| x / norm(axis));
        assert!(fillet.triangle_count > 0);
        for tri in
            &mesh.triangles[fillet.triangle_start..fillet.triangle_start + fillet.triangle_count]
        {
            let [p, q, r] = tri.map(|i| mesh.vertices[i]);
            let radial = sub(p, near);
            let radial = sub(radial, axis.map(|x| x * dot(radial, axis)));
            assert!(
                dot(cross(sub(q, p), sub(r, p)), radial) < 0.,
                "{name}: reversed fillet"
            );
        }
        assert!(winding(mesh, expected.inside_void_mm).abs() < 1e-5);
        assert!((winding(mesh, expected.inside_material_mm) - 1.).abs() < 1e-5);
    }
}

fn check_planar_fallback(document: &ProvenancedDocumentConversion) {
    let cad = &document.report.cad;
    assert_eq!(cad.solids, 1);
    assert_eq!(cad.planar_faces, cad.faces);
    let source = &document.report.bodies[0].conversion.analysis;
    assert!((cad.volume_mm3 - source.report.volume_mm3).abs() < 1e-6 * cad.volume_mm3);
    let native = document.step.result_faces().unwrap();
    let audit = fabstep_verify::compare_meshes(
        &source.mesh,
        &native.result.mesh,
        native.result.deflection_mm,
    )
    .unwrap();
    assert!(audit.source_to_result_mm < 1e-5 && audit.result_to_source_mm < 1e-5);
}

#[test]
fn oblique_binary_roundoff_keeps_a_faithful_faceted_solid() {
    let doc =
        convert_document_with_provenance(&fixture("rotated_f32.stl"), InputFormat::Stl, &options())
            .unwrap();
    check_planar_fallback(&doc);
}

#[test]
fn open_floor_and_inconsistent_fillet_evidence_do_not_create_a_false_cylinder() {
    let options = options();
    let error =
        convert_document_with_provenance(&fixture("missing_floor.stl"), InputFormat::Stl, &options)
            .err()
            .unwrap();
    assert_eq!(error.code, "mesh.open");
    for name in ["warped.stl", "warped_stop.stl"] {
        let document = convert_document_with_provenance(&fixture(name), InputFormat::Stl, &options)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        check_planar_fallback(&document);
    }
}
