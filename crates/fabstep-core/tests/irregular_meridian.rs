use fabstep_core::{
    analyze_mesh, analyze_stl, recognize_automatic, recognize_rounded, Options, ReconstructionMode,
};
const BULGE: &[u8] = include_bytes!("../../../fixtures/irregular_revolution/bulged-coarse.stl");
const ABOVE: &[u8] = include_bytes!("../../../fixtures/irregular_revolution/inflected-fine.stl");

#[test]
fn unsupported_samples_and_insufficient_budget_do_not_authorize_a_revolution() {
    let mut o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut mesh = analyze_stl(BULGE, &o).unwrap().mesh;
    let v = mesh
        .vertices
        .iter_mut()
        .find(|p| p[2] > 8. && p[2] < 12.)
        .unwrap();
    v[0] *= 1.003;
    v[1] *= 1.003;
    let off = analyze_mesh(&mesh, &o).unwrap();
    assert!(recognize_rounded(&off, &o).map_or(true, |r| r.model.smooth_revolution.is_none()));
    let over = analyze_stl(ABOVE, &o).unwrap();
    assert_eq!(
        recognize_rounded(&over, &o).unwrap_err().code,
        "recognition.deviation_exceeded"
    );
    assert!(recognize_automatic(
        &over,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..o.clone()
        }
    )
    .unwrap()
    .model
    .smooth_revolution
    .is_none());
    o.max_deviation_mm = 0.001;
    assert_eq!(
        recognize_rounded(&analyze_stl(BULGE, &o).unwrap(), &o)
            .unwrap_err()
            .code,
        "recognition.deviation_exceeded"
    );
}
#[test]
fn crossed_facets_on_the_exact_meridian_fail_radial_coverage() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let mut mesh = analyze_stl(BULGE, &o).unwrap().mesh;
    let start = mesh
        .vertices
        .iter()
        .position(|p| p[2] > 5. && p[2] < 12.)
        .unwrap();
    let a = mesh.vertices[start];
    let nearest = mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(i, p)| *i != start && (p[2] - a[2]).abs() < 1e-6)
        .min_by(|(_, p), (_, q)| {
            (p[0] - a[0])
                .hypot(p[1] - a[1])
                .total_cmp(&(q[0] - a[0]).hypot(q[1] - a[1]))
        })
        .unwrap()
        .0;
    mesh.vertices.swap(start, nearest);
    let damaged = analyze_mesh(&mesh, &o).unwrap();
    assert!(recognize_rounded(&damaged, &o).map_or(true, |r| r.model.smooth_revolution.is_none()));
}

#[test]
fn an_irregular_remeshed_cusp_cannot_acquire_a_tangent_joint() {
    let o = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Options::default()
    };
    let a = analyze_stl(
        include_bytes!("../../../fixtures/irregular_revolution/kinked-coarse.stl"),
        &o,
    )
    .unwrap();
    assert!(recognize_rounded(&a, &o).map_or(true, |r| r.model.smooth_revolution.is_none()));
    assert!(recognize_automatic(
        &a,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..o
        }
    )
    .unwrap()
    .model
    .smooth_revolution
    .is_none());
}
