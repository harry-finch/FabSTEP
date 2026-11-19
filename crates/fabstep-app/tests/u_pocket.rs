use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn both_junctions_share_the_central_wall_after_step_transfer() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (source, volume, width) in [
        (
            include_bytes!("../../../fixtures/u_pocket/u-coarse.stl").as_slice(),
            17894.797218,
            22.,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/u-fine.stl").as_slice(),
            17894.797218,
            22.,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/wide-coarse.stl").as_slice(),
            17428.748259,
            29.,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/wide-fine.stl").as_slice(),
            17428.748259,
            29.,
        ),
        (
            include_bytes!("../../../fixtures/u_pocket/narrow-fine.stl").as_slice(),
            20844.678295,
            7.,
        ),
    ] {
        let doc = convert_document_with_provenance(source, InputFormat::Stl, &o).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.cylindrical_faces,
                cad.planar_faces
            ),
            (1, 14, 3, 11)
        );
        assert!((cad.volume_mm3 - volume).abs() < 0.001);
        let conversion = &doc.report.bodies[0].conversion;
        let audit = conversion.fidelity.as_ref().unwrap();
        assert!(audit.source_samples > 0 && audit.result_samples > 0);
        assert!(
            audit.source_to_result_mm <= o.max_deviation_mm
                && audit.result_to_source_mm <= o.max_deviation_mm
        );
        let faces = doc.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let mut owners = Vec::new();
        let mut wall_groups = std::collections::BTreeSet::new();
        for owner in &faces.source_correspondence.faces {
            assert!(!owner.source_triangle_ids.is_empty());
            let f = &faces.result.faces[owner.face_id];
            let mut classes = std::collections::BTreeSet::new();
            for &id in &owner.source_triangle_ids {
                let start = 84 + 50 * id + 12;
                let points: [[f64; 3]; 3] = std::array::from_fn(|v| {
                    std::array::from_fn(|axis| {
                        let a = start + 12 * v + 4 * axis;
                        f32::from_le_bytes(source[a..a + 4].try_into().unwrap()) as f64
                    })
                });
                // Every planar triangle in this independent drawing has one
                // constant Cartesian coordinate. A wall facet varies in all three.
                let plane =
                    (0..3).any(|i| points[0][i] == points[1][i] && points[0][i] == points[2][i]);
                assert_eq!(plane, f.family == ResultFaceFamily::Plane);
                if !plane {
                    let x = points.iter().map(|q| q[0] - 5.).sum::<f64>() / 3.;
                    let y = points.iter().map(|q| q[1] - 6.).sum::<f64>() / 3.;
                    let distance = [x, width - x, y];
                    let which = (0..3)
                        .min_by(|&a, &b| distance[a].total_cmp(&distance[b]))
                        .unwrap();
                    classes.insert(which);
                }
            }
            if f.family == ResultFaceFamily::Cylinder {
                let [side]: [usize; 1] =
                    classes.into_iter().collect::<Vec<_>>().try_into().unwrap();
                assert!(wall_groups.insert(side));
                let links = f
                    .adjacent_face_ids
                    .iter()
                    .filter(|&&i| faces.result.faces[i].family == ResultFaceFamily::Cylinder)
                    .count();
                assert_eq!(links, if side == 2 { 2 } else { 1 });
            }
            owners.extend_from_slice(&owner.source_triangle_ids);
        }
        owners.sort_unstable();
        assert!(owners
            .into_iter()
            .eq(0..conversion.analysis.mesh.triangles.len()));
        assert_eq!(wall_groups.len(), 3);
    }
}

#[test]
fn forged_geometry_reusing_one_seam_or_moving_its_center_is_rejected_before_export() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let a = fabstep_core::analyze_stl(
        include_bytes!("../../../fixtures/u_pocket/u-coarse.stl"),
        &o,
    )
    .unwrap();
    let model = fabstep_core::recognize_automatic(&a, &o).unwrap().model;
    let mut collapsed = model.clone();
    collapsed.revolved.mixed.partial_cylinders[2].arcs = [0, 0];
    let mut moved = model.clone();
    moved.revolved.mixed.elliptic_arcs[0].center[0] += 0.25;
    let mut incompatible = model;
    incompatible.revolved.mixed.partial_cylinders[2].elliptic_start = false;
    for rejected in [collapsed, moved, incompatible] {
        assert_eq!(
            fabstep_cad::build_rounded_step(&rejected)
                .err()
                .unwrap()
                .code,
            "cad.invalid_model"
        );
    }
}

#[test]
fn isolated_u_and_two_wall_pockets_keep_their_own_faces_after_document_transfer() {
    let u = include_bytes!("../../../fixtures/u_pocket/u-coarse.stl").as_slice();
    let adjacent = include_bytes!("../../../fixtures/adjacent_pocket/nominal.stl").as_slice();
    let mut combined = u.to_vec();
    let u_count = u32::from_le_bytes(u[80..84].try_into().unwrap());
    let a_count = u32::from_le_bytes(adjacent[80..84].try_into().unwrap());
    combined[80..84].copy_from_slice(&(u_count + a_count).to_le_bytes());
    for raw in adjacent[84..].chunks_exact(50) {
        let mut facet = raw.to_vec();
        for start in [20, 32, 44] {
            let z = f32::from_le_bytes(raw[start..start + 4].try_into().unwrap()) + 40.;
            facet[start..start + 4].copy_from_slice(&z.to_le_bytes());
        }
        combined.extend(facet);
    }
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let doc = convert_document_with_provenance(&combined, InputFormat::Stl, &o).unwrap();
    let cad = &doc.report.cad;
    assert_eq!(
        (cad.solids, cad.planar_faces, cad.cylindrical_faces),
        (2, 22, 5)
    );
    assert!((cad.volume_mm3 - 17894.797218 - 5781.551628).abs() < 0.001);
    let faces = doc.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let mut counts = [0usize; 2];
    for owner in &faces.source_correspondence.faces {
        if faces.result.faces[owner.face_id].family == ResultFaceFamily::Cylinder {
            let second = owner.source_triangle_ids[0] >= u_count as usize;
            assert!(owner
                .source_triangle_ids
                .iter()
                .all(|&i| (i >= u_count as usize) == second));
            counts[usize::from(second)] += 1;
        }
    }
    assert_eq!(counts, [3, 2]);
}
