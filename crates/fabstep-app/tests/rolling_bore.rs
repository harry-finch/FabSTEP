use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{analyze_stl, rolling_bore::candidate, Options, ReconstructionMode};

const SHAFT: &[u8] = include_bytes!("../../../fixtures/rolling_bore/shaft-coarse.stl");
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}
fn certified(d: &fabstep_app::ProvenancedDocumentConversion) -> bool {
    d.report.bodies[0]
        .conversion
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "recognition.rolling_bore_fitted")
}

#[test]
fn auto_preserves_every_face_and_its_source_at_two_densities() {
    let o = options();
    for (src, annulus) in [
        (SHAFT, 989),
        (
            include_bytes!("../../../fixtures/rolling_bore/shaft-fine.stl").as_slice(),
            1384,
        ),
        (
            include_bytes!("../../../fixtures/rolling_bore/short-shaft-coarse.stl").as_slice(),
            993,
        ),
        (
            include_bytes!("../../../fixtures/rolling_bore/short-shaft-fine.stl").as_slice(),
            1392,
        ),
    ] {
        let a = analyze_stl(src, &o).unwrap();
        let model = candidate(&a, &o).unwrap();
        let d = convert_document_with_provenance(src, InputFormat::Stl, &o).unwrap();
        assert!(certified(&d), "{} triangles", a.mesh.triangles.len());
        let r = &d.report.cad;
        assert_eq!(
            (r.solids, r.faces, r.planar_faces, r.cylindrical_faces),
            (1, 5, 2, 2)
        );
        let f = d.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(
            f.source_samples > 0
                && f.result_samples > 0
                && f.source_to_result_mm.max(f.result_to_source_mm) < 0.05
        );
        let result = d.step.result_faces().unwrap();
        assert_eq!(result.source_correspondence.status, "exact");
        let mut seen = vec![false; model.owners.len()];
        for src in &result.source_correspondence.faces {
            let slot = model.owners[src.source_triangle_ids[0]];
            let face = result
                .result
                .faces
                .iter()
                .find(|f| f.id == src.face_id)
                .unwrap();
            if slot < 2 {
                assert_eq!(face.family, fabstep_cad::ResultFaceFamily::Plane);
            }
            if (2..=3).contains(&slot) {
                assert_eq!(face.family, fabstep_cad::ResultFaceFamily::Cylinder);
            }
            if slot == 4 {
                assert_eq!(face.family, fabstep_cad::ResultFaceFamily::Other);
                assert_eq!(src.source_triangle_ids.len(), annulus);
            }
            for &id in &src.source_triangle_ids {
                assert_eq!(slot, model.owners[id]);
                assert!(!std::mem::replace(&mut seen[id], true));
            }
        }
        assert!(seen.into_iter().all(|value| value));
    }
}

#[test]
fn single_stl_entry_point_validates_the_same_five_faces() {
    let d = fabstep_app::convert_stl(SHAFT, &options()).unwrap();
    assert_eq!((d.report.cad.solids, d.report.cad.faces), (1, 5));
    assert!(d
        .report
        .analysis
        .report
        .diagnostics
        .iter()
        .any(|d| d.code == "recognition.rolling_bore_fitted"));
}

#[test]
fn displaced_annular_interior_and_ambiguous_contact_preserve_the_fallback() {
    let options = options();
    let a = analyze_stl(SHAFT, &options).unwrap();
    let model = candidate(&a, &options).unwrap();
    let mut blend = vec![false; a.mesh.vertices.len()];
    let mut neighbors = blend.clone();
    for (&f, tri) in model.owners.iter().zip(&a.mesh.triangles) {
        for &v in tri {
            if f == 4 {
                blend[v] = true
            } else {
                neighbors[v] = true
            }
        }
    }
    let inside = (0..blend.len())
        .find(|&i| blend[i] && !neighbors[i])
        .unwrap();
    let point = a.mesh.vertices[inside].map(|x| x as f32);
    let mut bad = SHAFT.to_vec();
    let mut changed = 0;
    for t in bad[84..].chunks_exact_mut(50) {
        for p in t[12..48].chunks_exact_mut(12) {
            let read = |k: usize| f32::from_le_bytes(p[4 * k..4 * k + 4].try_into().unwrap());
            if [read(0), read(1), read(2)] == point {
                p[..4].copy_from_slice(&(point[0] + 0.12).to_le_bytes());
                changed += 1;
            }
        }
    }
    assert!(changed >= 3);
    for src in [
        &bad,
        include_bytes!("../../../fixtures/rolling_bore/spindle-coarse.stl").as_slice(),
    ] {
        let d = convert_document_with_provenance(src, InputFormat::Stl, &options).unwrap();
        assert!(!certified(&d));
        assert!(d.report.cad.faces > 5);
    }
}

#[test]
fn a_second_root_keeps_its_faces_separate_from_the_round_mouth() {
    let cube = fabstep_core::read_stl_mesh(
        include_bytes!("../../../fixtures/cube.stl"),
        fabstep_core::InputUnit::Mm,
    )
    .unwrap();
    let mut stl = SHAFT.to_vec();
    let first_count = (stl.len() - 84) / 50;
    let count = (first_count + cube.triangles.len()) as u32;
    stl[80..84].copy_from_slice(&count.to_le_bytes());
    for triangle in &cube.triangles {
        stl.extend([0; 12]);
        for i in triangle {
            let [x, y, z] = cube.vertices[*i];
            for a in [x + 60., y, z] {
                stl.extend((a as f32).to_le_bytes());
            }
        }
        stl.extend([0; 2]);
    }
    let d = convert_document_with_provenance(&stl, InputFormat::Stl, &options()).unwrap();
    assert_eq!((d.report.cad.solids, d.report.cad.faces), (2, 11));
    let result = d.step.result_faces().unwrap();
    let sources = result.source_correspondence;
    assert_eq!(sources.status, "exact");
    assert_eq!(sources.faces.len(), 11);
    assert_eq!(
        sources
            .faces
            .iter()
            .filter(|f| f.source_triangle_ids[0] < first_count)
            .count(),
        5
    );
    for f in sources.faces {
        let shaft = f.source_triangle_ids[0] < first_count;
        assert!(f
            .source_triangle_ids
            .iter()
            .all(|&id| (id < first_count) == shaft));
    }
}
