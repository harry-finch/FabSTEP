use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_revolved, recognize_rounded, Mesh, Options,
    ReconstructionMode,
};
use std::f64::consts::{PI, TAU};

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Primitives,
        ..Options::default()
    }
}

// Independent triangulation of an annular, piecewise linear axial profile.
// No fitted parameters, generated STEP, or production recognizer is used.
fn annular_profile(profile: &[(f64, f64)]) -> Mesh {
    varied_profile(profile, &vec![96; profile.len() + 2])
}

fn varied_profile(profile: &[(f64, f64)], counts: &[usize]) -> Mesh {
    let mut vertices = Vec::new();
    for (level, &(z, radius)) in profile.iter().enumerate() {
        let segments = counts[level];
        for i in 0..segments {
            let u = TAU * i as f64 / segments as f64;
            vertices.push([radius * u.cos(), radius * u.sin(), z]);
        }
    }
    // Close the profile through the actual bore, never via invented caps.
    for (end, z) in [profile.last().unwrap().0, profile[0].0]
        .into_iter()
        .enumerate()
    {
        let segments = counts[profile.len() + end];
        for i in 0..segments {
            let u = TAU * i as f64 / segments as f64;
            vertices.push([5. * u.cos(), 5. * u.sin(), z]);
        }
    }
    let levels = profile.len() + 2;
    let mut triangles = Vec::new();
    let mut offsets = vec![0];
    for &count in counts {
        offsets.push(offsets.last().unwrap() + count);
    }
    for level in 0..levels {
        let next = (level + 1) % levels;
        let (lower, upper) = (offsets[level], offsets[next]);
        let (n, m) = (counts[level], counts[next]);
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            let low = lower + i % n;
            let up = upper + j % m;
            if (i + 1) * m <= (j + 1) * n {
                triangles.push([low, lower + (i + 1) % n, up]);
                i += 1;
            } else {
                triangles.push([low, upper + (j + 1) % m, up]);
                j += 1;
            }
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

#[test]
fn opposed_chamfers_preserve_annular_source_volume_and_fallback() {
    let nominal = [(0., 12.), (3., 15.), (13., 15.), (17., 11.)];
    for transformed in [false, true] {
        let mut mesh = annular_profile(&nominal);
        if transformed {
            for p in &mut mesh.vertices {
                let [x, y, z] = *p;
                *p = [0.6 * x - 0.8 * z + 27., -y + 14., 0.8 * x + 0.6 * z - 9.];
            }
        }
        let o = options();
        let a = analyze_mesh(&mesh, &o).unwrap();
        let r = recognize_revolved(&a, &o).unwrap();
        assert_eq!(r.model.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.report.cones, 2);
        assert_eq!(r.report.cylinders, 2);
        assert_eq!(r.model.mixed.planes.len(), 2);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!((r.report.expected_volume_mm3 - PI * (3055.3333333333335)).abs() < 1e-8);
        assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
        assert!(r.report.max_radial_residual_mm <= o.cylinder_fit_tolerance_mm);
        // The new two-cone chain opts into the existing UV proof even
        // without a rounded face. Single-cone fallback remains unchanged.
        let rounded = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..o
        };
        let r = recognize_rounded(&a, &rounded).unwrap();
        assert_eq!(r.report.cones, 2);
        assert_eq!(r.report.spheres + r.report.tori, 0);
    }
}

#[test]
fn unresolved_lands_and_same_sign_or_flat_cones_remain_blocking() {
    for profile in [
        vec![(0., 12.), (3., 15.), (13., 15.), (17., 18.)],
        vec![(0., 12.), (3., 15.), (3.00006, 15.), (7., 11.)],
        vec![(0., 14.99997), (10., 15.), (11., 15.), (21., 14.99997)],
        vec![(0., 12.), (1., 13.2), (3., 15.), (13., 15.), (17., 11.)],
    ] {
        let o = options();
        let a = analyze_mesh(&annular_profile(&profile), &o).unwrap();
        let e = recognize_revolved(&a, &o).unwrap_err();
        assert_eq!(e.code, "recognition.conic_unproven", "{profile:?}: {e}");
        let rounded = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..o
        };
        let e = recognize_rounded(&a, &rounded).unwrap_err();
        assert_eq!(e.code, "recognition.surface_unproven", "{profile:?}: {e}");
    }
    for bytes in [
        include_bytes!("../../../fixtures/m4_alternating_taper.stl").as_slice(),
        include_bytes!("../../../fixtures/m4_piecewise_taper.stl").as_slice(),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_revolved(&a, &o).unwrap_err().code,
            "recognition.conic_unproven"
        );
        let rounded = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..o
        };
        assert_eq!(
            recognize_rounded(&a, &rounded).unwrap_err().code,
            "recognition.surface_unproven"
        );
    }
}

#[test]
fn larger_fit_does_not_resolve_a_land_and_deviation_budget_stays_independent() {
    let mesh = annular_profile(&[(0., 12.), (3., 15.), (3.6, 15.), (7.6, 11.)]);
    let o = options();
    let a = analyze_mesh(&mesh, &o).unwrap();
    assert_eq!(recognize_revolved(&a, &o).unwrap().report.cones, 2);
    let loose = Options {
        cylinder_fit_tolerance_mm: 0.1,
        ..o.clone()
    };
    let a = analyze_mesh(&mesh, &loose).unwrap();
    assert_eq!(
        recognize_revolved(&a, &loose).unwrap_err().code,
        "recognition.conic_unproven"
    );
    let rounded = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..loose
    };
    assert_eq!(
        recognize_rounded(&a, &rounded).unwrap_err().code,
        "recognition.surface_unproven"
    );
    let strict = Options {
        max_deviation_mm: 0.001,
        ..o
    };
    let a = analyze_mesh(&mesh, &strict).unwrap();
    assert_eq!(
        recognize_revolved(&a, &strict).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
}

#[test]
fn double_chamfers_allow_independent_latitude_sampling_in_surfaces() {
    let profile = [
        (0., 12.),
        (1.5, 13.5),
        (3., 15.),
        (13., 15.),
        (15., 13.),
        (17., 11.),
    ];
    let mesh = varied_profile(&profile, &[96, 48, 96, 96, 48, 96, 96, 96]);
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let a = analyze_mesh(&mesh, &o).unwrap();
    let r = recognize_rounded(&a, &o).unwrap();
    assert_eq!(r.report.cones, 2);
    assert_eq!(r.report.cylinders, 2);
    assert_eq!(r.report.spheres + r.report.tori, 0);
    assert_eq!(r.model.revolved.mixed.planes.len(), 2);
    assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
    assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
    assert!((r.report.expected_volume_mm3 - 9598.612754268013).abs() < 1e-8);
    assert!(r.report.max_deviation_bound_mm <= o.max_deviation_mm);
    assert!(r
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "recognition.conic_intent_assumed" && d.count == 2));
}
