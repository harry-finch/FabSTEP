//! Differential reference: the previous global split followed by original raw
//! per-body preparation. This test-only path deliberately repeats that work.
use super::*;

const CUBE: &[u8] = include_bytes!("../../../fixtures/cube.stl");

fn historical(
    raw: Vec<[V; 3]>,
    options: &Options,
) -> Result<Vec<crate::ProvenancedAnalysis>, FabError> {
    source_components(&raw, options)?
        .into_iter()
        .map(|topology| {
            let (analysis, planar_triangle_faces) = analyze_raw_with_provenance(
                topology
                    .analysis_to_source_triangles
                    .iter()
                    .map(|&i| raw[i])
                    .collect(),
                options,
            )?;
            Ok(crate::ProvenancedAnalysis {
                analysis,
                topology,
                planar_triangle_faces,
            })
        })
        .collect()
}

fn point_bits(points: &[V]) -> Vec<[u64; 3]> {
    points.iter().map(|p| p.map(f64::to_bits)).collect()
}

fn assert_same(
    actual: Result<Vec<crate::ProvenancedAnalysis>, FabError>,
    expected: Result<Vec<crate::ProvenancedAnalysis>, FabError>,
) {
    match (actual, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual.len(), expected.len());
            for (a, b) in actual.iter().zip(&expected) {
                assert_eq!(a.topology, b.topology);
                assert_eq!(a.planar_triangle_faces, b.planar_triangle_faces);
                let (a, b) = (&a.analysis, &b.analysis);
                assert_eq!(point_bits(&a.mesh.vertices), point_bits(&b.mesh.vertices));
                assert_eq!(a.mesh.triangles, b.mesh.triangles);
                assert_eq!(point_bits(&a.model.vertices), point_bits(&b.model.vertices));
                assert_eq!(a.model.faces.len(), b.model.faces.len());
                for (a, b) in a.model.faces.iter().zip(&b.model.faces) {
                    assert_eq!(a.normal.map(f64::to_bits), b.normal.map(f64::to_bits));
                    assert_eq!(a.loops, b.loops);
                }
                let (a, b) = (&a.report, &b.report);
                assert_eq!(
                    (
                        a.input_triangles,
                        a.vertices,
                        a.planar_faces,
                        a.welded_vertices
                    ),
                    (
                        b.input_triangles,
                        b.vertices,
                        b.planar_faces,
                        b.welded_vertices
                    )
                );
                assert_eq!(a.volume_mm3.to_bits(), b.volume_mm3.to_bits());
                assert_eq!(
                    a.bounds_mm.map(|p| p.map(f64::to_bits)),
                    b.bounds_mm.map(|p| p.map(f64::to_bits))
                );
                assert_eq!(
                    format!("{:?}", a.diagnostics),
                    format!("{:?}", b.diagnostics)
                );
            }
        }
        (Err(actual), Err(expected)) => assert_eq!(format!("{actual:?}"), format!("{expected:?}")),
        (actual, expected) => {
            panic!("preparation differs: actual={actual:?}; historical={expected:?}")
        }
    }
}

fn differential(raw: Vec<[V; 3]>, options: &Options) {
    let expected = historical(raw.clone(), options);
    let actual = analyze_bodies_with_provenance(raw, options);
    assert_same(actual, expected);
}

#[test]
fn identity_reuse_keeps_exact_mesh_report_faces_and_source_provenance() {
    for bytes in [
        CUBE,
        include_bytes!("../../../fixtures/rectangular_hole.stl").as_slice(),
        include_bytes!("../../../fixtures/m2c_open_arc_subdivided.stl").as_slice(),
        include_bytes!("../../../fixtures/m6a_rounded_box_rotated.stl").as_slice(),
    ] {
        let options = Options::default();
        let raw = read_stl_mm(bytes, &options).unwrap();
        let result = analyze_bodies_with_provenance(raw.clone(), &options).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].topology.analysis_to_source_triangles,
            (0..raw.len()).collect::<Vec<_>>()
        );
        differential(raw, &options);
    }
}

#[test]
fn identity_reuse_preserves_units_displaced_welds_representatives_and_inversion() {
    for unit in [InputUnit::Mm, InputUnit::Inch] {
        let options = Options {
            input_unit: unit,
            ..Options::default()
        };
        let raw = read_stl_mm(CUBE, &options).unwrap();
        differential(raw, &options);
    }
    let options = Options::default();
    let raw = read_stl_mm(CUBE, &options).unwrap();
    let mut displaced = raw.clone();
    displaced
        .iter_mut()
        .flatten()
        .rfind(|p| **p == [1., 1., 0.])
        .unwrap()[0] += 0.5e-6;
    let report = analyze_bodies_with_provenance(displaced.clone(), &options)
        .unwrap()
        .remove(0)
        .analysis
        .report;
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.vertices_moved" && d.count == 1));
    differential(displaced, &options);
    let mut straddling = raw.clone();
    let mut occurrence = 0;
    for p in straddling.iter_mut().flatten() {
        if *p == [1., 0., 0.] {
            p[0] += if occurrence % 2 == 0 { -0.2e-6 } else { 0.2e-6 };
            occurrence += 1;
        }
    }
    differential(straddling, &options);
    let mut inverted = raw.clone();
    for triangle in &mut inverted {
        triangle.swap(1, 2);
    }
    assert!(
        analyze_bodies_with_provenance(inverted.clone(), &options).unwrap()[0]
            .analysis
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "mesh.orientation_reversed")
    );
    differential(inverted, &options);
    let mut reordered = raw;
    reordered.reverse();
    for triangle in &mut reordered {
        triangle.rotate_left(1);
    }
    differential(reordered, &options);
}

#[test]
fn interleaved_components_retain_historical_local_preparation() {
    let options = Options::default();
    let cube = read_stl_mm(CUBE, &options).unwrap();
    let mut raw = Vec::new();
    for triangle in cube {
        for body in 0..3 {
            let mut translated = triangle.map(|p| [p[0] + 3. * body as f64, p[1], p[2]]);
            if body == 1 {
                translated.swap(1, 2);
            }
            raw.push(translated);
        }
    }
    let bodies = analyze_bodies_with_provenance(raw.clone(), &options).unwrap();
    assert_eq!(bodies.len(), 3);
    for (index, body) in bodies.iter().enumerate() {
        assert_eq!(
            body.topology.analysis_to_source_triangles,
            (index..36).step_by(3).collect::<Vec<_>>()
        );
    }
    differential(raw, &options);
}

#[test]
fn section_preparation_matches_local_welding_and_volume_for_every_body() {
    let options = Options::default();
    let mut cube = read_stl_mm(CUBE, &options).unwrap();
    cube.last_mut().unwrap()[0][0] += 0.5e-6;
    for triangle in &mut cube {
        triangle.swap(1, 2);
    }
    let interleaved = cube
        .iter()
        .flat_map(|&triangle| [triangle, triangle.map(|[x, y, z]| [x + 3., y, z])])
        .collect();
    for raw in [cube, interleaved] {
        let expected = source_components(&raw, &options).unwrap();
        let mesh = Mesh {
            triangles: (0..raw.len())
                .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
                .collect(),
            vertices: raw.iter().flatten().copied().collect(),
        };
        let actual = prepare_mesh_bodies_for_sections(&mesh, &options).unwrap();
        assert_eq!(actual.len(), expected.len());
        for (body, component) in actual.iter().zip(expected) {
            for code in ["mesh.vertices_moved", "mesh.orientation_reversed"] {
                assert!(body.diagnostics.iter().any(|d| d.code == code));
            }
            let local_raw = component
                .analysis_to_source_triangles
                .into_iter()
                .map(|i| raw[i])
                .collect();
            let (local, _) = prepare_raw(local_raw, &options, &mut || Ok(())).unwrap();
            assert_eq!(
                point_bits(&body.mesh.vertices),
                point_bits(&local.mesh.vertices)
            );
            assert_eq!(body.mesh.triangles, local.mesh.triangles);
            assert_eq!(body.volume_mm3.to_bits(), local.volume_mm3.to_bits());
            assert_eq!(
                body.bounds_mm.map(|p| p.map(f64::to_bits)),
                local.bounds_mm.map(|p| p.map(f64::to_bits))
            );
            assert_eq!(body.input_triangles, local.input_triangles);
            assert_eq!(body.welded_vertices, local.welded_vertices);
            assert_eq!(
                format!("{:?}", body.diagnostics),
                format!("{:?}", local.diagnostics)
            );
        }
    }
}

#[test]
fn invalid_topology_and_volume_keep_the_same_first_refusal() {
    let options = Options::default();
    let cube = read_stl_mm(CUBE, &options).unwrap();
    let mut cases = Vec::new();
    cases.push((Vec::new(), "mesh.empty"));
    let mut open = cube.clone();
    open.pop();
    cases.push((open, "mesh.open"));
    let mut duplicate = cube.clone();
    duplicate.push(duplicate[0]);
    cases.push((duplicate, "mesh.duplicate_triangle"));
    let mut orientation = cube.clone();
    orientation[0].swap(1, 2);
    cases.push((orientation, "mesh.inconsistent_orientation"));
    let mut edge = cube.clone();
    edge.push([edge[0][0], edge[0][1], [0.5; 3]]);
    cases.push((edge, "mesh.non_manifold_edge"));
    let mut pinched = cube.clone();
    pinched.extend(cube.iter().map(|t| t.map(|p| p.map(|c| c + 1.))));
    cases.push((pinched, "mesh.non_manifold_vertex"));
    let mut nonfinite = cube.clone();
    nonfinite[0][0][0] = f64::NAN;
    cases.push((nonfinite, "mesh.non_finite"));
    let flat = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
    cases.push((
        [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
            .map(|t| t.map(|v| flat[v]))
            .to_vec(),
        "mesh.invalid_volume",
    ));
    for (raw, code) in cases {
        assert_eq!(
            analyze_bodies_with_provenance(raw.clone(), &options)
                .unwrap_err()
                .code,
            code
        );
        differential(raw, &options);
    }
    differential(
        cube,
        &Options {
            weld_tolerance_mm: 0.,
            ..options
        },
    );
}

#[test]
fn retained_preparation_and_volume_finish_preserve_cooperative_cancellation() {
    let options = Options::default();
    let raw = read_stl_mm(CUBE, &options).unwrap();
    for stop in 1..=8 {
        let mut calls = 0;
        let error = prepare_source_components_controlled(&raw, &options, &mut || {
            calls += 1;
            if calls == stop {
                Err(FabError::new("test.cancelled", "cancelled"))
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
        assert_eq!(error.code, "test.cancelled");
    }
    for stop in 1..=3 {
        let SourcePreparation {
            mesh,
            welded,
            adjacency,
            ..
        } = prepare_source_components_controlled(&raw, &options, &mut || Ok(())).unwrap();
        let mut calls = 0;
        let error = finish_preparation(mesh, welded, raw.len(), adjacency, &mut || {
            calls += 1;
            if calls == stop {
                Err(FabError::new("test.cancelled", "cancelled"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error.code, "test.cancelled");
    }
}

#[test]
#[ignore = "set FABSTEP_PREPARATION_BENCH_STL to an explicit STL path; use source-large for files above 100k"]
fn benchmark_single_component_reuse_against_historical_preparation() {
    let path =
        std::env::var_os("FABSTEP_PREPARATION_BENCH_STL").expect("explicit STL path required");
    let bytes = std::fs::read(path).unwrap();
    let options = Options::default();
    let raw = read_stl_mm(&bytes, &options).unwrap();
    let count = raw.len();
    let started = std::time::Instant::now();
    let expected = historical(raw.clone(), &options);
    let historical_time = started.elapsed();
    let started = std::time::Instant::now();
    let actual = analyze_bodies_with_provenance(raw, &options);
    let reused_time = started.elapsed();
    eprintln!("triangles={count}; order=historical_then_reused; historical={historical_time:?}; reused={reused_time:?}; successful={}", actual.is_ok());
    assert_same(actual, expected);
}
