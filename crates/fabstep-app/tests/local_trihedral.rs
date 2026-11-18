use fabstep_app::{
    convert_document_with_provenance, convert_stl, selection_topology, DocumentResultFaces,
    DocumentStep, InputFormat,
};
use fabstep_cad::{tessellate_step, ResultFaceFamily};
use fabstep_core::{
    GuidanceFamily, Mesh, Options, ProfileSegment, ReconstructionMode, RoundedFaceRef,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::{FRAC_PI_2, TAU},
    fs,
    ops::Range,
    path::Path,
};

const POSITIVES: [&str; 4] = [
    "d_corner",
    "d_corner_mirror",
    "d_corner_permuted",
    "d_corner_oblique",
];

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/local_blends")
            .join(file),
    )
    .unwrap()
}

fn manifest() -> Value {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

fn options(reconstruction: ReconstructionMode) -> Options {
    Options {
        reconstruction,
        ..Options::default()
    }
}

fn vector3(value: &Value) -> [f64; 3] {
    std::array::from_fn(|i| value[i].as_f64().unwrap())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(u, v)| u * v).sum()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn winding_number(mesh: &Mesh, p: [f64; 3]) -> f64 {
    // Each reference probe is off the boundary. The sum uses only oriented
    // triangle solid angles and also applies to independent STEP tessellation.
    mesh.triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|i| std::array::from_fn(|j| mesh.vertices[i][j] - p[j]));
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
    family: ResultFaceFamily,
}

fn reference_patches(case: &Value) -> Vec<Patch<'_>> {
    case["patches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let label = row["owner"].as_str().unwrap();
            let first = row["first_source_triangle"].as_u64().unwrap() as usize;
            let count = row["triangles"].as_u64().unwrap() as usize;
            let family = match label.split('_').next() {
                Some("plane") => ResultFaceFamily::Plane,
                Some("cylinder") => ResultFaceFamily::Cylinder,
                Some("sphere") => ResultFaceFamily::Sphere,
                _ => panic!("unknown reference patch {label}"),
            };
            Patch {
                label,
                source_ids: first..first + count,
                family,
            }
        })
        .collect()
}

fn assert_exhaustive_source_owners(
    patches: &[Patch<'_>],
    owners: &[RoundedFaceRef],
    bytes: &[u8],
    o: &Options,
) {
    let topology = selection_topology(bytes, InputFormat::Stl, o).unwrap();
    let [component] = topology.components.as_slice() else {
        panic!("expected one source body");
    };
    let ids = &component.analysis_to_source_triangles;
    assert_eq!(owners.len(), ids.len());
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>(),
        (0..ids.len()).collect::<BTreeSet<_>>()
    );
    let mut assigned_owners = Vec::with_capacity(patches.len());
    for patch in patches {
        let mut faces = owners
            .iter()
            .zip(ids)
            .filter_map(|(&face, id)| patch.source_ids.contains(id).then_some(face));
        let first = faces.next().expect("unassigned source patch");
        assert!(
            faces.all(|face| face == first),
            "split source patch {}",
            patch.label
        );
        assert!(
            !assigned_owners.contains(&first),
            "merged source patch {}",
            patch.label
        );
        assigned_owners.push(first);
    }
}

fn assert_native_seams(faces: &DocumentResultFaces) -> usize {
    let mut spheres = faces
        .result
        .faces
        .iter()
        .filter(|f| f.family == ResultFaceFamily::Sphere);
    let sphere = spheres.next().expect("missing spherical octant");
    assert!(spheres.next().is_none());
    let cylinders: BTreeSet<_> = faces
        .result
        .faces
        .iter()
        .filter(|f| f.family == ResultFaceFamily::Cylinder)
        .map(|f| f.id)
        .collect();
    assert_eq!(cylinders.len(), 3);
    assert_eq!(sphere.adjacent_face_ids.len(), 3);
    assert_eq!(
        sphere
            .adjacent_face_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        cylinders
    );
    for f in &faces.result.faces {
        for &id in &f.adjacent_face_ids {
            assert!(faces.result.faces[id].adjacent_face_ids.contains(&f.id));
        }
        if f.family == ResultFaceFamily::Cylinder {
            assert_eq!(f.adjacent_face_ids.len(), 4);
            assert_eq!(
                f.adjacent_face_ids
                    .iter()
                    .filter(|id| { faces.result.faces[**id].family == ResultFaceFamily::Plane })
                    .count(),
                3
            );
        }
    }
    sphere.id
}

fn assert_native_partition_and_caps(
    case: &Value,
    patches: &[Patch<'_>],
    faces: &DocumentResultFaces,
    step: &DocumentStep,
) {
    let source = &faces.source_correspondence;
    assert_eq!(source.status, "exact");
    assert_eq!(source.faces.len(), patches.len());
    let mut face_ids = BTreeMap::new();
    for patch in patches {
        let mut rows = source.faces.iter().filter(|row| {
            row.source_triangle_ids
                .iter()
                .copied()
                .eq(patch.source_ids.clone())
        });
        let row = rows
            .next()
            .unwrap_or_else(|| panic!("lost source patch {}", patch.label));
        assert!(rows.next().is_none());
        let face = &faces.result.faces[row.face_id];
        assert_eq!(face.id, row.face_id);
        assert_eq!(face.family, patch.family);
        face_ids.insert(patch.label, row.face_id);
    }
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
    assert_eq!(expected_joins, actual_joins);

    let parameters = &case["parameters"];
    let radius = parameters["radius_mm"].as_f64().unwrap();
    for axis in 0..3 {
        let far = step
            .planar_face_profile(face_ids[format!("plane_{axis}_far").as_str()])
            .unwrap();
        let mut arcs = far
            .loops
            .iter()
            .flat_map(|lp| &lp.segments)
            .filter_map(|seg| match seg {
                ProfileSegment::Arc {
                    center_mm,
                    radius_mm,
                    sweep_angle_rad,
                    ..
                } => Some((center_mm, *radius_mm, *sweep_angle_rad)),
                ProfileSegment::Line { .. } => None,
                _ => panic!("unexpected trim on the rounded far plane"),
            });
        let (uv, actual_radius, sweep) = arcs.next().expect("missing circular far trim");
        assert!(arcs.next().is_none());
        assert!((actual_radius - radius).abs() < 1e-5);
        assert!((sweep.abs() - FRAC_PI_2).abs() < 1e-6);
        let expected = vector3(&parameters["shape"]["cylinder_center_segments_mm"][axis][1]);
        for (component, target) in expected.into_iter().enumerate() {
            let world = far.frame.origin_mm[component]
                + far.frame.x_axis[component] * uv[0]
                + far.frame.y_axis[component] * uv[1];
            assert!((world - target).abs() < 2e-5);
        }
        let near = step
            .planar_face_profile(face_ids[format!("plane_{axis}_near").as_str()])
            .unwrap();
        assert!(near
            .loops
            .iter()
            .flat_map(|lp| &lp.segments)
            .all(|segment| matches!(segment, ProfileSegment::Line { .. })));
    }
}

#[test]
fn trihedral_source_reimports_its_analytic_corner_and_shared_boundaries() {
    let manifest = manifest();
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        for name in POSITIVES {
            let case = &manifest["cases"][name];
            let patches = reference_patches(case);
            let bytes = fixture(&format!("{name}.stl"));
            let converted = convert_document_with_provenance(&bytes, InputFormat::Stl, &o)
                .unwrap_or_else(|e| panic!("{name}/{mode:?}: {e}"));
            let [body] = converted.report.bodies.as_slice() else {
                panic!("{name}/{mode:?}: expected one body");
            };
            let report = &body.conversion;
            let cad = &report.cad;
            assert_eq!(
                (cad.solids, cad.faces, cad.planar_faces),
                (1, 10, 6),
                "{name}/{mode:?}"
            );
            assert_eq!((cad.cylindrical_faces, cad.spherical_faces), (3, 1));
            assert_eq!(cad.conical_faces + cad.toroidal_faces, 0);
            let expected_volume = case["parameters"]["volume_mm3"].as_f64().unwrap();
            assert!(
                (cad.volume_mm3 - expected_volume).abs() < 1e-7 * expected_volume,
                "{name}/{mode:?}: {cad:?}"
            );
            for (end, actual) in cad.bounds_mm.iter().enumerate() {
                let expected = vector3(&case["analytic_bounds_mm"][end]);
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 2e-5),
                    "{name}/{mode:?}: {cad:?}"
                );
            }
            let recognition = report.rounded_recognition.as_ref().unwrap();
            assert!(recognition.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert_eq!(
                report.analysis.mesh.triangles.len(),
                case["triangles"].as_u64().unwrap() as usize
            );
            assert_exhaustive_source_owners(&patches, &recognition.triangle_faces, &bytes, &o);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= o.max_deviation_mm);
            assert!(audit.result_to_source_mm <= o.max_deviation_mm);

            let faces = converted.step.result_faces().unwrap();
            assert_eq!(faces.result.faces.len(), 10);
            let sphere = assert_native_seams(&faces);
            let step_text = std::str::from_utf8(converted.step.bytes()).unwrap();
            assert_eq!(step_text.matches("SPHERICAL_SURFACE(").count(), 1);
            assert_eq!(step_text.matches("CYLINDRICAL_SURFACE(").count(), 3);
            if mode == ReconstructionMode::Auto {
                assert_native_partition_and_caps(case, &patches, &faces, &converted.step);
                if name == "d_corner" {
                    let error = converted
                        .step
                        .guided_surface(&[sphere], GuidanceFamily::Cylinder)
                        .err()
                        .expect("wrong-family guidance accepted");
                    assert_eq!(error.code, "guidance.family_unproven");
                }
            }

            let mesh = tessellate_step(converted.step.bytes(), 0.01).unwrap();
            for probe in case["material_probes"].as_array().unwrap() {
                let p = vector3(&probe["point_mm"]);
                let expected = if probe["material"].as_bool().unwrap() {
                    1.
                } else {
                    0.
                };
                for source in [&report.analysis.mesh, &mesh] {
                    let value = winding_number(source, p).abs();
                    assert!(
                        (value - expected).abs() < 1e-6,
                        "{name}/{mode:?}: {p:?} {value}"
                    );
                }
            }
        }
    }
}

#[test]
fn trihedral_refuses_open_or_warped_analytic_sources_and_respects_deviation() {
    let open = fixture("d_corner_missing_plane.stl");
    let dent = fixture("d_corner_dent.stl");
    let nominal = fixture("d_corner.stl");
    for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
        let o = options(mode);
        assert_eq!(convert_stl(&open, &o).err().unwrap().code, "mesh.open");
        let strict = Options {
            max_deviation_mm: 1e-4,
            ..o.clone()
        };
        if mode == ReconstructionMode::Auto {
            let fallback = convert_stl(&nominal, &strict)
                .expect("Auto failed to retain the source within the tighter budget");
            let report = &fallback.report;
            assert_eq!(report.cad.solids, 1);
            assert_eq!(report.cad.spherical_faces, 0);
            assert_eq!(report.cad.faces, report.cad.planar_faces);
            let proof = report.rounded_recognition.as_ref().unwrap();
            assert!(proof.model.octants.is_empty() && proof.model.spheres.is_empty());
            assert!(proof.report.max_deviation_bound_mm <= strict.max_deviation_mm);
            let audit = report.fidelity.as_ref().unwrap();
            assert!(audit.source_samples > 0 && audit.result_samples > 0);
            assert!(audit.source_to_result_mm <= strict.max_deviation_mm);
            assert!(audit.result_to_source_mm <= strict.max_deviation_mm);

            let result =
                convert_stl(&dent, &o).expect("Auto failed to retain the closed dented source");
            assert_eq!(result.report.cad.solids, 1);
            assert_eq!(result.report.cad.spherical_faces, 0);
        } else {
            assert_eq!(
                convert_stl(&nominal, &strict).err().unwrap().code,
                "recognition.deviation_exceeded"
            );
            assert_eq!(
                convert_stl(&dent, &o).err().unwrap().code,
                "recognition.surface_unproven"
            );
        }
    }
}
