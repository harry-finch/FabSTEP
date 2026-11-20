use super::{native_step, FabError, NativeStep, ValidatedStep};
use fabstep_core::{unequal_corner::UnequalCorner, Mesh};
use std::ffi::c_char;

extern "C" {
    fn fabstep_build_unequal_corner(
        parameters: *const [f64; 3],
        points: *const [f64; 3],
        nv: usize,
        triangles: *const [usize; 3],
        nt: usize,
        owners: *mut usize,
        tokens: *const usize,
        fit: f64,
        deviation: f64,
        bound: *mut f64,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

/// The returned partition includes the certified boundary triangles. A
/// geometric refusal preserves Auto’s previous reconstruction candidate.
pub fn build_unequal_corner(
    mesh: &Mesh,
    candidate: &UnequalCorner,
    first_token: usize,
    fit: f64,
    limit: f64,
) -> Result<(ValidatedStep, Vec<usize>, f64), FabError> {
    let parameters = [candidate.origin, candidate.size, candidate.radii];
    let mut owners = candidate.owners.clone();
    if owners.len() != mesh.triangles.len() || first_token == 0 || first_token > 99_991 {
        return Err(FabError::new(
            "cad.invalid_model",
            "Attributions du coin incomplètes.",
        ));
    }
    let tokens: [usize; 10] = std::array::from_fn(|i| i + first_token);
    let mut bound = 0.;
    let step = native_step(|output, error, capacity| unsafe {
        fabstep_build_unequal_corner(
            parameters.as_ptr(),
            mesh.vertices.as_ptr(),
            mesh.vertices.len(),
            mesh.triangles.as_ptr(),
            mesh.triangles.len(),
            owners.as_mut_ptr(),
            tokens.as_ptr(),
            fit,
            limit,
            &mut bound,
            output,
            error,
            capacity,
        )
    })?;
    Ok((step, owners, bound))
}
