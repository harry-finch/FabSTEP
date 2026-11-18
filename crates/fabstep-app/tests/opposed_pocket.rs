use fabstep_app::{convert_document_with_provenance, InputFormat, ProvenancedDocumentConversion};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{export_planar_face_dxf, Options, ProfileSegment, ReconstructionMode};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::f64::consts::FRAC_PI_2;
use std::{fs, path::Path};

#[derive(Deserialize)]
struct Fillet {
    radius_mm: f64,
    ends_mm: [[f64; 3]; 2],
}

#[derive(Deserialize)]
struct Case {
    groups: BTreeMap<String, [usize; 2]>,
    triangles: usize,
    bounds_mm: Option<[[f64; 3]; 2]>,
    volume_mm3: Option<f64>,
    fillets: Option<[Fillet; 2]>,
}

#[derive(Deserialize)]
struct Corpus {
    positives: Vec<String>,
    cases: BTreeMap<String, Case>,
}

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/opposed_pocket")
            .join(file),
    )
    .unwrap()
}

fn convert(name: &str, reconstruction: ReconstructionMode) -> ProvenancedDocumentConversion {
    convert_document_with_provenance(
        &fixture(&format!("{name}.stl")),
        InputFormat::Stl,
        &Options {
            reconstruction,
            ..Options::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn opposite_fillets_keep_individual_radii_neighbors_and_source_triangles() {
    let corpus: Corpus = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for name in corpus.positives {
        let expected = &corpus.cases[&name];
        for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let document = convert(&name, mode);
            let cad = &document.report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.cylindrical_faces
                ),
                (1, 13, 11, 2),
                "{name} {mode:?}"
            );
            assert!((cad.volume_mm3 - expected.volume_mm3.unwrap()).abs() < 1e-4);
            for (got, want) in cad
                .bounds_mm
                .iter()
                .flatten()
                .zip(expected.bounds_mm.as_ref().unwrap().iter().flatten())
            {
                assert!((got - want).abs() < 1e-5);
            }
            let fidelity = document.report.bodies[0]
                .conversion
                .fidelity
                .as_ref()
                .unwrap();
            let tolerance = Options::default().max_deviation_mm;
            assert!(fidelity.source_samples > 0 && fidelity.result_samples > 0);
            assert!(
                fidelity.source_to_result_mm <= tolerance
                    && fidelity.result_to_source_mm <= tolerance
            );
            if mode != ReconstructionMode::Auto {
                continue;
            }
            let faces = document.step.result_faces().unwrap();
            let correspondence = &faces.source_correspondence;
            assert_eq!(correspondence.status, "exact");
            let mut owners = Vec::new();
            for face in &correspondence.faces {
                owners.extend_from_slice(&face.source_triangle_ids);
            }
            owners.sort_unstable();
            assert!(owners.into_iter().eq(0..expected.triangles));
            let id = |group: &str| {
                let [start, end] = expected.groups[group];
                correspondence
                    .faces
                    .iter()
                    .find(|row| row.source_triangle_ids.iter().copied().eq(start..end))
                    .unwrap_or_else(|| panic!("{name}: missing {group}"))
                    .face_id
            };
            for (fillet, wall) in [("left_fillet", "left_wall"), ("right_fillet", "right_wall")] {
                let face = &faces.result.faces[id(fillet)];
                assert_eq!(face.family, ResultFaceFamily::Cylinder);
                let mut neighbors = face.adjacent_face_ids.clone();
                neighbors.sort_unstable();
                let mut expected_neighbors =
                    [id("floor"), id(wall), id("stop_min"), id("stop_max")];
                expected_neighbors.sort_unstable();
                assert_eq!(neighbors, expected_neighbors);
            }
            for (end, stop) in ["stop_min", "stop_max"].into_iter().enumerate() {
                let profile = document.step.planar_face_profile(id(stop)).unwrap();
                assert_eq!(profile.loops.len(), 1);
                let mut matched = [false; 2];
                for segment in &profile.loops[0].segments {
                    match segment {
                        ProfileSegment::Arc {
                            center_mm,
                            radius_mm,
                            sweep_angle_rad,
                            ..
                        } => {
                            assert!((sweep_angle_rad.abs() - FRAC_PI_2).abs() < 1e-5);
                            let frame = &profile.frame;
                            let center: [f64; 3] = std::array::from_fn(|i| {
                                frame.origin_mm[i]
                                    + center_mm[0] * frame.x_axis[i]
                                    + center_mm[1] * frame.y_axis[i]
                            });
                            let index = expected
                                .fillets
                                .as_ref()
                                .unwrap()
                                .iter()
                                .position(|fillet| {
                                    (fillet.radius_mm - radius_mm).abs() < 1e-5
                                        && fillet.ends_mm[end]
                                            .iter()
                                            .zip(center)
                                            .all(|(a, b)| (a - b).abs() < 1e-5)
                                })
                                .unwrap_or_else(|| panic!("{name}: misplaced arc on {stop}"));
                            assert!(!matched[index]);
                            matched[index] = true;
                        }
                        ProfileSegment::Line { .. } => (),
                        other => panic!("Unexpected trim on {stop}: {other:?}"),
                    }
                }
                assert_eq!(matched, [true, true]);
                let dxf = String::from_utf8(export_planar_face_dxf(&profile).unwrap()).unwrap();
                assert_eq!(dxf.matches("\n0\nARC\n").count(), 2);
            }
        }
    }
}

#[test]
fn deforming_one_end_keeps_only_the_other_wall_cylindrical() {
    let corpus: Corpus = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    let case = &corpus.cases["warped_stop"];
    let document = convert("warped_stop", ReconstructionMode::Auto);
    let cad = &document.report.cad;
    assert_eq!(
        (
            cad.solids,
            cad.cylindrical_faces,
            cad.faces - cad.planar_faces
        ),
        (1, 1, 1)
    );
    let faces = document.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let [start, end] = case.groups["left_fillet"];
    let mut all_triangles = Vec::new();
    for row in faces.source_correspondence.faces {
        all_triangles.extend_from_slice(&row.source_triangle_ids);
        if faces.result.faces[row.face_id].family == ResultFaceFamily::Cylinder {
            assert!(row.source_triangle_ids.into_iter().eq(start..end));
        }
    }
    all_triangles.sort_unstable();
    assert!(all_triangles.into_iter().eq(0..case.triangles));
    let fidelity = document.report.bodies[0]
        .conversion
        .fidelity
        .as_ref()
        .unwrap();
    assert!(fidelity.source_to_result_mm <= Options::default().max_deviation_mm);
    assert!(fidelity.result_to_source_mm <= Options::default().max_deviation_mm);
}
