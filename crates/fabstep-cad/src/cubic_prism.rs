use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::CubicPrism;
use std::ffi::c_char;

#[repr(C)]
struct Data {
    origin: [f64; 3],
    x: [f64; 3],
    axis: [f64; 3],
    width: f64,
    depth: f64,
    height: f64,
    ordinates: [f64; 2],
    bore: [f64; 3],
}
extern "C" {
    fn fabstep_build_cubic_prism(
        data: *const Data,
        tokens: *const usize,
        count: usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub fn build_cubic_prism(
    prism: &CubicPrism,
    tokens: Option<&[usize]>,
) -> Result<ValidatedStep, FabError> {
    let data = Data {
        origin: prism.origin,
        x: prism.x_axis,
        axis: prism.axis,
        width: prism.width_mm,
        depth: prism.depth_mm,
        height: prism.height_mm,
        ordinates: prism.ordinates_mm,
        bore: prism.bore.unwrap_or_default(),
    };
    if prism.bore.is_some_and(|hole| hole[2] <= 0.)
        || tokens.is_some_and(|t| t.len() != 6 + usize::from(prism.bore.is_some()))
    {
        return Err(FabError::new(
            "cad.invalid_model",
            "Extrusion cubique ou nombre de jetons de face invalide.",
        ));
    }
    native_step(|output, error, capacity| unsafe {
        fabstep_build_cubic_prism(
            &data,
            tokens.map_or(std::ptr::null(), |t| t.as_ptr()),
            tokens.map_or(0, |t| t.len()),
            output,
            error,
            capacity,
        )
    })
}
