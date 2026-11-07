use fabstep_core::{analyze_stl, recognize_rounded, Options, ReconstructionMode};
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    }
}
#[test]
fn independent_spheres_and_fillets_keep_source_and_analytic_volume() {
    for (name, bytes, spheres, tori, volume) in [
        (
            "hemisphere",
            include_bytes!("../../../fixtures/m5_hemisphere.stl").as_slice(),
            1,
            0,
            452.3893421169302,
        ),
        (
            "sphere_rotated",
            include_bytes!("../../../fixtures/m5_hemisphere_rotated.stl").as_slice(),
            1,
            0,
            452.3893421169302,
        ),
        (
            "fillet",
            include_bytes!("../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
            0,
            1,
            1567.7566636691636,
        ),
        (
            "fillet_rotated",
            include_bytes!("../../../fixtures/m5_toroidal_fillet_rotated.stl").as_slice(),
            0,
            1,
            1567.7566636691636,
        ),
        (
            "mixed",
            include_bytes!("../../../fixtures/m5_rounded_mixed.stl").as_slice(),
            0,
            1,
            55936.13565206597,
        ),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let r = recognize_rounded(&a, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.report.spheres, spheres);
        assert_eq!(r.report.tori, tori);
        assert_eq!(r.model.revolved.mixed.vertices, a.mesh.vertices);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        assert!(
            (r.report.expected_volume_mm3 - volume).abs() < volume * 1e-9,
            "{name}: volume {}",
            r.report.expected_volume_mm3
        );
        assert!(
            r.report.max_deviation_bound_mm <= o.max_deviation_mm,
            "{name}"
        );
    }
}
#[test]
fn warped_sparse_and_non_tangent_sources_are_not_regularized() {
    for bytes in [
        include_bytes!("../../../fixtures/m5_warped_sphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5_sparse_sphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5_warped_torus.stl").as_slice(),
        include_bytes!("../../../fixtures/m5_non_tangent_torus.stl").as_slice(),
    ] {
        let o = options();
        let a = analyze_stl(bytes, &o).unwrap();
        let e = recognize_rounded(&a, &o).unwrap_err();
        assert!(
            matches!(
                e.code.as_str(),
                "recognition.surface_unproven" | "recognition.unsupported"
            ),
            "{e}"
        );
    }
}

#[test]
fn rounded_triangle_interiors_respect_the_reported_composed_bound() {
    for bytes in [
        include_bytes!("../../../fixtures/m5_hemisphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
    ] {
        let opts = options();
        let source = analyze_stl(bytes, &opts).unwrap();
        let result = recognize_rounded(&source, &opts).unwrap();
        let bound = result.report.max_deviation_bound_mm;
        let distance =
            |a: [f64; 3], b: [f64; 3]| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
        for (triangle, owner) in source.mesh.triangles.iter().zip(&result.triangle_faces) {
            let points = triangle.map(|v| source.mesh.vertices[v]);
            // Interior points exercise sagitta/Taylor terms, not just the
            // vertex-fitting predicate used by the implementation.
            for weights in [[0.5, 0.5, 0.], [0.2, 0.3, 0.5], [1. / 3.; 3]] {
                let point =
                    std::array::from_fn(|i| (0..3).map(|j| weights[j] * points[j][i]).sum());
                let residual = match owner {
                    fabstep_core::RoundedFaceRef::Sphere { index } => {
                        let sphere = &result.model.spheres[*index];
                        (distance(point, sphere.center) - sphere.radius_mm).abs()
                    }
                    fabstep_core::RoundedFaceRef::Torus { index } => {
                        let torus = &result.model.tori[*index];
                        let axis = result.model.revolved.mixed.rings[torus.rings.unwrap()[0]].axis;
                        let d: [f64; 3] = std::array::from_fn(|i| point[i] - torus.center[i]);
                        let z: f64 = (0..3).map(|i| d[i] * axis[i]).sum();
                        let radial: [f64; 3] = std::array::from_fn(|i| d[i] - z * axis[i]);
                        let rho = radial[0].hypot(radial[1]).hypot(radial[2]);
                        ((rho - torus.major_radius_mm).hypot(z) - torus.minor_radius_mm).abs()
                    }
                    _ => continue,
                };
                assert!(residual <= bound + 1e-12, "{residual} > {bound}");
            }
        }
    }
}

#[test]
fn rounding_fit_does_not_override_the_independent_distance_budget() {
    let opts = Options {
        cylinder_fit_tolerance_mm: 0.0001,
        max_deviation_mm: 0.001,
        ..options()
    };
    for bytes in [
        include_bytes!("../../../fixtures/m5_hemisphere.stl").as_slice(),
        include_bytes!("../../../fixtures/m5_toroidal_fillet.stl").as_slice(),
    ] {
        let source = analyze_stl(bytes, &opts).unwrap();
        assert_eq!(
            recognize_rounded(&source, &opts).unwrap_err().code,
            "recognition.deviation_exceeded"
        );
    }
}
