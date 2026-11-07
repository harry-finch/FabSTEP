use fabstep_core::{analyze_stl, Analysis, InputUnit, Options, Severity};

const CUBE: &[u8] = include_bytes!("../../../fixtures/cube.stl");
const CONCAVE: &[u8] = include_bytes!("../../../fixtures/concave_prism.stl");
const HOLE: &[u8] = include_bytes!("../../../fixtures/rectangular_hole.stl");

fn analyze(bytes: &[u8]) -> Analysis {
    analyze_stl(bytes, &Options::default()).unwrap()
}
fn raw(analysis: &Analysis) -> Vec<[[f64; 3]; 3]> {
    analysis
        .mesh
        .triangles
        .iter()
        .map(|t| t.map(|i| analysis.mesh.vertices[i]))
        .collect()
}
fn ascii(triangles: &[[[f64; 3]; 3]]) -> Vec<u8> {
    let mut text = String::from("solid test\n");
    for tri in triangles {
        text.push_str("facet normal 0 0 0\nouter loop\n");
        for p in tri {
            text.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
        }
        text.push_str("endloop\nendfacet\n");
    }
    text.push_str("endsolid test\n");
    text.into_bytes()
}
fn binary(triangles: &[[[f64; 3]; 3]]) -> Vec<u8> {
    let mut bytes = vec![0; 80];
    bytes[..5].copy_from_slice(b"solid");
    bytes.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for tri in triangles {
        bytes.extend_from_slice(&[0; 12]);
        for p in tri {
            for c in p {
                bytes.extend_from_slice(&(*c as f32).to_le_bytes());
            }
        }
        bytes.extend_from_slice(&[0; 2]);
    }
    bytes
}
fn error(bytes: &[u8]) -> String {
    analyze_stl(bytes, &Options::default()).unwrap_err().code
}

#[test]
fn cube_twelve_triangles_becomes_six_faces() {
    let a = analyze(CUBE);
    assert_eq!(a.report.input_triangles, 12);
    assert_eq!(a.report.vertices, 8);
    assert_eq!(a.report.planar_faces, 6);
    assert_eq!(a.report.welded_vertices, 28);
    assert!((a.report.volume_mm3 - 1.).abs() < 1e-12);
    assert_eq!(a.report.bounds_mm, [[0., 0., 0.], [1., 1., 1.]]);
    assert!(a
        .model
        .faces
        .iter()
        .all(|f| f.loops.len() == 1 && f.loops[0].len() == 4));
}

#[test]
fn concave_profile_and_rectangular_hole_preserve_topology() {
    let a = analyze(CONCAVE);
    assert_eq!(a.report.planar_faces, 8);
    assert!((a.report.volume_mm3 - 6.).abs() < 1e-12);
    assert_eq!(
        a.model
            .faces
            .iter()
            .filter(|f| f.loops[0].len() == 6)
            .count(),
        2
    );
    let b = analyze(HOLE);
    assert_eq!(b.report.planar_faces, 10);
    assert!((b.report.volume_mm3 - 24.).abs() < 1e-12);
    assert_eq!(
        b.model.faces.iter().filter(|f| f.loops.len() == 2).count(),
        2
    );
    for face in &b.model.faces {
        for (index, lp) in face.loops.iter().enumerate() {
            let mut area = [0.; 3];
            for i in 0..lp.len() {
                let p = b.model.vertices[lp[i]];
                let q = b.model.vertices[lp[(i + 1) % lp.len()]];
                area[0] += p[1] * q[2] - p[2] * q[1];
                area[1] += p[2] * q[0] - p[0] * q[2];
                area[2] += p[0] * q[1] - p[1] * q[0];
            }
            let signed = area
                .iter()
                .zip(face.normal)
                .map(|(a, n)| a * n)
                .sum::<f64>();
            assert_eq!(signed > 0., index == 0);
        }
    }
}

#[test]
fn binary_header_starting_solid_and_truncation() {
    let bytes = binary(&raw(&analyze(CUBE)));
    assert_eq!(analyze(&bytes).report.planar_faces, 6);
    assert_eq!(error(&bytes[..bytes.len() - 1]), "stl.invalid_length");
    assert_eq!(
        error(b"solid broken\nfacet normal 0 0 0\n"),
        "stl.invalid_ascii"
    );
    assert_eq!(error(b"solid empty\nendsolid empty\n"), "mesh.empty");
}

#[test]
fn ascii_footer_accepts_arbitrary_names_but_rejects_trailing_payload() {
    let named = std::str::from_utf8(CUBE)
        .unwrap()
        .replace("solid cube", "solid my facet and vertex");
    let padded = format!("{}\r\n \t\r\n", named.trim_end());
    assert_eq!(analyze(padded.as_bytes()).report.planar_faces, 6);
    let appended = format!("{named}\nnon STL payload\n");
    assert_eq!(error(appended.as_bytes()), "stl.invalid_ascii");
}

#[test]
fn reject_open_duplicate_nonmanifold_and_bad_orientation() {
    let mut triangles = raw(&analyze(CUBE));
    triangles.pop();
    assert_eq!(error(&ascii(&triangles)), "mesh.open");
    let mut triangles = raw(&analyze(CUBE));
    triangles.push(triangles[0]);
    assert_eq!(error(&ascii(&triangles)), "mesh.duplicate_triangle");
    let mut triangles = raw(&analyze(CUBE));
    let t = triangles[0];
    triangles.push([t[0], t[1], [0.5, 0.5, 0.5]]);
    assert_eq!(error(&ascii(&triangles)), "mesh.non_manifold_edge");
    let mut triangles = raw(&analyze(CUBE));
    triangles[0].swap(1, 2);
    assert_eq!(error(&ascii(&triangles)), "mesh.inconsistent_orientation");
}

#[test]
fn reject_degenerates_nonfinite_and_disjoint_solids() {
    assert_eq!(
        error(&ascii(&[[[0.; 3], [0.; 3], [1.; 3]]])),
        "mesh.degenerate_triangle"
    );
    assert_eq!(
        error(&ascii(&[[[f64::NAN, 0., 0.], [1., 0., 0.], [0., 1., 0.]]])),
        "mesh.non_finite"
    );
    let mut triangles = raw(&analyze(CUBE));
    let mut second = triangles.clone();
    for t in &mut second {
        for p in t {
            p[0] += 3.;
        }
    }
    triangles.extend(second);
    assert_eq!(error(&ascii(&triangles)), "mesh.multiple_components");
}

#[test]
fn global_orientation_and_stable_volume_under_translation_and_rotation() {
    let mut triangles = raw(&analyze(HOLE));
    for t in &mut triangles {
        for p in t.iter_mut() {
            let [x, y, z] = *p;
            let a = 0.37_f64;
            let b = 0.71_f64;
            let y2 = y * a.cos() - z * a.sin();
            let z2 = y * a.sin() + z * a.cos();
            *p = [
                x * b.cos() + z2 * b.sin() + 1e6,
                y2 - 2e6,
                -x * b.sin() + z2 * b.cos() + 3e6,
            ];
        }
        t.swap(1, 2);
    }
    let a = analyze(&ascii(&triangles));
    assert!((a.report.volume_mm3 - 24.).abs() < 1e-7);
    assert_eq!(a.report.planar_faces, 10);
    assert!(a
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.orientation_reversed"));
    assert_eq!(
        a.model.faces.iter().filter(|f| f.loops.len() == 2).count(),
        2
    );
}

#[test]
fn units_are_applied_before_welding_and_reporting() {
    let options = Options {
        input_unit: InputUnit::Inch,
        ..Options::default()
    };
    let a = analyze_stl(CUBE, &options).unwrap();
    assert_eq!(a.report.bounds_mm[1], [25.4; 3]);
    assert!((a.report.volume_mm3 - 25.4_f64.powi(3)).abs() < 1e-9);
    let mut invalid = Options {
        weld_tolerance_mm: 0.,
        ..Options::default()
    };
    assert_eq!(
        analyze_stl(CUBE, &invalid).unwrap_err().code,
        "options.invalid_tolerance"
    );
    invalid.weld_tolerance_mm = f64::NAN;
    assert_eq!(
        analyze_stl(CUBE, &invalid).unwrap_err().code,
        "options.invalid_tolerance"
    );
}

#[test]
fn weld_searches_adjacent_cells_and_obeys_euclidean_distance() {
    let mut triangles = raw(&analyze(CUBE));
    // Copies of one vertex straddle the x=1 grid boundary; all remain within tol.
    let mut occurrence = 0;
    for t in &mut triangles {
        for p in t {
            if *p == [1., 0., 0.] {
                p[0] += if occurrence % 2 == 0 { -0.2e-6 } else { 0.2e-6 };
                occurrence += 1;
            }
        }
    }
    let a = analyze(&ascii(&triangles));
    assert_eq!(a.report.vertices, 8);
    // Both points occupy the same cell, but diagonal distance exceeds tolerance.
    // A triangle corner must not be welded merely because its hash matches.
    let tri = [[[0., 0., 0.], [0.9e-6, 0.9e-6, 0.], [0., 2e-6, 0.]]];
    assert_eq!(error(&ascii(&tri)), "mesh.open");
}

#[test]
fn weld_rejects_ranges_where_adjacent_cells_lose_precision() {
    let mut triangles = raw(&analyze(CUBE));
    for p in triangles.iter_mut().flatten() {
        p[0] = if p[0] == 0. { -1e12 } else { 6.083515625e-5 };
    }
    // This corner is only 0.4 µm away from its representative, but for
    // this span the old f64 grid rounded it 128 cells away.
    let corner = triangles.iter_mut().flatten().rfind(|p| p[0] > 0.).unwrap();
    corner[0] = 6.123515625e-5;
    assert_eq!(error(&ascii(&triangles)), "mesh.coordinate_range");
}

#[test]
fn weld_reports_moved_occurrences_separately_from_exact_reuse() {
    let original = analyze(CUBE);
    assert!(!original
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "mesh.vertices_moved"));

    let mut triangles = raw(&original);
    let duplicate = triangles
        .iter_mut()
        .flatten()
        .rfind(|p| **p == [1., 1., 0.])
        .unwrap();
    duplicate[0] += 0.5e-6;
    let corrected = analyze(&ascii(&triangles));
    assert_eq!(corrected.mesh.vertices, original.mesh.vertices);
    assert_eq!(corrected.report.welded_vertices, 28);
    let diagnostic = corrected
        .report
        .diagnostics
        .iter()
        .find(|d| d.code == "mesh.vertices_moved")
        .unwrap();
    assert_eq!(diagnostic.count, 1);
    assert!(matches!(diagnostic.severity, Severity::Warning));
    assert!(diagnostic.message.contains("5.000000e-7 mm"));
}

#[test]
fn repeated_analysis_has_identical_topology() {
    let a = analyze(HOLE);
    let b = analyze(HOLE);
    assert_eq!(a.model.vertices, b.model.vertices);
    assert_eq!(
        a.model.faces.iter().map(|f| &f.loops).collect::<Vec<_>>(),
        b.model.faces.iter().map(|f| &f.loops).collect::<Vec<_>>()
    );
}

#[test]
fn binary_triangle_count_is_bounded() {
    let count = 100_001_u32;
    let mut bytes = vec![0; 84 + 50 * count as usize];
    bytes[80..84].copy_from_slice(&count.to_le_bytes());
    assert_eq!(error(&bytes), "input.too_many_triangles");
}

#[test]
fn collinear_boundary_vertices_remain_shared_after_retessellation() {
    let mut triangles = Vec::new();
    for [a, b, c] in raw(&analyze(CUBE)) {
        let ab = std::array::from_fn(|i| (a[i] + b[i]) * 0.5);
        let bc = std::array::from_fn(|i| (b[i] + c[i]) * 0.5);
        let ca = std::array::from_fn(|i| (c[i] + a[i]) * 0.5);
        triangles.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
    }
    let analysis = analyze(&ascii(&triangles));
    assert_eq!(analysis.report.input_triangles, 48);
    assert_eq!(analysis.report.planar_faces, 6);
    assert!(analysis
        .model
        .faces
        .iter()
        .all(|face| face.loops[0].len() == 8));
    let mut uses = std::collections::BTreeMap::<(usize, usize), Vec<bool>>::new();
    for face in &analysis.model.faces {
        for lp in &face.loops {
            for i in 0..lp.len() {
                let a = lp[i];
                let b = lp[(i + 1) % lp.len()];
                uses.entry((a.min(b), a.max(b))).or_default().push(a < b);
            }
        }
    }
    assert!(uses
        .values()
        .all(|directions| directions.len() == 2 && directions[0] != directions[1]));
}
