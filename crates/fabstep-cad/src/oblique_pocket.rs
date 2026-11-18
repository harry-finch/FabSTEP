//! One concave quarter cylinder with a circular and an elliptic planar stop.
use super::*;
use fabstep_core::{CircularArc, EllipticArc};
use std::f64::consts::FRAC_PI_2;

pub(super) type Point = [f64; 3];
pub(super) const GUARD: f64 = 1e-7; // The corner identity required by the OCCT transfer.

unsafe extern "C" {
    fn fabstep_build_oblique_pocket_step(
        points: *const Point,
        point_count: usize,
        normals: *const Point,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        circle: *const NativeMixedArc,
        ellipse: *const NativeEllipse,
        plane_tokens: *const usize,
        wall_token: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_build_double_oblique_pocket_step(
        points: *const Point,
        point_count: usize,
        normals: *const Point,
        plane_loops: *const usize,
        loops: *const NativeMixedLoop,
        segments: *const NativeMixedSegment,
        circle: *const NativeMixedArc,
        ellipse: *const NativeEllipse,
        plane_tokens: *const usize,
        wall_token: *const usize,
        output: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

pub(super) fn invalid() -> FabError {
    FabError::new(
        "cad.invalid_model",
        "Les congés de la poche et les contours de leurs faces voisines sont incohérents.",
    )
}

fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}

pub(super) fn dot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn cross([x, y, z]: Point, [u, v, w]: Point) -> Point {
    [y * w - z * v, z * u - x * w, x * v - y * u]
}

fn norm(p: Point) -> f64 {
    p[0].hypot(p[1]).hypot(p[2])
}

fn offset(p: Point, direction: Point, distance: f64) -> Point {
    std::array::from_fn(|i| p[i] + distance * direction[i])
}

pub(super) fn finite(p: &Point) -> bool {
    p.iter().all(|v| v.is_finite())
}

pub(super) fn check_curves(
    vertices: &[Point],
    circle: &CircularArc,
    ellipse: &EllipticArc,
    shared_by_cylinders: bool,
) -> Result<(), FabError> {
    let ids = [circle.vertices, ellipse.vertices].concat();
    if ids.iter().any(|&id| id >= vertices.len())
        || ids.iter().enumerate().any(|(i, id)| ids[..i].contains(id))
    {
        return Err(invalid());
    }
    check_section(
        circle,
        ellipse,
        circle.vertices.map(|i| vertices[i]),
        ellipse.vertices.map(|i| vertices[i]),
        shared_by_cylinders,
    )
}

fn check_section(
    circle: &CircularArc,
    ellipse: &EllipticArc,
    low: [Point; 2],
    high: [Point; 2],
    shared_by_cylinders: bool,
) -> Result<(), FabError> {
    let (r, major) = (circle.radius_mm, ellipse.major_mm);
    if !finite(&circle.center)
        || !finite(&ellipse.center)
        || !unit_direction(&circle.axis)
        || !unit_direction(&ellipse.axis)
        || !unit_direction(&ellipse.major_direction)
        || ![
            r,
            major,
            ellipse.minor_mm,
            circle.sweep_rad,
            ellipse.sweep_rad,
            ellipse.start_rad,
        ]
        .iter()
        .all(|a| a.is_finite())
        || r <= GUARD
        || ellipse.minor_mm <= GUARD
        || major <= ellipse.minor_mm
        || (ellipse.minor_mm - r).abs() > GUARD
        || circle.sweep_rad <= 0.
        || ellipse.sweep_rad <= 0.
        || (circle.sweep_rad - FRAC_PI_2).abs() > 1e-5
        || (ellipse.sweep_rad - circle.sweep_rad).abs() * major > GUARD
    {
        return Err(invalid());
    }
    let z = circle.axis;
    let cosine = dot(ellipse.axis, z);
    let major_dir = ellipse.major_direction;
    let minor_dir = cross(ellipse.axis, major_dir);
    let centers = sub(ellipse.center, circle.center);
    // Projecting the ellipse along the axis must give this exact circle:
    // its minor axis stays transverse, its major axis contracts by cosine.
    if !(0.1..0.99999).contains(&cosine)
        || dot(ellipse.axis, major_dir).abs() * major > GUARD
        || dot(minor_dir, z).abs() * ellipse.minor_mm > GUARD
        || (major * cosine - r).abs() > GUARD
        || norm(cross(centers, z)) > GUARD
        || (!shared_by_cylinders && dot(centers, z) <= major * dot(major_dir, z).abs() + GUARD)
    {
        return Err(invalid());
    }
    let start = sub(low[0], circle.center);
    let x = start.map(|v| v / r);
    if dot(start, z).abs() > GUARD || (norm(start) - r).abs() > GUARD {
        return Err(invalid());
    }
    let y = cross(z, x);
    if shared_by_cylinders {
        // In a short pocket the rest of the full ellipse can cross the stop
        // plane. Only the quarter between the witnessed rails bounds this wall.
        let h = dot(centers, z);
        let hc = -r * dot(ellipse.axis, x) / cosine;
        let hs = -r * dot(ellipse.axis, y) / cosine;
        let u = (-hs).atan2(-hc).rem_euclid(std::f64::consts::TAU);
        let (sin, cos) = circle.sweep_rad.sin_cos();
        let mut min_height = (h + hc).min(h + hc * cos + hs * sin);
        if u <= circle.sweep_rad {
            min_height = min_height.min(h - hc.hypot(hs));
        }
        if !min_height.is_finite() || min_height <= GUARD {
            return Err(invalid());
        }
    }
    for (i, t) in [0., circle.sweep_rad].into_iter().enumerate() {
        let lo = low[i];
        let hi = high[i];
        let near = offset(offset(circle.center, x, r * t.cos()), y, r * t.sin());
        let u = ellipse.start_rad + i as f64 * ellipse.sweep_rad;
        let far = offset(
            offset(ellipse.center, major_dir, major * u.cos()),
            minor_dir,
            ellipse.minor_mm * u.sin(),
        );
        if norm(sub(lo, near)) > GUARD
            || norm(sub(hi, far)) > GUARD
            || norm(cross(sub(hi, lo), z)) > GUARD
            || dot(sub(hi, lo), z) <= GUARD
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct Planes {
    pub(super) normals: Vec<Point>,
    pub(super) offsets: Vec<usize>,
    pub(super) loops: Vec<NativeMixedLoop>,
    pub(super) segments: Vec<NativeMixedSegment>,
}

// Every material line has two oppositely oriented uses on distinct faces.
// Cylinder face indices immediately follow the eleven planes.
fn register_line(
    lines: &mut BTreeMap<(usize, usize), [Option<usize>; 2]>,
    [from, to]: [usize; 2],
    face: usize,
) -> Result<(), FabError> {
    if from == to {
        return Err(invalid());
    }
    let entry = lines.entry(edge_key([from, to])).or_default();
    let side = usize::from(to < from);
    if entry[side].is_some() || entry[1 - side] == Some(face) {
        return Err(invalid());
    }
    entry[side] = Some(face);
    Ok(())
}

/// The two arcs appearing on planar stops. For adjacent walls the common
/// ellipse is internal to their pair and never appears in these records.
pub(super) struct Stop {
    arc: usize,
    ellipse: bool,
    vertices: [usize; 2],
    normal: Point,
    area_adjustment: f64,
    reversed: bool,
}

impl Stop {
    pub(super) fn circle(arc: usize, c: &CircularArc) -> Self {
        Self {
            arc,
            reversed: false,
            ellipse: false,
            vertices: c.vertices,
            normal: c.axis,
            area_adjustment: c.radius_mm.powi(2) * (c.sweep_rad - c.sweep_rad.sin()),
        }
    }

    fn ellipse(e: &EllipticArc) -> Self {
        Self {
            arc: 0,
            reversed: true,
            ellipse: true,
            vertices: [e.vertices[1], e.vertices[0]],
            normal: e.axis.map(|v| -v),
            area_adjustment: e.major_mm * e.minor_mm * (e.sweep_rad - e.sweep_rad.sin()),
        }
    }
}

/// Lines are oriented along the boundary of the concave cylindrical face.
pub(super) struct Rail {
    vertices: [usize; 2],
    seam_vertex: usize,
    inward: Point,
    radius: f64,
}

pub(super) fn wall_rails(points: &[Point], c: &CircularArc, far: [usize; 2]) -> [Rail; 2] {
    std::array::from_fn(|i| Rail {
        vertices: if i == 0 {
            [c.vertices[i], far[i]]
        } else {
            [far[i], c.vertices[i]]
        },
        seam_vertex: far[i],
        inward: sub(c.center, points[c.vertices[i]]),
        radius: c.radius_mm,
    })
}

impl Planes {
    pub(super) fn encode(
        model: &MixedModel,
        curves: [Stop; 2],
        rails: &[Rail],
    ) -> Result<Self, FabError> {
        let mut out = Self::default();
        let mut edges = BTreeMap::new();
        for (i, rail) in rails.iter().enumerate() {
            register_line(&mut edges, rail.vertices, model.planes.len() + i / 2)?;
        }
        let mut stops = [None; 2];
        let points = &model.vertices;
        for (face, plane) in model.planes.iter().enumerate() {
            let normal = plane.normal;
            if !unit_direction(&normal) || !(1..=2).contains(&plane.loops.len()) {
                return Err(invalid());
            }
            out.normals.push(normal);
            out.offsets.push(out.loops.len());
            let mut plane_origin = None;
            for (boundary, path) in plane.loops.iter().enumerate() {
                let count = match path {
                    MixedLoop::Polygon { vertices } => vertices.len(),
                    MixedLoop::Segments { segments } => segments.len(),
                    MixedLoop::Circle { .. } => return Err(invalid()),
                };
                if count < 3 || count > MAX_SOURCE_VERTICES - out.segments.len() {
                    return Err(invalid());
                }
                let start = out.segments.len();
                let mut patch_area = 0.;
                for i in 0..count {
                    let segment = match path {
                        MixedLoop::Polygon { vertices } => MixedSegment::Line {
                            vertices: [vertices[i], vertices[(i + 1) % count]],
                        },
                        MixedLoop::Segments { segments } => segments[i].clone(),
                        MixedLoop::Circle { .. } => unreachable!(),
                    };
                    let (arc, vertices) = match segment {
                        MixedSegment::Line { vertices } => {
                            if vertices.iter().any(|&id| id >= points.len()) {
                                return Err(invalid());
                            }
                            register_line(&mut edges, vertices, face)?;
                            (usize::MAX, vertices)
                        }
                        MixedSegment::Arc { arc, reversed }
                        | MixedSegment::EllipticArc { arc, reversed } => {
                            let elliptic = matches!(segment, MixedSegment::EllipticArc { .. });
                            let slot = curves
                                .iter()
                                .position(|curve| curve.arc == arc && curve.ellipse == elliptic)
                                .ok_or_else(invalid)?;
                            let curve = &curves[slot];
                            if reversed != curve.reversed
                                || stops[slot].replace(face).is_some()
                                || norm(sub(normal, curve.normal)) > 1e-7
                            {
                                return Err(invalid());
                            }
                            patch_area += dot(normal, curve.normal) * curve.area_adjustment;
                            (slot, curve.vertices)
                        }
                    };
                    let origin = *plane_origin.get_or_insert(points[vertices[0]]);
                    if dot(sub(points[vertices[0]], origin), normal).abs() > GUARD {
                        return Err(invalid());
                    }
                    out.segments.push(NativeMixedSegment { vertices, arc });
                }
                let loop_segments = &out.segments[start..];
                let ids: Vec<_> = loop_segments.iter().map(|s| s.vertices[0]).collect();
                let area = signed_area(&ids, points, normal) + patch_area;
                if !area.is_finite()
                    || (if boundary == 0 { area } else { -area }) <= 0.
                    || loop_segments
                        .iter()
                        .zip(loop_segments.iter().cycle().skip(1))
                        .any(|(s, t)| s.vertices[1] != t.vertices[0])
                {
                    return Err(invalid());
                }
                out.loops.push(NativeMixedLoop { start, count });
            }
        }
        out.offsets.push(out.loops.len());
        if out.loops.len() != 12
            || stops.contains(&None)
            || stops[0] == stops[1]
            || edges.values().any(|pair| pair.contains(&None))
        {
            return Err(invalid());
        }
        let mut supports = vec![];
        for rail in rails {
            let [from, to] = rail.vertices;
            let face =
                edges[&edge_key(rail.vertices)][usize::from(to >= from)].ok_or_else(invalid)?;
            if face >= model.planes.len()
                || stops.contains(&Some(face))
                || norm(sub(rail.inward, out.normals[face].map(|x| x * rail.radius))) > GUARD
            {
                return Err(invalid());
            }
            supports.push((face, rail.seam_vertex));
        }
        if supports.chunks_exact(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(invalid());
        }
        if let [a, b, c, d] = supports.as_slice() {
            // Exactly one endpoint of the ellipse lies on their common
            // tangent plane (the floor); the remaining supports are distinct.
            let mut shared = [(a, c), (a, d), (b, c), (b, d)]
                .into_iter()
                .filter(|(first, second)| first.0 == second.0);
            if shared
                .next()
                .is_none_or(|(first, second)| first.1 != second.1)
                || shared.next().is_some()
            {
                return Err(invalid());
            }
        }
        Ok(out)
    }
}

pub(super) fn build(
    model: &MixedModel,
    tokens: Option<SurfaceTokens<'_>>,
) -> Result<ValidatedStep, FabError> {
    let [wall] = model.partial_cylinders.as_slice() else {
        return Err(invalid());
    };
    if !wall.elliptic_end
        || !wall.inner
        || wall.arcs != [0, usize::from(wall.elliptic_start)]
        || model.planes.len() != 11
        || !model.rings.is_empty()
        || !model.cylinders.is_empty()
        || model.vertices.len() > MAX_SOURCE_VERTICES
        || !model.vertices.iter().all(finite)
    {
        return Err(invalid());
    }
    let double_stop = wall.elliptic_start;
    let (circle, curves) = match (
        model.arcs.as_slice(),
        model.elliptic_arcs.as_slice(),
        double_stop,
    ) {
        ([circle], [ellipse], false) => {
            check_curves(&model.vertices, circle, ellipse, false)?;
            (
                circle.clone(),
                [Stop::circle(0, circle), Stop::ellipse(ellipse)],
            )
        }
        ([], [low, high], true) => {
            let frame = double_frame(&model.vertices, low, high)?;
            let mut near = Stop::ellipse(low);
            near.vertices = low.vertices;
            near.normal = low.axis;
            near.reversed = false;
            let mut far = Stop::ellipse(high);
            far.arc = 1;
            (frame, [near, far])
        }
        _ => return Err(invalid()),
    };
    let far = model.elliptic_arcs.last().ok_or_else(invalid)?;
    let mut rails = wall_rails(&model.vertices, &circle, far.vertices);
    if double_stop {
        for rail in &mut rails {
            rail.inward = offset(rail.inward, circle.axis, -dot(rail.inward, circle.axis));
        }
    }
    let planes = Planes::encode(model, curves, &rails)?;
    let c = native_circle(&circle);
    let ellipses: Vec<_> = model.elliptic_arcs.iter().map(native_ellipse).collect();
    let build = if double_stop {
        fabstep_build_double_oblique_pocket_step
    } else {
        fabstep_build_oblique_pocket_step
    };
    native_step(|output, error, capacity| unsafe {
        build(
            model.vertices.as_ptr(),
            model.vertices.len(),
            planes.normals.as_ptr(),
            planes.offsets.as_ptr(),
            planes.loops.as_ptr(),
            planes.segments.as_ptr(),
            &c,
            ellipses.as_ptr(),
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}

/// Validate the two independent sections on a common transverse frame.
/// For each section use a reference circle just below its complete ellipse
/// and reuse the existing endpoint, incidence, basis and angular checks.
fn double_frame(
    points: &[Point],
    low: &EllipticArc,
    high: &EllipticArc,
) -> Result<CircularArc, FabError> {
    let ids = [low.vertices, high.vertices].concat();
    if ids
        .iter()
        .enumerate()
        .any(|(i, id)| *id >= points.len() || ids[..i].contains(id))
    {
        return Err(invalid());
    }
    let delta = sub(high.center, low.center);
    let distance = norm(delta);
    if !distance.is_finite() || distance <= GUARD {
        return Err(invalid());
    }
    let axis = delta.map(|v| v / distance);
    let r = low.minor_mm;
    let ray = sub(points[low.vertices[0]], low.center);
    let x = offset(ray, axis, -dot(ray, axis));
    let length = norm(x);
    if !length.is_finite() || (length - r).abs() > GUARD || length <= GUARD {
        return Err(invalid());
    }
    let x = x.map(|v| v / length);
    let y = cross(axis, x);
    let frame = CircularArc {
        axis,
        center: low.center,
        radius_mm: r,
        sweep_rad: FRAC_PI_2,
        vertices: low.vertices,
    };
    for e in [low, high] {
        let center = offset(e.center, axis, -2. * e.major_mm - 1.);
        let base = CircularArc {
            center,
            ..frame.clone()
        };
        check_section(
            &base,
            e,
            [offset(center, x, r), offset(center, y, r)],
            e.vertices.map(|i| points[i]),
            false,
        )?;
    }
    let grad = sub(
        low.axis.map(|v| v * r / dot(low.axis, axis)),
        high.axis.map(|v| v * r / dot(high.axis, axis)),
    );
    if distance <= dot(grad, x).hypot(dot(grad, y)) + GUARD {
        return Err(invalid());
    }
    Ok(frame)
}

pub(super) fn native_circle(circle: &CircularArc) -> NativeMixedArc {
    NativeMixedArc {
        center: circle.center,
        axis: circle.axis,
        radius: circle.radius_mm,
        sweep: circle.sweep_rad,
        vertices: circle.vertices,
    }
}

pub(super) fn native_ellipse(ellipse: &EllipticArc) -> NativeEllipse {
    NativeEllipse {
        center: ellipse.center,
        axis: ellipse.axis,
        major_direction: ellipse.major_direction,
        major: ellipse.major_mm,
        minor: ellipse.minor_mm,
        start: ellipse.start_rad,
        sweep: ellipse.sweep_rad,
        vertices: ellipse.vertices,
    }
}
