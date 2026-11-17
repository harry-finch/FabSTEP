use fabstep_core::{analyze_mesh, recognize_mixed, Mesh, Options, ReconstructionMode};

// Independent D prism. Its top cap remains planar: z = 7 + tilt * x / 5.
// A shifted and tilted top has endpoint errors in two orthogonal directions.
fn prism(shift: f64, tilt: f64, rotated: bool) -> Mesh {
    let profile: Vec<_> = (0..=24)
        .map(|i| {
            let angle = std::f64::consts::PI * i as f64 / 24.;
            [5. * angle.cos(), 5. * angle.sin()]
        })
        .collect();
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (i, &[x, y]) in profile.iter().enumerate() {
        let [u, v] = profile[(i + 1) % profile.len()];
        let a = [x, y, 0.];
        let b = [u, v, 0.];
        let c = [x + shift, y, 7. + tilt * x / 5.];
        let d = [u + shift, v, 7. + tilt * u / 5.];
        for triangle in [
            [[0., 1., 0.], b, a],
            [[shift, 1., 7.], c, d],
            [a, b, d],
            [a, d, c],
        ] {
            let start = vertices.len();
            triangles.push([start, start + 1, start + 2]);
            vertices.extend(triangle.map(|[x, y, z]| {
                if !rotated {
                    return [x, y, z];
                }
                let (sx, cx) = 0.37_f64.sin_cos();
                let (sy, cy) = (-0.51_f64).sin_cos();
                let (y, z) = (cx * y - sx * z, sx * y + cx * z);
                [cy * x + sy * z + 4., y + 8., -sy * x + cy * z + 12.]
            }));
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Cylinders,
        ..Options::default()
    }
}

#[test]
fn combined_endpoint_errors_are_refused_before_native_construction() {
    for rotated in [false, true] {
        // Both components satisfy the former separate 1e-7 guards, but their
        // Euclidean length exceeds the native endpoint matching tolerance.
        let a = analyze_mesh(&prism(0.8e-7, 0.8e-7, rotated), &options()).unwrap();
        let error = recognize_mixed(&a, &options()).unwrap_err();
        assert_eq!(error.code, "recognition.partial_unproven");
    }
}

#[test]
fn endpoint_acceptance_keeps_the_existing_native_tolerance() {
    for rotated in [false, true] {
        for (shift, tilt) in [(0., 0.), (0.6e-7, 0.6e-7), (0.95e-7, 0.), (0., 0.95e-7)] {
            let a = analyze_mesh(&prism(shift, tilt, rotated), &options()).unwrap();
            let recognized = recognize_mixed(&a, &options()).unwrap();
            assert_eq!(recognized.model.partial_cylinders.len(), 1);
            assert_eq!(recognized.model.vertices, a.mesh.vertices);
        }
    }
}
