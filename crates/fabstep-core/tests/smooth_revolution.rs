use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_automatic_guided, recognize_rounded,
    GuidanceFamily, Options, ReconstructionMode, RoundedFaceRef,
};
use std::{fs, path::Path};

fn data(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}
fn fixture(name: &str) -> Vec<u8> {
    data(&format!("smooth_revolution/{name}"))
}
fn opts(mode: ReconstructionMode) -> Options {
    Options {
        reconstruction: mode,
        ..Options::default()
    }
}

#[test]
fn complete_source_profiles_keep_both_caps_their_rings_and_all_wall_triangles() {
    // Independent measurements from fixtures/smooth_revolution/manifest.json.
    for (name, axis, poles, vol, segments) in [
        (
            "bulge",
            [0.0, 0.0, 1.0],
            [5.0, 9.0, 9.0, 5.0],
            3129.026282975438,
            64usize,
        ),
        (
            "neck",
            [0.0, 0.0, 1.0],
            [8.0, 3.0, 3.0, 8.0],
            1979.2033717615761,
            64usize,
        ),
        (
            "inflection",
            [0.0, 0.0, 1.0],
            [4.0, 9.0, 3.0, 8.0],
            2747.7266946197547,
            64usize,
        ),
        (
            "rotated",
            [0.4444444444444444, -0.4444444444444444, -0.7777777777777778],
            [4.0, 9.0, 3.0, 8.0],
            2747.7266946197547,
            64usize,
        ),
        (
            "mirrored",
            [0.0, 0.0, -1.0],
            [4.0, 9.0, 3.0, 8.0],
            2747.7266946197547,
            64usize,
        ),
        (
            "scaled",
            [0.0, 0.0, 1.0],
            [7.5, 13.5, 13.5, 7.5],
            10560.463705042102,
            64usize,
        ),
        (
            "retessellated",
            [0.0, 0.0, 1.0],
            [8.0, 3.0, 3.0, 8.0],
            1979.2033717615761,
            80usize,
        ),
    ] {
        let a = analyze_stl(&fixture(name), &Options::default()).unwrap();
        for mode in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = opts(mode);
            let r = if mode == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{mode:?} {name}: {e}"));
            let f = r
                .model
                .smooth_revolution
                .as_ref()
                .unwrap_or_else(|| panic!("{mode:?} {name}: {:?}", r.report));
            assert_eq!(r.report.revolutions, 1);
            assert_eq!(
                r.report.cones + r.report.spheres + r.report.tori + r.report.cylinders,
                0
            );
            assert_eq!(f.rings, [0, 1]);
            assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
            assert_eq!(r.model.revolved.mixed.planes.len(), 2);
            assert_eq!(r.model.revolved.mixed.rings.len(), 2);
            let sign: f64 = (0..3).map(|i| f.axis[i] * axis[i]).sum();
            assert!(sign.abs() > 1. - 1e-10);
            for i in 0..4 {
                let j = if sign > 0. { i } else { 3 - i };
                assert!((f.radii_mm[i] - poles[j]).abs() < 1e-5);
            }
            assert!((r.report.expected_volume_mm3 - vol).abs() < vol * 1e-7);
            assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!(r.report.max_wall_residual_mm >= 1e-6);
            assert!(r
                .report
                .diagnostics
                .iter()
                .any(|d| d.code == "reconstruction.fitted_revolution"));
            let bottom = usize::from(sign < 0.);
            assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
            for (i, &face) in r.triangle_faces.iter().enumerate() {
                let expected = if i < segments {
                    RoundedFaceRef::Plane { index: bottom }
                } else if i >= a.mesh.triangles.len() - segments {
                    RoundedFaceRef::Plane { index: 1 - bottom }
                } else {
                    RoundedFaceRef::SmoothRevolution { index: 0 }
                };
                assert_eq!(face, expected);
            }
        }
    }
}
#[test]
fn off_profile_underconstrained_open_or_folded_sources_cannot_authorize_a_cubic() {
    let o = opts(ReconstructionMode::Surfaces);
    for name in [
        "sparse",
        "folded",
        "quartic",
        "incomplete_level",
        "axial_tip",
    ] {
        let a = analyze_stl(&fixture(name), &o).unwrap();
        if let Ok(r) = recognize_rounded(&a, &o) {
            assert_eq!(name, "sparse");
            assert!(r.model.smooth_revolution.is_none());
            assert_eq!((r.report.cones, r.report.cylinders), (2, 1));
        }
        let auto = recognize_automatic(&a, &opts(ReconstructionMode::Auto)).unwrap();
        assert!(auto.model.smooth_revolution.is_none(), "{name}");
    }
    assert_eq!(
        analyze_stl(&fixture("missing_cap"), &o).unwrap_err().code,
        "mesh.open"
    );
}
#[test]
fn bounded_error_local_guidance_and_previous_primitives_keep_their_contracts() {
    let mut o = opts(ReconstructionMode::Surfaces);
    let a = analyze_stl(&fixture("bulge"), &o).unwrap();
    o.max_deviation_mm = 0.0001;
    assert_eq!(
        recognize_rounded(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert!(recognize_automatic(&a, &o)
        .unwrap()
        .model
        .smooth_revolution
        .is_none());
    let auto = opts(ReconstructionMode::Auto);
    assert_eq!(
        recognize_automatic_guided(&a, &auto, &[(GuidanceFamily::Auto, vec![65])])
            .unwrap_err()
            .code,
        "guidance.family_unproven"
    );
    for name in [
        "m2_cylinder_subdivided",
        "m4_chamfered_cylinder",
        "double_torus/nominal",
    ] {
        let a = analyze_stl(&data(name), &auto).unwrap();
        let r = recognize_automatic(&a, &auto).unwrap();
        assert!(r.model.smooth_revolution.is_none(), "{name}");
        assert!(
            r.report.cylinders + r.report.cones + r.report.tori > 0,
            "{name}"
        );
    }
}

#[test]
fn planar_disk_tolerance_is_independent_of_radial_fit() {
    let o = opts(ReconstructionMode::Surfaces);
    let mut mesh = analyze_stl(&fixture("bulge"), &o).unwrap().mesh;
    let center = mesh
        .vertices
        .iter_mut()
        .find(|p| p[0].hypot(p[1]) < 1e-6 && p[2] > 19.)
        .unwrap();
    center[2] += 4e-6;
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert!(recognize_rounded(&a, &o).is_err());
    assert!(recognize_automatic(&a, &opts(ReconstructionMode::Auto))
        .unwrap()
        .model
        .smooth_revolution
        .is_none());
}

#[test]
fn near_quadratic_circular_meridian_is_not_relabelled_as_a_cubic() {
    let o = opts(ReconstructionMode::Surfaces);
    let mut mesh = analyze_stl(&fixture("bulge"), &o).unwrap().mesh;
    for p in &mut mesh.vertices {
        let r = p[0].hypot(p[1]);
        if r > 1. {
            let z = p[2] - 10.;
            // Inner band of a ring torus (major 1510 mm, minor 1500 mm).
            // Over this short arc its deviation from a parabola is <1 µm.
            let radius = 10. + z * z / (1500. + (1500_f64.powi(2) - z * z).sqrt());
            p[0] *= radius / r;
            p[1] *= radius / r;
        }
    }
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_rounded(&a, &o);
    assert!(r.is_err() || r.unwrap().model.smooth_revolution.is_none());
}
