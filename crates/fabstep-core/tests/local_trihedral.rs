use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_automatic_guided, recognize_rounded,
    GuidanceFamily, Options, ReconstructionMode, RoundedFaceRef,
};
use std::{collections::BTreeSet, fs, path::Path};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/local_blends")
            .join(format!("{name}.stl")),
    )
    .unwrap()
}

fn distance(p: [f64; 3], q: [f64; 3]) -> f64 {
    (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt()
}

#[test]
fn folded_interior_with_all_vertices_on_the_sphere_is_rejected() {
    let surfaces = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut mesh = analyze_stl(&fixture("d_corner"), &surfaces).unwrap().mesh;
    let &[a, b, _] = mesh
        .triangles
        .iter()
        .find(|ids| {
            ids.iter().all(|&v| {
                let p = mesh.vertices[v];
                p.iter().all(|&x| x < 1.6) && (distance(p, [2.; 3]) - 2.).abs() < 1e-6
            })
        })
        .expect("nominal fixture has a spherical triangle clear of every trim");
    // Keep the trim and every radial residual intact, reversing the two
    // incident chart triangles by swapping the positions across their edge.
    mesh.vertices.swap(a, b);
    let folded = analyze_mesh(&mesh, &surfaces).expect("mesh still has closed manifold incidence");
    assert_eq!(folded.mesh.triangles.len(), mesh.triangles.len());
    let error = recognize_rounded(&folded, &surfaces).unwrap_err();
    assert_eq!(error.code, "recognition.surface_unproven");
    assert!(error.message.contains("triangle replié"), "{error}");

    let auto = Options {
        reconstruction: ReconstructionMode::Auto,
        ..surfaces
    };
    assert!(recognize_automatic(&folded, &auto)
        .unwrap()
        .model
        .octants
        .is_empty());
}

#[test]
fn three_source_quarters_bound_the_unique_spherical_octant() {
    for (name, sphere_triangles, cylinder_triangles) in [
        ("d_corner", 256, 96),
        ("d_corner_mirror", 256, 96),
        ("d_corner_permuted", 400, 160),
        ("d_corner_oblique", 576, 240),
    ] {
        let bytes = fixture(name);
        for reconstruction in [ReconstructionMode::Auto, ReconstructionMode::Surfaces] {
            let o = Options {
                reconstruction,
                ..Options::default()
            };
            let a = analyze_stl(&bytes, &o).unwrap();
            let rec = if reconstruction == ReconstructionMode::Auto {
                recognize_automatic(&a, &o)
            } else {
                recognize_rounded(&a, &o)
            }
            .unwrap_or_else(|e| panic!("{name}/{reconstruction:?}: {e}"));
            let mixed = &rec.model.revolved.mixed;
            let [octant] = rec.model.octants.as_slice() else {
                panic!("{name}: missing unique octant");
            };
            assert_eq!(mixed.vertices, a.mesh.vertices);
            assert_eq!(mixed.planes.len(), 6);
            assert_eq!(mixed.arcs.len(), 6);
            assert_eq!(mixed.partial_cylinders.len(), 3);
            assert_eq!(rec.report.spheres, 1);
            assert_eq!(rec.report.cylinders, 3);
            assert!(rec.report.max_deviation_bound_mm <= o.max_deviation_mm);
            assert!((rec.report.expected_volume_mm3 - 3161.8524963483783).abs() < 1e-5);
            assert!((octant.radius_mm - 2.).abs() < 1e-6 && !octant.inner);
            let mut section_arcs = BTreeSet::new();
            for (i, &(index, reversed)) in octant.arc_uses.iter().enumerate() {
                assert!(section_arcs.insert(index));
                let arc = &mixed.arcs[index];
                assert!(distance(arc.center, octant.center) < 1e-7);
                assert!((arc.sweep_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
                let mut endpoints = arc.vertices;
                if reversed {
                    endpoints.reverse();
                }
                for (j, &id) in endpoints.iter().enumerate() {
                    let d = octant.directions[(i + j) % 3];
                    let expected =
                        std::array::from_fn(|k| octant.center[k] + octant.radius_mm * d[k]);
                    assert!(distance(mixed.vertices[id], expected) < 1e-7);
                }
                assert_eq!(
                    mixed
                        .partial_cylinders
                        .iter()
                        .filter(|c| c.arcs.contains(&index))
                        .count(),
                    1
                );
            }
            assert_eq!(rec.triangle_faces.len(), a.mesh.triangles.len());
            let mut planes = [0; 6];
            let mut cylinders = [0; 3];
            let mut sphere = 0;
            for owner in &rec.triangle_faces {
                match *owner {
                    RoundedFaceRef::Plane { index } => planes[index] += 1,
                    RoundedFaceRef::PartialCylinder { index } => cylinders[index] += 1,
                    RoundedFaceRef::SphericalOctant { index: 0 } => sphere += 1,
                    _ => panic!("Unexpected {name} source owner {owner:?}"),
                }
            }
            assert!(planes.iter().all(|&count| count > 0));
            assert_eq!(sphere, sphere_triangles);
            assert_eq!(cylinders, [cylinder_triangles; 3]);
        }
    }
}

#[test]
fn missing_or_dented_source_and_too_tight_budget_never_gain_an_octant() {
    let surfaces = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    assert_eq!(
        analyze_stl(&fixture("d_corner_missing_plane"), &surfaces)
            .unwrap_err()
            .code,
        "mesh.open"
    );
    let a = analyze_stl(&fixture("d_corner_dent"), &surfaces).unwrap();
    assert_eq!(
        recognize_rounded(&a, &surfaces).unwrap_err().code,
        "recognition.surface_unproven"
    );
    let auto = Options {
        reconstruction: ReconstructionMode::Auto,
        ..surfaces.clone()
    };
    assert!(recognize_automatic(&a, &auto)
        .unwrap()
        .model
        .octants
        .is_empty());
    let a = analyze_stl(&fixture("d_corner"), &surfaces).unwrap();
    let tight = Options {
        max_deviation_mm: 1e-4,
        ..surfaces
    };
    assert_eq!(
        recognize_rounded(&a, &tight).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert_eq!(
        recognize_automatic_guided(&a, &auto, &[(GuidanceFamily::Cylinder, vec![0])])
            .unwrap_err()
            .code,
        "guidance.family_unproven"
    );
}
