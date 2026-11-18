//! One concave quarter cylinder with a circular and an elliptic planar stop.
use super::*;
use fabstep_core::{CircularArc, EllipticArc};
use std::f64::consts::FRAC_PI_2;

type Point = [f64; 3];
const GUARD: f64 = 1e-7; // The corner identity required by the OCCT transfer.

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
}

pub(super) fn invalid() -> FabError {
    FabError::new(
        "cad.invalid_model",
        "Le cylindre de poche et les contours de ses butées droite et oblique sont incohérents.",
    )
}

fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}

fn dot(a: Point, b: Point) -> f64 {
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

fn finite(p: &Point) -> bool {
    p.iter().all(|v| v.is_finite())
}

fn check_curves(
    vertices: &[Point],
    circle: &CircularArc,
    ellipse: &EllipticArc,
) -> Result<(), FabError> {
    let ids = [circle.vertices, ellipse.vertices].concat();
    let (r, major) = (circle.radius_mm, ellipse.major_mm);
    if ids.iter().any(|&id| id >= vertices.len())
        || ids.iter().enumerate().any(|(i, id)| ids[..i].contains(id))
        || !finite(&circle.center)
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
        || dot(centers, z) <= major * dot(major_dir, z).abs() + GUARD
    {
        return Err(invalid());
    }
    let start = sub(vertices[circle.vertices[0]], circle.center);
    let x = start.map(|v| v / r);
    if dot(start, z).abs() > GUARD || (norm(start) - r).abs() > GUARD {
        return Err(invalid());
    }
    let y = cross(z, x);
    for (i, t) in [0., circle.sweep_rad].into_iter().enumerate() {
        let lo = vertices[circle.vertices[i]];
        let hi = vertices[ellipse.vertices[i]];
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
struct Planes {
    normals: Vec<Point>,
    offsets: Vec<usize>,
    loops: Vec<NativeMixedLoop>,
    segments: Vec<NativeMixedSegment>,
}

// Every material line has two oppositely oriented uses on distinct faces.
// Face index 11 is the concave cylinder, after the eleven planes.
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

impl Planes {
    fn encode(
        model: &MixedModel,
        circle: &CircularArc,
        ellipse: &EllipticArc,
    ) -> Result<Self, FabError> {
        let mut out = Self::default();
        let mut edges = BTreeMap::new();
        let rails = [
            [circle.vertices[0], ellipse.vertices[0]],
            [ellipse.vertices[1], circle.vertices[1]],
        ];
        for rail in rails {
            register_line(&mut edges, rail, 11)?;
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
                        MixedSegment::Arc {
                            arc: 0,
                            reversed: false,
                        } if stops[0].is_none() => {
                            if norm(sub(normal, circle.axis)) > 1e-7 {
                                return Err(invalid());
                            }
                            stops[0] = Some(face);
                            let angle = circle.sweep_rad;
                            patch_area += dot(normal, circle.axis)
                                * circle.radius_mm.powi(2)
                                * (angle - angle.sin());
                            (0, circle.vertices)
                        }
                        MixedSegment::EllipticArc {
                            arc: 0,
                            reversed: true,
                        } if stops[1].is_none() => {
                            if norm(sub(normal, ellipse.axis.map(|v| -v))) > 1e-7 {
                                return Err(invalid());
                            }
                            stops[1] = Some(face);
                            let angle = ellipse.sweep_rad;
                            patch_area -= dot(normal, ellipse.axis)
                                * ellipse.major_mm
                                * ellipse.minor_mm
                                * (angle - angle.sin());
                            (1, [ellipse.vertices[1], ellipse.vertices[0]])
                        }
                        _ => return Err(invalid()),
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
        let mut neighbors = stops.to_vec();
        for (i, rail) in rails.into_iter().enumerate() {
            let face =
                edges[&edge_key(rail)][usize::from(rail[1] >= rail[0])].ok_or_else(invalid)?;
            if neighbors.contains(&Some(face)) {
                return Err(invalid());
            }
            neighbors.push(Some(face));
            let inward = sub(circle.center, points[circle.vertices[i]]);
            if norm(sub(inward, out.normals[face].map(|x| x * circle.radius_mm))) > GUARD {
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
    let ([wall], [circle], [ellipse]) = (
        model.partial_cylinders.as_slice(),
        model.arcs.as_slice(),
        model.elliptic_arcs.as_slice(),
    ) else {
        return Err(invalid());
    };
    if !wall.elliptic_end
        || !wall.inner
        || wall.arcs != [0, 0]
        || model.planes.len() != 11
        || !model.rings.is_empty()
        || !model.cylinders.is_empty()
        || model.vertices.len() > MAX_SOURCE_VERTICES
        || !model.vertices.iter().all(finite)
    {
        return Err(invalid());
    }
    check_curves(&model.vertices, circle, ellipse)?;
    let planes = Planes::encode(model, circle, ellipse)?;
    let c = NativeMixedArc {
        center: circle.center,
        axis: circle.axis,
        radius: circle.radius_mm,
        sweep: circle.sweep_rad,
        vertices: circle.vertices,
    };
    let e = NativeEllipse {
        center: ellipse.center,
        axis: ellipse.axis,
        major_direction: ellipse.major_direction,
        major: ellipse.major_mm,
        minor: ellipse.minor_mm,
        start: ellipse.start_rad,
        sweep: ellipse.sweep_rad,
        vertices: ellipse.vertices,
    };
    native_step(|output, error, capacity| unsafe {
        fabstep_build_oblique_pocket_step(
            model.vertices.as_ptr(),
            model.vertices.len(),
            planes.normals.as_ptr(),
            planes.offsets.as_ptr(),
            planes.loops.as_ptr(),
            planes.segments.as_ptr(),
            &c,
            &e,
            tokens.map_or(std::ptr::null(), |t| t.planes.as_ptr()),
            tokens.map_or(std::ptr::null(), |t| t.partial.as_ptr()),
            output,
            error,
            capacity,
        )
    })
}
