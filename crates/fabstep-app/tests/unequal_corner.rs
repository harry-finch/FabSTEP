use fabstep_app::convert_stl;
use fabstep_core::{Options, ReconstructionMode};

const CASES: [&[u8]; 4] = [
    include_bytes!("../../../fixtures/unequal_corner/corner-0-coarse.stl"),
    include_bytes!("../../../fixtures/unequal_corner/corner-0-fine.stl"),
    include_bytes!("../../../fixtures/unequal_corner/corner-1-coarse.stl"),
    include_bytes!("../../../fixtures/unequal_corner/corner-1-fine.stl"),
];

#[test]
fn both_three_radius_corners_have_verified_interiors_at_two_densities() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    for (i, bytes) in CASES.into_iter().enumerate() {
        let analysis = fabstep_core::analyze_stl(bytes, &options).unwrap();
        let seed =
            fabstep_core::unequal_corner::unequal_corner_candidate(&analysis, &options).unwrap();
        let direct =
            fabstep_cad::build_unequal_corner(&analysis.mesh, &seed, 1, 1e-5, 0.05 - 1e-6).unwrap();
        assert!(direct.2 < 0.05);
        let c = convert_stl(bytes, &options).unwrap();
        assert_eq!(
            (
                c.report.cad.solids,
                c.report.cad.faces,
                c.report.cad.cylindrical_faces
            ),
            (1, 10, 3),
            "piece {i}"
        );
        assert!(c
            .report
            .analysis
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "recognition.unequal_corner_fitted"));
        let f = c.report.fidelity.unwrap();
        assert!(f.source_to_result_mm < 0.05 && f.result_to_source_mm < 0.05);
        eprintln!(
            "corner {i} bound={} fwd={} back={} volume={}",
            direct.2, f.source_to_result_mm, f.result_to_source_mm, c.report.cad.volume_mm3
        );
    }
}

#[test]
fn identical_boundary_and_changed_patch_interior_decline_the_corner() {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let data = CASES[0];
    let analysis = fabstep_core::analyze_stl(data, &options).unwrap();
    let seed = fabstep_core::unequal_corner::unequal_corner_candidate(&analysis, &options).unwrap();
    let (_, owners, _) =
        fabstep_cad::build_unequal_corner(&analysis.mesh, &seed, 1, 1e-5, 0.049999).unwrap();
    let mut on_patch = vec![false; analysis.mesh.vertices.len()];
    let mut on_support = on_patch.clone();
    for (tri, id) in analysis.mesh.triangles.iter().zip(owners) {
        let used = if id == 9 {
            &mut on_patch
        } else {
            &mut on_support
        };
        for &index in tri {
            used[index] = true;
        }
    }
    let support_points = analysis
        .mesh
        .vertices
        .iter()
        .zip(&on_support)
        .filter_map(|(&p, &used)| used.then_some(p))
        .collect::<Vec<_>>();
    // Pick the patch vertex farthest from its support vertices. Leave every
    // shared border, plane and cylinder bit-for-bit intact in the negative.
    let moved = analysis
        .mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(i, _)| on_patch[*i] && !on_support[*i])
        .max_by_key(|(_, p)| {
            let clearance = support_points
                .iter()
                .map(|q| p.iter().zip(q).map(|(x, y)| (x - y).powi(2)).sum::<f64>())
                .fold(f64::INFINITY, f64::min);
            clearance.to_bits()
        })
        .unwrap()
        .1;
    let mut dented = data.to_vec();
    for record in dented[84..].chunks_exact_mut(50) {
        for v in record[12..48].chunks_exact_mut(12) {
            let xyz: [f32; 3] = std::array::from_fn(|k| {
                f32::from_le_bytes(v[4 * k..4 * k + 4].try_into().unwrap())
            });
            if (0..3).all(|k| (f64::from(xyz[k]) - moved[k]).abs() < 1e-7) {
                for k in 0..3 {
                    v[4 * k..4 * k + 4].copy_from_slice(&(xyz[k] - 0.075).to_le_bytes());
                }
            }
        }
    }
    assert_ne!(dented, data);
    let a = fabstep_core::analyze_stl(&dented, &options).unwrap();
    let seed = fabstep_core::unequal_corner::unequal_corner_candidate(&a, &options).unwrap();
    let refused = fabstep_cad::build_unequal_corner(&a.mesh, &seed, 1, 1e-5, 0.049999);
    assert_eq!(refused.err().unwrap().code, "cad.corner_unproven");
    let ordinary = convert_stl(&dented, &options).unwrap();
    assert!(ordinary
        .report
        .analysis
        .report
        .diagnostics
        .iter()
        .all(|d| d.code != "recognition.unequal_corner_fitted"));
    assert_eq!(ordinary.report.cad.solids, 1);
}

#[test]
fn the_document_keeps_the_source_partition_after_all_transfers() {
    use fabstep_app::{convert_document_with_provenance, InputFormat};
    use fabstep_cad::ResultFaceFamily;
    let opts = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Default::default()
    };
    let c = convert_document_with_provenance(CASES[0], InputFormat::Stl, &opts).unwrap();
    let inspected = c.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let mut ids = inspected
        .source_correspondence
        .faces
        .iter()
        .flat_map(|f| f.source_triangle_ids.iter().copied())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    let n = u32::from_le_bytes(CASES[0][80..84].try_into().unwrap()) as usize;
    assert!((0..n).eq(ids));
    let patch = inspected
        .result
        .faces
        .iter()
        .find(|f| f.family == ResultFaceFamily::Other)
        .unwrap();
    assert_eq!(patch.adjacent_face_ids.len(), 4);
    let cylinders = patch
        .adjacent_face_ids
        .iter()
        .filter(|&&id| inspected.result.faces[id].family == ResultFaceFamily::Cylinder)
        .count();
    assert_eq!(cylinders, 3);

    let input = std::str::from_utf8(c.step.bytes()).unwrap();
    let net = input
        .split_once("B_SPLINE_SURFACE(")
        .unwrap()
        .1
        .split_once(".UNSPECIFIED.")
        .unwrap()
        .0;
    let poles: Vec<_> = net
        .split('#')
        .skip(1)
        .map(|s| s.split(|c: char| !c.is_ascii_digit()).next().unwrap())
        .collect();
    let internal = poles[poles.len() / 2];
    let prefix = format!("#{internal} = CARTESIAN_POINT('',(");
    let start = input.find(&prefix).unwrap() + prefix.len();
    let end = start + input[start..].find(',').unwrap();
    let mut modified = input.to_string();
    modified.replace_range(
        start..end,
        &(input[start..end].parse::<f64>().unwrap() + 0.25).to_string(),
    );
    assert!(fabstep_cad::result_faces(modified.as_bytes()).is_err());
}
