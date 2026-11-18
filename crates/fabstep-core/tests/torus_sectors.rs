use fabstep_core::{
    analyze_stl, recognize_automatic, recognize_automatic_guided, recognize_rounded,
    GuidanceFamily, Options, ReconstructionMode, RoundedFaceRef,
};
use std::path::Path;

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../fixtures/torus_sectors/{name}.stl")),
    )
    .unwrap()
}

#[test]
fn swept_quarter_round_covers_exactly_seven_source_disks() {
    for name in ["nominal", "oblique", "mirrored", "retessellated"] {
        let opt = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Default::default()
        };
        let a = analyze_stl(&bytes(name), &opt).unwrap();
        let counts = if name == "retessellated" {
            [65, 21]
        } else {
            [48, 16]
        };
        let [n, m] = counts;
        let r = recognize_rounded(&a, &opt).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(r.model.torus_sectors.len(), 1);
        let ranges = [
            (2 * n, RoundedFaceRef::Plane { index: 0 }),
            (2 * n, RoundedFaceRef::PartialCylinder { index: 0 }),
            (2 * m * n, RoundedFaceRef::TorusSector { index: 0 }),
            (2 * n, RoundedFaceRef::Plane { index: 1 }),
            (2 * n, RoundedFaceRef::PartialCylinder { index: 1 }),
        ];
        let mut start = 0;
        for (count, owner) in ranges {
            assert!(r.triangle_faces[start..start + count]
                .iter()
                .all(|p| *p == owner));
            start += count;
        }
        let (start_cap, end_cap) = if name == "mirrored" { (3, 2) } else { (2, 3) };
        let end = start + m + 4;
        assert!(r.triangle_faces[start..end]
            .iter()
            .all(|p| *p == RoundedFaceRef::Plane { index: start_cap }));
        assert!(r.triangle_faces[end..]
            .iter()
            .all(|p| *p == RoundedFaceRef::Plane { index: end_cap }));
        assert!(r.report.max_deviation_bound_mm < opt.max_deviation_mm);
        assert!((r.report.expected_volume_mm3 - 264.31261025114605).abs() < 0.001);
        assert_eq!(r.report.cylinders, 2);
        let automatic = Options {
            reconstruction: ReconstructionMode::Auto,
            ..opt.clone()
        };
        let auto = recognize_automatic(&a, &automatic).unwrap();
        assert_eq!(auto.triangle_faces, r.triangle_faces);
        let refused =
            recognize_automatic_guided(&a, &automatic, &[(GuidanceFamily::Torus, vec![4 * n])])
                .unwrap_err();
        assert_eq!(refused.code, "guidance.family_unproven");
    }
}

#[test]
fn defective_or_over_budget_sectors_are_never_promoted() {
    let opt = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Default::default()
    };
    assert_eq!(
        analyze_stl(&bytes("open"), &opt).unwrap_err().code,
        "mesh.open"
    );
    for name in ["dented", "folded"] {
        let a = analyze_stl(&bytes(name), &opt).unwrap();
        assert!(recognize_rounded(&a, &opt).is_err(), "{name}");
        let automatic = Options {
            reconstruction: ReconstructionMode::Auto,
            ..opt.clone()
        };
        assert!(
            recognize_automatic(&a, &automatic).map_or(true, |r| r.model.torus_sectors.is_empty())
        );
    }
    let a = analyze_stl(&bytes("nominal"), &opt).unwrap();
    let small = Options {
        max_deviation_mm: 0.0001,
        ..opt
    };
    assert!(recognize_rounded(&a, &small).is_err());
}

#[test]
fn reversing_source_triangle_order_preserves_the_sector_domain() {
    let src = bytes("nominal");
    let mut reordered = src[..84].to_vec();
    for facet in src[84..].chunks_exact(50).rev() {
        reordered.extend_from_slice(facet);
    }
    let opt = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..Default::default()
    };
    let convert = |s: &[u8]| recognize_rounded(&analyze_stl(s, &opt).unwrap(), &opt).unwrap();
    let nominal = convert(&src);
    let actual = convert(&reordered);
    assert!(nominal
        .triangle_faces
        .iter()
        .eq(actual.triangle_faces.iter().rev()));
    assert!((nominal.report.expected_volume_mm3 - actual.report.expected_volume_mm3).abs() < 1e-9);
}
