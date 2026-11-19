use fabstep_core::{analyze_stl, recognize_automatic, Options, ReconstructionMode};

#[test]
fn three_walls_own_two_ellipses_and_recover_the_design_volume() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (bytes, expected) in [
        (
            include_bytes!("../../../fixtures/u_pocket/u-coarse.stl").as_slice(),
            17894.797218,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/u-fine.stl").as_slice(),
            17894.797218,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/wide-coarse.stl").as_slice(),
            17428.748259,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/wide-fine.stl").as_slice(),
            17428.748259,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/narrow-coarse.stl").as_slice(),
            20844.678295,
        ),
    ] {
        let analysis = analyze_stl(bytes, &o).unwrap();
        let g = recognize_automatic(&analysis, &o).unwrap();
        let m = &g.model.revolved.mixed;
        assert_eq!(
            (
                m.planes.len(),
                m.partial_cylinders.len(),
                m.elliptic_arcs.len()
            ),
            (11, 3, 2)
        );
        assert!((g.report.expected_volume_mm3 - expected).abs() < 0.001);
        assert_eq!(g.triangle_faces.len(), analysis.mesh.triangles.len());
        assert!(g.report.max_deviation_bound_mm <= o.max_deviation_mm);
    }
}

#[test]
fn a_notch_interrupting_one_shared_edge_never_proves_a_partial_network() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/u_pocket/notched-coarse.stl"),
        &o,
    )
    .unwrap();
    let g = recognize_automatic(&a, &o).unwrap();
    assert!(g.model.revolved.mixed.partial_cylinders.is_empty());
}

#[test]
fn the_entire_network_survives_a_rigid_reflection_and_rejects_a_damaged_seam() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/u_pocket/u-coarse.stl"),
        &o,
    )
    .unwrap();
    let mut reflected = a.mesh.clone();
    for q in &mut reflected.vertices {
        let [x, y, z] = *q;
        *q = [
            4. - x,
            (3. * y + 4. * z) / 5. - 7.,
            (-4. * y + 3. * z) / 5. + 11.,
        ];
    }
    for t in &mut reflected.triangles {
        t.swap(0, 1);
    }
    reflected.triangles.reverse();
    let fitted =
        recognize_automatic(&fabstep_core::analyze_mesh(&reflected, &o).unwrap(), &o).unwrap();
    assert_eq!(fitted.model.revolved.mixed.partial_cylinders.len(), 3);
    assert!((fitted.report.expected_volume_mm3 - 17894.797218).abs() < 0.001);
    let mut warped = a.mesh;
    let knot = warped
        .vertices
        .iter_mut()
        .find(|[x, y, z]| (*x - 5. - (*y - 6.)).abs() < 1e-5 && *z > 3.2 && *z < 5.)
        .unwrap();
    knot[0] += 0.06;
    let fallback =
        recognize_automatic(&fabstep_core::analyze_mesh(&warped, &o).unwrap(), &o).unwrap();
    assert!(fallback.model.revolved.mixed.partial_cylinders.is_empty());
}
