use super::*;
use std::fmt::Write;

#[test]
fn similar_meridians_require_matching_trims_and_supports() {
    let plane = Rim {
        radius: 4.,
        axial: -3.,
        end: End::Plane,
    };
    let pole = Rim {
        radius: 0.,
        axial: 5.,
        end: End::Pole,
    };
    let cap = Meridian {
        ends: [plane, pole],
        length: 10.,
    };
    let zone = Meridian {
        ends: [
            plane,
            Rim {
                radius: 1e-8,
                end: End::Plane,
                ..pole
            },
        ],
        ..cap
    };
    assert!(!cap.matches(zone, 1e-5));
    let reflected = Meridian {
        ends: [
            Rim {
                axial: -pole.axial,
                ..pole
            },
            Rim {
                axial: -plane.axial,
                ..plane
            },
        ],
        ..cap
    };
    assert!(cap.matches(reflected, 1e-5));
    let fillet = Meridian {
        ends: [
            plane,
            Rim {
                end: End::Cylinder { inner: true },
                ..plane
            },
        ],
        length: 1.6,
    };
    assert!(!fillet.matches(
        Meridian {
            length: 2.5,
            ..fillet
        },
        1e-5
    ));
    assert!(!fillet.matches(
        Meridian {
            ends: [
                plane,
                Rim {
                    end: End::Cylinder { inner: false },
                    ..plane
                }
            ],
            ..fillet
        },
        1e-5
    ));
    let full = Support {
        family: ResultFaceFamily::Sphere,
        partial: false,
        inner: false,
        radius: 5.,
        other_radius: 5.,
        height: 1.,
        meridian: None,
    };
    assert!(!full.matches(
        Support {
            partial: true,
            meridian: Some(cap),
            ..full
        },
        1e-5
    ));
    assert!(!parallel_within([0., 0., 1.], [1e-4, 0., 1.], 10., 1e-6));
    assert!(parallel_within([0., 0., 1.], [0., 0., -1.], 10., 1e-6));
}

#[test]
fn four_equal_bores_apply_together_and_leave_the_different_radius_faceted() {
    // Five cells from m2_plate_hole.stl share one top and bottom plane.
    // Internal end walls were omitted; the last hole was scaled to R3.9.
    let source = include_bytes!("../../../../../fixtures/r120-repeated-bores.stl");
    let planar =
        convert_document_with_provenance(source, InputFormat::Stl, &Options::default()).unwrap();
    assert_eq!(
        planar
            .step
            .similar_regions_controlled(&mut |_| Ok(()))
            .unwrap_err()
            .code,
        "guidance.provenance_unavailable"
    );
    let faces = planar.step.result_faces().unwrap();
    let mesh = &planar.report.bodies[0].conversion.analysis.mesh;
    let seed = faces
        .source_correspondence
        .faces
        .iter()
        .find(|row| {
            row.source_triangle_ids.iter().all(|&t| {
                mesh.triangles[t].iter().all(|&v| {
                    let [x, y, _] = mesh.vertices[v];
                    x.hypot(y) < 3.01
                })
            })
        })
        .unwrap()
        .face_id;
    let guided = planar.step.guided_cylinder(&[seed]).unwrap();
    assert_eq!(guided.report.cad.cylindrical_faces, 1);
    let found = guided
        .step
        .similar_regions_controlled(&mut |_| Ok(()))
        .unwrap();
    assert!(!found.truncated);
    assert_eq!(found.candidates.len(), 3);
    for c in &found.candidates {
        assert_eq!(c.family, "cylinder");
        assert!((c.radius_mm.unwrap() - 3.).abs() < 1e-6);
        assert!(c.face_ids.len() > 10);
    }
    assert_eq!(
        guided
            .step
            .apply_similar_controlled(&[0, 0], &mut |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "guidance.invalid_selection"
    );
    assert_eq!(
        guided
            .step
            .apply_similar_controlled(&[3], &mut |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "guidance.invalid_selection"
    );
    let bytes = guided.step.bytes().to_vec();
    let err = guided
        .step
        .apply_similar_controlled(&[1], &mut |_| Err(FabError::new("engine.stale", "annulé")))
        .err()
        .unwrap();
    assert_eq!(err.code, "engine.stale");
    assert_eq!(guided.step.bytes(), bytes);
    let all = guided
        .step
        .apply_similar_controlled(&[2, 0, 1], &mut |_| Ok(()))
        .unwrap();
    assert_eq!(all.report.cad.solids, 1);
    assert_eq!(all.report.cad.cylindrical_faces, 4);
    assert_eq!(all.report.guidance_hints.len(), 4);
    assert!(all.report.bodies[0].conversion.fidelity.is_some());
    assert_eq!(
        all.step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
    assert!(all
        .step
        .similar_regions_controlled(&mut |_| Ok(()))
        .unwrap()
        .candidates
        .is_empty());
}
#[test]
fn repeated_fragmented_planes_preserve_component_identity_and_report_capacity() {
    // Quantified pitched roof. Both planar fragments can be certified
    // together. Already-flat sides and a taller final ridge stay untouched.
    let mut input = String::from("solid roofs\n");
    for i in 0..18 {
        let ridge = if i == 1 { 0.002 } else { 2e-8 };
        let section = [
            [-0.001, 0.],
            [0.001, 0.],
            [0.001, 1.],
            [0., 1. + ridge],
            [-0.001, 1.],
        ];
        let points: Vec<_> = [-1., 1.]
            .into_iter()
            .flat_map(|y| section.map(move |[x, z]| [x + i as f64 * 3., y, z]))
            .collect();
        let mut tris = Vec::new();
        for j in 1..4 {
            tris.extend([[0, j, j + 1], [5, j + 6, j + 5]]);
        }
        for j in 0..5 {
            let k = (j + 1) % 5;
            tris.extend([[j, k + 5, k], [j, j + 5, k + 5]]);
        }
        for triangle in tris {
            input.push_str("facet normal 0 0 0\nouter loop\n");
            for v in triangle {
                let [x, y, z] = points[v];
                writeln!(input, "vertex {x} {y} {z}").unwrap();
            }
            input.push_str("endloop\nendfacet\n");
        }
    }
    input.push_str("endsolid roofs\n");
    let mesh = read_stl_mesh(input.as_bytes(), InputUnit::Mm).unwrap();
    let initial =
        convert_document_with_provenance(input.as_bytes(), InputFormat::Stl, &Options::default())
            .unwrap();
    let faces = initial.step.result_faces().unwrap();
    let roof: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .filter(|r| {
            r.analysis_body_id == "stl/component/0"
                && r.source_triangle_ids
                    .iter()
                    .all(|&t| mesh.triangles[t].iter().all(|&v| mesh.vertices[v][2] >= 1.))
        })
        .map(|r| r.face_id)
        .collect();
    assert_eq!(roof.len(), 2);
    let accepted = initial
        .step
        .guided_surface(&roof, GuidanceFamily::Plane)
        .unwrap();
    let found = accepted
        .step
        .similar_regions_controlled(&mut |_| Ok(()))
        .unwrap();
    assert_eq!(found.candidates.len(), 15);
    let accepted_faces = accepted.step.result_faces().unwrap();
    assert!(found
        .candidates
        .iter()
        .flat_map(|c| &c.face_ids)
        .all(
            |&face| accepted_faces.source_correspondence.faces[face].analysis_body_id
                != "stl/component/1"
        ));
    assert!(found.truncated);
    assert!(found
        .candidates
        .iter()
        .all(|c| c.family == "plane" && c.face_ids.len() == 2));
    let applied = accepted
        .step
        .apply_similar_controlled(&[1], &mut |_| Ok(()))
        .unwrap();
    assert_eq!(applied.report.guidance_hints.len(), 2);
    assert_eq!(applied.report.cad.faces + 1, accepted.report.cad.faces);
    assert_eq!(applied.report.cad.solids, 18);
    let smaller = applied
        .step
        .similar_regions_controlled(&mut |_| Ok(()))
        .unwrap();
    assert!(smaller.truncated);
    assert_eq!(smaller.candidates.len(), 14);
}

#[test]
fn placed_cones_spherical_caps_and_toroidal_fillets_repeat_across_bodies() {
    for (bytes, family) in [
        (
            include_bytes!("../../../../../fixtures/m4_frustum.stl").as_slice(),
            GuidanceFamily::Cone,
        ),
        (
            include_bytes!("../../../../../fixtures/m5b_small_cap.stl").as_slice(),
            GuidanceFamily::Sphere,
        ),
        (
            include_bytes!("../../../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
            GuidanceFamily::Torus,
        ),
    ] {
        let options = Options::default();
        let mesh = read_stl_mesh(bytes, InputUnit::Mm).unwrap();
        let mut input = String::from("solid pair\n");
        for shift in [0., 100.] {
            for &triangle in &mesh.triangles {
                input.push_str("facet normal 0 0 0\nouter loop\n");
                for v in triangle {
                    let [x, y, z] = mesh.vertices[v];
                    writeln!(input, "vertex {} {y} {z}", x + shift).unwrap();
                }
                input.push_str("endloop\nendfacet\n");
            }
        }
        input.push_str("endsolid pair\n");
        let first =
            convert_document_with_provenance(input.as_bytes(), InputFormat::Stl, &options).unwrap();
        let r = fabstep_core::recognize_automatic(
            &first.report.bodies[0].conversion.analysis,
            &options,
        )
        .unwrap();
        let seed = r
            .triangle_faces
            .iter()
            .position(|o| {
                matches!(
                    (family, o),
                    (GuidanceFamily::Cone, RoundedFaceRef::Cone { .. })
                        | (GuidanceFamily::Sphere, RoundedFaceRef::Sphere { .. })
                        | (GuidanceFamily::Torus, RoundedFaceRef::Torus { .. })
                )
            })
            .unwrap_or_else(|| panic!("candidat {family:?} absent"));
        let faces = first.step.result_faces().unwrap();
        let id = faces
            .source_correspondence
            .faces
            .iter()
            .find(|r| {
                r.analysis_body_id == "stl/component/0" && r.source_triangle_ids.contains(&seed)
            })
            .unwrap()
            .face_id;
        let guided = first.step.guided_surface(&[id], family).unwrap();
        let found = guided
            .step
            .similar_regions_controlled(&mut |_| Ok(()))
            .unwrap();
        assert_eq!(found.candidates.len(), 1, "{family:?}");
        let candidate = &found.candidates[0];
        assert_eq!(
            candidate.semi_angle_rad.is_some(),
            family == GuidanceFamily::Cone
        );
        let applied = guided
            .step
            .apply_similar_controlled(&[candidate.id], &mut |_| Ok(()))
            .unwrap();
        assert_eq!(applied.report.cad.solids, 2);
        let count = match family {
            GuidanceFamily::Cone => applied.report.cad.conical_faces,
            GuidanceFamily::Sphere => applied.report.cad.spherical_faces,
            GuidanceFamily::Torus => applied.report.cad.toroidal_faces,
            _ => unreachable!(),
        };
        assert_eq!(count, 2);
        assert_eq!(
            applied
                .step
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
    }
}
