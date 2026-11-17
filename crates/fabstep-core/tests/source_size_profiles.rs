use fabstep_core::{
    analyze_stl, recognize_automatic_guided, recognize_guided_cylinder, recognize_guided_surfaces,
    GuidanceFamily, Options, SOURCE_MAX_TRIANGLES, SOURCE_MAX_VERTICES,
};

#[test]
fn source_storage_profile_is_compile_time_and_independent_of_interactive_guidance() {
    assert_eq!(SOURCE_MAX_VERTICES, 3 * SOURCE_MAX_TRIANGLES);
    assert_eq!(
        SOURCE_MAX_TRIANGLES,
        if cfg!(feature = "large-stl") {
            600_000
        } else {
            100_000
        }
    );
    let mut a = analyze_stl(
        include_bytes!("../../../fixtures/cube.stl"),
        &Options::default(),
    )
    .unwrap();
    a.mesh.triangles.resize(100_001, [0, 1, 2]);
    let o = Options::default();
    assert_eq!(
        recognize_guided_cylinder(&a, &o, &[0]).unwrap_err().code,
        "guidance.limit"
    );
    assert_eq!(
        recognize_guided_surfaces(&a, &o, &[(GuidanceFamily::Plane, vec![0])])
            .unwrap_err()
            .code,
        "guidance.limit"
    );
    assert_eq!(
        recognize_automatic_guided(&a, &o, &[(GuidanceFamily::Auto, vec![0])])
            .unwrap_err()
            .code,
        "guidance.limit"
    );
}
