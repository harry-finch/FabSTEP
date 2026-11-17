use fabstep_core::{
    analyze_mesh_bodies, analyze_mesh_bodies_with_provenance, analyze_stl_bodies_with_provenance,
    read_stl_mesh, selection_mesh_topology, InputUnit, Mesh, Options, SourceComponentTopology,
};

fn cubes(count: usize) -> Mesh {
    let cube = read_stl_mesh(include_bytes!("../../../fixtures/cube.stl"), InputUnit::Mm).unwrap();
    let mut mesh = Mesh {
        vertices: Vec::new(),
        triangles: Vec::new(),
    };
    for i in 0..count {
        mesh.vertices.extend(
            cube.vertices
                .iter()
                .map(|p| [p[0] + i as f64 * 3., p[1], p[2]]),
        );
    }
    // Deliberately interleave every component's triangles.
    for t in &cube.triangles {
        for i in 0..count {
            mesh.triangles.push(t.map(|v| v + i * cube.vertices.len()));
        }
    }
    mesh
}

fn check_graph(component: &SourceComponentTopology) {
    let ids = &component.analysis_to_source_triangles;
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(ids.len(), component.source_triangle_neighbors.len());
    for (i, neighbors) in component.source_triangle_neighbors.iter().enumerate() {
        assert_eq!(neighbors.len(), 3);
        assert!(neighbors.windows(2).all(|pair| pair[0] < pair[1]));
        for &neighbor in neighbors {
            assert_ne!(neighbor, ids[i]);
            let j = ids.binary_search(&neighbor).unwrap();
            assert!(component.source_triangle_neighbors[j].contains(&ids[i]));
        }
    }
}

#[test]
fn exact_interleaved_provenance_survives_weld_and_orientation_without_reindexing() {
    let options = Options::default();
    let mut source = cubes(2);
    source.vertices[0][0] += 1e-8;
    for t in source.triangles.iter_mut().skip(1).step_by(2) {
        t.swap(1, 2);
    }
    let bodies = analyze_mesh_bodies_with_provenance(&source, &options).unwrap();
    let topology = selection_mesh_topology(&source, &options).unwrap();
    let legacy = analyze_mesh_bodies(&source, &options).unwrap();
    for (index, body) in bodies.iter().enumerate() {
        assert_eq!(body.topology, topology[index]);
        assert_eq!(
            body.topology.analysis_to_source_triangles,
            (index..24).step_by(2).collect::<Vec<_>>()
        );
        assert_eq!(body.analysis.mesh.vertices, legacy[index].mesh.vertices);
        assert_eq!(body.analysis.mesh.triangles, legacy[index].mesh.triangles);
        assert_eq!(body.planar_triangle_faces.len(), 12);
        let mut face_members = vec![Vec::new(); body.analysis.model.faces.len()];
        for (triangle, &face) in body.planar_triangle_faces.iter().enumerate() {
            face_members[face].push(body.topology.analysis_to_source_triangles[triangle]);
        }
        assert_eq!(face_members.len(), 6);
        for members in face_members {
            assert_eq!(members.len(), 2);
            assert!(members.iter().all(|source| source % 2 == index));
            assert_eq!(members[1] - members[0], 2);
        }
        check_graph(&body.topology);
        for (i, &source_id) in body
            .topology
            .analysis_to_source_triangles
            .iter()
            .enumerate()
        {
            let source_triangle = source.triangles[source_id].map(|v| source.vertices[v]);
            for &vertex in &body.analysis.mesh.triangles[i] {
                let point = body.analysis.mesh.vertices[vertex];
                assert!(source_triangle.iter().any(|p| p
                    .iter()
                    .zip(point)
                    .all(|(a, b)| (a - b).abs() <= options.weld_tolerance_mm)));
            }
        }
    }
    assert!(bodies[0]
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.vertices_moved"));
    assert!(bodies[1]
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.orientation_reversed"));
}

#[test]
fn stl_mm_and_inch_keep_identity_and_apply_scale_once() {
    let bytes = include_bytes!("../../../fixtures/cube.stl");
    let mm = analyze_stl_bodies_with_provenance(bytes, &Options::default()).unwrap();
    let inch_options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    let inch = analyze_stl_bodies_with_provenance(bytes, &inch_options).unwrap();
    assert_eq!(mm[0].topology, inch[0].topology);
    assert_eq!(inch[0].analysis.report.bounds_mm[1], [25.4; 3]);
    let preview = read_stl_mesh(bytes, InputUnit::Inch).unwrap();
    assert_eq!(
        selection_mesh_topology(&preview, &inch_options).unwrap()[0],
        inch[0].topology
    );
}

#[test]
fn topology_is_deterministic_exhaustive_and_bounded() {
    let options = Options::default();
    let mesh = cubes(32);
    let a = selection_mesh_topology(&mesh, &options).unwrap();
    assert_eq!(a, selection_mesh_topology(&mesh, &options).unwrap());
    let mut all = a
        .iter()
        .flat_map(|c| c.analysis_to_source_triangles.clone())
        .collect::<Vec<_>>();
    all.sort_unstable();
    assert_eq!(all, (0..mesh.triangles.len()).collect::<Vec<_>>());
    for c in &a {
        check_graph(c);
    }
    assert_eq!(
        selection_mesh_topology(&cubes(33), &options)
            .unwrap_err()
            .code,
        "input.too_many_bodies"
    );
    let mut oversized = cubes(1);
    oversized.triangles.resize(100_001, [0, 1, 2]);
    assert_eq!(
        selection_mesh_topology(&oversized, &options)
            .unwrap_err()
            .code,
        "input.too_many_triangles"
    );
    let mut open = cubes(1);
    open.triangles.pop();
    assert_eq!(
        selection_mesh_topology(&open, &options).unwrap_err().code,
        "mesh.open"
    );
}

#[test]
fn topology_does_not_require_a_successful_analysis_or_certify_a_solid() {
    // A closed tetrahedral combinatorial surface flattened into a plane. It
    // has valid manifold adjacency, but no volume and must never be exported.
    let mesh = Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
    };
    assert_eq!(
        analyze_mesh_bodies(&mesh, &Options::default())
            .unwrap_err()
            .code,
        "mesh.invalid_volume"
    );
    let components = selection_mesh_topology(&mesh, &Options::default()).unwrap();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].analysis_to_source_triangles, vec![0, 1, 2, 3]);
    check_graph(&components[0]);
}
