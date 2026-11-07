use fabstep_core::{
    analyze_mesh, analyze_mesh_bodies, analyze_stl, analyze_stl_bodies, read_stl_mesh, InputUnit,
    Mesh, Options,
};
fn cubes(count: usize) -> Mesh {
    let cube = read_stl_mesh(include_bytes!("../../../fixtures/cube.stl"), InputUnit::Mm).unwrap();
    let mut mesh = Mesh {
        vertices: Vec::new(),
        triangles: Vec::new(),
    };
    for i in 0..count {
        let offset = mesh.vertices.len();
        mesh.vertices.extend(
            cube.vertices
                .iter()
                .map(|v| [v[0] + 3. * i as f64, v[1], v[2]]),
        );
        mesh.triangles
            .extend(cube.triangles.iter().map(|t| t.map(|v| v + offset)));
    }
    mesh
}
fn ascii(mesh: &Mesh) -> Vec<u8> {
    use std::fmt::Write;
    let mut s = String::from("solid components\n");
    for t in &mesh.triangles {
        s.push_str("facet normal 0 0 0\nouter loop\n");
        for &i in t {
            let v = mesh.vertices[i];
            writeln!(s, "vertex {} {} {}", v[0], v[1], v[2]).unwrap();
        }
        s.push_str("endloop\nendfacet\n");
    }
    s.push_str("endsolid components\n");
    s.into_bytes()
}
#[test]
fn split_uses_edges_and_preserves_strict_legacy_and_units() {
    let mesh = cubes(2);
    let o = Options::default();
    let a = analyze_mesh_bodies(&mesh, &o).unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].report.volume_mm3, 1.);
    assert_eq!(a[1].report.bounds_mm[0], [3., 0., 0.]);
    assert_eq!(
        analyze_mesh(&mesh, &o).unwrap_err().code,
        "mesh.multiple_components"
    );
    let bytes = ascii(&mesh);
    assert_eq!(
        analyze_stl(&bytes, &o).unwrap_err().code,
        "mesh.multiple_components"
    );
    let inch = Options {
        input_unit: InputUnit::Inch,
        ..o
    };
    let a = analyze_stl_bodies(&bytes, &inch).unwrap();
    assert!((a[0].report.volume_mm3 - 25.4_f64.powi(3)).abs() < 1e-8);
    let preview = read_stl_mesh(&bytes, InputUnit::Inch).unwrap();
    let mm = analyze_mesh_bodies(&preview, &inch).unwrap();
    assert!((mm[0].report.volume_mm3 - a[0].report.volume_mm3).abs() < 1e-8);
}
#[test]
fn indexed_api_rejects_bad_indices_finiteness_and_component_budgets() {
    let o = Options::default();
    let mut m = cubes(1);
    m.triangles[0][0] = usize::MAX;
    assert_eq!(analyze_mesh(&m, &o).unwrap_err().code, "mesh.invalid_index");
    m = cubes(1);
    m.vertices[0][0] = f64::NAN;
    assert_eq!(
        analyze_mesh_bodies(&m, &o).unwrap_err().code,
        "mesh.non_finite"
    );
    assert_eq!(
        analyze_mesh_bodies(&cubes(33), &o).unwrap_err().code,
        "input.too_many_bodies"
    );
    m = cubes(1);
    m.triangles.pop();
    let bytes = ascii(&m);
    assert!(read_stl_mesh(&bytes, InputUnit::Mm).is_ok());
    assert_eq!(
        analyze_stl_bodies(&bytes, &o).unwrap_err().code,
        "mesh.open"
    );
}
#[test]
fn independent_component_orientation_and_weld_diagnostics_are_local() {
    let mut m = cubes(2);
    for t in &mut m.triangles[12..] {
        t.swap(1, 2);
    }
    m.vertices[0][0] += 1e-8;
    let a = analyze_mesh_bodies(&m, &Options::default()).unwrap();
    assert!(a[0]
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.vertices_moved"));
    assert!(!a[1]
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.vertices_moved"));
    assert!(!a[0]
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.orientation_reversed"));
    assert!(a[1]
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.orientation_reversed"));
}
