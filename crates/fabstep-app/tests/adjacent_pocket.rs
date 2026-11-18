use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{export_planar_face_dxf, Options, ProfileSegment, ReconstructionMode};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::f64::consts::FRAC_PI_2;
use std::{fs, path::Path};

fn options(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

#[derive(Deserialize)]
struct Case {
    triangles: usize,
    groups: BTreeMap<String, [usize; 2]>,
    volume_mm3: f64,
    bounds_mm: [[f64; 3]; 2],
    radius_mm: f64,
}

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/adjacent_pocket")
            .join(file),
    )
    .unwrap()
}

#[test]
fn two_analytic_faces_meet_and_keep_their_own_triangles() {
    let manifest: serde_json::Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for case in manifest["positives"].as_array().unwrap() {
        let name = case.as_str().unwrap();
        let expected: Case = serde_json::from_value(manifest["cases"][name].clone()).unwrap();
        let bytes = fixture(&format!("{name}.stl"));
        for mode in [ReconstructionMode::Surfaces, ReconstructionMode::Auto] {
            let o = options(mode);
            let doc = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let cad = &doc.report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.cylindrical_faces
                ),
                (1, 13, 11, 2),
                "{name}/{mode:?}"
            );
            assert!((cad.volume_mm3 - expected.volume_mm3).abs() < 0.0003);
            for (a, b) in cad
                .bounds_mm
                .iter()
                .flatten()
                .zip(expected.bounds_mm.iter().flatten())
            {
                assert!((a - b).abs() < 1e-5);
            }
            let conversion = &doc.report.bodies[0].conversion;
            let result = conversion.rounded_recognition.as_ref().unwrap();
            let mixed = &result.model.revolved.mixed;
            assert_eq!(mixed.arcs.len(), 2);
            assert_eq!(mixed.elliptic_arcs.len(), 1);
            assert_eq!(mixed.partial_cylinders.len(), 2);
            assert!(mixed
                .partial_cylinders
                .iter()
                .all(|w| w.elliptic_end && w.inner && w.arcs[1] == 0));
            assert_ne!(
                mixed.partial_cylinders[0].arcs[0],
                mixed.partial_cylinders[1].arcs[0]
            );
            assert!((result.report.expected_volume_mm3 - expected.volume_mm3).abs() < 0.0003);
            assert!(result.report.max_deviation_bound_mm <= o.max_deviation_mm);
            let audit = conversion.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(
                audit.source_to_result_mm <= o.max_deviation_mm
                    && audit.result_to_source_mm <= o.max_deviation_mm
            );
            if mode != ReconstructionMode::Auto {
                continue;
            }
            let faces = doc.step.result_faces().unwrap();
            assert_eq!(faces.source_correspondence.status, "exact");
            let mut covered = Vec::new();
            let mut ids = BTreeMap::new();
            for (group, &[a, b]) in &expected.groups {
                let face = faces
                    .source_correspondence
                    .faces
                    .iter()
                    .find(|r| r.source_triangle_ids.iter().copied().eq(a..b))
                    .unwrap_or_else(|| panic!("{name}: lost {group}"));
                covered.extend_from_slice(&face.source_triangle_ids);
                ids.insert(group.as_str(), face.face_id);
            }
            covered.sort_unstable();
            assert!(covered.into_iter().eq(0..expected.triangles));
            for (cyl, other, side, stop) in [
                ("fillet_x", "fillet_y", "wall_y_min", "stop_x_max"),
                ("fillet_y", "fillet_x", "wall_x_min", "stop_y_max"),
            ] {
                let face = &faces.result.faces[ids[cyl]];
                assert_eq!(face.family, ResultFaceFamily::Cylinder);
                let mut neighbors = face.adjacent_face_ids.clone();
                neighbors.sort_unstable();
                let mut expected_neighbors = [ids[other], ids["floor"], ids[side], ids[stop]];
                expected_neighbors.sort_unstable();
                assert_eq!(neighbors, expected_neighbors);
                let profile = doc.step.planar_face_profile(ids[stop]).unwrap();
                assert_eq!(profile.loops.len(), 1);
                assert_eq!(profile.loops[0].segments.iter().filter(|s| matches!(s,
                    ProfileSegment::Arc { radius_mm, sweep_angle_rad,.. } if (*radius_mm-expected.radius_mm).abs()<1e-5 && (sweep_angle_rad.abs()-FRAC_PI_2).abs()<1e-5)).count(),1);
                let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
                assert_eq!(dxf.matches("\n0\nARC\n").count(), 1);
            }
        }
    }
}

#[test]
fn two_different_pocket_domains_remain_separate_on_reimport() {
    let adjacent = fixture("nominal.stl");
    let oblique = include_bytes!("../../../fixtures/oblique_pocket/nominal.stl");
    let sources: [&[u8]; 2] = [&adjacent, oblique];
    let counts = sources.map(|b| u32::from_le_bytes(b[80..84].try_into().unwrap()));
    let mut combined = adjacent[..80].to_vec();
    combined.extend((counts[0] + counts[1]).to_le_bytes());
    let mut positions = [Vec::new(), Vec::new()];
    for i in 0..*counts.iter().max().unwrap() as usize {
        for (body, source) in sources.iter().enumerate() {
            if i >= counts[body] as usize {
                continue;
            }
            let mut tri = source[84 + i * 50..84 + (i + 1) * 50].to_vec();
            if body == 1 {
                for j in [20, 32, 44] {
                    let z = f32::from_le_bytes(tri[j..j + 4].try_into().unwrap());
                    tri[j..j + 4].copy_from_slice(&(z + 16.).to_le_bytes());
                }
            }
            positions[body].push((combined.len() - 84) / 50);
            combined.extend(tri);
        }
    }
    let doc = convert_document_with_provenance(
        &combined,
        InputFormat::Stl,
        &options(ReconstructionMode::Auto),
    )
    .unwrap();
    let cad = &doc.report.cad;
    assert_eq!(
        (cad.solids, cad.planar_faces, cad.cylindrical_faces),
        (2, 22, 3)
    );
    assert!((cad.volume_mm3 - 11553.331868144).abs() < 0.0005);
    let faces = doc.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let mut actual: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .filter(|r| faces.result.faces[r.face_id].family == ResultFaceFamily::Cylinder)
        .map(|r| {
            let f = &faces.result.faces[r.face_id];
            assert!(f
                .adjacent_face_ids
                .iter()
                .all(|&j| faces.result.faces[j].solid_id == f.solid_id));
            r.source_triangle_ids.clone()
        })
        .collect();
    let mut expected = [
        positions[0][40..104].to_vec(),
        positions[0][240..304].to_vec(),
        positions[1][34..98].to_vec(),
    ];
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn a_displaced_shared_knot_is_not_turned_into_a_cylinder_intersection() {
    let o = options(ReconstructionMode::Auto);
    let doc = convert_document_with_provenance(&fixture("warped_seam.stl"), InputFormat::Stl, &o)
        .unwrap();
    let cad = &doc.report.cad;
    assert_eq!(cad.solids, 1);
    assert_eq!(cad.faces, cad.planar_faces);
    let source = &doc.report.bodies[0].conversion.analysis;
    assert!((cad.volume_mm3 - source.report.volume_mm3).abs() < 1e-5);
    let faces = doc.step.result_faces().unwrap();
    let audit = fabstep_verify::compare_meshes(
        &source.mesh,
        &faces.result.mesh,
        faces.result.deflection_mm,
    )
    .unwrap();
    assert!(audit.source_to_result_mm < 1e-5 && audit.result_to_source_mm < 1e-5);
}
