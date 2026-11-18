//! Read back each STEP and verify that the source labels remain on the two
//! measured cylinders meeting at a non-orthogonal corner.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{analyze_mesh, analyze_stl, recognize_automatic, Options, ReconstructionMode};

const ACUTE: &[u8] = include_bytes!("../../../fixtures/angled_pocket/angle-65-coarse.stl");
const OBTUSE: &[u8] = include_bytes!("../../../fixtures/angled_pocket/angle-115-coarse.stl");

#[test]
fn shared_seam_survives_transfer_and_source_provenance_at_two_corner_angles() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    for (bytes, angle, volume) in [
        (ACUTE, 65_f64, 69666.47354194829),
        (OBTUSE, 115., 65333.54089293344),
    ] {
        let doc = convert_document_with_provenance(bytes, InputFormat::Stl, &o).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.planar_faces,
                cad.cylindrical_faces
            ),
            (1, 13, 11, 2)
        );
        assert!((cad.volume_mm3 - volume).abs() < 0.001);
        let body = &doc.report.bodies[0].conversion;
        let fit = body.rounded_recognition.as_ref().unwrap();
        let model = &fit.model.revolved.mixed;
        assert_eq!(model.elliptic_arcs.len(), 1);
        let ellipse = &model.elliptic_arcs[0];
        assert!((ellipse.major_mm - 2.5 / (angle.to_radians() / 2.).sin()).abs() < 2e-5);
        assert!((ellipse.minor_mm - 2.5).abs() < 2e-5);
        assert!(fit.report.max_deviation_bound_mm <= o.max_deviation_mm);
        let fidelity = body.fidelity.as_ref().unwrap();
        assert!(
            fidelity.source_to_result_mm <= o.max_deviation_mm
                && fidelity.result_to_source_mm <= o.max_deviation_mm
        );
        let faces = doc.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let walls: Vec<_> = faces
            .result
            .faces
            .iter()
            .filter(|f| f.family == ResultFaceFamily::Cylinder)
            .collect();
        assert_eq!(walls.len(), 2);
        let (sin, cos) = angle.to_radians().sin_cos();
        let mut sides = vec![[0., 1.], [sin, -cos]];
        let mut all = Vec::new();
        for row in &faces.source_correspondence.faces {
            assert!(!row.source_triangle_ids.is_empty());
            all.extend(&row.source_triangle_ids);
            if let Some(wall) = walls.iter().find(|w| w.id == row.face_id) {
                assert!(wall
                    .adjacent_face_ids
                    .iter()
                    .any(|&id| id != wall.id && walls.iter().any(|w| w.id == id)));
                let side = sides
                    .iter()
                    .position(|&[nx, ny]| {
                        row.source_triangle_ids.iter().all(|&id| {
                            let offset = 84 + 50 * id + 12;
                            bytes[offset..offset + 36].chunks_exact(12).all(|v| {
                                let [x, y, z] = std::array::from_fn(|i| {
                                    f32::from_le_bytes(v[i * 4..i * 4 + 4].try_into().unwrap())
                                        as f64
                                });
                                ((nx * x + ny * y - 2.5).hypot(z - 5.5) - 2.5).abs() < 2e-5
                            })
                        })
                    })
                    .expect("source vertices must lie on the claimed cylinder");
                sides.remove(side);
            }
        }
        assert!(sides.is_empty());
        all.sort_unstable();
        assert!(all.into_iter().copied().eq(0..(bytes.len() - 84) / 50));
    }
}

#[test]
fn a_closed_mesh_with_a_lifted_seam_knot_keeps_its_facets() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let mut mesh = analyze_stl(ACUTE, &o).unwrap().mesh;
    let a = 65_f64.to_radians();
    let id = mesh
        .vertices
        .iter()
        .position(|&[x, y, z]| {
            z > 3.15
                && z < 5.35
                && x > 0.
                && y > 0.
                && ((1. + a.cos()) * y - a.sin() * x).abs() < 1e-5
        })
        .expect("an interior source knot of the ellipse");
    mesh.vertices[id][2] += 0.14;
    let faceted = recognize_automatic(&analyze_mesh(&mesh, &o).unwrap(), &o).unwrap();
    assert!(faceted.model.revolved.mixed.elliptic_arcs.is_empty());
    assert_eq!(faceted.report.cylinders, 0);
}

#[test]
fn rediscovering_the_binary_supports_keeps_the_requested_planarity() {
    let o = Options {
        reconstruction: ReconstructionMode::Auto,
        plane_tolerance_mm: 1e-10,
        ..Options::default()
    };
    let source = analyze_stl(ACUTE, &o).unwrap();
    let result = recognize_automatic(&source, &o).unwrap();
    assert_eq!(result.report.cylinders, 0);
    assert!(result.model.revolved.mixed.elliptic_arcs.is_empty());
}
