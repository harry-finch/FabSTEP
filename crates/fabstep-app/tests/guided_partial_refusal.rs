//! Private-model regression for the 144-triangle rounded slot ends reported
//! on 2026-11-16. All four cylinder/chamfer/cylinder chains must be proven
//! jointly, natively validated and retain exact source correspondence.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{GuidanceFamily, Options, ReconstructionMode};
use std::collections::BTreeSet;

#[test]
#[ignore = "private body 26mm.stl; set FABSTEP_BODY26_STL and run explicitly"]
fn body26_partial_cylinder_chamfer_junction_is_proven_jointly() {
    let path = std::env::var("FABSTEP_BODY26_STL").expect("set FABSTEP_BODY26_STL");
    let bytes = std::fs::read(path).unwrap();
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let base = convert_document_with_provenance(&bytes, InputFormat::Stl, &options).unwrap();
    assert_eq!(base.report.cad.solids, 1);
    assert_eq!(base.report.cad.cylindrical_faces, 14);
    assert_eq!(base.report.cad.conical_faces, 7);
    assert_eq!(base.report.bodies.len(), 1);
    let analysis = &base.report.bodies[0].conversion.analysis;
    assert_eq!(analysis.mesh.triangles.len(), 18_248);
    let topology = fabstep_app::selection_topology(&bytes, InputFormat::Stl, &options).unwrap();
    let component = &topology.components[0];
    let recognition = fabstep_core::recognize_automatic(analysis, &options).unwrap();
    let inspected = base.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    eprintln!("baseline native: {:?}", base.report.cad);

    for center_x in [-72., -53., -25., -5.] {
        let selection: Vec<_> = analysis
            .mesh
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(i, triangle)| {
                triangle
                    .iter()
                    .all(|&v| {
                        let [x, y, z] = analysis.mesh.vertices[v];
                        ((x - center_x).hypot(y + 25.) - 2.5).abs() < 1e-5 && z >= 3. - 1e-6
                    })
                    .then_some(i)
            })
            .collect();
        assert_eq!(selection.len(), 144);
        let min_z = selection
            .iter()
            .flat_map(|&t| analysis.mesh.triangles[t])
            .map(|v| analysis.mesh.vertices[v][2])
            .fold(f64::INFINITY, f64::min);
        let max_z = selection
            .iter()
            .flat_map(|&t| analysis.mesh.triangles[t])
            .map(|v| analysis.mesh.vertices[v][2])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_z - 4.).abs() < 1e-6);
        assert!(
            (min_z
                - if center_x == -72. || center_x == -5. {
                    3.
                } else {
                    3.3
                })
            .abs()
                < 1e-6
        );
        let group =
            fabstep_core::guided_rounded_region(&recognition, GuidanceFamily::Cylinder, &selection)
                .unwrap();
        assert_eq!(group.len(), 432);
        assert!(selection.iter().all(|t| group.contains(t)));
        let source_triangles: BTreeSet<_> = selection
            .iter()
            .map(|&t| component.analysis_to_source_triangles[t])
            .collect();
        eprintln!("wall center=({center_x},-25), radius=2.5, z=[{min_z},{max_z}], source={source_triangles:?}");
        // No native face can be cut to manufacture a convenient selection.
        let faces: Vec<_> = inspected
            .source_correspondence
            .faces
            .iter()
            .filter(|face| {
                face.source_triangle_ids
                    .iter()
                    .all(|t| source_triangles.contains(t))
            })
            .map(|face| face.face_id)
            .collect();
        let covered: BTreeSet<_> = inspected
            .source_correspondence
            .faces
            .iter()
            .filter(|face| faces.contains(&face.face_id))
            .flat_map(|face| face.source_triangle_ids.iter().copied())
            .collect();
        assert_eq!(covered, source_triangles);
        if center_x == -25. {
            let guided = base
                .step
                .guided_surface(&faces, GuidanceFamily::Cylinder)
                .unwrap();
            assert_eq!(guided.report.cad.solids, 1);
            assert_eq!(guided.report.cad.cylindrical_faces, 14);
            assert_eq!(guided.report.cad.conical_faces, 7);
            assert_eq!(
                guided.report.guidance_hints[0]
                    .selected_source_triangle_ids
                    .len(),
                144
            );
            assert_eq!(
                guided.report.guidance_hints[0]
                    .reconstructed_source_triangle_ids
                    .len(),
                432
            );
            assert_eq!(
                guided
                    .step
                    .result_faces()
                    .unwrap()
                    .source_correspondence
                    .status,
                "exact"
            );
            eprintln!("guided native: {:?}", guided.report.cad);
            if let Ok(directory) = std::env::var("FABSTEP_DIAGNOSTIC_DIR") {
                let directory = std::path::Path::new(&directory);
                std::fs::write(directory.join("body26-chain-auto.step"), base.step.bytes())
                    .unwrap();
                std::fs::write(
                    directory.join("body26-chain-guided.step"),
                    guided.step.bytes(),
                )
                .unwrap();
                std::fs::write(
                    directory.join("body26-chain-auto.json"),
                    serde_json::to_vec_pretty(&base.report).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    directory.join("body26-chain-guided.json"),
                    serde_json::to_vec_pretty(&guided.report).unwrap(),
                )
                .unwrap();
            }
        }
    }
}
