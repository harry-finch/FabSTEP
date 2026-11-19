//! Canonical three-sector input for the OCCT spherical primitive. Reject any
//! extra face or inconsistent model boundary before handing the frame to OCCT.
use super::*;

extern "C" {
    fn fabstep_build_cut_octant(
        center: *const f64,
        axes: *const f64,
        radius: f64,
        tokens: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

pub(super) fn build(
    m: &RoundedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les trois plans et arcs ne délimitent pas un octant sphérique.",
        )
    };
    let mixed = &m.revolved.mixed;
    let [s] = m.octants.as_slice() else {
        return Err(invalid());
    };
    let r = s.radius_mm;
    let axes = s.directions;
    if s.inner
        || !r.is_finite()
        || r <= 4e-7
        || s.center.iter().any(|v| !v.is_finite())
        || !axes.iter().all(unit_direction)
        || s.arc_uses != [(0, false), (1, false), (2, false)]
        || mixed.planes.len() != 3
        || mixed.arcs.len() != 3
        || !mixed.elliptic_arcs.is_empty()
        || !mixed.rings.is_empty()
        || !mixed.cylinders.is_empty()
        || !mixed.partial_cylinders.is_empty()
        || !m.revolved.cones.is_empty()
        || !m.spheres.is_empty()
        || !m.tori.is_empty()
        || !m.torus_sectors.is_empty()
        || !m.partial_cones.is_empty()
        || m.drilled_shaft.is_some()
        || m.unequal_pocket.is_some()
        || m.cross_drill.is_some()
        || m.linear_blend.is_some()
        || m.smooth_revolution.is_some()
        || mixed.vertices.len() > MAX_SOURCE_VERTICES
        || mixed.vertices.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(invalid());
    }
    let coord = |id: usize| mixed.vertices.get(id).copied().ok_or_else(invalid);
    let mut corner = None;
    let eps = 5e-8;
    for (i, arc) in mixed.arcs.iter().enumerate() {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let cross = std::array::from_fn(|n| {
            axes[i][(n + 1) % 3] * axes[j][(n + 2) % 3]
                - axes[i][(n + 2) % 3] * axes[j][(n + 1) % 3]
        });
        if distance(cross, axes[k]) > (eps / r).min(1e-9)
            || !arc.sweep_rad.is_finite()
            || !arc.radius_mm.is_finite()
            || arc.axis.iter().chain(&arc.center).any(|v| !v.is_finite())
            || (arc.radius_mm - r).abs() > eps
            || (arc.sweep_rad - std::f64::consts::FRAC_PI_2).abs() * r > eps
            || distance(arc.center, s.center) > eps
            || distance(arc.axis, axes[k]) > (eps / r).min(1e-9)
            || arc.vertices[1] != mixed.arcs[j].vertices[0]
            || distance(
                coord(arc.vertices[0])?,
                std::array::from_fn(|n| s.center[n] + r * axes[i][n]),
            ) > eps
        {
            return Err(invalid());
        }
        let plane = &mixed.planes[i];
        let [MixedLoop::Segments { segments }] = plane.loops.as_slice() else {
            return Err(invalid());
        };
        let [MixedSegment::Line { vertices: ab }, MixedSegment::Arc {
            arc: id,
            reversed: true,
        }, MixedSegment::Line { vertices: ca }] = segments.as_slice()
        else {
            return Err(invalid());
        };
        if *id != i
            || ab[1] != arc.vertices[1]
            || ca[0] != arc.vertices[0]
            || ab[0] != ca[1]
            || *corner.get_or_insert(ab[0]) != ab[0]
            || distance(coord(ab[0])?, s.center) > eps
            || !unit_direction(&plane.normal)
            || distance(plane.normal, axes[k].map(|x| -x)) > (eps / r).min(1e-9)
        {
            return Err(invalid());
        }
    }
    let ids = tokens.map(|t| [t.planes[0], t.planes[1], t.planes[2], t.octants[0]]);
    native_step(|output, error, capacity| unsafe {
        fabstep_build_cut_octant(
            s.center.as_ptr(),
            axes.as_ptr().cast(),
            r,
            ids.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
