use fabstep_app::{
    convert_document_controlled, convert_document_with_provenance, convert_stl, ConversionPhase,
    InputFormat,
};
use fabstep_core::{FabError, Options, ReconstructionMode};

// Translating before quantization to STL f32 changes rounding on the three
// support planes. The curved surface and topology of the octant are identical.
const OCTANT: &[u8] = include_bytes!("../../../fixtures/r133-translated-octant.stl");

fn auto() -> Options {
    Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    }
}

#[test]
fn screened_octant_validates_once_and_preserves_provenance_and_requested_settings() {
    let bytes = OCTANT;
    let options = auto();
    let mut builds = 0;
    let result = convert_document_controlled(bytes, InputFormat::Stl, &options, &mut |p| {
        builds += usize::from(p.phase == ConversionPhase::Build);
        Ok(())
    })
    .unwrap();
    assert_eq!(builds, 2, "one native build bracketed by two checkpoints");
    let report = &result.report;
    assert_eq!(
        (
            report.cad.faces,
            report.cad.planar_faces,
            report.cad.spherical_faces
        ),
        (4, 3, 1)
    );
    assert_eq!(
        report.settings.plane_tolerance_mm,
        options.plane_tolerance_mm
    );
    let body = &report.bodies[0].conversion;
    let change = body.auto_adjustment.unwrap();
    assert_eq!(
        change.requested_plane_tolerance_mm,
        options.plane_tolerance_mm
    );
    assert_eq!(change.plane_tolerance_mm, body.settings.plane_tolerance_mm);
    assert!(change.plane_tolerance_mm > options.plane_tolerance_mm);
    let measured = body.fidelity.as_ref().unwrap();
    assert!(
        measured
            .source_to_result_mm
            .max(measured.result_to_source_mm)
            <= options.max_deviation_mm
    );
    let direct = convert_stl(bytes, &options).unwrap();
    assert_eq!(direct.report.cad.faces, 4);
    assert_eq!(
        direct.report.auto_adjustment.unwrap().plane_tolerance_mm,
        change.plane_tolerance_mm
    );
    let tracked = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
    let inspected = tracked.step.result_faces().unwrap();
    assert_eq!(inspected.source_correspondence.status, "exact");
    let sphere = inspected
        .result
        .faces
        .iter()
        .find(|f| f.family == fabstep_cad::ResultFaceFamily::Sphere)
        .unwrap();
    assert_eq!(
        inspected.source_correspondence.faces[sphere.id]
            .source_triangle_ids
            .len(),
        324
    );
    let region = tracked
        .step
        .suggest_region_controlled(sphere.id, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(region, vec![sphere.id]);
}

#[test]
fn pinned_default_and_cancelled_winner_cannot_publish_a_tuned_result() {
    let bytes = OCTANT;
    let options = Options {
        auto_tune: false,
        ..auto()
    };
    let baseline = convert_document_with_provenance(bytes, InputFormat::Stl, &options).unwrap();
    assert_eq!(baseline.report.cad.spherical_faces, 0);
    assert!(baseline.report.bodies[0]
        .conversion
        .auto_adjustment
        .is_none());
    let mut phases = Vec::new();
    let failure = convert_document_controlled(bytes, InputFormat::Stl, &auto(), &mut |p| {
        phases.push(p.phase);
        if p.phase == ConversionPhase::AuditComparison {
            return Err(FabError::new("test.stale", "stop"));
        }
        Ok(())
    })
    .err()
    .unwrap();
    assert_eq!(failure.code, "test.stale");
    assert_eq!(phases.last(), Some(&ConversionPhase::AuditComparison));
    assert_eq!(
        phases
            .iter()
            .filter(|&&p| p == ConversionPhase::Build)
            .count(),
        2
    );
}

fn add_tetrahedron(bytes: &mut Vec<u8>, origin: [f32; 3], side: f32) {
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
    bytes[80..84].copy_from_slice(&(n + 4).to_le_bytes());
    let mut points = [origin; 4];
    for axis in 0..3 {
        points[axis + 1][axis] += side;
    }
    for tri in [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]] {
        bytes.extend([0u8; 12]);
        for vertex in tri {
            for coord in points[vertex] {
                bytes.extend(coord.to_le_bytes());
            }
        }
        bytes.extend([0u8; 2]);
    }
}

#[test]
fn document_keeps_original_solids_if_an_individually_valid_adjustment_encloses_its_neighbor() {
    let mut bytes = OCTANT.to_vec();
    // Halfway from the centroid of facet 190 to the analytic sphere. The facet
    // has 0.01522 mm of sag; a 0.002 mm tetrahedron fits strictly in that gap.
    add_tetrahedron(&mut bytes, [15.77543, 16.95932, 6.03271], 0.002);
    let mut combine_count = 0;
    let converted = convert_document_controlled(&bytes, InputFormat::Stl, &auto(), &mut |p| {
        combine_count += usize::from(p.phase == ConversionPhase::Combine);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        combine_count, 4,
        "attempted then rejected the analytic combination"
    );
    assert_eq!(converted.report.cad.solids, 2);
    assert_eq!(converted.report.cad.spherical_faces, 0);
    assert!(converted
        .report
        .bodies
        .iter()
        .all(|b| b.conversion.auto_adjustment.is_none()));
}

#[test]
fn guidance_on_a_second_body_replays_the_first_bodys_effective_tolerances() {
    let mut bytes = OCTANT.to_vec();
    add_tetrahedron(&mut bytes, [40., 10., 6.], 1.);
    let result = convert_document_with_provenance(&bytes, InputFormat::Stl, &auto()).unwrap();
    let id = result
        .report
        .bodies
        .iter()
        .find(|b| b.conversion.cad.spherical_faces == 0)
        .unwrap()
        .id
        .clone();
    let display = result.step.result_faces().unwrap();
    let face = display
        .source_correspondence
        .faces
        .iter()
        .find(|f| f.analysis_body_id == id)
        .unwrap()
        .face_id;
    let applied = result
        .step
        .guided_surface(&[face], fabstep_core::GuidanceFamily::Plane)
        .unwrap();
    assert_eq!(applied.report.cad.solids, 2);
    assert_eq!(applied.report.cad.spherical_faces, 1);
    let changed: Vec<_> = applied
        .report
        .bodies
        .iter()
        .filter(|b| b.conversion.auto_adjustment.is_some())
        .collect();
    assert_eq!(changed.len(), 1);
    assert_ne!(changed[0].id, id);
    assert_eq!(
        applied
            .step
            .result_faces()
            .unwrap()
            .source_correspondence
            .status,
        "exact"
    );
}
