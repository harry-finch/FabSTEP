use fabstep_core::{analyze_mesh, Mesh, Options};

#[test]
fn dense_prism_preserves_cap_contours_and_polygonal_volume() {
    // A single cap formerly needed over eight million pair comparisons.
    // The source is still well below the ordinary 100,000-triangle limit.
    let sides = 4096;
    let radius = 10.0;
    let height = 5.0;
    let mut vertices = Vec::with_capacity(2 * sides + 2);
    for z in [0.0, height] {
        for i in 0..sides {
            let angle = std::f64::consts::TAU * i as f64 / sides as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), z]);
        }
    }
    let bottom = vertices.len();
    vertices.push([0.0, 0.0, 0.0]);
    let top = vertices.len();
    vertices.push([0.0, 0.0, height]);
    let mut triangles = Vec::with_capacity(4 * sides);
    for i in 0..sides {
        let j = (i + 1) % sides;
        triangles.extend([
            [bottom, j, i],
            [top, i + sides, j + sides],
            [i, j, j + sides],
            [i, j + sides, i + sides],
        ]);
    }
    let source = Mesh {
        vertices,
        triangles,
    };
    let result = analyze_mesh(&source, &Options::default()).unwrap();
    assert_eq!(result.report.input_triangles, 4 * sides);
    assert_eq!(result.report.planar_faces, sides + 2);
    assert_eq!(result.model.vertices, result.mesh.vertices);
    assert_eq!(result.mesh.vertices.len(), source.vertices.len());
    let coordinates = |mesh: &Mesh| {
        mesh.vertices
            .iter()
            .map(|p| p.map(f64::to_bits))
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(coordinates(&result.mesh), coordinates(&source));
    let caps: Vec<_> = result
        .model
        .faces
        .iter()
        .filter(|f| f.loops[0].len() == sides)
        .collect();
    assert_eq!(caps.len(), 2);
    for cap in caps {
        assert_eq!(cap.loops.len(), 1);
        let z = result.model.vertices[cap.loops[0][0]][2];
        assert!(cap.loops[0].iter().all(|&i| {
            let p = result.model.vertices[i];
            p[2] == z && (p[0].hypot(p[1]) - radius).abs() < 1e-12
        }));
    }
    let expected = 0.5
        * sides as f64
        * radius
        * radius
        * (std::f64::consts::TAU / sides as f64).sin()
        * height;
    assert!((result.report.volume_mm3 - expected).abs() < expected * 1e-12);
}
