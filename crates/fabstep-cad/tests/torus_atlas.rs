use fabstep_core::{
    torus_atlas::{TorusAtlas, TorusChart, TorusTrimVertex},
    Mesh,
};
use std::collections::BTreeMap;

fn ramp() -> (Mesh, TorusAtlas) {
    let (n, m) = (8usize, 6usize);
    let mut vertices = Vec::new();
    let uv = |i: usize, j: usize| [0.58 + 0.04 * i as f64, 0.25 + 0.055 * j as f64];
    for top in [true, false] {
        for i in 0..=n {
            for j in 0..=m {
                let [u, v] = uv(i, j);
                let r = 8. + v.cos();
                vertices.push([if top { v.sin() } else { -1. }, r * u.cos(), r * u.sin()]);
            }
        }
    }
    let count = (n + 1) * (m + 1);
    let mut top = Vec::new();
    for i in 0..n {
        for j in 0..m {
            if (3..5).contains(&i) && j >= 4 {
                continue;
            }
            let a = i * (m + 1) + j;
            let b = a + m + 1;
            top.extend([[a, b, b + 1], [a, b + 1, a + 1]]);
        }
    }
    let mut edges = BTreeMap::new();
    for t in &top {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if edges.remove(&(b, a)).is_none() {
                edges.insert((a, b), ());
            }
        }
    }
    let mut triangles = top.clone();
    triangles.extend(
        top.iter()
            .map(|t| [t[2] + count, t[1] + count, t[0] + count]),
    );
    let border: BTreeMap<_, _> = edges.keys().copied().collect();
    for (&a, &b) in &border {
        triangles.extend([[a, b + count, b], [a, a + count, b + count]]);
    }
    let first = *border.keys().next().unwrap();
    let mut v = first;
    let mut ring = Vec::new();
    loop {
        let next = border[&v];
        ring.push(TorusTrimVertex {
            vertex: v,
            uv: uv(v / (m + 1), v % (m + 1)),
            constant_u: v / (m + 1) == next / (m + 1),
        });
        v = next;
        if v == first {
            break;
        }
    }
    (
        Mesh {
            vertices,
            triangles,
        },
        TorusAtlas {
            axis: [1., 0., 0.],
            radial: [0., 1., 0.],
            charts: vec![TorusChart {
                center_mm: [0.; 3],
                major_radius_mm: 8.,
                minor_radius_mm: 1.,
                orientation: 1,
                triangles: (0..top.len()).collect(),
                loops: vec![ring],
                bound_mm: 0.03,
            }],
        },
    )
}

#[test]
fn native_atlas_retains_every_chart_and_residual_token_after_reload() {
    let (mesh, atlas) = ramp();
    let n = mesh.triangles.len() - atlas.charts[0].triangles.len() + 1;
    let tokens: Vec<_> = (1..=n).rev().collect();
    let (step, bound) =
        fabstep_cad::build_torus_atlas_step_with_provenance(&mesh, &atlas, &tokens, 0.05).unwrap();
    assert_eq!(step.report.solids, 1);
    assert_eq!(step.report.faces, n);
    assert_eq!(step.report.toroidal_faces, 1);
    assert!(bound > 0. && bound <= 0.05);
    let inspection = fabstep_cad::result_faces(&step.bytes).unwrap();
    assert_eq!(inspection.faces.len(), n);
    for face in &inspection.faces {
        assert!(face.source_face_token.is_some() && face.triangle_count > 0);
        if face.source_face_token == Some(tokens[0]) {
            assert_eq!(face.family, fabstep_cad::ResultFaceFamily::Torus);
        }
    }
    let mut found: Vec<_> = inspection
        .faces
        .iter()
        .map(|f| f.source_face_token.unwrap())
        .collect();
    found.sort();
    assert_eq!(found, (1..=n).collect::<Vec<_>>());
    assert!(!fabstep_cad::tessellate_step(&step.bytes, 0.005)
        .unwrap()
        .triangles
        .is_empty());
}

#[test]
fn separated_atlases_keep_root_identity_and_disjoint_face_ranges() {
    let (mesh, atlas) = ramp();
    let n = mesh.triangles.len() - atlas.charts[0].triangles.len() + 1;
    let first = fabstep_cad::build_torus_atlas_step_with_provenance(
        &mesh,
        &atlas,
        &(1..=n).collect::<Vec<_>>(),
        0.05,
    )
    .unwrap()
    .0;
    let mut moved = mesh;
    for p in &mut moved.vertices {
        p[0] += 16.;
    }
    let mut translated = atlas;
    translated.charts[0].center_mm[0] += 16.;
    let second = fabstep_cad::build_torus_atlas_step_with_provenance(
        &moved,
        &translated,
        &(n + 1..=2 * n).collect::<Vec<_>>(),
        0.05,
    )
    .unwrap()
    .0;
    let output = fabstep_cad::combine_steps_with_provenance(&[
        fabstep_cad::StepPart {
            name: "Atlas A",
            bytes: &first.bytes,
        },
        fabstep_cad::StepPart {
            name: "Atlas B",
            bytes: &second.bytes,
        },
    ])
    .unwrap();
    let result = fabstep_cad::result_faces(&output.bytes).unwrap();
    let mut end = 0;
    for f in &result.faces {
        assert_eq!(f.triangle_start, end);
        end += f.triangle_count;
        let id = f.source_face_token.unwrap();
        assert_eq!(f.solid_id, usize::from(id > n));
    }
    assert_eq!(result.faces.len(), 2 * n);
    assert_eq!(end, result.mesh.triangles.len());
}

#[test]
fn forged_trims_missing_and_duplicate_owners_cannot_create_a_result() {
    let (mesh, atlas) = ramp();
    let tokens: Vec<_> = (1..=mesh.triangles.len() - atlas.charts[0].triangles.len() + 1).collect();
    for mutation in 0..4 {
        let mut copy = atlas.clone();
        match mutation {
            0 => {
                copy.charts[0].bound_mm = 0.;
            }
            1 => {
                copy.charts[0].loops[0][0].uv[0] += 0.04;
            }
            2 => {
                copy.charts[0].triangles[0] = copy.charts[0].triangles[1];
            }
            _ => {
                copy.charts[0].triangles.pop();
            }
        }
        assert!(
            fabstep_cad::build_torus_atlas_step_with_provenance(&mesh, &copy, &tokens, 0.05)
                .is_err()
        );
    }
}

/// Complete path using only the supplied STL and the compiled input profile.
/// Set FABSTEP_ATLAS_STL to an explicit source and FABSTEP_ATLAS_OUT to retain
/// the verified STEP, then run this ignored test with --nocapture.
#[test]
#[ignore = "explicit independent or private STL qualification"]
fn convert_independent_stl_with_native_atlas() {
    let path = std::env::var("FABSTEP_ATLAS_STL").unwrap();
    let bytes = std::fs::read(path).unwrap();
    let analysis = fabstep_core::analyze_stl(&bytes, &fabstep_core::Options::default()).unwrap();
    let mesh = &analysis.mesh;
    let atlas = fabstep_core::torus_atlas::recognize(mesh, 0.05)
        .unwrap()
        .unwrap();
    let residual = mesh.triangles.len()
        - atlas
            .charts
            .iter()
            .map(|c| c.triangles.len())
            .sum::<usize>();
    let tokens: Vec<_> = (1..=atlas.charts.len() + residual).collect();
    eprintln!(
        "Source {} triangles; {} charts, {} residuals",
        mesh.triangles.len(),
        atlas.charts.len(),
        residual
    );
    let (step, bound) =
        fabstep_cad::build_torus_atlas_step_with_provenance(mesh, &atlas, &tokens, 0.05).unwrap();
    eprintln!(
        "Reimported {} faces, volume {}, {} STEP bytes, continuous source bound {}",
        step.report.faces,
        step.report.volume_mm3,
        step.bytes.len(),
        bound
    );
    if let Ok(output) = std::env::var("FABSTEP_ATLAS_OUT") {
        std::fs::write(output, &step.bytes).unwrap();
    }
    let result = fabstep_cad::result_faces(&step.bytes).unwrap();
    assert_eq!(result.faces.len(), tokens.len());
    assert!(result
        .faces
        .iter()
        .all(|f| f.source_face_token.is_some() && f.triangle_count > 0));
    eprintln!(
        "Inspected {} faces, {} view triangles",
        result.faces.len(),
        result.mesh.triangles.len()
    );
    let exact = fabstep_cad::tessellate_step(&step.bytes, 0.005).unwrap();
    eprintln!(
        "Tessellated {} triangles at 0.005 mm",
        exact.triangles.len()
    );
}
