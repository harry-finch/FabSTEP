use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ReconstructionMode};

#[test]
fn a_deformed_oblique_end_wall_keeps_a_faithful_faceted_pocket() {
    let document = convert_document_with_provenance(
        include_bytes!("../../../fixtures/oblique_pocket/warped_stop.stl"),
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        },
    )
    .unwrap();
    let cad = &document.report.cad;
    assert_eq!(cad.solids, 1);
    assert_eq!(cad.faces, cad.planar_faces);
    let faces = document.step.result_faces().unwrap();
    let source = &document.report.bodies[0].conversion.analysis;
    assert!((cad.volume_mm3 - source.report.volume_mm3).abs() < 1e-5);
    assert_eq!(faces.source_correspondence.status, "exact");
    let mut triangles: Vec<_> = faces
        .source_correspondence
        .faces
        .iter()
        .flat_map(|face| face.source_triangle_ids.iter().copied())
        .collect();
    triangles.sort_unstable();
    assert!(triangles.into_iter().eq(0..source.mesh.triangles.len()));
    let comparison = fabstep_verify::compare_meshes(
        &source.mesh,
        &faces.result.mesh,
        faces.result.deflection_mm,
    )
    .unwrap();
    assert!(comparison.source_to_result_mm < 1e-5 && comparison.result_to_source_mm < 1e-5);
}
