use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::{rolling_bore::RollingBore, Mesh};
use std::ffi::c_char;

extern "C" {
    fn fabstep_build_rolling_bore(
        parameters: *const f64,
        vertices: *const [f64; 3],
        vertex_count: usize,
        indices: *const [usize; 3],
        owners: *const usize,
        count: usize,
        first_token: usize,
        fit: f64,
        deviation: f64,
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
    native_step(|output, error, capacity| unsafe {
        fabstep_build_rolling_bore(
            parameters.as_ptr(),
            mesh.vertices.as_ptr(),
            mesh.vertices.len(),
            mesh.triangles.as_ptr(),
            model.owners.as_ptr(),
            mesh.triangles.len(),
            first_token,
            fit.min(1e-5),
            deviation,
            output,
            error,
            capacity,
        )
    })
}
