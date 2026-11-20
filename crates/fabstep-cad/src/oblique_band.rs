use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::oblique_band::{Family, ObliqueBand};
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
    sphere: usize,
}

unsafe extern "C" {
    fn fabstep_build_oblique_band(
        data: *const Data,
        tokens: *const usize,
        count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub fn build_oblique_band(
    band: &ObliqueBand,
    tokens: Option<&[usize]>,
) -> Result<ValidatedStep, FabError> {
    let lip = &band.frame;
    if tokens.is_some_and(|t| t.len() != 4) {
        return Err(FabError::new(
            "step.invalid_source_provenance",
            "La bande courbe attend quatre jetons de face.",
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
        sphere: usize::from(band.family == Family::Sphere),
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_oblique_band(
            &data,
            tokens.map_or(std::ptr::null(), |t| t.as_ptr()),
            tokens.map_or(0, |t| t.len()),
            output,
            error,
            capacity,
        )
    })
}
