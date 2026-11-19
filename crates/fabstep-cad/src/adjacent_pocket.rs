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
    fn fabstep_build_u_pocket_step(
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
    fn fabstep_build_closed_pocket_step(
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
        || [a, b].iter().any(|w| {
            !w.inner || w.elliptic_start || !w.elliptic_end || w.arcs[1] != 0 || w.arcs[0] >= 2
        })
    {
        return Err(invalid());
    }
    let circles = [&model.arcs[a.arcs[0]], &model.arcs[b.arcs[0]]];
    if dot(circles[0].axis, circles[1].axis).abs() >= std::f64::consts::FRAC_1_SQRT_2
        || (circles[0].radius_mm - circles[1].radius_mm).abs() > GUARD
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

/// Both ellipses use the orientation of the central wall. This chart has
/// elliptic stops at both ends; each flank uses a circular, planar stop.
pub(super) fn build_network(
    model: &MixedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    use super::oblique_pocket::{double_frame, transverse_rails};
    let ([a, b, c], [left, right], [lo, hi]) = (
        model.partial_cylinders.as_slice(),
        model.arcs.as_slice(),
        model.elliptic_arcs.as_slice(),
    ) else {
        return Err(invalid());
    };
    if model.planes.len() != 11
        || !model.cylinders.is_empty()
        || !model.rings.is_empty()
        || model.vertices.len() > MAX_SOURCE_VERTICES
        || !model.vertices.iter().all(finite)
        || [a, b, c].iter().any(|w| !w.inner || !w.elliptic_end)
        || a.elliptic_start
        || b.elliptic_start
        || !c.elliptic_start
        || a.arcs != [0, 0]
        || b.arcs != [1, 1]
        || c.arcs != [0, 1]
        || dot(left.axis, right.axis) < 1. - 1e-10
    {
        return Err(invalid());
    }
    let frame = double_frame(&model.vertices, lo, hi, true)?;
    if dot(left.axis, frame.axis).abs() > 1e-7 {
        return Err(invalid());
    }
    let mut rails = Vec::new();
    for (circle, ellipse) in [(left, lo), (right, hi)] {
        let mut e = ellipse.clone();
        if dot(circle.axis, e.axis) < 0. {
            e.axis = e.axis.map(|v| -v);
            e.start_rad = -(e.start_rad + e.sweep_rad);
            e.vertices.swap(0, 1);
        }
        check_curves(&model.vertices, circle, &e, true)?;
        rails.extend(wall_rails(&model.vertices, circle, e.vertices));
    }
    rails.extend(transverse_rails(&model.vertices, &frame, hi.vertices));
    let planes = Planes::encode(
        model,
        [Stop::circle(0, left), Stop::circle(1, right)],
        &rails,
    )?;
    let circles = [
        native_circle(left),
        native_circle(right),
        native_circle(&frame),
    ];
    let ellipses = [native_ellipse(lo), native_ellipse(hi)];
    native_step(|output, error, capacity| unsafe {
        fabstep_build_u_pocket_step(
            model.vertices.as_ptr(),
            model.vertices.len(),
            planes.normals.as_ptr(),
            planes.offsets.as_ptr(),
            planes.loops.as_ptr(),
            planes.segments.as_ptr(),
            circles.as_ptr(),
            ellipses.as_ptr(),
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}

pub(super) fn build_cycle(
    model: &MixedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    use super::oblique_pocket::{double_frame, transverse_rails};
    if model.planes.len() != 11
        || !model.arcs.is_empty()
        || !model.rings.is_empty()
        || !model.cylinders.is_empty()
        || model.vertices.len() > MAX_SOURCE_VERTICES
        || !model.vertices.iter().all(finite)
        || model.elliptic_arcs.len() != 4
        || model.partial_cylinders.len() != 4
    {
        return Err(invalid());
    }
    let mut circles = Vec::new();
    let mut rails = Vec::new();
    for (i, wall) in model.partial_cylinders.iter().enumerate() {
        let next = (i + 1) % 4;
        if !wall.inner || !wall.elliptic_start || !wall.elliptic_end || wall.arcs != [i, next] {
            return Err(invalid());
        }
        let (low, high) = (&model.elliptic_arcs[i], &model.elliptic_arcs[next]);
        let frame = double_frame(&model.vertices, low, high, true)?;
        rails.extend(transverse_rails(&model.vertices, &frame, high.vertices));
        circles.push(native_circle(&frame));
    }
    for i in 0..4 {
        if dot(circles[i].axis, circles[(i + 1) % 4].axis).abs() > 1e-7
            || dot(circles[i].axis, circles[(i + 2) % 4].axis) > -1. + 1e-10
        {
            return Err(invalid());
        }
    }
    let planes = Planes::encode(model, [], &rails)?;
    let ellipses: Vec<_> = model.elliptic_arcs.iter().map(native_ellipse).collect();
    native_step(|output, error, capacity| unsafe {
        fabstep_build_closed_pocket_step(
            model.vertices.as_ptr(),
            model.vertices.len(),
            planes.normals.as_ptr(),
            planes.offsets.as_ptr(),
            planes.loops.as_ptr(),
            planes.segments.as_ptr(),
            circles.as_ptr(),
            ellipses.as_ptr(),
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
