//! The bounded sector builder consumes scalar dimensions. Check that those
//! dimensions describe every supplied edge and face before crossing the ABI.
use super::*;

type Point = [f64; 3];
const GUARD: f64 = 5e-8;

#[repr(C)]
pub(super) struct NativeTorusSector {
    origin: Point,
    axis: Point,
    start: Point,
    height: f64,
    inner_radius: f64,
    major_radius: f64,
    minor_radius: f64,
    sweep: f64,
}

fn along(p: Point, d: Point, t: f64) -> Point {
    std::array::from_fn(|i| p[i] + d[i] * t)
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn near(a: Point, b: Point, tolerance: f64) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]) <= tolerance
}

fn arc_matches(
    arc: &fabstep_core::CircularArc,
    center: Point,
    axis: Point,
    radius: f64,
    angle: f64,
    angle_guard: f64,
) -> bool {
    near(arc.center, center, GUARD)
        && near(arc.axis, axis, angle_guard)
        && (arc.radius_mm - radius).abs() <= GUARD
        && (arc.sweep_rad - angle).abs() * radius <= GUARD
}

fn invalid() -> FabError {
    FabError::new(
        "cad.invalid_model",
        "Les limites du secteur torique et de ses faces voisines sont incohérentes.",
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Arc(usize, bool),
    Line(usize, usize),
}

fn check_plane(
    plane: &fabstep_core::MixedPlane,
    normal: Point,
    angle_guard: f64,
    expected: &[Edge],
) -> bool {
    let [MixedLoop::Segments { segments }] = plane.loops.as_slice() else {
        return false;
    };
    near(plane.normal, normal, angle_guard)
        && segments.len() == expected.len()
        && (0..expected.len()).any(|start| {
            segments.iter().enumerate().all(|(offset, segment)| {
                let actual = match segment {
                    MixedSegment::Arc { arc, reversed } => Edge::Arc(*arc, *reversed),
                    MixedSegment::Line { vertices: [a, b] } => Edge::Line(*a, *b),
                    _ => return false,
                };
                actual == expected[(start + offset) % expected.len()]
            })
        })
}

impl NativeTorusSector {
    fn from_model(model: &RoundedModel) -> Result<Self, FabError> {
        let [sector] = model.torus_sectors.as_slice() else {
            return Err(invalid());
        };
        let m = &model.revolved.mixed;
        if sector.inner
            || model.smooth_revolution.is_some()
            || model.cross_drill.is_some()
            || model.linear_blend.is_some()
            || !model.octants.is_empty()
            || !model.tori.is_empty()
            || !model.partial_cones.is_empty()
            || !model.revolved.cones.is_empty()
            || !model.spheres.is_empty()
            || !m.cylinders.is_empty()
            || !m.rings.is_empty()
            || !m.elliptic_arcs.is_empty()
            || m.planes.len() != 4
            || m.arcs.len() != 7
            || m.partial_cylinders.len() != 2
            || m.vertices.len() > MAX_SOURCE_VERTICES
            || !m.vertices.iter().flatten().all(|v| v.is_finite())
            || sector.arc_uses != [(2, false), (6, false), (3, true), (5, true)]
        {
            return Err(invalid());
        }
        let (r, major, height, inner) = (
            sector.minor_radius_mm,
            sector.major_radius_mm,
            sector.height_mm,
            sector.inner_radius_mm,
        );
        let (axis, start, sweep) = (sector.axis, sector.start_direction, sector.sweep_rad);
        if ![r, major, height, inner, sweep]
            .into_iter()
            .all(f64::is_finite)
            || !(r > GUARD
                && major - r > GUARD
                && height - r > GUARD
                && inner > GUARD
                && major - inner > GUARD
                && sweep > 0.
                && sweep < std::f64::consts::PI)
        {
            return Err(invalid());
        }
        let origin = along(sector.center, axis, r - height);
        let extent = (major + r).max(height).max(1.);
        let angle_guard = GUARD / extent;
        let tangent = cross(axis, start);
        let dot: f64 = axis.iter().zip(start).map(|(a, b)| a * b).sum();
        let unit = |v: Point| (v[0].hypot(v[1]).hypot(v[2]) - 1.).abs() <= angle_guard;
        if !unit(axis)
            || !unit(start)
            || dot.abs() > angle_guard
            || origin
                .iter()
                .chain(&sector.center)
                .any(|x| !x.is_finite() || x.abs() * 512. * f64::EPSILON > GUARD)
            || extent * 512. * f64::EPSILON > GUARD
        {
            return Err(invalid());
        }
        let end = along(start.map(|x| x * sweep.cos()), tangent, sweep.sin());
        let end_tangent = cross(axis, end);
        let mut endpoints = [[0usize; 2]; 5];
        let parallels = [
            (inner, 0.),
            (major + r, 0.),
            (major + r, height - r),
            (major, height),
            (inner, height),
        ];
        for (j, (radius, z)) in parallels.into_iter().enumerate() {
            let center = along(origin, axis, z);
            let arc = &m.arcs[j];
            if !arc_matches(arc, center, axis, radius, sweep, angle_guard) {
                return Err(invalid());
            }
            for (k, dir) in [start, end].into_iter().enumerate() {
                let id = arc.vertices[k];
                if !m
                    .vertices
                    .get(id)
                    .is_some_and(|&p| near(p, along(center, dir, radius), GUARD))
                {
                    return Err(invalid());
                }
                endpoints[j][k] = id;
            }
        }
        if endpoints
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 10
        {
            return Err(invalid());
        }
        for (k, (dir, tan)) in [(start, tangent), (end, end_tangent)]
            .into_iter()
            .enumerate()
        {
            let arc = &m.arcs[5 + k];
            if !arc_matches(
                arc,
                along(sector.center, dir, major),
                tan.map(|x| -x),
                r,
                std::f64::consts::FRAC_PI_2,
                angle_guard,
            ) || arc.vertices != [endpoints[2][k], endpoints[3][k]]
            {
                return Err(invalid());
            }
        }
        for (wall, (arcs, inner_wall)) in m
            .partial_cylinders
            .iter()
            .zip([([1, 2], false), ([0, 4], true)])
        {
            if wall.arcs != arcs || wall.inner != inner_wall {
                return Err(invalid());
            }
        }
        use Edge::{Arc, Line};
        let line = |p: usize, q: usize, k: usize| Line(endpoints[p][k], endpoints[q][k]);
        if !check_plane(
            &m.planes[0],
            axis.map(|x| -x),
            angle_guard,
            &[Arc(0, false), line(0, 1, 1), Arc(1, true), line(1, 0, 0)],
        ) || !check_plane(
            &m.planes[1],
            axis,
            angle_guard,
            &[Arc(3, false), line(3, 4, 1), Arc(4, true), line(4, 3, 0)],
        ) || !check_plane(
            &m.planes[2],
            tangent.map(|x| -x),
            angle_guard,
            &[
                line(0, 1, 0),
                line(1, 2, 0),
                Arc(5, false),
                line(3, 4, 0),
                line(4, 0, 0),
            ],
        ) || !check_plane(
            &m.planes[3],
            end_tangent,
            angle_guard,
            &[
                line(0, 4, 1),
                line(4, 3, 1),
                Arc(6, true),
                line(2, 1, 1),
                line(1, 0, 1),
            ],
        ) {
            return Err(invalid());
        }
        Ok(Self {
            origin,
            axis,
            start,
            height,
            inner_radius: inner,
            major_radius: major,
            minor_radius: r,
            sweep,
        })
    }
}

pub(super) fn build(
    model: &RoundedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let domain = NativeTorusSector::from_model(model)?;
    let ordered = tokens.map(|t| {
        [
            t.planes[0],
            t.planes[1],
            t.planes[2],
            t.planes[3],
            t.partial[1],
            t.partial[0],
            t.torus_sectors[0],
        ]
    });
    native_step(|output, error, capacity| unsafe {
        fabstep_build_torus_sector_step(
            &domain,
            ordered.as_ref().map_or(std::ptr::null(), |a| a.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
