use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{Options, ReconstructionMode};

const STL: &[u8] = include_bytes!("../../../fixtures/pierced_tray/rim_and_divider.stl");

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn source_faces_trace_both_pockets_the_rim_and_the_through_bore() {
    let o = options();
    let doc = convert_document_with_provenance(STL, InputFormat::Stl, &o).unwrap();
    let cad = &doc.report.cad;
    assert_eq!(
        (
            cad.solids,
            cad.faces,
            cad.planar_faces,
            cad.cylindrical_faces
        ),
        (1, 38, 21, 17)
    );
    assert!((cad.volume_mm3 - 13755.66058222074).abs() < 0.001);
    let audit = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
    assert!(
        audit.source_to_result_mm <= o.max_deviation_mm
            && audit.result_to_source_mm <= o.max_deviation_mm
    );
    let faces = doc.step.result_faces().unwrap();
    assert_eq!(faces.source_correspondence.status, "exact");
    let triangles = (STL.len() - 84) / 50;
    let mut seen = vec![false; triangles];
    let mut supports = vec![(-12.5_f64, 1_f64, 1.6_f64, 0_f64, 2_f64)];
    for (w, d, r, x, z0, z1) in [
        (58., 32., 4., 0., 0., 9.),
        (62., 36., 5.5, 0., 9., 12.),
        (22., 24., 2., -13.5, 2., 12.),
        (22., 24., 2.5, 13.5, 4., 12.),
    ] {
        for sx in [-1., 1.] {
            for sy in [-1., 1.] {
                supports.push((x + sx * (w / 2. - r), sy * (d / 2. - r), r, z0, z1));
            }
        }
    }
    for row in &faces.source_correspondence.faces {
        assert!(!row.source_triangle_ids.is_empty());
        let mut vertices = Vec::new();
        for &id in &row.source_triangle_ids {
            assert!(!std::mem::replace(&mut seen[id], true));
            let offset = 84 + 50 * id + 12;
            for v in STL[offset..offset + 36].chunks_exact(12) {
                let p: [f64; 3] = std::array::from_fn(|i| {
                    f32::from_le_bytes(v[4 * i..4 * i + 4].try_into().unwrap()) as f64
                });
                vertices.push(p);
            }
        }
        if faces.result.faces[row.face_id].family == ResultFaceFamily::Cylinder {
            let index = supports
                .iter()
                .position(|&(x, y, r, z0, z1)| {
                    vertices.iter().all(|p| {
                        ((p[0] - x).hypot(p[1] - y) - r).abs() < 2e-5 && p[2] >= z0 && p[2] <= z1
                    }) && vertices.iter().any(|p| p[2] == z0)
                        && vertices.iter().any(|p| p[2] == z1)
                })
                .expect("each cylindrical face must own its source wall");
            supports.remove(index);
        }
    }
    assert!(supports.is_empty() && seen.into_iter().all(|s| s));
}

#[test]
fn a_missing_wall_triangle_cannot_close_either_compartment() {
    let mut open = STL[..STL.len() - 50].to_vec();
    let n = ((open.len() - 84) / 50) as u32;
    open[80..84].copy_from_slice(&n.to_le_bytes());
    let err = convert_document_with_provenance(&open, InputFormat::Stl, &options())
        .err()
        .expect("an open tray must not export");
    assert_eq!(err.code, "mesh.open");
}
