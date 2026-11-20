use fabstep_core::{
    analyze_stl,
    oblique_band::{self, Family},
    Options,
};

#[test]
fn independently_labelled_cap_cut_and_wall_triangles_are_recovered() {
    let o = Options::default();
    for (name, family, volume) in [
        ("cone1", Family::Cone, 933.4868103465583),
        ("cone2", Family::Cone, 1486.0918540042196),
        ("sphere1", Family::Sphere, 1175.27733667),
        ("sphere3", Family::Sphere, 2298.292477105),
    ] {
        for density in ["coarse", "fine"] {
            let key = format!("../../fixtures/oblique_bands/{name}-{density}");
            let bytes = std::fs::read(format!("{key}.stl")).unwrap();
            let owners = std::fs::read(format!("{key}.owners")).unwrap();
            let a = analyze_stl(&bytes, &o).unwrap();
            let fit = oblique_band::recognize(&a, &o)
                .unwrap()
                .unwrap_or_else(|| panic!("{key}"));
            assert_eq!(fit.model.family, family);
            assert!(fit.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!((fit.model.volume() - volume).abs() < 0.001);
            assert_eq!(
                fit.triangle_faces,
                owners.into_iter().map(usize::from).collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn a_valid_undersampled_cone_or_a_dented_wall_is_declined() {
    let options = Options::default();
    let sparse = include_bytes!("../../../fixtures/oblique_bands/cone2-sparse.stl");
    let analysis = analyze_stl(sparse, &options).unwrap();
    assert!(oblique_band::recognize(&analysis, &options)
        .unwrap()
        .is_none());
    let source = include_bytes!("../../../fixtures/oblique_bands/sphere1-fine.stl");
    let mut dented = source.to_vec();
    let mut selected = None;
    for record in dented[84..].chunks_exact_mut(50) {
        for vertex in record[12..48].chunks_exact_mut(12) {
            let point: [u8; 12] = vertex.try_into().unwrap();
            let y = f32::from_le_bytes(point[4..8].try_into().unwrap());
            let z = f32::from_le_bytes(point[8..].try_into().unwrap());
            if selected.is_none() && y.abs() > 6. && (1.0..5.0).contains(&z) {
                selected = Some(point);
            }
            if selected == Some(point) {
                vertex[4..8].copy_from_slice(&(y + 0.1).to_le_bytes());
            }
        }
    }
    assert!(selected.is_some());
    let sphere = analyze_stl(&dented, &options).unwrap();
    assert!(oblique_band::recognize(&sphere, &options)
        .unwrap()
        .is_none());
}
