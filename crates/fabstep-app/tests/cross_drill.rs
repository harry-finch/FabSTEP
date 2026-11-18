use fabstep_app::{
    convert_document_with_provenance, selection_topology, DocumentResultFaces, InputFormat,
};
use fabstep_cad::{tessellate_step, ResultFaceFamily};
use fabstep_core::{
    analyze_stl, recognize_rounded, GuidanceFamily, Mesh, Options, ReconstructionMode,
    RoundedFaceRef, RoundedModel,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
    fs,
    ops::Range,
    path::Path,
};

const POSITIVES: [&str; 10] = [
    "f_nominal",
    "f_scaled",
    "f_rotated",
    "f_mirror",
    "f_retessellated",
    "f_ratio",
    "f_ratio_mid",
    "f_ratio_near",
    "f_off_center",
    "f_off_center_rotated",
];
const CLOSED_NEGATIVES: [&str; 4] = [
    "f_ratio_boundary",
    "f_ratio_above",
    "f_offset_clearance",
    "f_fold",
];

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/cross_drill")
            .join(name),
    )
    .unwrap()
}

fn options(reconstruction: ReconstructionMode) -> Options {
    Options {
        reconstruction,
        ..Options::default()
    }
}

fn vector(value: &Value) -> [f64; 3] {
    std::array::from_fn(|i| value[i].as_f64().unwrap())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn winding(mesh: &Mesh, point: [f64; 3]) -> f64 {
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] =
                triangle.map(|i| std::array::from_fn(|j| mesh.vertices[i][j] - point[j]));
            let [la, lb, lc] = [a, b, c].map(|v| dot(v, v).sqrt());
            2. * dot(a, cross(b, c))
                .atan2(la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb)
        })
        .sum::<f64>()
        / (2. * TAU)
}

struct Patch<'a> {
    label: &'a str,
    source_ids: Range<usize>,
}

impl Patch<'_> {
    fn family(&self) -> ResultFaceFamily {
        if self.label.starts_with("plane_") {
            ResultFaceFamily::Plane
        } else {
            ResultFaceFamily::Cylinder
        }
    }
}

fn patches(case: &Value) -> Vec<Patch<'_>> {
    case["patches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let first = row["first_source_triangle"].as_u64().unwrap() as usize;
            let count = row["triangles"].as_u64().unwrap() as usize;
            Patch {
                label: row["owner"].as_str().unwrap(),
                source_ids: first..first + count,
            }
        })
        .collect()
}

fn assert_source_partition(
    patches: &[Patch<'_>],
    owners: &[RoundedFaceRef],
    bytes: &[u8],
    o: &Options,
    small_axis_reversed: bool,
) {
    let topology = selection_topology(bytes, InputFormat::Stl, o).unwrap();
    let [component] = topology.components.as_slice() else {
        panic!("expected one source body");
    };
    assert_eq!(owners.len(), component.analysis_to_source_triangles.len());
    let mut by_source: Vec<_> = component
        .analysis_to_source_triangles
        .iter()
        .copied()
        .zip(owners.iter().copied())
        .collect();
    by_source.sort_unstable_by_key(|&(id, _)| id);
    assert!(by_source.iter().map(|&(id, _)| id).eq(0..owners.len()));
    assert_eq!(
        patches.iter().map(|p| p.source_ids.len()).sum::<usize>(),
        owners.len()
    );
    let mut assigned = Vec::new();
    for patch in patches {
        let group = &by_source[patch.source_ids.clone()];
        let first = group[0].1;
        assert!(
            group.iter().all(|&(_, face)| face == first),
            "split patch {}",
            patch.label
        );
        assert!(!assigned.contains(&first), "merged patch {}", patch.label);
        match patch.label {
            "small_bore_negative" => assert_eq!(
                first,
                RoundedFaceRef::CrossDrill {
                    index: usize::from(small_axis_reversed)
                }
            ),
            "small_bore_positive" => assert_eq!(
                first,
                RoundedFaceRef::CrossDrill {
                    index: usize::from(!small_axis_reversed)
                }
            ),
            "large_bore" => assert_eq!(first, RoundedFaceRef::CrossDrill { index: 2 }),
            label if label.starts_with("plane_") => {
                assert!(matches!(first, RoundedFaceRef::Plane { .. }));
            }
            _ => panic!("unknown patch {}", patch.label),
        }
        assigned.push(first);
    }
}

fn assert_native_partition(
    case: &Value,
    patches: &[Patch<'_>],
    body_id: &str,
    inspected: &DocumentResultFaces,
) -> usize {
    let source = &inspected.source_correspondence;
    let faces = &inspected.result.faces;
    assert_eq!(source.status, "exact");
    assert_eq!(source.faces.len(), patches.len());
    let mut face_ids = BTreeMap::new();
    for patch in patches {
        let row = source
            .faces
            .iter()
            .find(|r| {
                r.source_triangle_ids
                    .iter()
                    .copied()
                    .eq(patch.source_ids.clone())
            })
            .unwrap_or_else(|| panic!("missing native owner of {}", patch.label));
        assert_eq!(row.analysis_body_id, body_id);
        let face = &faces[row.face_id];
        assert_eq!(face.id, row.face_id);
        assert_eq!(face.family, patch.family());
        face_ids.insert(patch.label, row.face_id);
    }
    assert_eq!(
        face_ids.values().collect::<BTreeSet<_>>().len(),
        patches.len()
    );
    let expected: BTreeSet<_> = case["topology"]["joined_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let a = face_ids[edge[0].as_str().unwrap()];
            let b = face_ids[edge[1].as_str().unwrap()];
            (a.min(b), a.max(b))
        })
        .collect();
    let actual: BTreeSet<_> = faces
        .iter()
        .flat_map(|f| {
            f.adjacent_face_ids
                .iter()
                .copied()
                .filter_map(|other| (f.id < other).then_some((f.id, other)))
        })
        .collect();
    assert_eq!(actual, expected);
    face_ids["large_bore"]
}

#[test]
fn cross_drill_reimports_both_bores_and_their_shared_boundaries() {
    let manifest: Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        for name in POSITIVES {
            let bytes = fixture(&format!("{name}.stl"));
            let case = &manifest["cases"][name];
            let patches = patches(case);
            let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let [body] = converted.report.bodies.as_slice() else {
                panic!("expected one body: {name}/{mode:?}");
            };
            let report = &body.conversion;
            let cad = &report.cad;
            assert_eq!(
                (
                    cad.solids,
                    cad.faces,
                    cad.planar_faces,
                    cad.cylindrical_faces
                ),
                (1, 9, 6, 3),
                "{name}/{mode:?}"
            );
            assert_eq!(
                cad.conical_faces + cad.spherical_faces + cad.toroidal_faces,
                0
            );
            let parameters = &case["parameters"];
            let volume = parameters["volume_mm3"].as_f64().unwrap();
            assert!(
                (cad.volume_mm3 - volume).abs() < 1e-7 * volume,
                "{name}/{mode:?}: {cad:?}"
            );
            for (end, actual) in cad.bounds_mm.into_iter().enumerate() {
                assert!(
                    actual
                        .into_iter()
                        .zip(vector(&case["analytic_bounds_mm"][end]))
                        .all(|(a, b)| (a - b).abs() < 2e-5),
                    "{name}/{mode:?}: {cad:?}"
                );
            }

            let proof = report.rounded_recognition.as_ref().unwrap();
            let drill = proof.model.cross_drill.as_ref().unwrap();
            assert_eq!(drill.rims.len(), 4);
            let expected_small_axis = vector(&parameters["small_axis"]);
            let aligned = dot(drill.axes[2], expected_small_axis);
            assert!((aligned.abs() - 1.).abs() < 1e-7);
            assert!(
                (dot(drill.axes[0], vector(&parameters["large_axis"])).abs() - 1.).abs() < 1e-7
            );
            for (actual, key) in drill
                .radii
                .into_iter()
                .zip(["small_radius_mm", "large_radius_mm"])
            {
                assert!((actual - parameters[key].as_f64().unwrap()).abs() < 2e-5);
            }
            assert!((proof.report.expected_volume_mm3 - volume).abs() < 1e-7 * volume);
            assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert_eq!(
                report.analysis.mesh.triangles.len(),
                case["triangles"].as_u64().unwrap() as usize
            );
            assert_source_partition(&patches, &proof.triangle_faces, &bytes, &o, aligned < 0.);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);

            let step = &converted.step;
            let inspected = step.result_faces().unwrap();
            assert_eq!(inspected.result.faces.len(), 9);
            // One large wall touches both small walls, and each small wall
            // touches only the large wall among cylindrical faces.
            let mut cylinder_degrees: Vec<_> = inspected
                .result
                .faces
                .iter()
                .filter(|f| f.family == ResultFaceFamily::Cylinder)
                .map(|f| {
                    f.adjacent_face_ids
                        .iter()
                        .filter(|&&id| {
                            inspected.result.faces[id].family == ResultFaceFamily::Cylinder
                        })
                        .count()
                })
                .collect();
            cylinder_degrees.sort_unstable();
            assert_eq!(cylinder_degrees, [1, 1, 2]);
            if mode == ReconstructionMode::Auto {
                let large = assert_native_partition(case, &patches, &body.id, &inspected);
                if name == "f_nominal" {
                    let error = step
                        .guided_surface(&[large], GuidanceFamily::Auto)
                        .err()
                        .expect("local mutation of the coupled bores accepted");
                    assert_eq!(error.code, "guidance.family_unproven");
                }
            }

            let result = tessellate_step(step.bytes(), 0.01).unwrap();
            for probe in case["material_probes"].as_array().unwrap() {
                let point = vector(&probe["point_mm"]);
                let expected = f64::from(u8::from(probe["material"].as_bool().unwrap()));
                for mesh in [&report.analysis.mesh, &result] {
                    let actual = winding(mesh, point).abs();
                    assert!(
                        (actual - expected).abs() < 1e-6,
                        "{name}/{mode:?}: {point:?} winding {actual}, expected {expected}"
                    );
                }
            }
        }
    }
}

#[test]
fn cross_drill_rejects_open_folded_and_unsupported_sources_or_uses_plain_faces() {
    let open = fixture("f_missing_mouth.stl");
    let nominal = fixture("f_nominal.stl");
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        let error = convert_document_with_provenance(&open, InputFormat::Stl, &o)
            .err()
            .expect("open mesh accepted");
        assert_eq!(error.code, "mesh.open");

        for name in CLOSED_NEGATIVES {
            let bytes = fixture(&format!("{name}.stl"));
            match convert_document_with_provenance(&bytes, InputFormat::Stl, &o) {
                Ok(conversion) => {
                    assert_ne!(name, "f_fold", "folded wall accepted as a solid");
                    assert_eq!(conversion.report.cad.solids, 1);
                    if let Some(proof) = &conversion.report.bodies[0].conversion.rounded_recognition
                    {
                        assert!(proof.model.cross_drill.is_none(), "{name}/{mode:?}");
                        assert!(!proof
                            .triangle_faces
                            .iter()
                            .any(|owner| matches!(owner, RoundedFaceRef::CrossDrill { .. })));
                    }
                }
                Err(error) => assert!(
                    matches!(
                        error.code.as_str(),
                        "recognition.surface_unproven"
                            | "recognition.unsupported"
                            | "recognition.partial_unproven"
                            | "recognition.conic_unproven"
                            | "recognition.deviation_exceeded"
                            | "cad.invalid_solid"
                    ),
                    "{name}/{mode:?}: {error}"
                ),
            }
        }

        let strict = Options {
            max_deviation_mm: 1e-4,
            ..o
        };
        let result = convert_document_with_provenance(&nominal, InputFormat::Stl, &strict);
        if mode == ReconstructionMode::Surfaces {
            let error = result.err().expect("over-budget analytic result accepted");
            assert_eq!(error.code, "recognition.deviation_exceeded");
        } else {
            let fallback = result.expect("Auto failed to retain the closed source");
            assert_eq!(fallback.report.cad.solids, 1);
            assert_eq!(fallback.report.cad.faces, fallback.report.cad.planar_faces);
            let report = &fallback.report.bodies[0].conversion;
            let proof = report.rounded_recognition.as_ref().unwrap();
            assert!(proof.model.cross_drill.is_none());
            assert!(proof.report.max_deviation_bound_mm <= strict.max_deviation_mm);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_to_result_mm <= strict.max_deviation_mm);
            assert!(audit.result_to_source_mm <= strict.max_deviation_mm);
        }
    }
}

#[test]
fn cross_drill_json_extends_legacy_models_and_roundtrips_its_owners() {
    let legacy: RoundedModel = serde_json::from_value(json!({
        "revolved": {
            "mixed": {"vertices": [], "rings": [], "planes": [], "cylinders": []},
            "cones": []
        },
        "spheres": [], "tori": []
    }))
    .unwrap();
    assert!(legacy.cross_drill.is_none());
    assert!(serde_json::to_value(legacy)
        .unwrap()
        .get("cross_drill")
        .is_none());

    let o = options(ReconstructionMode::Surfaces);
    let source = analyze_stl(&fixture("f_nominal.stl"), &o).unwrap();
    let proof = recognize_rounded(&source, &o).unwrap();
    let model = serde_json::to_value(proof.model).unwrap();
    assert_eq!(model["cross_drill"]["rims"], json!([0, 1, 2, 3]));
    let branches = model["cross_drill"]["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 2);
    assert!(branches
        .iter()
        .all(|branch| branch.as_array().unwrap().len() == 64));
    let mut old = model.clone();
    old["cross_drill"]
        .as_object_mut()
        .unwrap()
        .remove("small_offset");
    let previous: RoundedModel = serde_json::from_value(old).unwrap();
    assert_eq!(previous.cross_drill.unwrap().small_offset, [0.; 2]);
    let decoded: RoundedModel = serde_json::from_value(model.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), model);

    for index in 0..3 {
        let face = RoundedFaceRef::CrossDrill { index };
        let encoded = serde_json::to_value(face).unwrap();
        assert_eq!(encoded, json!({"kind": "cross_drill", "index": index}));
        assert_eq!(
            serde_json::from_value::<RoundedFaceRef>(encoded).unwrap(),
            face
        );
    }
}

#[test]
fn separate_blind_drills_still_convert_through_the_historical_surface_path() {
    let bytes = include_bytes!("../../../fixtures/historical_four_blind_drills.stl");
    for mode in [ReconstructionMode::Primitives, ReconstructionMode::Surfaces] {
        let converted = convert_document_with_provenance(bytes, InputFormat::Stl, &options(mode))
            .unwrap_or_else(|error| panic!("{mode:?}: {error}"));
        let cad = &converted.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.planar_faces,
                cad.cylindrical_faces,
                cad.conical_faces
            ),
            (1, 6, 4, 4)
        );
        assert!(converted.report.bodies[0]
            .conversion
            .rounded_recognition
            .is_none());
    }
}

#[test]
fn freecad_bore_strips_keep_both_openings_and_every_source_face() {
    // Independent FreeCAD tessellation of a 24 x 20 x 18 block. The two
    // small walls span from rim to intersection without interior vertices.
    let cases: Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    let case = &cases["cases"]["f_ratio_mid"];
    let volume = case["parameters"]["volume_mm3"].as_f64().unwrap();
    let local_probes = [
        ([0., 0., 0.], false),
        ([0., 0., 7.], false),
        ([10., 0., 0.], false),
        ([5., 1., 6.], true),
        ([5., 1., -6.], true),
    ];
    check_freecad_bores("freecad_mid_coarse.stl", volume, 0.0001, &local_probes);

    let offset = &cases["cases"]["f_off_center"];
    let offset_probes: Vec<_> = offset["material_probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|probe| {
            (
                vector(&probe["point_mm"]),
                probe["material"].as_bool().unwrap(),
            )
        })
        .collect();
    check_freecad_bores(
        "freecad_offset_coarse.stl",
        offset["parameters"]["volume_mm3"].as_f64().unwrap(),
        0.0001,
        &offset_probes,
    );

    let rotated_probes: Vec<_> = case["material_probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|probe| {
            (
                vector(&probe["point_mm"]),
                probe["material"].as_bool().unwrap(),
            )
        })
        .collect();
    check_freecad_bores(
        "freecad_mid_rotated.stl",
        volume,
        volume * 1e-7,
        &rotated_probes,
    );
}

fn check_freecad_bores(
    name: &str,
    volume: f64,
    volume_tolerance: f64,
    probes: &[([f64; 3], bool)],
) {
    let bytes = fixture(name);
    let o = options(ReconstructionMode::Auto);
    let doc = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
        .unwrap_or_else(|err| panic!("{name}: {err}"));
    let cad = &doc.report.cad;
    assert_eq!(
        (
            cad.solids,
            cad.faces,
            cad.planar_faces,
            cad.cylindrical_faces
        ),
        (1, 9, 6, 3),
        "{name}"
    );
    assert!(
        (cad.volume_mm3 - volume).abs() < volume_tolerance,
        "{name}: {cad:?}"
    );
    let conversion = &doc.report.bodies[0].conversion;
    let proof = conversion.rounded_recognition.as_ref().unwrap();
    assert!(proof.model.cross_drill.is_some());
    assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
    let inspection = doc.step.result_faces().unwrap();
    assert_eq!(inspection.source_correspondence.status, "exact");
    let mut attributed: Vec<_> = inspection
        .source_correspondence
        .faces
        .iter()
        .flat_map(|owner| owner.source_triangle_ids.iter().copied())
        .collect();
    attributed.sort_unstable();
    assert!(attributed
        .into_iter()
        .eq(0..conversion.analysis.mesh.triangles.len()));

    let mesh = &inspection.result.mesh;
    for &(point, material) in probes {
        assert!((winding(mesh, point).abs() - f64::from(u8::from(material))).abs() < 1e-6);
    }
}
