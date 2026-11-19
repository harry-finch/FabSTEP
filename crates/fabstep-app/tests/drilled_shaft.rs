//! Independent FreeCAD solids and tessellations from V-1264. The four STEP
//! faces also have to own the triangles of their respective CAD face.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;
const COARSE: &[u8] = include_bytes!("../../../fixtures/r113-cross-bored-shaft.stl");
const FINE: &[u8] = include_bytes!("../../../fixtures/r113-cross-bored-shaft-fine.stl");
const VARIANT: &[u8] = include_bytes!("../../../fixtures/r113-cross-bored-shaft-variant.stl");
fn transformed(bytes: &[u8], f: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    for record in out[84..].chunks_exact_mut(50) {
        for v in record[12..48].chunks_exact_mut(12) {
            let p = std::array::from_fn(|i| {
                f32::from_le_bytes(v[i * 4..i * 4 + 4].try_into().unwrap())
            });
            for (out, value) in v.chunks_exact_mut(4).zip(f(p)) {
                out.copy_from_slice(&value.to_le_bytes());
            }
        }
    }
    out
}
fn options() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}
#[test]
fn reimport_recovers_four_faces_and_all_independently_labelled_source_triangles() {
    for (stl, expected_volume, counts) in [
        (COARSE, 4686.771216, [218, 47, 47, 116]),
        (FINE, 4686.771216, [326, 72, 72, 174]),
        (VARIANT, 4301.440831, [328, 72, 72, 176]),
    ] {
        let doc = convert_document_with_provenance(stl, InputFormat::Stl, &options()).unwrap();
        let cad = &doc.report.cad;
        assert_eq!(
            (
                cad.solids,
                cad.faces,
                cad.cylindrical_faces,
                cad.planar_faces
            ),
            (1, 4, 2, 2)
        );
        assert!((cad.volume_mm3 - expected_volume).abs() < 0.001);
        let fidelity = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(fidelity.source_to_result_mm < 0.05 && fidelity.result_to_source_mm < 0.05);
        let faces = doc.step.result_faces().unwrap();
        assert_eq!(faces.source_correspondence.status, "exact");
        let mut partition = BTreeSet::new();
        let mut groups = Vec::new();
        for f in &faces.source_correspondence.faces {
            let ids = f
                .source_triangle_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            assert_eq!(ids.len(), f.source_triangle_ids.len());
            assert!(partition.is_disjoint(&ids));
            partition.extend(&ids);
            let family = faces
                .result
                .faces
                .iter()
                .find(|face| face.id == f.face_id)
                .unwrap()
                .family;
            groups.push((ids, family));
        }
        assert_eq!(partition, (0..(stl.len() - 84) / 50).collect());
        let mut start = 0;
        for (size, family) in counts.into_iter().zip([
            fabstep_cad::ResultFaceFamily::Cylinder,
            fabstep_cad::ResultFaceFamily::Plane,
            fabstep_cad::ResultFaceFamily::Plane,
            fabstep_cad::ResultFaceFamily::Cylinder,
        ]) {
            let ids = (start..start + size).collect();
            assert!(groups
                .iter()
                .any(|(set, kind)| set == &ids && *kind == family));
            start += size;
        }
    }
}

#[test]
fn rotated_f32_source_and_off_support_counterexample() {
    let bytes = transformed(COARSE, |[x, y, z]| {
        [12. + x * 0.8 - z * 0.6, 2. + y, 8. + x * 0.6 + z * 0.8]
    });
    let doc = convert_document_with_provenance(&bytes, InputFormat::Stl, &options()).unwrap();
    assert_eq!(
        (doc.report.cad.faces, doc.report.cad.cylindrical_faces),
        (4, 2)
    );
    // Perturb one complete shared-vertex occurrence, keeping the mesh closed.
    let o = options();
    let a = fabstep_core::analyze_stl(COARSE, &o).unwrap();
    let rec = fabstep_core::recognize_automatic(&a, &o).unwrap();
    let corner = rec.model.drilled_shaft.as_ref().unwrap().cycles[2][0];
    let point = a.mesh.vertices[corner].map(|x| x as f32);
    let malformed = transformed(COARSE, |p| {
        if p == point {
            [p[0], p[1], p[2] + 0.03]
        } else {
            p
        }
    });
    let doc = convert_document_with_provenance(&malformed, InputFormat::Stl, &o).unwrap();
    assert_eq!(doc.report.cad.solids, 1);
    assert_eq!(doc.report.cad.cylindrical_faces, 0);
}

#[test]
fn forged_cycle_and_insufficient_deviation_do_not_build_the_specialized_solid() {
    let o = options();
    let a = fabstep_core::analyze_stl(COARSE, &o).unwrap();
    let mut rec = fabstep_core::recognize_automatic(&a, &o).unwrap();
    for field in ["ring_radius", "vertex", "count", "shifted_bore"] {
        let mut m = rec.model.clone();
        match field {
            "ring_radius" => m.revolved.mixed.rings[0].radius_mm = f64::NAN,
            "vertex" => m.revolved.mixed.vertices[0][0] = f64::NAN,
            "count" => m.revolved.mixed.vertices.resize(3 * 4096 + 1, [0.; 3]),
            _ => m.drilled_shaft.as_mut().unwrap().bore_height_mm += 0.3,
        }
        assert_eq!(
            fabstep_cad::build_rounded_step(&m).unwrap_err().code,
            "cad.invalid_model"
        );
    }
    let f = rec.model.drilled_shaft.as_mut().unwrap();
    f.cycles[2].reverse();
    assert_eq!(
        fabstep_cad::build_rounded_step(&rec.model)
            .unwrap_err()
            .code,
        "cad.invalid_model"
    );
    let strict = Options {
        max_deviation_mm: 0.005,
        ..o
    };
    assert!(fabstep_core::recognize_automatic(&a, &strict)
        .is_ok_and(|r| r.model.drilled_shaft.is_none()));
}
