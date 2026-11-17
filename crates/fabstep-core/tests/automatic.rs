use fabstep_core::*;
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}
#[test]
fn automatic_proves_cylinders_cones_fillets_and_complete_surfaces() {
    for (bytes, kind) in [
        (
            include_bytes!("../../../fixtures/m2_cylinder.stl").as_slice(),
            "cylinder",
        ),
        (
            include_bytes!("../../../fixtures/m4_frustum.stl").as_slice(),
            "cone",
        ),
        (
            include_bytes!("../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
            "torus",
        ),
        (
            include_bytes!("../../../fixtures/m5d_ring_torus.stl").as_slice(),
            "torus",
        ),
        (
            include_bytes!("../../../fixtures/m5c_icosphere.stl").as_slice(),
            "sphere",
        ),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_automatic(&a, &o).unwrap();
        assert!(
            match kind {
                "cylinder" => r.report.cylinders > 0,
                "cone" => r.report.cones > 0,
                "torus" => r.report.tori > 0,
                _ => r.report.spheres > 0,
            },
            "{kind}"
        );
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(r.report.expected_volume_mm3 > 0.);
    }
}
#[test]
fn torus_region_includes_required_cylinder_but_not_planes() {
    let o = options();
    let a = analyze_stl(
        include_bytes!("../../../fixtures/m5_toroidal_fillet.stl"),
        &o,
    )
    .unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    let seed = r
        .triangle_faces
        .iter()
        .position(|f| matches!(f, RoundedFaceRef::Torus { .. }))
        .unwrap();
    let region = automatic_region(&a, &o, seed).unwrap();
    assert!(region
        .iter()
        .any(|&t| matches!(r.triangle_faces[t], RoundedFaceRef::Cylinder { .. })));
    assert!(region
        .iter()
        .all(|&t| !matches!(r.triangle_faces[t], RoundedFaceRef::Plane { .. })));
    let guided =
        recognize_automatic_guided(&a, &o, &[(GuidanceFamily::Torus, region.clone())]).unwrap();
    assert_eq!(guided.triangle_faces, r.triangle_faces);
    assert_eq!(
        guided_rounded_region(&guided, GuidanceFamily::Torus, &region).unwrap(),
        region
    );
    assert!(recognize_automatic_guided(&a, &o, &[(GuidanceFamily::Cone, region)]).is_err());
}
#[test]
fn malformed_hypotheses_and_invalid_options_are_not_fallbacks() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/cube.stl"), &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert_eq!(r.model.revolved.mixed.planes.len(), 6);
    assert_eq!(
        recognize_automatic_guided(&a, &o, &[(GuidanceFamily::Auto, vec![usize::MAX])])
            .unwrap_err()
            .code,
        "guidance.invalid_selection"
    );
    let mut bad = o.clone();
    bad.max_deviation_mm = f64::NAN;
    assert_eq!(
        recognize_automatic(&a, &bad).unwrap_err().code,
        "options.invalid_tolerance"
    );
}
#[test]
fn failed_rounded_candidate_stays_faceted() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/m5_warped_torus.stl"), &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert!(r.model.tori.is_empty());
    assert!(r
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "auto.regions_faceted" && d.count > 0));
}
#[test]
fn nonconstructible_planar_groups_split_without_changing_coordinates() {
    let o = options();
    let mut mesh = analyze_stl(include_bytes!("../../../fixtures/cube.stl"), &o)
        .unwrap()
        .mesh;
    let v = mesh
        .vertices
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a[0].total_cmp(&b[0])
                .then(a[1].total_cmp(&b[1]))
                .then(a[2].total_cmp(&b[2]))
        })
        .unwrap()
        .0;
    mesh.vertices[v][2] += 4e-7;
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
    assert!(r
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "auto.planar_regions_split"));
    assert_eq!(r.triangle_faces.len(), mesh.triangles.len());
}
#[test]
fn failed_patch_does_not_remove_an_independent_cylinder() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/m5_rounded_mixed.stl"), &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    let torus_tri = r
        .triangle_faces
        .iter()
        .position(|f| matches!(f, RoundedFaceRef::Torus { .. }))
        .unwrap();
    let mut mesh = a.mesh;
    let v = mesh.triangles[torus_tri][0];
    mesh.vertices[v][0] += 0.01;
    let changed = analyze_mesh(&mesh, &o).unwrap();
    let refused = recognize_automatic(&changed, &o).unwrap();
    assert!(refused.model.tori.is_empty());
    assert!(refused.report.cylinders > 0);
}
#[test]
fn scoped_torus_refuses_conical_neighbours_instead_of_modifying_them() {
    let o = options();
    let a = analyze_stl(include_bytes!("../../../fixtures/m5_rounded_mixed.stl"), &o).unwrap();
    let r = recognize_automatic(&a, &o).unwrap();
    let seed = r
        .triangle_faces
        .iter()
        .position(|f| matches!(f, RoundedFaceRef::Torus { .. }))
        .unwrap();
    assert_eq!(
        recognize_automatic_guided(
            &a,
            &Options::default(),
            &[(GuidanceFamily::Torus, vec![seed])]
        )
        .unwrap_err()
        .code,
        "guidance.family_unproven"
    );
}
#[test]
fn independent_partial_arcs_survive_a_local_short_arc_refusal() {
    let o = options();
    let good = analyze_stl(
        include_bytes!("../../../fixtures/short_arcs/convex.stl"),
        &o,
    )
    .unwrap();
    assert_eq!(
        recognize_automatic(&good, &o)
            .unwrap()
            .model
            .revolved
            .mixed
            .partial_cylinders
            .len(),
        3
    );
    let bad = analyze_stl(include_bytes!("../../../fixtures/short_arcs/bulge.stl"), &o).unwrap();
    let r = recognize_automatic(&bad, &o).unwrap();
    assert_eq!(r.model.revolved.mixed.partial_cylinders.len(), 2);
}
