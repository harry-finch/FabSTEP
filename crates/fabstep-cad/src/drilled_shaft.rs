//! Independently check the two cap rings and every source trim witness
//! before passing scalar dimensions to the OCCT cylinder/cylinder builder.
use super::*;

#[repr(C)]
struct NativeDrilledShaft {
    origin: [f64; 3],
    shaft: [f64; 3],
    bore: [f64; 3],
    height: f64,
    radius: f64,
    bore_radius: f64,
    offset: f64,
    level: f64,
}
unsafe extern "C" {
    fn fabstep_build_drilled_shaft(
        d: *const NativeDrilledShaft,
        tokens: *const usize,
        out: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}
pub(super) fn build(
    model: &RoundedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les quatre faces ou les contours de l’arbre percé sont incohérents.",
        )
    };
    let s = model.drilled_shaft.as_deref().ok_or_else(invalid)?;
    let m = &model.revolved.mixed;
    if !m.arcs.is_empty()
        || !m.elliptic_arcs.is_empty()
        || !m.cylinders.is_empty()
        || !m.partial_cylinders.is_empty()
        || !model.revolved.cones.is_empty()
        || !model.partial_cones.is_empty()
        || !model.tori.is_empty()
        || !model.spheres.is_empty()
        || !model.octants.is_empty()
        || !model.torus_sectors.is_empty()
        || model.smooth_revolution.is_some()
        || model.cross_drill.is_some()
        || model.linear_blend.is_some()
        || model.unequal_pocket.is_some()
        || m.planes.len() != 2
        || m.rings.len() != 2
        || !s.valid_witnesses(&m.vertices)
    {
        return Err(invalid());
    }
    for (i, (p, r)) in m.planes.iter().zip(&m.rings).enumerate() {
        let sign = if i == 0 { -1. } else { 1. };
        let z = if i == 0 { 0. } else { s.height_mm };
        if !matches!(p.loops.as_slice(),[MixedLoop::Circle {ring}] if *ring==i)
            || !r.radius_mm.is_finite()
            || (r.radius_mm - s.radius_mm).abs() > 1e-8
            || (0..3).any(|j| {
                [
                    (r.center[j] - (s.origin[j] + s.shaft_axis[j] * z)).abs(),
                    (r.axis[j] - s.shaft_axis[j]).abs(),
                    (p.normal[j] - sign * s.shaft_axis[j]).abs(),
                ]
                .into_iter()
                .any(|v| !v.is_finite() || v > 1e-7)
            })
        {
            return Err(invalid());
        }
    }
    let native = NativeDrilledShaft {
        origin: s.origin,
        shaft: s.shaft_axis,
        bore: s.bore_axis,
        height: s.height_mm,
        radius: s.radius_mm,
        bore_radius: s.bore_radius_mm,
        offset: s.bore_offset_mm,
        level: s.bore_height_mm,
    };
    let ids = tokens.map(|t| [t.planes[0], t.planes[1], t.cylinders[0], t.cylinders[1]]);
    native_step(|out, error, capacity| unsafe {
        fabstep_build_drilled_shaft(
            &native,
            ids.as_ref().map_or(std::ptr::null(), |a| a.as_ptr()),
            out,
            error,
            capacity,
        )
    })
}
