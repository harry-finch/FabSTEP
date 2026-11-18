use fabstep_cad::{
    build_rounded_step, build_rounded_step_with_cross_drill_provenance,
    build_rounded_step_with_smooth_revolution_provenance, result_faces, tessellate_step,
    ResultFaceFamily,
};
use fabstep_core::{
    analyze_stl, recognize_rounded, MixedCylinder, Options, ReconstructionMode, RoundedModel,
};
use std::collections::BTreeSet;

fn model() -> RoundedModel {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let analysis = analyze_stl(
        include_bytes!("../../../fixtures/smooth_revolution/bulge.stl"),
        &options,
    )
    .unwrap();
    recognize_rounded(&analysis, &options).unwrap().model
}

#[test]
fn malformed_cubic_domains_are_rejected_before_native_construction() {
    let base = model();
    for mutate in [
        (|m: &mut RoundedModel| m.smooth_revolution.as_mut().unwrap().height_mm = 0.)
            as fn(&mut RoundedModel),
        |m| m.smooth_revolution.as_mut().unwrap().axis = [0., 0., 0.],
        |m| m.smooth_revolution.as_mut().unwrap().origin[0] = f64::NAN,
        |m| {
            // A nearly unit axis must not hide a displacement when OCCT normalizes it.
            let f = m.smooth_revolution.as_mut().unwrap();
            f.axis = f.axis.map(|v| v * (1. + 4e-9));
            for end in 0..2 {
                m.revolved.mixed.rings[end].axis = f.axis;
                m.revolved.mixed.rings[end].center =
                    std::array::from_fn(|i| f.origin[i] + end as f64 * f.height_mm * f.axis[i]);
                m.revolved.mixed.planes[end].normal =
                    f.axis.map(|v| v * if end == 0 { -1. } else { 1. });
            }
        },
        |m| m.smooth_revolution.as_mut().unwrap().radii_mm[1] = 0.,
        |m| m.smooth_revolution.as_mut().unwrap().radii_mm[3] += 1.,
        |m| m.smooth_revolution.as_mut().unwrap().rings = [0, 0],
        |m| m.smooth_revolution.as_mut().unwrap().rings = [0, usize::MAX],
        |m| m.smooth_revolution.as_mut().unwrap().witness_tolerance_mm = f64::INFINITY,
        |m| m.smooth_revolution.as_mut().unwrap().witness_tolerance_mm = 10.,
        |m| m.revolved.mixed.planes[0].loops.clear(),
        |m| m.revolved.mixed.planes[0].normal = [0., 0., 0.],
        |m| m.revolved.mixed.rings[1].center[0] += 0.01,
        |m| m.revolved.mixed.rings[0].radius_mm += 0.01,
        |m| m.revolved.mixed.vertices[0][1] = f64::INFINITY,
        |m| {
            m.revolved.mixed.cylinders.push(MixedCylinder {
                rings: [0, 1],
                inner: false,
            })
        },
    ] {
        let mut wrong = base.clone();
        mutate(&mut wrong);
        let error = build_rounded_step(&wrong).unwrap_err();
        assert_eq!(error.code, "cad.invalid_model");
        assert_eq!(
            error.message,
            "Les supports et les limites de la révolution cubique sont incohérents."
        );
    }
}

#[test]
fn cubic_step_keeps_exact_owner_groups_and_tessellation_limits() {
    let model = model();
    let build = |caps: &[usize], wall: &[usize]| {
        build_rounded_step_with_smooth_revolution_provenance(
            &model,
            caps,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            wall,
        )
    };
    for (caps, wall) in [
        (&[11, 22][..], &[][..]),
        (&[11, 22][..], &[22][..]),
        (&[11][..], &[33][..]),
        (&[11, 22][..], &[100_001][..]),
    ] {
        assert_eq!(build(caps, wall).unwrap_err().code, "cad.invalid_model");
    }
    assert_eq!(
        build_rounded_step_with_cross_drill_provenance(
            &model,
            &[11, 22],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[]
        )
        .unwrap_err()
        .code,
        "cad.invalid_model"
    );
    let step = build(&[11, 22], &[33]).unwrap();
    assert_eq!(
        (
            step.report.faces,
            step.report.planar_faces,
            step.report.revolution_faces
        ),
        (3, 2, 1)
    );
    assert_eq!(
        step.report.cylindrical_faces
            + step.report.conical_faces
            + step.report.spherical_faces
            + step.report.toroidal_faces,
        0
    );
    let faces = result_faces(&step.bytes).unwrap().faces;
    assert_eq!(
        faces
            .iter()
            .map(|f| f.source_face_token.unwrap())
            .collect::<BTreeSet<_>>(),
        [11, 22, 33].into_iter().collect()
    );
    for face in faces {
        assert_eq!(
            face.family,
            if face.source_face_token == Some(33) {
                ResultFaceFamily::Revolution
            } else {
                ResultFaceFamily::Plane
            }
        );
    }
    // Corrupt a physical interior pole of the stored basis while preserving
    // endpoints and degree. A nominal surface kind alone must not admit it.
    let text = std::str::from_utf8(&step.bytes).unwrap();
    let basis = text
        .split("B_SPLINE_CURVE_WITH_KNOTS('',3,(")
        .nth(1)
        .unwrap();
    let point_id = basis.split(')').next().unwrap().split(',').nth(1).unwrap();
    let entity = text
        .split(';')
        .find(|entity| {
            entity
                .trim_start()
                .starts_with(&format!("{point_id} = CARTESIAN_POINT("))
        })
        .unwrap();
    let coordinates = entity
        .rsplit('(')
        .next()
        .unwrap()
        .trim_end_matches(')')
        .split(',')
        .map(|v| v.trim().parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    let axis = model.smooth_revolution.as_ref().unwrap().axis;
    let moved: Vec<_> = (0..3).map(|i| coordinates[i] + 0.125 * axis[i]).collect();
    let replacement = format!(
        "\n{point_id} = CARTESIAN_POINT('',({},{},{}))",
        moved[0], moved[1], moved[2]
    );
    let corrupted = text.replacen(entity, &replacement, 1);
    let error = tessellate_step(corrupted.as_bytes(), 0.01).unwrap_err();
    assert_eq!(error.code, "cad.tessellation_failed");
    assert!(error.message.contains("révolution cubique"), "{error}");
    let mesh = tessellate_step(&step.bytes, 0.01).unwrap();
    assert!(!mesh.triangles.is_empty() && mesh.triangles.len() <= 100_000);
    assert_eq!(
        tessellate_step(&step.bytes, 1e-6).unwrap_err().code,
        "cad.tessellation_too_complex"
    );
}

#[test]
fn inflected_and_mirrored_walls_keep_the_requested_chart_tessellation_accuracy() {
    let options = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    for raw in [
        include_bytes!("../../../fixtures/smooth_revolution/inflection.stl").as_slice(),
        include_bytes!("../../../fixtures/smooth_revolution/mirrored.stl").as_slice(),
    ] {
        let analysis = analyze_stl(raw, &options).unwrap();
        let recognized = recognize_rounded(&analysis, &options).unwrap();
        let f = recognized.model.smooth_revolution.as_ref().unwrap();
        let step = build_rounded_step(&recognized.model).unwrap();
        let mesh = tessellate_step(&step.bytes, 0.005).unwrap();
        let error = |p: [f64; 3]| {
            let q: [f64; 3] = std::array::from_fn(|i| p[i] - f.origin[i]);
            let z = (0..3).map(|i| q[i] * f.axis[i]).sum::<f64>();
            let rho = (0..3)
                .map(|i| (q[i] - z * f.axis[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            if z.abs() < 1e-6 {
                return (rho - f.radii_mm[0]).max(0.);
            }
            if (z - f.height_mm).abs() < 1e-6 {
                return (rho - f.radii_mm[3]).max(0.);
            }
            assert!(z > 0. && z < f.height_mm);
            let t = z / f.height_mm;
            let s = 1. - t;
            let b = f.radii_mm;
            (rho - (b[0] * s * s * s
                + 3. * b[1] * s * s * t
                + 3. * b[2] * s * t * t
                + b[3] * t * t * t))
                .abs()
        };
        for p in &mesh.vertices {
            assert!(error(*p) < 1e-6);
        }
        for ids in &mesh.triangles {
            let center =
                std::array::from_fn(|i| ids.iter().map(|&v| mesh.vertices[v][i]).sum::<f64>() / 3.);
            assert!(
                error(center) <= 0.005 + 1e-6,
                "cubic chord deviation: {}",
                error(center)
            );
        }
    }
}
