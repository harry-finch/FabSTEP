use fabstep_app::{
    analyze_document, convert_document, preview_document, selection_topology, InputFormat,
};
use fabstep_core::{InputUnit, Options, ReconstructionMode};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
    .unwrap()
}

#[test]
fn placed_instances_mirror_composition_and_disconnected_mesh_keep_exact_source_ids() {
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    for name in [
        "m3_instances.3mf",
        "m3_nested_components.3mf",
        "m3_mirror.3mf",
        "m3_disconnected_mesh.3mf",
        "post_v1_zones/instances.3mf",
    ] {
        let bytes = fixture(name);
        let preview = preview_document(&bytes, InputFormat::ThreeMf, &options).unwrap();
        let analysis = analyze_document(&bytes, InputFormat::ThreeMf, &options).unwrap();
        let topology = selection_topology(&bytes, InputFormat::ThreeMf, &options).unwrap();
        assert_eq!(topology.components.len(), analysis.bodies.len());
        let ids = topology
            .components
            .iter()
            .map(|c| &c.analysis_body_id)
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), topology.components.len());
        for (component, analyzed) in topology.components.iter().zip(&analysis.bodies) {
            assert_eq!(component.analysis_body_id, analyzed.id);
            let source = preview
                .bodies
                .iter()
                .find(|b| b.id == component.preview_body_id)
                .unwrap();
            assert_eq!(
                component.analysis_to_source_triangles.len(),
                analyzed.analysis.mesh.triangles.len()
            );
            for (i, &source_id) in component.analysis_to_source_triangles.iter().enumerate() {
                let source_points =
                    source.mesh.triangles[source_id].map(|v| source.mesh.vertices[v]);
                let analyzed_points =
                    analyzed.analysis.mesh.triangles[i].map(|v| analyzed.analysis.mesh.vertices[v]);
                for point in analyzed_points {
                    assert!(source_points.contains(&point), "{name}: exact source point");
                }
                assert!(component.source_triangle_neighbors[i]
                    .iter()
                    .all(|neighbor| component.analysis_to_source_triangles.contains(neighbor)));
            }
        }
        for source in &preview.bodies {
            let mut ids = topology
                .components
                .iter()
                .filter(|c| c.preview_body_id == source.id)
                .flat_map(|c| c.analysis_to_source_triangles.clone())
                .collect::<Vec<_>>();
            ids.sort_unstable();
            assert_eq!(ids, (0..source.mesh.triangles.len()).collect::<Vec<_>>());
        }
    }
}

#[test]
fn selection_topology_remains_available_after_recognition_or_cad_refusal() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    for (name, format, error) in [
        ("cube.stl", InputFormat::Stl, "recognition.surface_unproven"),
        (
            "m3_touching.3mf",
            InputFormat::ThreeMf,
            "cad.bodies_not_separated",
        ),
    ] {
        let bytes = fixture(name);
        assert_eq!(
            convert_document(&bytes, format, &options)
                .err()
                .unwrap()
                .code,
            error
        );
        let topology = selection_topology(&bytes, format, &options).unwrap();
        assert!(!topology.components.is_empty());
    }
}

#[test]
fn topology_error_has_body_context_and_does_not_prevent_source_preview() {
    let bytes = fixture("m3_open_mesh.3mf");
    let options = Options::default();
    let preview = preview_document(&bytes, InputFormat::ThreeMf, &options).unwrap();
    let error = selection_topology(&bytes, InputFormat::ThreeMf, &options).unwrap_err();
    assert_eq!(error.code, "mesh.open");
    assert_eq!(error.body.unwrap().id, preview.bodies[0].id);
}
