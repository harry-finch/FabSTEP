use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::oblique_lip::ObliqueLip;
use std::ffi::c_char;

#[repr(C)]
struct Data {
    origin: [f64; 3],
    axis: [f64; 3],
    radial: [f64; 3],
    outer: f64,
    major: f64,
    height: f64,
    offset: f64,
    slope: f64,
}

unsafe extern "C" {
    fn fabstep_build_oblique_lip(
        data: *const Data,
        tokens: *const usize,
        count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub fn build_oblique_lip(
    lip: &ObliqueLip,
    tokens: Option<&[usize]>,
) -> Result<ValidatedStep, FabError> {
    if tokens.is_some_and(|t| t.len() != 5) {
        return Err(FabError::new(
            "step.invalid_source_provenance",
            "La lèvre oblique exige cinq jetons de face.",
        ));
    }
    let data = Data {
        origin: lip.origin,
        axis: lip.axis,
        radial: lip.radial,
        outer: lip.outer_radius_mm,
        major: lip.major_radius_mm,
        height: lip.height_mm,
        offset: lip.cut_offset_mm,
        slope: lip.slope,
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_oblique_lip(
            &data,
            tokens.map_or(std::ptr::null(), |t| t.as_ptr()),
            tokens.map_or(0, |t| t.len()),
            output,
            error,
            capacity,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{result_faces, ResultFaceFamily};
    #[test]
    fn both_transversal_branches_and_five_face_labels_survive_transfer() {
        let slope = 15f64.to_radians().tan();
        for (outer, major, h, k, s, volume) in [
            (12., 10., 8., -2. - 6.5 * slope, slope, 1273.7066533029881),
            (15., 12.2, 11., -3., -0.17, 2545.456797489),
        ] {
            let d = ObliqueLip {
                origin: [4., -6., 2.],
                axis: [0., 0.6, 0.8],
                radial: [1., 0., 0.],
                outer_radius_mm: outer,
                major_radius_mm: major,
                height_mm: h,
                cut_offset_mm: k,
                slope: s,
            };
            let native = build_oblique_lip(&d, Some(&[1, 2, 3, 4, 5])).unwrap();
            assert_eq!(
                (
                    native.report.solids,
                    native.report.faces,
                    native.report.planar_faces,
                    native.report.cylindrical_faces,
                    native.report.toroidal_faces
                ),
                (1, 5, 3, 1, 1)
            );
            assert!((native.report.volume_mm3 - volume).abs() <= volume * 1e-8);
            let inspected = result_faces(&native.bytes).unwrap();
            let wall = inspected
                .faces
                .iter()
                .find(|f| f.family == ResultFaceFamily::Torus)
                .unwrap();
            assert_eq!(wall.adjacent_face_ids.len(), 3);
            for f in inspected.faces {
                let id = f.source_face_token.unwrap();
                assert_eq!(
                    f.family,
                    [
                        ResultFaceFamily::Plane,
                        ResultFaceFamily::Plane,
                        ResultFaceFamily::Plane,
                        ResultFaceFamily::Cylinder,
                        ResultFaceFamily::Torus
                    ][id - 1]
                );
            }
        }
    }

    #[test]
    fn altered_branch_control_and_grazing_cut_are_rejected() {
        let mut model = ObliqueLip {
            origin: [0.; 3],
            axis: [0., 0., 1.],
            radial: [1., 0., 0.],
            outer_radius_mm: 12.,
            major_radius_mm: 10.,
            height_mm: 8.,
            cut_offset_mm: -3.74,
            slope: 0.268,
        };
        let native = build_oblique_lip(&model, None).unwrap();
        // Substitute just one interior 3D pole on a spline branch. The two
        // surface curves and the endpoints of this shared edge stay intact.
        let mut step = String::from_utf8(native.bytes).unwrap();
        let mut changed = false;
        for entity in step.clone().split(';') {
            let Some((_, controls)) = entity.split_once("B_SPLINE_CURVE_WITH_KNOTS('',6,(") else {
                continue;
            };
            let Some((controls, _)) = controls.split_once(')') else {
                continue;
            };
            let Some(id) = controls.split(',').nth(8) else {
                continue;
            };
            let label = format!("{} = CARTESIAN_POINT('',(", id.trim());
            let Some(start) = step.find(&label).map(|i| i + label.len()) else {
                continue;
            };
            let stop = start + step[start..].find(')').unwrap();
            let coords = step[start..stop].split(',').collect::<Vec<_>>();
            if coords.len() == 3 {
                let value = format!(
                    "{:.12}, {}, {}",
                    coords[0].parse::<f64>().unwrap() + 0.02,
                    coords[1],
                    coords[2]
                );
                step.replace_range(start..stop, &value);
                changed = true;
                break;
            }
        }
        assert!(changed);
        assert!(result_faces(step.as_bytes()).is_err());
        model.cut_offset_mm = -11.9;
        assert!(build_oblique_lip(&model, None).is_err());
    }
}
