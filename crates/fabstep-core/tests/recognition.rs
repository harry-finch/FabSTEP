use fabstep_core::{analyze_stl, recognize_extrusion, Options, ProfileLoop, ReconstructionMode};
use std::f64::consts::PI;

const CYLINDER: &[u8] = include_bytes!("../../../fixtures/m2_cylinder.stl");
const SUBDIVIDED: &[u8] = include_bytes!("../../../fixtures/m2_cylinder_subdivided.stl");
const ROTATED: &[u8] = include_bytes!("../../../fixtures/m2_cylinder_rotated.stl");
const PLATE: &[u8] = include_bytes!("../../../fixtures/m2_plate_hole.stl");
const TUBE: &[u8] = include_bytes!("../../../fixtures/m2_tube.stl");
const CONE: &[u8] = include_bytes!("../../../fixtures/m2_conical_frustum.stl");
const WAISTED: &[u8] = include_bytes!("../../../fixtures/m2_waisted.stl");

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}
fn recognize(bytes: &[u8]) -> fabstep_core::Recognition {
    let o = options();
    let a = analyze_stl(bytes, &o).unwrap();
    recognize_extrusion(&a, &o).unwrap()
}

#[test]
fn independent_cylinder_and_axial_subdivisions_become_one_cylinder() {
    for bytes in [CYLINDER, SUBDIVIDED, ROTATED] {
        let result = recognize(bytes);
        assert_eq!(result.report.cylinders, 1);
        assert!(result.extrusion.holes.is_empty());
        let ProfileLoop::Circle { radius_mm, .. } = result.extrusion.outer else {
            panic!("expected circle");
        };
        assert!((radius_mm - 10.).abs() < 1e-8);
        assert!((result.extrusion.height_mm - 20.).abs() < 1e-8);
        assert!((result.report.expected_volume_mm3 - 2000. * PI).abs() < 1e-6);
        let expected_sagitta = 10. * (1. - (PI / 64.).cos());
        assert!((result.report.max_chord_error_mm - expected_sagitta).abs() < 1e-8);
        assert!(result.report.max_deviation_bound_mm >= expected_sagitta);
        assert!(result.report.max_deviation_bound_mm < 0.013);
        let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
        assert!((norm(result.extrusion.axis) - 1.).abs() < 1e-12);
        assert!((norm(result.extrusion.x_axis) - 1.).abs() < 1e-12);
        assert!(
            result
                .extrusion
                .axis
                .iter()
                .zip(result.extrusion.x_axis)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs()
                < 1e-12
        );
    }
}

#[test]
fn plate_hole_and_tube_keep_distinct_outer_and_inner_profiles() {
    let plate = recognize(PLATE);
    let ProfileLoop::Polygon { points } = plate.extrusion.outer else {
        panic!("expected rectangle");
    };
    assert_eq!(points.len(), 4, "remove globally collinear outer vertices");
    assert_eq!(plate.extrusion.holes.len(), 1);
    let ProfileLoop::Circle { radius_mm, .. } = plate.extrusion.holes[0] else {
        panic!("expected hole");
    };
    assert!((radius_mm - 3.).abs() < 1e-10);
    assert!((plate.report.expected_volume_mm3 - (600. - 9. * PI) * 5.).abs() < 1e-7);
    let tube = recognize(TUBE);
    assert_eq!(tube.report.cylinders, 2);
    assert_eq!(tube.extrusion.holes.len(), 1);
    assert!(matches!(tube.extrusion.outer, ProfileLoop::Circle { .. }));
    assert!(matches!(
        tube.extrusion.holes[0],
        ProfileLoop::Circle { .. }
    ));
}

#[test]
fn reject_taper_and_hidden_midsection_changes() {
    let o = options();
    for bytes in [CONE, WAISTED] {
        let a = analyze_stl(bytes, &o).unwrap();
        assert_eq!(
            recognize_extrusion(&a, &o).unwrap_err().code,
            "recognition.unsupported"
        );
    }
}

#[test]
fn explicit_mode_and_deviation_budget_are_required() {
    let defaults = Options::default();
    let a = analyze_stl(CYLINDER, &defaults).unwrap();
    assert_eq!(
        recognize_extrusion(&a, &defaults).unwrap_err().code,
        "recognition.disabled"
    );
    let o = Options {
        max_deviation_mm: 0.001,
        ..options()
    };
    assert_eq!(
        recognize_extrusion(&a, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    let cube = analyze_stl(include_bytes!("../../../fixtures/cube.stl"), &o).unwrap();
    assert_eq!(
        recognize_extrusion(&cube, &o).unwrap_err().code,
        "recognition.unsupported"
    );
    let o = Options {
        cylinder_fit_tolerance_mm: f64::NAN,
        ..options()
    };
    assert_eq!(
        recognize_extrusion(&a, &o).unwrap_err().code,
        "options.invalid_tolerance"
    );
}

#[test]
fn forged_analysis_indices_fail_without_panicking() {
    let o = options();
    let mut a = analyze_stl(CYLINDER, &o).unwrap();
    a.mesh.triangles[0][0] = usize::MAX;
    assert_eq!(
        recognize_extrusion(&a, &o).unwrap_err().code,
        "recognition.invalid_analysis"
    );
}

#[test]
fn binary_f32_rotated_caps_are_regrouped_with_fit_tolerance() {
    let o = options();
    let source = analyze_stl(ROTATED, &o).unwrap();
    let mut bytes = vec![0_u8; 80];
    bytes.extend_from_slice(&(source.mesh.triangles.len() as u32).to_le_bytes());
    for triangle in &source.mesh.triangles {
        bytes.extend_from_slice(&[0; 12]);
        for &v in triangle {
            for coordinate in source.mesh.vertices[v] {
                bytes.extend_from_slice(&(coordinate as f32).to_le_bytes());
            }
        }
        bytes.extend_from_slice(&[0; 2]);
    }
    let a = analyze_stl(&bytes, &o).unwrap();
    let result = recognize_extrusion(&a, &o).unwrap();
    assert_eq!(result.report.cylinders, 1);
    assert!(result.report.max_wall_residual_mm > 1e-7);
    assert!(result.report.max_deviation_bound_mm < 0.013);
}

// Independent indexed extrusion fixture constructed only in this test. Near-cap
// rings deliberately sit inside the fit tolerance, with a radial protrusion
// hidden from the bounding box. Altitude must never replace cap membership.
fn near_cap_flange() -> Vec<u8> {
    let n = 64;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (ring, z) in [0., 3e-6, 8e-6, 20.].into_iter().enumerate() {
        for i in 0..n {
            let radius = if (ring == 1 || ring == 2) && i == 8 {
                11.
            } else {
                10.
            };
            let angle = 2. * PI * i as f64 / n as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), z]);
        }
    }
    for ring in 0..3 {
        for i in 0..n {
            let j = (i + 1) % n;
            let a = ring * n + i;
            let b = ring * n + j;
            let c = (ring + 1) * n + j;
            let d = (ring + 1) * n + i;
            triangles.extend([[a, b, c], [a, c, d]]);
        }
    }
    let bottom = vertices.len();
    vertices.push([0., 0., 0.]);
    let top = vertices.len();
    vertices.push([0., 0., 20.]);
    for i in 0..n {
        let j = (i + 1) % n;
        triangles.extend([[bottom, j, i], [top, 3 * n + i, 3 * n + j]]);
    }
    let mut text = String::from("solid flange\n");
    for triangle in triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for v in triangle {
            let p = vertices[v];
            text.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid flange\n");
    text.into_bytes()
}

#[test]
fn near_cap_flange_cannot_be_hidden_by_altitude_tolerance() {
    let o = options();
    let a = analyze_stl(&near_cap_flange(), &o).unwrap();
    assert_eq!(
        recognize_extrusion(&a, &o).unwrap_err().code,
        "recognition.unsupported"
    );
}
