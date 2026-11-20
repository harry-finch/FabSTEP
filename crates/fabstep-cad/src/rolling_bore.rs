use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::{rolling_bore::RollingBore, Mesh};
use std::ffi::c_char;

extern "C" {
    fn fabstep_build_rolling_bore_with_patch_bound(
        parameters: *const f64,
        vertices: *const [f64; 3],
        vertex_count: usize,
        indices: *const [usize; 3],
        owners: *const usize,
        count: usize,
        first_token: usize,
        fit: f64,
        deviation: f64,
        bound: *mut f64,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub fn build_rolling_bore(
    mesh: &Mesh,
    model: &RollingBore,
    first_token: usize,
    fit: f64,
    deviation: f64,
) -> Result<ValidatedStep, FabError> {
    build_rolling_bore_with_patch_bound(mesh, model, first_token, fit, deviation)
        .map(|(step, _)| step)
}

/// Returns the greatest certified deviation of the rolling surface and its
/// rails before and after STEP transfer. The planar and cylindrical supports
/// have their own source certificate in the core.
pub fn build_rolling_bore_with_patch_bound(
    mesh: &Mesh,
    model: &RollingBore,
    first_token: usize,
    fit: f64,
    deviation: f64,
) -> Result<(ValidatedStep, f64), FabError> {
    if model.owners.len() != mesh.triangles.len() {
        return Err(FabError::new(
            "cad.invalid_model",
            "La partition du perçage est incomplète.",
        ));
    }
    let [x, y, z] = model.origin;
    let parameters = [
        x,
        y,
        z,
        model.height,
        model.shaft,
        model.bore,
        model.offset,
        model.level,
        model.radius,
        model.radius_spread,
    ];
    let mut bound = 0.;
    let step = native_step(|output, error, capacity| unsafe {
        fabstep_build_rolling_bore_with_patch_bound(
            parameters.as_ptr(),
            mesh.vertices.as_ptr(),
            mesh.vertices.len(),
            mesh.triangles.as_ptr(),
            model.owners.as_ptr(),
            mesh.triangles.len(),
            first_token,
            fit.min(1e-5),
            deviation,
            &mut bound,
            output,
            error,
            capacity,
        )
    })?;
    Ok((step, bound))
}
