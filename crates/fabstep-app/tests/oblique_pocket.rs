use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn a_deformed_oblique_end_wall_keeps_a_faithful_faceted_pocket() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/oblique_pocket/warped_stop.stl"),
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        },
    )
    .unwrap();
    let cad = &document.report.cad;
    assert_eq!(cad.solids, 1);
    assert_eq!(cad.faces, cad.planar_faces);
    let faces = document.step.result_faces().unwrap();
    let source = &document.report.bodies[0].conversion.analysis;
    assert!((cad.volume_mm3 - source.report.volume_mm3).abs() < 1e-5);
    assert_eq!(faces.source_correspondence.status, "exact");
    let mut triangles: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .flat_map(|face| face.source_triangle_ids.iter().copied())
        .collect();
    triangles.sort_unstable();
    assert!(triangles.into_iter().eq(0..source.mesh.triangles.len()));
    let comparison = fabstep_verify::compare_meshes(
        &source.mesh,
        &faces.result.mesh,
        faces.result.deflection_mm,
    )
    .unwrap();
    assert!(comparison.source_to_result_mm < 1e-5 && comparison.result_to_source_mm < 1e-5);
}

#[derive(serde::Deserialize)]
struct Pocket {
    groups: std::collections::BTreeMap<String, [usize; 2]>,
    volume_mm3: f64,
    radius_mm: f64,
    ellipse_major_radius_mm: f64,
    ellipse_center_mm: [f64; 3],
    ellipse_midpoint_mm: [f64; 3],
    bounds_mm: [[f64; 3]; 2],
}

#[test]
fn reconstructed_oblique_wall_has_exact_shared_trims_and_exhaustive_source_faces() {
    use fabstep_cad::ResultFaceFamily;
    use fabstep_core::{export_planar_face_dxf, ProfileSegment};
    use std::{collections::BTreeMap, f64::consts::FRAC_PI_2, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/oblique_pocket");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in manifest["positives"].as_array().unwrap() {
        let name = case.as_str().unwrap();
        let expected: Pocket = serde_json::from_value(manifest["cases"][name].clone()).unwrap();
        let bytes = std::fs::read(root.join(format!("{name}.stl"))).unwrap();
        for reconstruction in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = Options {
                reconstruction,
                ..Options::default()
            };
            let doc = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{reconstruction:?}: {e}"));
            let cad = &doc.report.cad;
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
                (cad.volume_mm3 - expected.volume_mm3).abs() < 0.0003,
                "{name}: {cad:?}"
            );
            for (got, want) in cad
                .bounds_mm
                .iter()
                .flatten()
                .zip(expected.bounds_mm.iter().flatten())
            {
                assert!((got - want).abs() < 1e-5);
            }
            let conversion = &doc.report.bodies[0].conversion;
            let proof = &conversion.rounded_recognition.as_ref().unwrap().report;
            assert!((proof.expected_volume_mm3 - cad.volume_mm3).abs() < 0.0003);
            assert!(proof.max_deviation_bound_mm <= o.max_deviation_mm);
            let audit = conversion.fidelity.as_ref().unwrap();
            assert!(
                audit.source_to_result_mm <= o.max_deviation_mm
                    && audit.result_to_source_mm <= o.max_deviation_mm
            );
            let faces = doc.step.result_faces().unwrap();
            if reconstruction == ReconstructionMode::Surfaces {
                // Manual Surfaces does not publish a native label sidecar.
                // Source ownership is still available in its recognition report.
                let report = conversion.rounded_recognition.as_ref().unwrap();
                let [start, end] = expected.groups["fillet"];
                assert!(report
                    .triangle_faces
                    .iter()
                    .enumerate()
                    .all(|(i, owner)| matches!(
                        owner,
                        fabstep_core::RoundedFaceRef::PartialCylinder { index: 0 }
                    ) == (start..end).contains(&i)));
                continue;
            }
            assert_eq!(
                faces.source_correspondence.status, "exact",
                "{name}/{reconstruction:?}"
            );
            assert_eq!(faces.source_correspondence.faces.len(), 12);
            let ids: BTreeMap<_, _> = expected
                .groups
                .iter()
                .map(|(group, &[start, end])| {
                    let row = faces
                        .source_correspondence
                        .faces
                        .iter()
                        .find(|r| r.source_triangle_ids.iter().copied().eq(start..end))
                        .unwrap_or_else(|| panic!("{name}: lost {group}"));
                    (group.as_str(), row.face_id)
                })
                .collect();
            let cyl = &faces.result.faces[ids["fillet"]];
            assert_eq!(cyl.family, ResultFaceFamily::Cylinder);
            let mut expected_neighbors =
                ["stop_min", "stop_max", "floor", "tangent_wall"].map(|group| ids[group]);
            expected_neighbors.sort_unstable();
            let mut neighbors = cyl.adjacent_face_ids.clone();
            neighbors.sort_unstable();
            assert_eq!(neighbors, expected_neighbors);
            let circle = doc.step.planar_face_profile(ids["stop_min"]).unwrap();
            assert_eq!(circle.loops.len(), 1);
            let circular = circle.loops[0].segments.iter().filter(|segment| matches!(segment,
                ProfileSegment::Arc { radius_mm, sweep_angle_rad, .. }
                if (radius_mm - expected.radius_mm).abs() < 1e-5 && (sweep_angle_rad.abs() - FRAC_PI_2).abs() < 1e-5)).count();
            assert_eq!(circular, 1);
            let ellipse = doc.step.planar_face_profile(ids["stop_max"]).unwrap();
            assert_eq!(ellipse.loops.len(), 1);
            let mut matched = 0;
            for segment in &ellipse.loops[0].segments {
                if let ProfileSegment::Ellipse {
                    center_mm,
                    major_axis_mm: [mx, my],
                    ratio,
                    start_param_rad,
                    sweep_param_rad,
                } = segment
                {
                    let major = mx.hypot(*my);
                    assert!((major - expected.ellipse_major_radius_mm).abs() < 1e-5);
                    assert!((major * ratio - expected.radius_mm).abs() < 1e-5);
                    assert!((sweep_param_rad.abs() - FRAC_PI_2).abs() < 1e-5);
                    let (sin, cos) = (start_param_rad + sweep_param_rad / 2.).sin_cos();
                    let middle = [
                        center_mm[0] + mx * cos - my * ratio * sin,
                        center_mm[1] + my * cos + mx * ratio * sin,
                    ];
                    for (p, world) in [
                        (*center_mm, expected.ellipse_center_mm),
                        (middle, expected.ellipse_midpoint_mm),
                    ] {
                        for (i, w) in world.into_iter().enumerate() {
                            let f = &ellipse.frame;
                            assert!(
                                (f.origin_mm[i] + f.x_axis[i] * p[0] + f.y_axis[i] * p[1] - w)
                                    .abs()
                                    < 1e-5
                            );
                        }
                    }
                    matched += 1;
                }
            }
            assert_eq!(matched, 1);
            let dxf = String::from_utf8(export_planar_face_dxf(&ellipse).unwrap()).unwrap();
            assert_eq!(dxf.matches("\n0\nELLIPSE\n").count(), 1);
        }
    }
}

#[test]
fn two_pockets_reimport_with_separate_trims_and_triangle_owners() {
    // Interleave the triangles so a global face number cannot masquerade as
    // the source identity of a wall in the other body.
    let source = include_bytes!("../../../fixtures/oblique_pocket/nominal.stl");
    let count = u32::from_le_bytes(source[80..84].try_into().unwrap());
    let mut bytes = source[..80].to_vec();
    bytes.extend((count * 2).to_le_bytes());
    for tri in source[84..].chunks_exact(50) {
        bytes.extend(tri);
        let mut translated = tri.to_vec();
        for i in [20, 32, 44] {
            let z = f32::from_le_bytes(tri[i..i + 4].try_into().unwrap());
            translated[i..i + 4].copy_from_slice(&(z + 16.).to_le_bytes());
        }
        bytes.extend(translated);
    }
    let doc = convert_document_with_provenance(
        &bytes,
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        (
            doc.report.cad.solids,
            doc.report.cad.faces,
            doc.report.cad.cylindrical_faces
        ),
        (2, 24, 2)
    );
    let inspected = doc.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let mut solid_ids = std::collections::BTreeSet::new();
    for row in &inspected.source_correspondence.faces {
        let face = &inspected.result.faces[row.face_id];
        if face.family != fabstep_cad::ResultFaceFamily::Cylinder {
            continue;
        }
        assert!(solid_ids.insert(face.solid_id));
        let body = doc
            .report
            .bodies
            .iter()
            .find(|b| b.id == row.analysis_body_id)
            .unwrap();
        let parity = usize::from(body.conversion.cad.bounds_mm[0][2] > 8.);
        assert!(row
            .source_triangle_ids
            .iter()
            .copied()
            .eq((34..98).map(|i| 2 * i + parity)));
        assert!(face
            .adjacent_face_ids
            .iter()
            .all(|&id| inspected.result.faces[id].solid_id == face.solid_id));
    }
    assert_eq!(solid_ids.len(), 2);
}

#[test]
fn the_two_circular_ends_of_an_existing_model_keep_their_serialized_form() {
    let old = serde_json::json!({"inner": true, "arcs": [3, 8]});
    let mut side: fabstep_core::MixedPartialCylinder = serde_json::from_value(old.clone()).unwrap();
    assert!(!side.elliptic_end);
    assert_eq!(serde_json::to_value(&side).unwrap(), old);
    side.elliptic_end = true;
    let new: fabstep_core::MixedPartialCylinder =
        serde_json::from_value(serde_json::to_value(side).unwrap()).unwrap();
    assert!(new.elliptic_end);
    assert_eq!(new.arcs, [3, 8]);
}

#[test]
fn double_oblique_stops_export_two_exact_face_profiles_and_a_selectable_wall() {
    use fabstep_cad::ResultFaceFamily;
    use fabstep_core::{export_planar_face_dxf, ProfileSegment};
    let cases: [(&[u8], f64); 3] = [
        (
            include_bytes!("../../../fixtures/double_oblique_pocket/opposed.stl"),
            20109.552138971678,
        ),
        (
            include_bytes!("../../../fixtures/double_oblique_pocket/parallel.stl"),
            19776.849014011626,
        ),
        (
            include_bytes!("../../../fixtures/double_oblique_pocket/compound.stl"),
            20169.91463915609,
        ),
    ];
    for (bytes, volume) in cases {
        let o = Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        };
        let doc = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces
            ),
            (1, 12, 11, 1)
        );
        assert!((cad.volume_mm3 - volume).abs() < 0.001);
        let audit = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(
            audit.source_to_result_mm <= o.max_deviation_mm
                && audit.result_to_source_mm <= o.max_deviation_mm
        );
        let faces = doc.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let wall = faces
            .result
            .faces
            .iter()
            .find(|f| f.family == ResultFaceFamily::Cylinder)
            .unwrap();
        assert_eq!(wall.adjacent_face_ids.len(), 4);
        let owned = faces
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.face_id == wall.id)
            .unwrap();
        assert!(owned.source_triangle_ids.len() >= 12);
        let mut all: Vec<_> = faces
            .source_correspondence
            .faces
            .iter()
            .flat_map(|f| f.source_triangle_ids.iter().copied())
            .collect();
        all.sort_unstable();
        assert!(all.into_iter().eq(0..((bytes.len() - 84) / 50)));
        let mut stops = 0;
        for &id in &wall.adjacent_face_ids {
            let profile = doc.step.planar_face_profile(id).unwrap();
            let ellipses=profile.loops.iter().flat_map(|l|&l.segments).filter(|s| matches!(s, ProfileSegment::Ellipse { major_axis_mm:[x,y], ratio, sweep_param_rad,.. } if
                (x.hypot(*y)*ratio-2.5).abs()<1e-5 && (sweep_param_rad.abs()-std::f64::consts::FRAC_PI_2).abs()<1e-6)).count();
            assert!(ellipses <= 1);
            let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
            assert_eq!(dxf.matches("\n0\nELLIPSE\n").count(), ellipses);
            stops += ellipses;
        }
        assert_eq!(stops, 2);
    }
}
