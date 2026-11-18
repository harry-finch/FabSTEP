// Draft integration test. Install as crates/fabstep-app/tests/linear_blend.rs
// once the G recognition model, native builder and app routing are available.
use fabstep_app::{
    convert_document_with_provenance, selection_topology, DocumentResultFaces, DocumentStep,
    InputFormat,
};
use fabstep_cad::{tessellate_step, ResultFaceFamily};
use fabstep_core::{
    analyze_stl, export_planar_face_dxf, recognize_rounded, GuidanceFamily, Mesh, Options,
    PlanarFaceProfile, ProfileFrame, ProfileSegment, ReconstructionMode, RoundedFaceRef,
    RoundedModel,
};
use serde_json::{json, Value};
use std::{collections::BTreeSet, f64::consts::TAU, fs, ops::Range, path::Path};

const POSITIVES: [&str; 7] = [
    "g_increasing",
    "g_decreasing",
    "g_mirror",
    "g_permuted",
    "g_oblique_staggered",
    "g_ruled_strip",
    "g_ruled_decreasing",
];
const CLOSED_NEGATIVES: [&str; 4] = [
    "g_circular_stack",
    "g_nonlinear",
    "g_parabolic_cap",
    "g_ruled_circle",
];
const GEOMETRY_TOLERANCE: f64 = 2e-5;

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/local_blends")
            .join(file),
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

fn difference(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    length(difference(a, b)) < GEOMETRY_TOLERANCE
}

fn same_endpoints(actual: [[f64; 3]; 2], expected: [[f64; 3]; 2]) -> bool {
    (close(actual[0], expected[0]) && close(actual[1], expected[1]))
        || (close(actual[0], expected[1]) && close(actual[1], expected[0]))
}

fn world(frame: &ProfileFrame, point: [f64; 2]) -> [f64; 3] {
    std::array::from_fn(|i| {
        frame.origin_mm[i] + frame.x_axis[i] * point[0] + frame.y_axis[i] * point[1]
    })
}

fn winding(mesh: &Mesh, point: [f64; 3]) -> f64 {
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|i| difference(mesh.vertices[i], point));
            let [la, lb, lc] = [a, b, c].map(length);
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

fn reference_patches(case: &Value) -> Vec<Patch<'_>> {
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
) {
    let topology = selection_topology(bytes, InputFormat::Stl, o).unwrap();
    let [component] = topology.components.as_slice() else {
        panic!("expected one source body");
    };
    let source_ids = &component.analysis_to_source_triangles;
    assert_eq!(owners.len(), source_ids.len());
    assert_eq!(
        source_ids.iter().copied().collect::<BTreeSet<_>>(),
        (0..source_ids.len()).collect()
    );
    let mut patch_owners = Vec::new();
    for patch in patches {
        let mut faces = source_ids
            .iter()
            .zip(owners)
            .filter_map(|(id, &face)| patch.source_ids.contains(id).then_some(face));
        let first = faces.next().unwrap();
        assert!(
            faces.all(|face| face == first),
            "split patch {}",
            patch.label
        );
        assert!(
            !patch_owners.contains(&first),
            "merged patch {}",
            patch.label
        );
        if patch.label == "inclined_cone" {
            assert_eq!(first, RoundedFaceRef::LinearBlend { index: 0 });
        } else {
            assert!(matches!(first, RoundedFaceRef::Plane { .. }));
        }
        patch_owners.push(first);
    }
}

fn assert_cap_profile(profile: &PlanarFaceProfile, caps: &[Value]) -> Option<usize> {
    let [wire] = profile.loops.as_slice() else {
        panic!("G planar face must contain one wire");
    };
    assert!(wire.is_outer);
    let mut ellipses = wire.segments.iter().filter_map(|segment| match *segment {
        ProfileSegment::Ellipse {
            center_mm,
            major_axis_mm,
            ratio,
            start_param_rad,
            sweep_param_rad,
        } => Some((
            center_mm,
            major_axis_mm,
            ratio,
            start_param_rad,
            sweep_param_rad,
        )),
        ProfileSegment::Line { .. } => None,
        _ => panic!("unexpected curve on a G plane"),
    });
    let (center, major, ratio, start, sweep) = ellipses.next()?;
    assert!(ellipses.next().is_none(), "multiple ellipses on one cap");
    let center_world = world(&profile.frame, center);
    let cap_index = caps
        .iter()
        .position(|cap| close(center_world, vector(&cap["center_mm"])))
        .unwrap_or_else(|| panic!("unexpected ellipse center {center_world:?}"));
    let cap = &caps[cap_index];
    let semimajor = cap["semimajor_mm"].as_f64().unwrap();
    let semiminor = cap["semiminor_mm"].as_f64().unwrap();
    let actual_major = major[0].hypot(major[1]);
    assert!((actual_major - semimajor).abs() < GEOMETRY_TOLERANCE);
    assert!((actual_major * ratio - semiminor).abs() < GEOMETRY_TOLERANCE);
    assert!((sweep.abs() - cap["sweep_rad"].as_f64().unwrap()).abs() < 1e-6);

    let evaluate = |parameter: f64| {
        let (sin, cos) = parameter.sin_cos();
        world(
            &profile.frame,
            [
                center[0] + major[0] * cos - ratio * major[1] * sin,
                center[1] + major[1] * cos + ratio * major[0] * sin,
            ],
        )
    };
    let endpoints = [0, 1].map(|i| vector(&cap["endpoints_mm"][i]));
    assert!(same_endpoints(
        [evaluate(start), evaluate(start + sweep)],
        endpoints
    ));

    // The endpoints alone allow the wrong axis orientation or arc branch.
    // The inward radial direction from the ellipse center to their midpoint
    // determines the center of the trimmed parameter interval independently.
    let expected_center = vector(&cap["center_mm"]);
    let direction: [f64; 3] =
        std::array::from_fn(|i| (endpoints[0][i] + endpoints[1][i]) / 2. - expected_center[i]);
    let expected_midpoint =
        std::array::from_fn(|i| expected_center[i] + semimajor * direction[i] / length(direction));
    assert!(close(evaluate(start + sweep / 2.), expected_midpoint));

    let dxf = String::from_utf8(export_planar_face_dxf(profile).unwrap()).unwrap();
    assert_eq!(dxf.matches("\n0\nELLIPSE\n").count(), 1);
    Some(cap_index)
}

fn assert_native_boundaries(case: &Value, faces: &DocumentResultFaces, step: &DocumentStep) {
    let cone = faces
        .result
        .faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Cone)
        .unwrap();
    assert_eq!(cone.adjacent_face_ids.len(), 4);
    let neighbors: BTreeSet<_> = cone.adjacent_face_ids.iter().copied().collect();
    assert_eq!(neighbors.len(), 4);
    let caps = case["parameters"]["shape"]["ellipse_caps"]
        .as_array()
        .unwrap();
    let rails = [0, 1].map(|i| {
        [
            vector(&caps[0]["endpoints_mm"][i]),
            vector(&caps[1]["endpoints_mm"][i]),
        ]
    });
    let mut found_caps = BTreeSet::new();
    let mut found_rails = BTreeSet::new();
    for face in &faces.result.faces {
        if face.id == cone.id {
            continue;
        }
        assert_eq!(face.family, ResultFaceFamily::Plane);
        let profile = step.planar_face_profile(face.id).unwrap();
        assert_eq!(profile.solid_id, cone.solid_id);
        if let Some(i) = assert_cap_profile(&profile, caps) {
            assert!(neighbors.contains(&face.id));
            assert!(face.adjacent_face_ids.contains(&cone.id));
            assert!(found_caps.insert(i));
            continue;
        }
        if neighbors.contains(&face.id) {
            assert!(face.adjacent_face_ids.contains(&cone.id));
            let mut matches = profile.loops[0].segments.iter().filter_map(|segment| {
                let ProfileSegment::Line { start_mm, end_mm } = *segment else {
                    return None;
                };
                let endpoints = [
                    world(&profile.frame, start_mm),
                    world(&profile.frame, end_mm),
                ];
                rails
                    .iter()
                    .position(|&rail| same_endpoints(endpoints, rail))
            });
            let i = matches.next().expect("missing rail on the tangent plane");
            assert!(matches.next().is_none());
            assert!(found_rails.insert(i));
        }
    }
    assert_eq!(found_caps, BTreeSet::from([0, 1]));
    assert_eq!(found_rails, BTreeSet::from([0, 1]));
}

fn assert_native_partition(
    case: &Value,
    patches: &[Patch<'_>],
    body_id: &str,
    faces: &DocumentResultFaces,
) -> usize {
    let correspondence = &faces.source_correspondence;
    assert_eq!(correspondence.status, "exact");
    assert_eq!(correspondence.faces.len(), patches.len());
    let mut face_ids = std::collections::BTreeMap::new();
    for patch in patches {
        let row = correspondence
            .faces
            .iter()
            .find(|row| {
                row.source_triangle_ids
                    .iter()
                    .copied()
                    .eq(patch.source_ids.clone())
            })
            .unwrap_or_else(|| panic!("missing native owner of {}", patch.label));
        assert_eq!(row.analysis_body_id, body_id);
        let face = &faces.result.faces[row.face_id];
        assert_eq!(face.id, row.face_id);
        let family = if patch.label == "inclined_cone" {
            ResultFaceFamily::Cone
        } else {
            ResultFaceFamily::Plane
        };
        assert_eq!(face.family, family);
        face_ids.insert(patch.label, face.id);
    }
    assert_eq!(
        face_ids.values().collect::<BTreeSet<_>>().len(),
        patches.len()
    );
    let expected_joins: BTreeSet<_> = case["topology"]["joined_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let a = face_ids[edge[0].as_str().unwrap()];
            let b = face_ids[edge[1].as_str().unwrap()];
            (a.min(b), a.max(b))
        })
        .collect();
    let actual_joins: BTreeSet<_> = faces
        .result
        .faces
        .iter()
        .flat_map(|face| {
            face.adjacent_face_ids
                .iter()
                .copied()
                .filter_map(|other| (face.id < other).then_some((face.id, other)))
        })
        .collect();
    assert_eq!(actual_joins, expected_joins);
    face_ids["inclined_cone"]
}

#[test]
fn linear_blend_reimports_the_inclined_cone_and_its_elliptic_trims() {
    let manifest: Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        for name in POSITIVES {
            let bytes = fixture(&format!("{name}.stl"));
            let case = &manifest["cases"][name];
            let patches = reference_patches(case);
            let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let [body] = converted.report.bodies.as_slice() else {
                panic!("expected one body: {name}/{mode:?}");
            };
            let report = &body.conversion;
            let cad = &report.cad;
            assert_eq!(
                (cad.solids, cad.faces, cad.planar_faces, cad.conical_faces),
                (1, 7, 6, 1)
            );
            assert_eq!(
                cad.cylindrical_faces + cad.spherical_faces + cad.toroidal_faces,
                0
            );
            let expected_volume = case["parameters"]["volume_mm3"].as_f64().unwrap();
            assert!(
                (cad.volume_mm3 - expected_volume).abs() < 1e-7 * expected_volume,
                "{name}/{mode:?}: {cad:?}"
            );
            for (end, actual) in cad.bounds_mm.into_iter().enumerate() {
                assert!(
                    close(actual, vector(&case["analytic_bounds_mm"][end])),
                    "{name}/{mode:?}: {cad:?}"
                );
            }

            let proof = report.rounded_recognition.as_ref().unwrap();
            assert!(proof.model.linear_blend.is_some());
            assert_eq!(proof.model.revolved.mixed.elliptic_arcs.len(), 2);
            assert!(proof.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert_eq!(
                report.analysis.mesh.triangles.len(),
                case["triangles"].as_u64().unwrap() as usize
            );
            assert_source_partition(&patches, &proof.triangle_faces, &bytes, &o);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);

            let step = &converted.step;
            let faces = step.result_faces().unwrap();
            assert_eq!(faces.result.faces.len(), 7);
            assert_native_boundaries(case, &faces, step);
            if mode == ReconstructionMode::Auto {
                let cone = assert_native_partition(case, &patches, &body.id, &faces);
                if name == "g_increasing" {
                    let error = step
                        .guided_surface(&[cone], GuidanceFamily::Cylinder)
                        .err()
                        .expect("wrong-family guidance accepted");
                    assert_eq!(error.code, "guidance.family_unproven");
                }
            }

            // The circle-discriminating probes are closer to the boundary than the
            // normal preview tolerance. Refine the independent native tessellation.
            let reimported = tessellate_step(step.bytes(), 0.001).unwrap();
            for probe in case["material_probes"].as_array().unwrap() {
                let point = vector(&probe["point_mm"]);
                let expected = f64::from(u8::from(probe["material"].as_bool().unwrap()));
                for mesh in [&report.analysis.mesh, &reimported] {
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
fn nonmatching_sources_and_strict_deviation_cannot_gain_a_linear_blend() {
    let open = fixture("g_missing_cap.stl");
    let nominal = fixture("g_increasing.stl");
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        let err = convert_document_with_provenance(&open, InputFormat::Stl, &o)
            .err()
            .expect("open mesh accepted");
        assert_eq!(err.code, "mesh.open");

        for name in CLOSED_NEGATIVES {
            let bytes = fixture(&format!("{name}.stl"));
            match convert_document_with_provenance(&bytes, InputFormat::Stl, &o) {
                Ok(conversion) => {
                    assert_eq!(conversion.report.cad.solids, 1);
                    if let Some(proof) = &conversion.report.bodies[0].conversion.rounded_recognition
                    {
                        assert!(proof.model.linear_blend.is_none(), "{name}/{mode:?}");
                        assert!(!proof
                            .triangle_faces
                            .contains(&RoundedFaceRef::LinearBlend { index: 0 }));
                    }
                }
                Err(err) => assert!(
                    matches!(
                        err.code.as_str(),
                        "recognition.surface_unproven"
                            | "recognition.unsupported"
                            | "recognition.deviation_exceeded"
                    ),
                    "{name}/{mode:?}: {err}"
                ),
            }
        }

        let strict = Options {
            max_deviation_mm: 1e-4,
            ..o
        };
        let result = convert_document_with_provenance(&nominal, InputFormat::Stl, &strict);
        if mode == ReconstructionMode::Surfaces {
            let err = result.err().expect("over-budget analytic result accepted");
            assert_eq!(err.code, "recognition.deviation_exceeded");
        } else {
            let fallback = result.expect("Auto failed to retain the closed source");
            assert_eq!(fallback.report.cad.solids, 1);
            assert_eq!(fallback.report.cad.faces, fallback.report.cad.planar_faces);
            let report = &fallback.report.bodies[0].conversion;
            let proof = report.rounded_recognition.as_ref().unwrap();
            assert!(proof.model.linear_blend.is_none());
            assert!(proof.model.revolved.mixed.elliptic_arcs.is_empty());
            assert!(proof.report.max_deviation_bound_mm <= strict.max_deviation_mm);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_to_result_mm <= strict.max_deviation_mm);
            assert!(audit.result_to_source_mm <= strict.max_deviation_mm);
        }
    }
}

#[test]
fn linear_blend_serde_extends_legacy_rounded_models() {
    let legacy: RoundedModel = serde_json::from_value(json!({
        "revolved": {
            "mixed": {"vertices": [], "rings": [], "planes": [], "cylinders": []},
            "cones": []
        },
        "spheres": [], "tori": []
    }))
    .unwrap();
    assert!(legacy.linear_blend.is_none());
    assert!(legacy.revolved.mixed.elliptic_arcs.is_empty());
    let legacy = serde_json::to_value(legacy).unwrap();
    assert!(legacy.get("linear_blend").is_none());
    assert!(legacy["revolved"]["mixed"].get("elliptic_arcs").is_none());

    let o = options(ReconstructionMode::Surfaces);
    let source = analyze_stl(&fixture("g_increasing.stl"), &o).unwrap();
    let proof = recognize_rounded(&source, &o).unwrap();
    let value = serde_json::to_value(proof.model).unwrap();
    assert_eq!(value["linear_blend"]["arcs"], json!([0, 1]));
    assert_eq!(
        value["revolved"]["mixed"]["elliptic_arcs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let decoded: RoundedModel = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), value);

    let face = RoundedFaceRef::LinearBlend { index: 0 };
    let encoded = serde_json::to_value(face).unwrap();
    assert_eq!(encoded, json!({"kind": "linear_blend", "index": 0}));
    assert_eq!(
        serde_json::from_value::<RoundedFaceRef>(encoded).unwrap(),
        face
    );
}
