//! Closed 22° sector of radius 10 mm, extruded over 3 mm. The two end
//! faces are bounded by arcs and radii, with no complete circular rim.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{Options, ReconstructionMode};
use std::collections::BTreeSet;

const SECTOR: &[u8] = include_bytes!("../../../fixtures/r117-narrow-cylinder.stl");

fn convert(source: &[u8]) -> fabstep_app::ProvenancedDocumentConversion {
    convert_document_with_provenance(
        source,
        InputFormat::Stl,
        &Options {
            reconstruction: ReconstructionMode::Auto,
            ..Options::default()
        },
    )
    .unwrap()
}

fn transformed(source: &[u8], f: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<u8> {
    let mut stl = source.to_vec();
    for record in stl[84..].chunks_exact_mut(50) {
        for p in record[12..48].chunks_exact_mut(12) {
            let point = std::array::from_fn(|i| {
                f32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap())
            });
            for (bytes, v) in p.chunks_exact_mut(4).zip(f(point)) {
                bytes.copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    stl
}

#[test]
fn narrow_band_owns_the_cylindrical_wall_and_reimports_its_trim() {
    for bytes in [SECTOR.to_vec(), transformed(SECTOR, |[x, y, z]| [z, x, y])] {
        let doc = convert(&bytes);
        assert_eq!(
            (
                doc.report.cad.solids,
                doc.report.cad.faces,
                doc.report.cad.cylindrical_faces
            ),
            (1, 5, 1)
        );
        assert!((doc.report.cad.volume_mm3 - 0.5 * 100. * 22f64.to_radians() * 3.).abs() < 2e-5);
        let fidelity = doc.report.bodies[0].conversion.fidelity.as_ref().unwrap();
        assert!(fidelity.source_to_result_mm < 0.003 && fidelity.result_to_source_mm < 0.003);
        let result = doc.step.result_faces().unwrap();
        assert_eq!(result.source_correspondence.status, "exact");
        let face = result
            .result
            .faces
            .iter()
            .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cylinder)
            .unwrap();
        let owner = result
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.face_id == face.id)
            .unwrap();
        // In each slab only the 12 curved wall quads belong to the cylinder.
        let expected: BTreeSet<_> = (0..2)
            .flat_map(|slab| (26 + 28 * slab)..(50 + 28 * slab))
            .collect();
        assert_eq!(
            owner
                .source_triangle_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
        );
    }
}

#[test]
fn elliptic_wall_and_displaced_middle_row_remain_faceted() {
    for bytes in [
        transformed(SECTOR, |[x, y, z]| [x, y * 0.8, z]),
        transformed(SECTOR, |[x, y, z]| {
            [
                if z == 1. && y == 0. && x > 9. {
                    x + 0.015
                } else {
                    x
                },
                y,
                z,
            ]
        }),
    ] {
        let doc = convert(&bytes);
        assert_eq!(doc.report.cad.solids, 1);
        assert_eq!(doc.report.cad.cylindrical_faces, 0);
    }
}

#[test]
fn adjoining_tangent_planes_keep_their_own_triangles() {
    const FILLET: &[u8] = include_bytes!("../../../fixtures/r117-narrow-fillet.stl");
    let doc = convert(FILLET);
    assert_eq!(
        (
            doc.report.cad.solids,
            doc.report.cad.faces,
            doc.report.cad.cylindrical_faces
        ),
        (1, 8, 1)
    );
    let inspected = doc.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let id = inspected
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Cylinder)
        .unwrap()
        .id;
    let owner = inspected
        .source_correspondence
        .faces
        .iter()
        .find(|f| f.face_id == id)
        .unwrap();
    let expected: BTreeSet<_> = (0..2).flat_map(|i| (38 + 34 * i)..(62 + 34 * i)).collect();
    assert_eq!(
        owner
            .source_triangle_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected
    );
    let kink = transformed(FILLET, |[x, y, z]| {
        [if y > 4. && x > 9. { x + 0.05 } else { x }, y, z]
    });
    assert_eq!(convert(&kink).report.cad.cylindrical_faces, 0);
}
