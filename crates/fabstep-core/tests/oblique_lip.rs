use fabstep_core::{analyze_stl, oblique_lip::recognize, Options};

const CAP: &[u8] = include_bytes!("../../../fixtures/oblique_lip/cap-coarse.stl");
const CAP_FINE: &[u8] = include_bytes!("../../../fixtures/oblique_lip/cap-fine.stl");
const WIDE: &[u8] = include_bytes!("../../../fixtures/oblique_lip/wide-coarse.stl");
const WIDE_FINE: &[u8] = include_bytes!("../../../fixtures/oblique_lip/wide-fine.stl");

#[test]
fn recovers_both_cut_branches_and_all_five_disks_at_two_densities() {
    let options = Options::default();
    for (stl, radius, digest, volume) in [
        (CAP, 12., 0xfb3f33101176a1de_u64, 1273.70662),
        (CAP_FINE, 12., 0x1d3d3a33d9bb79dd, 1273.70662),
        (WIDE, 15., 0x019554420ac0d6dc, 2545.4566),
        (WIDE_FINE, 15., 0xe5b66961de5e5340, 2545.4566),
    ] {
        let a = analyze_stl(stl, &options).unwrap();
        let r = recognize(&a, &options)
            .unwrap()
            .expect("five certified disks");
        assert!((r.model.outer_radius_mm - radius).abs() < 1e-5);
        assert!((r.report.expected_volume_mm3 - volume).abs() < 0.002);
        assert!(r.report.max_deviation_bound_mm <= options.max_deviation_mm);
        assert_eq!(r.triangle_faces.len(), a.mesh.triangles.len());
        // Frozen from MeshPart.getSegment on the independently authored STEP,
        // after comparing every original STL record against the mesher's facet.
        let actual = r
            .triangle_faces
            .iter()
            .fold(0xcbf29ce484222325u64, |h, &slot| {
                (h ^ slot as u64).wrapping_mul(0x100000001b3)
            });
        assert_eq!(actual, digest);
    }
}
#[test]
fn ambiguous_or_uncovered_lip_falls_back() {
    let o = Options::default();
    let a = analyze_stl(
        include_bytes!("../../../fixtures/oblique_lip/near-tangent.stl"),
        &o,
    )
    .unwrap();
    assert!(recognize(&a, &o).unwrap().is_none());
    let a = analyze_stl(CAP, &o).unwrap();
    let mut tight = o.clone();
    tight.max_deviation_mm = 0.003;
    assert!(recognize(&a, &tight).unwrap().is_none());
}
