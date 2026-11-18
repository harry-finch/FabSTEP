//! Two concave quarter cylinders meet on a single elliptic edge.
use super::oblique_pocket::{
    check_curves, dot, finite, invalid, native_circle, native_ellipse, wall_rails, Planes, Point,
    Stop, GUARD,
};
use super::*;

unsafe extern "C" {
    fn fabstep_build_adjacent_pocket_step(
        points: *const Point,
        point_count: usize,
        normals: *const Point,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        circles: *const NativeMixedArc,
        ellipse: *const NativeEllipse,
        plane_tokens: *const usize,
        wall_tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub(super) fn build(
    model: &MixedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let ([a, b], [ellipse]) = (
        model.partial_cylinders.as_slice(),
        model.elliptic_arcs.as_slice(),
    ) else {
        return Err(invalid());
    };
    if model.planes.len() != 11
        || !model.rings.is_empty()
        || !model.cylinders.is_empty()
        || model.vertices.len() > MAX_SOURCE_VERTICES
        || !model.vertices.iter().all(finite)
        || model.arcs.len() != 2
        || a.arcs[0] == b.arcs[0]
        || [a, b]
            .iter()
            .any(|w| !w.inner || !w.elliptic_end || w.arcs[1] != 0 || w.arcs[0] >= 2)
    {
        return Err(invalid());
    }
    let circles = [&model.arcs[a.arcs[0]], &model.arcs[b.arcs[0]]];
    if dot(circles[0].axis, circles[1].axis).abs() > 1e-7
        || (circles[0].radius_mm - circles[1].radius_mm).abs() > GUARD
        || (ellipse.major_mm - ellipse.minor_mm * std::f64::consts::SQRT_2).abs() > GUARD
    {
        return Err(invalid());
    }
    let mut rails = vec![];
    let mut reversed = [false; 2];
    for (i, circle) in circles.iter().enumerate() {
        let mut local = ellipse.clone();
        reversed[i] = dot(circle.axis, ellipse.axis) < 0.;
        if reversed[i] {
            // Same 3D curve, increasing cylinder angle for the second support.
            local.axis = local.axis.map(|v| -v);
            local.start_rad = -(ellipse.start_rad + ellipse.sweep_rad);
            local.vertices.swap(0, 1);
        }
        check_curves(&model.vertices, circle, &local, true)?;
        rails.extend(wall_rails(&model.vertices, circle, local.vertices));
    }
    if reversed[0] == reversed[1] {
        return Err(invalid());
    }
    let planes = Planes::encode(
        model,
        [
            Stop::circle(a.arcs[0], circles[0]),
            Stop::circle(b.arcs[0], circles[1]),
        ],
        &rails,
    )?;
    let native_circles = circles.map(native_circle);
    let native_e = native_ellipse(ellipse);
    native_step(|output, error, capacity| unsafe {
        fabstep_build_adjacent_pocket_step(
            model.vertices.as_ptr(),
            model.vertices.len(),
            planes.normals.as_ptr(),
            planes.offsets.as_ptr(),
            planes.loops.as_ptr(),
            planes.segments.as_ptr(),
            native_circles.as_ptr(),
            &native_e,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
