//! Transfer the bounded two-radius pocket and the owners of its thirteen faces.
use super::oblique_pocket::{dot, finite, invalid, Point, GUARD};
use super::*;
use fabstep_core::UnequalPocket;

#[repr(C)]
struct NativeUnequalPocket {
    origin: Point,
    axes: [Point; 3],
    radii: [f64; 2],
    outer_min: Point,
    outer_max: Point,
    pocket_spans: [f64; 2],
}

unsafe extern "C" {
    fn fabstep_build_unequal_pocket_step(
        pocket: *const NativeUnequalPocket,
        plane_tokens: *const usize,
        wall_tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

fn valid_dimensions(p: &UnequalPocket) -> bool {
    let [small, large] = p.radii;
    let [width, depth] = p.pocket_spans;
    let height = p.outer_max[2];
    let finite_extents =
        p.outer_min.iter().zip(p.outer_max).all(|(lo, hi)| {
            lo.is_finite() && hi.is_finite() && (hi - lo).is_finite() && *lo < -GUARD
        });
    finite_extents
        && finite(&p.origin)
        && p.radii.iter().chain(&p.pocket_spans).all(|v| v.is_finite())
        && small > GUARD
        && large - small > GUARD
        && width - large > GUARD
        && depth - small > GUARD
        && height - large > GUARD
        && p.outer_max[0] - width > GUARD
        && p.outer_max[1] - depth > GUARD
}

pub(super) fn build(
    model: &RoundedModel,
    p: &UnequalPocket,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let m = &model.revolved.mixed;
    if !valid_dimensions(p)
        || !p.axes.iter().all(unit_direction)
        || [(0, 1), (0, 2), (1, 2)]
            .iter()
            .any(|&(i, j)| dot(p.axes[i], p.axes[j]).abs() > 1e-8)
        || m.planes.len() != 11
        || !m.partial_cylinders.is_empty()
        || !m.vertices.is_empty()
        || !m.rings.is_empty()
        || !m.cylinders.is_empty()
        || !m.arcs.is_empty()
        || !m.elliptic_arcs.is_empty()
        || !model.revolved.cones.is_empty()
        || !model.partial_cones.is_empty()
        || !model.spheres.is_empty()
        || !model.tori.is_empty()
        || !model.torus_sectors.is_empty()
        || !model.octants.is_empty()
        || model.linear_blend.is_some()
        || model.cross_drill.is_some()
        || model.smooth_revolution.is_some()
    {
        return Err(invalid());
    }
    // Six exterior faces, four inward-facing sides of the cavity, then its floor.
    let [x, y, z] = p.axes;
    let neg = |n: Point| n.map(|v| -v);
    let normals = [neg(x), x, neg(y), y, neg(z), z, x, neg(x), y, neg(y), z];
    for (marker, n) in m.planes.iter().zip(normals) {
        if !marker.loops.is_empty()
            || !unit_direction(&marker.normal)
            || (0..3).any(|i| (marker.normal[i] - n[i]).abs() > 1e-7)
        {
            return Err(invalid());
        }
    }
    let native = NativeUnequalPocket {
        origin: p.origin,
        axes: p.axes,
        radii: p.radii,
        outer_min: p.outer_min,
        outer_max: p.outer_max,
        pocket_spans: p.pocket_spans,
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_unequal_pocket_step(
            &native,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
