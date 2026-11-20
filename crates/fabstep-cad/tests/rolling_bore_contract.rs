use fabstep_cad::{
    build_rolling_bore_with_patch_bound, combine_steps_with_provenance, result_faces,
    tessellate_step, StepPart,
};
use fabstep_core::{analyze_stl, rolling_bore::candidate, Options};

fn move_pole(step: &str, id: &str) -> String {
    let label = format!("{} = CARTESIAN_POINT('',(", id.trim());
    let start = step.find(&label).expect("referenced Cartesian control") + label.len();
    let stop = start + step[start..].find(',').unwrap();
    let x = step[start..stop].parse::<f64>().unwrap();
    let mut changed = step.to_string();
    changed.replace_range(start..stop, &format!("{:.12}", x + 0.007));
    changed
}

#[test]
fn substituted_periodic_surface_or_sharp_mouth_control_is_rejected() {
    let options = Options::default();
    let analysis = analyze_stl(
        include_bytes!("../../../fixtures/rolling_bore/shaft-coarse.stl"),
        &options,
    )
    .unwrap();
    let model = candidate(&analysis, &options).unwrap();
    let (native, patch_bound) = build_rolling_bore_with_patch_bound(
        &analysis.mesh,
        &model,
        1,
        options.cylinder_fit_tolerance_mm,
        options.max_deviation_mm - options.weld_tolerance_mm,
    )
    .unwrap();
    // These broad support triangles and fine patch spans have different
    // certificates. Using only the four supports would omit the larger one.
    assert!(
        patch_bound > model.support_bound_mm
            && patch_bound < options.max_deviation_mm - options.weld_tolerance_mm,
        "patch={patch_bound}, supports={}",
        model.support_bound_mm
    );
    assert_eq!(native.report.faces, 5);
    let combined = combine_steps_with_provenance(&[StepPart {
        bytes: &native.bytes,
        name: "Perçage arrondi",
    }])
    .unwrap();
    assert_eq!((combined.report.solids, combined.report.faces), (1, 5));
    let result = result_faces(&combined.bytes).unwrap();
    assert!(result.faces.iter().all(|f| f.source_face_token.is_some()));

    let step = String::from_utf8(native.bytes).unwrap();
    // The middle control row affects the interior only. All boundary
    // identities, edges and the two end sections of the blend remain intact.
    let surface = step
        .split(';')
        .find(|entity| entity.contains("B_SPLINE_SURFACE("))
        .unwrap();
    let compact: String = surface.split_whitespace().collect();
    let controls = compact.split_once("B_SPLINE_SURFACE(").unwrap().1;
    let controls = controls.split_once(",((").unwrap().1;
    let row = controls.split("),").nth(1).unwrap().trim();
    let pole = row.trim_start_matches('(').split(',').nth(6).unwrap();
    assert!(result_faces(move_pole(&step, pole).as_bytes()).is_err());

    // Keep the same material poles and all knots, but replace one positive
    // weight inside the middle row of the rational surface.
    let weights = step.find("RATIONAL_B_SPLINE_SURFACE").unwrap();
    let first_end = weights + step[weights..].find(')').unwrap() + 1;
    let row = first_end + step[first_end..].find('(').unwrap() + 1;
    let start = row + step[row..].match_indices(',').nth(5).unwrap().0 + 1;
    let end = start + step[start..].find(',').unwrap();
    let old = step[start..end].trim().parse::<f64>().unwrap();
    let mut weighted = step.clone();
    weighted.replace_range(start..end, &format!("{:.12}", old + 0.004));
    assert!(result_faces(weighted.as_bytes()).is_err());

    // All three sharp-mouth arcs use the common quintic angular trace.
    // Alter its 3D curve, keeping both native pcurves and endpoints.
    let controls = step
        .split_once("B_SPLINE_CURVE_WITH_KNOTS('',5,(")
        .unwrap()
        .1;
    let pole = controls.split(',').nth(3).unwrap();
    let altered = move_pole(&step, pole);
    assert!(result_faces(altered.as_bytes()).is_err());
    assert!(tessellate_step(altered.as_bytes(), 0.05).is_err());
    assert!(tessellate_step(step.as_bytes(), 0.05).is_ok());
    let precise = tessellate_step(step.as_bytes(), 0.000001).unwrap_err();
    assert_eq!(precise.code, "cad.tessellation_too_complex");
}
