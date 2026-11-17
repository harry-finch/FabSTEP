//! Exact planar face boundaries, flattened in a millimetre local frame.
use crate::FabError;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Write};

#[derive(Debug, Clone, Serialize)]
pub struct PlanarFaceProfile {
    pub face_id: usize,
    pub solid_id: usize,
    pub frame: ProfileFrame,
    pub loops: Vec<PlanarProfileLoop>,
}

/// Right-handed orthonormal basis, mapping local millimetres back to the STEP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileFrame {
    pub origin_mm: [f64; 3],
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub normal: [f64; 3],
}

/// Explicit immutable section plane and sorted native solid identities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRequest {
    pub frame: ProfileFrame,
    pub solid_ids: Vec<usize>,
}

/// All selected bodies remain represented, including bodies missed by the plane.
#[derive(Debug, Clone, Serialize)]
pub struct SectionProfile {
    pub frame: ProfileFrame,
    pub bodies: Vec<SectionBodyProfile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionBodyProfile {
    pub solid_id: usize,
    pub loops: Vec<SectionLoop>,
}

/// Each disconnected component has one outer loop and zero or more holes.
#[derive(Debug, Clone, Serialize)]
pub struct SectionLoop {
    pub component_id: usize,
    pub is_outer: bool,
    pub segments: Vec<ProfileSegment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanarProfileLoop {
    pub is_outer: bool,
    pub segments: Vec<ProfileSegment>,
}

/// Coordinates are in the profile frame, with signed curve sweeps in radians.
/// Complete circles and ellipses must be alone in their loop.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProfileSegment {
    Line {
        start_mm: [f64; 2],
        end_mm: [f64; 2],
    },
    Arc {
        center_mm: [f64; 2],
        radius_mm: f64,
        start_angle_rad: f64,
        sweep_angle_rad: f64,
    },
    Circle {
        center_mm: [f64; 2],
        radius_mm: f64,
    },
    /// `major_axis_mm` is the vector from center to the major-axis endpoint.
    /// The minor vector is `ratio * [-major_axis_mm[1], major_axis_mm[0]]`.
    Ellipse {
        center_mm: [f64; 2],
        major_axis_mm: [f64; 2],
        ratio: f64,
        start_param_rad: f64,
        sweep_param_rad: f64,
    },
}

const MAX_LOOPS: usize = 4096;
const MAX_SEGMENTS: usize = 100_000;
const MAX_DXF_BYTES: usize = 16 * 1024 * 1024;
const JOIN_TOLERANCE_MM: f64 = 1e-7;
const FRAME_TOLERANCE: f64 = 1e-9;

fn invalid(message: &str) -> FabError {
    FabError::new("dxf.invalid_profile", message)
}

fn budget() -> FabError {
    FabError::new("dxf.too_complex", "Le profil dépasse les limites DXF.")
}

fn finite<const N: usize>(p: &[f64; N]) -> bool {
    p.iter().all(|v| v.is_finite())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn validate_frame(frame: &ProfileFrame) -> Result<(), FabError> {
    let axes = [frame.x_axis, frame.y_axis, frame.normal];
    if !finite(&frame.origin_mm) || axes.iter().any(|axis| !finite(axis)) {
        return Err(invalid(
            "Le repère du profil contient une valeur non finie.",
        ));
    }
    for (i, axis) in axes.iter().enumerate() {
        if (dot(*axis, *axis) - 1.).abs() > FRAME_TOLERANCE
            || axes[..i]
                .iter()
                .any(|previous| dot(*axis, *previous).abs() > FRAME_TOLERANCE)
        {
            return Err(invalid("Le repère du profil doit être orthonormé."));
        }
    }
    let [x, y, n] = axes;
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    if (dot(cross, n) - 1.).abs() > FRAME_TOLERANCE {
        return Err(invalid("Le repère du profil doit être direct."));
    }
    Ok(())
}

/// Validate a section plane, without normalizing or repairing its frame.
pub fn validate_profile_frame(frame: &ProfileFrame) -> Result<(), FabError> {
    validate_frame(frame).map_err(|error| FabError::new("dxf.section_invalid_plane", error.message))
}

fn validate_solid_ids(ids: impl Iterator<Item = usize>, count: usize) -> Result<(), FabError> {
    if count > 32 {
        return Err(budget());
    }
    if count == 0 {
        return Err(invalid(
            "La section doit conserver au moins un corps sélectionné.",
        ));
    }
    let mut previous = None;
    for id in ids {
        if id >= 32 || previous.is_some_and(|p| p >= id) {
            return Err(invalid(
                "Les identifiants de corps doivent être triés, uniques et inférieurs à 32.",
            ));
        }
        previous = Some(id);
    }
    Ok(())
}

pub fn validate_section_request(request: &SectionRequest) -> Result<(), FabError> {
    validate_profile_frame(&request.frame)?;
    validate_solid_ids(request.solid_ids.iter().copied(), request.solid_ids.len())
}

fn circle_point(center: [f64; 2], radius: f64, angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [center[0] + radius * cos, center[1] + radius * sin]
}

fn ellipse_point(center: [f64; 2], major: [f64; 2], ratio: f64, parameter: f64) -> [f64; 2] {
    let (sin, cos) = parameter.sin_cos();
    [
        center[0] + (major[0] * cos - ratio * major[1] * sin),
        center[1] + (major[1] * cos + ratio * major[0] * sin),
    ]
}

fn ellipse_phase(parameter: f64) -> f64 {
    // Trigonometric argument reduction also preserves finite, very large phases.
    let (sin, cos) = parameter.sin_cos();
    sin.atan2(cos).rem_euclid(std::f64::consts::TAU)
}

type Endpoints = ([f64; 2], [f64; 2]);

fn endpoints(segment: &ProfileSegment) -> Result<Option<Endpoints>, FabError> {
    let points = match *segment {
        ProfileSegment::Line { start_mm, end_mm } => {
            if start_mm == end_mm {
                return Err(invalid("Une ligne du profil est dégénérée."));
            }
            Some((start_mm, end_mm))
        }
        ProfileSegment::Arc {
            center_mm,
            radius_mm,
            start_angle_rad,
            sweep_angle_rad,
        } => {
            validate_circle(center_mm, radius_mm)?;
            if !start_angle_rad.is_finite()
                || !sweep_angle_rad.is_finite()
                || sweep_angle_rad == 0.
                || sweep_angle_rad.abs() >= std::f64::consts::TAU
            {
                return Err(invalid(
                    "Un arc doit avoir un balayage non nul inférieur à un tour.",
                ));
            }
            let start = start_angle_rad.rem_euclid(std::f64::consts::TAU);
            Some((
                circle_point(center_mm, radius_mm, start),
                circle_point(center_mm, radius_mm, start + sweep_angle_rad),
            ))
        }
        ProfileSegment::Circle {
            center_mm,
            radius_mm,
        } => {
            validate_circle(center_mm, radius_mm)?;
            None
        }
        ProfileSegment::Ellipse {
            center_mm,
            major_axis_mm,
            ratio,
            start_param_rad,
            sweep_param_rad,
        } => {
            validate_ellipse(
                center_mm,
                major_axis_mm,
                ratio,
                start_param_rad,
                sweep_param_rad,
            )?;
            if sweep_param_rad.abs() == std::f64::consts::TAU {
                None
            } else {
                let start = ellipse_phase(start_param_rad);
                Some((
                    ellipse_point(center_mm, major_axis_mm, ratio, start),
                    ellipse_point(center_mm, major_axis_mm, ratio, start + sweep_param_rad),
                ))
            }
        }
    };
    if points.is_some_and(|(a, b)| !finite(&a) || !finite(&b)) {
        return Err(invalid(
            "Un segment du profil contient une coordonnée non finie.",
        ));
    }
    Ok(points)
}

fn validate_circle(center: [f64; 2], radius: f64) -> Result<(), FabError> {
    // The whole circle must remain representable, including its extrema.
    if !finite(&center)
        || !radius.is_finite()
        || radius <= 0.
        || center.iter().any(|c| !(c.abs() + radius).is_finite())
    {
        return Err(invalid(
            "Un cercle du profil a un centre ou un rayon invalide.",
        ));
    }
    Ok(())
}

fn validate_ellipse(
    center: [f64; 2],
    major: [f64; 2],
    ratio: f64,
    start: f64,
    sweep: f64,
) -> Result<(), FabError> {
    let radius = major[0].hypot(major[1]);
    // Exact full-ellipse coordinate amplitudes bound all partial domains too.
    let amplitudes = [
        major[0].hypot(ratio * major[1]),
        major[1].hypot(ratio * major[0]),
    ];
    if !finite(&center)
        || !finite(&major)
        || !radius.is_finite()
        || radius <= 0.
        || !ratio.is_finite()
        || ratio <= 0.
        || ratio > 1.
        || radius * ratio == 0.
        || !start.is_finite()
        || !sweep.is_finite()
        || sweep == 0.
        || sweep.abs() > std::f64::consts::TAU
        || center
            .iter()
            .zip(amplitudes)
            .any(|(c, amplitude)| !(c.abs() + amplitude).is_finite())
    {
        return Err(invalid(
            "Une ellipse du profil a des axes, un rapport ou un domaine invalide.",
        ));
    }
    Ok(())
}

fn validate_profile(profile: &PlanarFaceProfile) -> Result<usize, FabError> {
    if profile.loops.len() > MAX_LOOPS {
        return Err(budget());
    }
    let count = profile.loops.iter().try_fold(0usize, |total, wire| {
        total
            .checked_add(wire.segments.len())
            .filter(|n| *n <= MAX_SEGMENTS)
            .ok_or_else(budget)
    })?;
    validate_frame(&profile.frame)?;
    if profile.loops.iter().filter(|wire| wire.is_outer).count() != 1 {
        return Err(invalid(
            "Le profil doit contenir exactement un contour extérieur.",
        ));
    }
    for wire in &profile.loops {
        validate_wire(&wire.segments)?;
    }
    Ok(count)
}

fn validate_wire(segments: &[ProfileSegment]) -> Result<(), FabError> {
    if segments.is_empty() {
        return Err(invalid("Un contour du profil est vide."));
    }
    let mut first = None;
    let mut previous = None;
    for segment in segments {
        let Some((start, end)) = endpoints(segment)? else {
            if segments.len() != 1 {
                return Err(invalid(
                    "Une courbe complète doit être seule dans son contour.",
                ));
            }
            continue;
        };
        if previous.is_some_and(|p| !joined(p, start)) {
            return Err(invalid("Le contour du profil est discontinu."));
        }
        first.get_or_insert(start);
        previous = Some(end);
    }
    if let (Some(first), Some(last)) = (first, previous) {
        if !joined(last, first) {
            return Err(invalid("Le contour du profil est ouvert."));
        }
    }
    Ok(())
}

fn section_segment_count(profile: &SectionProfile) -> Result<usize, FabError> {
    validate_solid_ids(
        profile.bodies.iter().map(|body| body.solid_id),
        profile.bodies.len(),
    )?;
    let mut loops = 0usize;
    let mut segments = 0usize;
    for body in &profile.bodies {
        loops = loops
            .checked_add(body.loops.len())
            .filter(|n| *n <= MAX_LOOPS)
            .ok_or_else(budget)?;
        for wire in &body.loops {
            segments = segments
                .checked_add(wire.segments.len())
                .filter(|n| *n <= MAX_SEGMENTS)
                .ok_or_else(budget)?;
        }
    }
    Ok(segments)
}

/// Validate every selected body's contours under shared document-wide budgets.
/// An empty body or entirely empty section is valid for preview.
pub fn validate_section_profile(profile: &SectionProfile) -> Result<(), FabError> {
    section_segment_count(profile)?;
    validate_profile_frame(&profile.frame)?;
    for body in &profile.bodies {
        let mut components = std::collections::BTreeMap::<usize, usize>::new();
        for wire in &body.loops {
            let outers = components.entry(wire.component_id).or_default();
            *outers += usize::from(wire.is_outer);
            validate_wire(&wire.segments)?;
        }
        if components.values().any(|outers| *outers != 1) {
            return Err(invalid(
                "Chaque composante de section doit contenir exactement un contour extérieur.",
            ));
        }
    }
    Ok(())
}

/// Validate the bounded, closed, exact-curve DTO without generating a file.
pub fn validate_planar_profile(profile: &PlanarFaceProfile) -> Result<(), FabError> {
    validate_profile(profile).map(|_| ())
}

fn joined(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) <= JOIN_TOLERANCE_MM
}

/// ASCII AutoCAD 2000 DXF, 1:1 in millimetres, in the profile's local XY plane.
/// Exact LINE/ARC/CIRCLE/ELLIPSE loci are preserved; clockwise curves swap their DXF
/// endpoints because DXF ARC entities sweep counterclockwise. This function
/// validates continuity but never inserts or snaps any closing geometry.
///
/// Limits: 4096 closed loops, 100000 segments, 16 MiB output. The frame is
/// retained in the DTO for provenance; the DXF deliberately flattens it.
pub fn export_planar_face_dxf(profile: &PlanarFaceProfile) -> Result<Vec<u8>, FabError> {
    let count = validate_profile(profile)?;
    let mut hole = 0;
    let loops: Vec<_> = profile
        .loops
        .iter()
        .map(|wire| ExportLoop {
            layer: layer_name(wire, &mut hole),
            segments: &wire.segments,
        })
        .collect();
    export_loops(&loops, count)
}

/// Export the exact section in its common local XY plane, in millimetres 1:1.
/// Layers retain zero-based native identities: `BODY_{solid_id}_COMPONENT_
/// {component_id}_OUTER` or `..._HOLE_n` (holes numbered from one per component).
/// Empty selected bodies remain in the DTO; an entirely empty export is refused.
pub fn export_section_dxf(profile: &SectionProfile) -> Result<Vec<u8>, FabError> {
    validate_section_profile(profile)?;
    let count = section_segment_count(profile)?;
    if count == 0 {
        return Err(FabError::new(
            "dxf.section_empty",
            "Le plan ne coupe aucun des corps sélectionnés.",
        ));
    }
    let mut loops = Vec::new();
    for body in &profile.bodies {
        let mut holes = std::collections::BTreeMap::<usize, usize>::new();
        for wire in &body.loops {
            let role = if wire.is_outer {
                "OUTER".to_owned()
            } else {
                let number = holes.entry(wire.component_id).or_default();
                *number += 1;
                format!("HOLE_{number}")
            };
            loops.push(ExportLoop {
                layer: format!(
                    "BODY_{}_COMPONENT_{}_{}",
                    body.solid_id, wire.component_id, role
                ),
                segments: &wire.segments,
            });
        }
    }
    export_loops(&loops, count)
}

struct ExportLoop<'a> {
    layer: String,
    segments: &'a [ProfileSegment],
}

fn export_loops(loops: &[ExportLoop<'_>], count: usize) -> Result<Vec<u8>, FabError> {
    let mut writer = DxfWriter {
        text: String::new(),
    };
    write_document(&mut writer, loops, count).map_err(|_| budget())?;
    Ok(writer.text.into_bytes())
}

struct DxfWriter {
    text: String,
}

impl Write for DxfWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > MAX_DXF_BYTES - self.text.len() {
            return Err(fmt::Error);
        }
        // Avoid geometrically growing an allocation beyond the output cap.
        if self.text.len() + s.len() > self.text.capacity() {
            let target = self
                .text
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .max(self.text.len() + s.len())
                .min(MAX_DXF_BYTES);
            self.text.reserve_exact(target - self.text.len());
        }
        self.text.push_str(s);
        Ok(())
    }
}

impl DxfWriter {
    fn group(&mut self, code: u16, value: impl fmt::Display) -> fmt::Result {
        writeln!(self, "{code}\n{value}")
    }

    fn point(&mut self, code: u16, point: [f64; 2]) -> fmt::Result {
        self.group(code, point[0])?;
        self.group(code + 10, point[1])?;
        self.group(code + 20, 0)
    }

    fn section(&mut self, name: &str) -> fmt::Result {
        self.group(0, "SECTION")?;
        self.group(2, name)
    }

    fn table(&mut self, name: &str, handle: u32, count: usize) -> fmt::Result {
        self.group(0, "TABLE")?;
        self.group(2, name)?;
        self.group(5, format_args!("{handle:X}"))?;
        self.group(330, 0)?;
        self.group(100, "AcDbSymbolTable")?;
        self.group(70, count)
    }

    fn record(&mut self, kind: &str, handle: u32, owner: u32, subclass: &str) -> fmt::Result {
        self.group(0, kind)?;
        self.group(5, format_args!("{handle:X}"))?;
        self.group(330, format_args!("{owner:X}"))?;
        self.group(100, "AcDbSymbolTableRecord")?;
        self.group(100, subclass)
    }

    fn entity(&mut self, kind: &str, handle: u32, owner: u32, layer: &str) -> fmt::Result {
        self.group(0, kind)?;
        self.group(5, format_args!("{handle:X}"))?;
        self.group(330, format_args!("{owner:X}"))?;
        self.group(100, "AcDbEntity")?;
        self.group(8, layer)
    }
}

fn layer_name(wire: &PlanarProfileLoop, hole: &mut usize) -> String {
    if wire.is_outer {
        "OUTER".into()
    } else {
        *hole += 1;
        format!("HOLE_{hole}")
    }
}

fn write_document(w: &mut DxfWriter, loops: &[ExportLoop<'_>], count: usize) -> fmt::Result {
    w.section("HEADER")?;
    w.group(9, "$ACADVER")?;
    w.group(1, "AC1015")?;
    w.group(9, "$INSUNITS")?;
    w.group(70, 4)?;
    w.group(9, "$MEASUREMENT")?;
    w.group(70, 1)?;
    w.group(9, "$HANDSEED")?;
    w.group(5, format_args!("{:X}", 0x10000 + count))?;
    w.group(0, "ENDSEC")?;
    w.section("TABLES")?;
    w.table("LTYPE", 1, 1)?;
    w.record("LTYPE", 2, 1, "AcDbLinetypeTableRecord")?;
    w.group(2, "CONTINUOUS")?;
    w.group(70, 0)?;
    w.group(3, "Solid line")?;
    w.group(72, 65)?;
    w.group(73, 0)?;
    w.group(40, 0)?;
    w.group(0, "ENDTAB")?;
    w.table("LAYER", 3, loops.len() + 1)?;
    w.record("LAYER", 4, 3, "AcDbLayerTableRecord")?;
    write_layer(w, "0")?;
    for (i, wire) in loops.iter().enumerate() {
        w.record("LAYER", 0x100 + i as u32, 3, "AcDbLayerTableRecord")?;
        write_layer(w, &wire.layer)?;
    }
    w.group(0, "ENDTAB")?;
    w.table("BLOCK_RECORD", 5, 2)?;
    for (handle, name) in [(6, "*Model_Space"), (7, "*Paper_Space")] {
        w.record("BLOCK_RECORD", handle, 5, "AcDbBlockTableRecord")?;
        w.group(2, name)?;
        w.group(70, 0)?;
    }
    w.group(0, "ENDTAB")?;
    w.group(0, "ENDSEC")?;
    w.section("BLOCKS")?;
    for (owner, begin, end, name) in [(6, 8, 9, "*Model_Space"), (7, 10, 11, "*Paper_Space")] {
        w.entity("BLOCK", begin, owner, "0")?;
        w.group(100, "AcDbBlockBegin")?;
        w.group(2, name)?;
        w.group(70, 0)?;
        w.point(10, [0., 0.])?;
        w.group(3, name)?;
        w.group(1, "")?;
        w.entity("ENDBLK", end, owner, "0")?;
        w.group(100, "AcDbBlockEnd")?;
    }
    w.group(0, "ENDSEC")?;
    w.section("ENTITIES")?;
    let mut handle = 0x10000;
    for wire in loops {
        for segment in wire.segments {
            write_segment(w, segment, handle, &wire.layer)?;
            handle += 1;
        }
    }
    w.group(0, "ENDSEC")?;
    w.group(0, "EOF")
}

fn write_layer(w: &mut DxfWriter, name: &str) -> fmt::Result {
    w.group(2, name)?;
    w.group(70, 0)?;
    w.group(62, 7)?;
    w.group(6, "CONTINUOUS")
}

fn write_segment(
    w: &mut DxfWriter,
    segment: &ProfileSegment,
    handle: u32,
    layer: &str,
) -> fmt::Result {
    match *segment {
        ProfileSegment::Line { start_mm, end_mm } => {
            w.entity("LINE", handle, 6, layer)?;
            w.group(100, "AcDbLine")?;
            w.point(10, start_mm)?;
            w.point(11, end_mm)
        }
        ProfileSegment::Circle {
            center_mm,
            radius_mm,
        } => {
            w.entity("CIRCLE", handle, 6, layer)?;
            w.group(100, "AcDbCircle")?;
            w.point(10, center_mm)?;
            w.group(40, radius_mm)
        }
        ProfileSegment::Arc {
            center_mm,
            radius_mm,
            start_angle_rad,
            sweep_angle_rad,
        } => {
            w.entity("ARC", handle, 6, layer)?;
            w.group(100, "AcDbCircle")?;
            w.point(10, center_mm)?;
            w.group(40, radius_mm)?;
            w.group(100, "AcDbArc")?;
            let start = start_angle_rad.rem_euclid(std::f64::consts::TAU);
            let end = start + sweep_angle_rad;
            let (from, to) = if sweep_angle_rad > 0. {
                (start, end)
            } else {
                (end, start)
            };
            w.group(50, from.rem_euclid(std::f64::consts::TAU).to_degrees())?;
            w.group(51, to.rem_euclid(std::f64::consts::TAU).to_degrees())
        }
        ProfileSegment::Ellipse {
            center_mm,
            major_axis_mm,
            ratio,
            start_param_rad,
            sweep_param_rad,
        } => {
            w.entity("ELLIPSE", handle, 6, layer)?;
            w.group(100, "AcDbEllipse")?;
            w.point(10, center_mm)?;
            w.point(11, major_axis_mm)?;
            w.group(40, ratio)?;
            let (from, to) = if sweep_param_rad.abs() == std::f64::consts::TAU {
                (0., std::f64::consts::TAU)
            } else {
                let start = ellipse_phase(start_param_rad);
                let end = (start + sweep_param_rad).rem_euclid(std::f64::consts::TAU);
                if sweep_param_rad > 0. {
                    (start, end)
                } else {
                    (end, start)
                }
            };
            w.group(41, from)?;
            w.group(42, to)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn frame() -> ProfileFrame {
        ProfileFrame {
            origin_mm: [100., -200., 300.],
            x_axis: [0., 1., 0.],
            y_axis: [0., 0., 1.],
            normal: [1., 0., 0.],
        }
    }

    fn polygon(points: &[[f64; 2]], is_outer: bool) -> PlanarProfileLoop {
        PlanarProfileLoop {
            is_outer,
            segments: points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(a, b)| ProfileSegment::Line {
                    start_mm: *a,
                    end_mm: *b,
                })
                .collect(),
        }
    }

    fn profile() -> PlanarFaceProfile {
        PlanarFaceProfile {
            face_id: 12,
            solid_id: 2,
            frame: frame(),
            loops: vec![polygon(&[[0., 0.], [20., 0.], [20., 10.], [0., 10.]], true)],
        }
    }

    fn dxf(p: &PlanarFaceProfile) -> String {
        String::from_utf8(export_planar_face_dxf(p).unwrap()).unwrap()
    }

    fn groups(text: &str) -> Vec<(u16, &str)> {
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len() % 2, 0);
        lines
            .chunks_exact(2)
            .map(|pair| (pair[0].parse().unwrap(), pair[1]))
            .collect()
    }

    #[test]
    fn rectangle_is_exact_local_mm_ascii_and_deterministic() {
        let p = profile();
        let output = dxf(&p);
        assert!(output.is_ascii());
        assert!(output.contains("9\n$ACADVER\n1\nAC1015\n"));
        assert!(output.contains("9\n$INSUNITS\n70\n4\n"));
        assert!(output.contains("10\n20\n20\n10\n30\n0\n"));
        assert_eq!(
            groups(&output)
                .iter()
                .filter(|g| **g == (0, "LINE"))
                .count(),
            4
        );
        assert_eq!(output, dxf(&p));
        assert!(output.ends_with("0\nENDSEC\n0\nEOF\n"));
    }

    #[test]
    fn preserves_separate_holes_and_native_circle() {
        let mut p = profile();
        p.loops.push(PlanarProfileLoop {
            is_outer: false,
            segments: vec![ProfileSegment::Circle {
                center_mm: [3., 4.],
                radius_mm: 1.25,
            }],
        });
        p.loops.push(polygon(
            &[[10., 2.], [10., 3.], [11., 3.], [11., 2.]],
            false,
        ));
        let output = dxf(&p);
        assert!(output.contains("2\nOUTER\n"));
        assert!(output.contains("2\nHOLE_1\n"));
        assert!(output.contains("2\nHOLE_2\n"));
        assert!(output.contains("8\nHOLE_1\n100\nAcDbCircle\n10\n3\n20\n4\n30\n0\n40\n1.25\n"));
        assert_eq!(
            groups(&output)
                .iter()
                .filter(|g| **g == (0, "CIRCLE"))
                .count(),
            1
        );
    }

    #[test]
    fn circle_is_valid_as_outer_boundary() {
        let mut p = profile();
        p.loops[0].segments = vec![ProfileSegment::Circle {
            center_mm: [0., 0.],
            radius_mm: 5.,
        }];
        assert!(validate_planar_profile(&p).is_ok());
        assert!(dxf(&p).contains("0\nCIRCLE\n"));
    }

    #[test]
    fn signed_major_and_minor_arcs_preserve_ccw_locus() {
        for sweep in [FRAC_PI_2, -FRAC_PI_2, 1.5 * PI, -1.5 * PI] {
            let mut p = profile();
            let center = [4., -3.];
            let radius = 2.5;
            let start = 0.3;
            p.loops[0].segments = vec![
                ProfileSegment::Arc {
                    center_mm: center,
                    radius_mm: radius,
                    start_angle_rad: start,
                    sweep_angle_rad: sweep,
                },
                ProfileSegment::Line {
                    start_mm: circle_point(center, radius, start + sweep),
                    end_mm: circle_point(center, radius, start),
                },
            ];
            let output = dxf(&p);
            let g = groups(&output);
            let from: f64 = g.iter().find(|g| g.0 == 50).unwrap().1.parse().unwrap();
            let to: f64 = g.iter().find(|g| g.0 == 51).unwrap().1.parse().unwrap();
            assert!(((to - from).rem_euclid(360.) - sweep.abs().to_degrees()).abs() < 1e-10);
            let expected_from = if sweep > 0. { start } else { start + sweep };
            assert!((from - expected_from.rem_euclid(TAU).to_degrees()).abs() < 1e-10);
            assert_eq!(g.iter().filter(|g| **g == (0, "ARC")).count(), 1);
        }
    }

    #[test]
    fn refuses_nonfinite_and_invalid_radii() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut p = profile();
            p.loops[0].segments[0] = ProfileSegment::Line {
                start_mm: [value, 0.],
                end_mm: [20., 0.],
            };
            assert_eq!(
                export_planar_face_dxf(&p).unwrap_err().code,
                "dxf.invalid_profile"
            );
        }
        for radius in [0., -1., f64::INFINITY, f64::NAN] {
            let mut p = profile();
            p.loops[0].segments = vec![ProfileSegment::Circle {
                center_mm: [0., 0.],
                radius_mm: radius,
            }];
            assert_eq!(
                validate_planar_profile(&p).unwrap_err().code,
                "dxf.invalid_profile"
            );
        }
    }

    #[test]
    fn refuses_bad_frames() {
        for bad in [
            ProfileFrame {
                origin_mm: [f64::NAN, 0., 0.],
                ..frame()
            },
            ProfileFrame {
                x_axis: [0., 2., 0.],
                ..frame()
            },
            ProfileFrame {
                x_axis: [0., 0., 1.],
                ..frame()
            },
            ProfileFrame {
                normal: [-1., 0., 0.],
                ..frame()
            },
        ] {
            let mut p = profile();
            p.frame = bad;
            assert_eq!(
                validate_planar_profile(&p).unwrap_err().code,
                "dxf.invalid_profile"
            );
        }
    }

    #[test]
    fn refuses_open_and_discontinuous_wires_without_closing_them() {
        let mut p = profile();
        p.loops[0].segments.pop();
        assert_eq!(
            export_planar_face_dxf(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
        let mut p = profile();
        p.loops[0].segments[1] = ProfileSegment::Line {
            start_mm: [20.000001, 0.],
            end_mm: [20., 10.],
        };
        assert_eq!(
            export_planar_face_dxf(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
    }

    #[test]
    fn continuity_is_absolute_and_does_not_snap_coordinates() {
        let mut p = profile();
        p.loops[0].segments[1] = ProfileSegment::Line {
            start_mm: [20.00000005, 0.],
            end_mm: [20., 10.],
        };
        assert!(dxf(&p).contains("10\n20.00000005\n"));
    }

    #[test]
    fn refuses_missing_duplicate_outer_empty_and_mixed_circle_wire() {
        let mut p = profile();
        p.loops[0].is_outer = false;
        assert!(validate_planar_profile(&p).is_err());
        let mut p = profile();
        p.loops.push(p.loops[0].clone());
        assert!(validate_planar_profile(&p).is_err());
        let mut p = profile();
        p.loops[0].segments.clear();
        assert!(validate_planar_profile(&p).is_err());
        let mut p = profile();
        p.loops[0].segments.push(ProfileSegment::Circle {
            center_mm: [1., 1.],
            radius_mm: 1.,
        });
        assert!(validate_planar_profile(&p).is_err());
    }

    #[test]
    fn refuses_degenerate_lines_and_full_turn_or_invalid_arcs() {
        let mut p = profile();
        p.loops[0].segments = vec![ProfileSegment::Line {
            start_mm: [0., 0.],
            end_mm: [0., 0.],
        }];
        assert!(validate_planar_profile(&p).is_err());
        for sweep in [0., TAU, -TAU, 2. * TAU, f64::NAN] {
            p.loops[0].segments = vec![ProfileSegment::Arc {
                center_mm: [0., 0.],
                radius_mm: 1.,
                start_angle_rad: 0.,
                sweep_angle_rad: sweep,
            }];
            assert!(validate_planar_profile(&p).is_err());
        }
    }

    #[test]
    fn refuses_loop_and_segment_budget_before_geometry_work() {
        let mut p = profile();
        p.loops = vec![p.loops[0].clone(); MAX_LOOPS + 1];
        assert_eq!(
            validate_planar_profile(&p).unwrap_err().code,
            "dxf.too_complex"
        );
        let mut p = profile();
        p.loops[0].segments = vec![p.loops[0].segments[0].clone(); MAX_SEGMENTS + 1];
        assert_eq!(
            validate_planar_profile(&p).unwrap_err().code,
            "dxf.too_complex"
        );
    }

    #[test]
    fn refuses_oversized_output_even_inside_segment_budget() {
        let mut p = profile();
        p.loops[0].segments = (0..MAX_SEGMENTS)
            .map(|i| {
                let (a, b) = if i % 2 == 0 {
                    (1e300, -1e300)
                } else {
                    (-1e300, 1e300)
                };
                ProfileSegment::Line {
                    start_mm: [a, a],
                    end_mm: [b, b],
                }
            })
            .collect();
        assert!(validate_planar_profile(&p).is_ok());
        assert_eq!(
            export_planar_face_dxf(&p).unwrap_err().code,
            "dxf.too_complex"
        );
    }

    fn ellipse_profile(start: f64, sweep: f64) -> PlanarFaceProfile {
        let mut p = profile();
        let center = [2., -3.];
        let major = [3., 4.];
        let ratio = 0.4;
        p.loops[0].segments = vec![ProfileSegment::Ellipse {
            center_mm: center,
            major_axis_mm: major,
            ratio,
            start_param_rad: start,
            sweep_param_rad: sweep,
        }];
        if sweep.abs() != TAU {
            let phase = ellipse_phase(start);
            p.loops[0].segments.push(ProfileSegment::Line {
                start_mm: ellipse_point(center, major, ratio, phase + sweep),
                end_mm: ellipse_point(center, major, ratio, phase),
            });
        }
        p
    }

    fn parameters(output: &str) -> (f64, f64) {
        let g = groups(output);
        (
            g.iter().find(|g| g.0 == 41).unwrap().1.parse().unwrap(),
            g.iter().find(|g| g.0 == 42).unwrap().1.parse().unwrap(),
        )
    }

    #[test]
    fn exact_tilted_full_ellipse_uses_standard_dxf_parameters() {
        for sweep in [TAU, -TAU] {
            let p = ellipse_profile(1.25, sweep);
            validate_planar_profile(&p).unwrap();
            let output = dxf(&p);
            assert!(output.contains("0\nELLIPSE\n"));
            assert!(output.contains(
                "100\nAcDbEllipse\n10\n2\n20\n-3\n30\n0\n11\n3\n21\n4\n31\n0\n40\n0.4\n"
            ));
            assert_eq!(parameters(&output), (0., TAU));
            assert!(!output.contains("0\nLINE\n"));
            assert_eq!(endpoints(&p.loops[0].segments[0]).unwrap(), None);
        }
    }

    #[test]
    fn exact_ellipse_minor_axis_and_signed_partial_domains() {
        assert_eq!(ellipse_point([2., -3.], [3., 4.], 0.4, 0.), [5., 1.]);
        let q = ellipse_point([2., -3.], [3., 4.], 0.4, FRAC_PI_2);
        assert!((q[0] - 0.4).abs() < 1e-14 && (q[1] + 1.8).abs() < 1e-14);
        for start in [0.3, 6., 1e300] {
            for sweep in [FRAC_PI_2, -FRAC_PI_2, 1.5 * PI, -1.5 * PI] {
                let p = ellipse_profile(start, sweep);
                let output = dxf(&p);
                let (from, to) = parameters(&output);
                assert!(((to - from).rem_euclid(TAU) - sweep.abs()).abs() < 1e-14);
                let (expected_from, expected_to) = if sweep > 0. {
                    (ellipse_phase(start), ellipse_phase(start) + sweep)
                } else {
                    (ellipse_phase(start) + sweep, ellipse_phase(start))
                };
                let a = ellipse_point([2., -3.], [3., 4.], 0.4, from);
                let b = ellipse_point([2., -3.], [3., 4.], 0.4, expected_from);
                assert!(joined(a, b));
                let a = ellipse_point([2., -3.], [3., 4.], 0.4, to);
                let b = ellipse_point([2., -3.], [3., 4.], 0.4, expected_to);
                assert!(joined(a, b));
                assert_eq!(
                    groups(&output)
                        .iter()
                        .filter(|g| **g == (0, "ELLIPSE"))
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn ellipse_rejects_invalid_axes_ratios_and_domains() {
        for major in [
            [0., 0.],
            [f64::NAN, 1.],
            [1., f64::INFINITY],
            [f64::MAX, f64::MAX],
        ] {
            assert!(validate_ellipse([0., 0.], major, 0.4, 0., PI).is_err());
        }
        for ratio in [0., -0.1, 1. + 1e-10, f64::NAN, f64::INFINITY] {
            assert!(validate_ellipse([0., 0.], [3., 4.], ratio, 0., PI).is_err());
        }
        for sweep in [0., TAU + 1e-10, -TAU - 1e-10, f64::NAN, f64::INFINITY] {
            assert!(validate_ellipse([0., 0.], [3., 4.], 0.4, 0., sweep).is_err());
        }
        assert!(validate_ellipse([0., 0.], [3., 4.], 0.4, f64::NAN, PI).is_err());
        assert!(validate_ellipse([f64::INFINITY, 0.], [3., 4.], 0.4, 0., PI).is_err());
        assert!(validate_ellipse([1e308, 0.], [1e308, 0.], 0.4, 0., PI).is_err());
        assert!(validate_ellipse([0., 0.], [1e-300, 0.], 1e-300, 0., PI).is_err());
        assert!(validate_ellipse([0., 0.], [3., 4.], 1., 0., TAU).is_ok());
    }

    #[test]
    fn ellipse_never_invents_closure_or_promotes_near_full_sweep() {
        let mut p = ellipse_profile(0.3, PI);
        p.loops[0].segments.pop();
        assert_eq!(
            export_planar_face_dxf(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
        let mut p = ellipse_profile(0.3, PI);
        if let ProfileSegment::Line { end_mm, .. } = &mut p.loops[0].segments[1] {
            end_mm[0] += 1e-6;
        }
        assert!(validate_planar_profile(&p).is_err());
        let mut p = ellipse_profile(0.3, TAU);
        p.loops[0].segments.push(ProfileSegment::Line {
            start_mm: [0., 0.],
            end_mm: [1., 1.],
        });
        assert!(validate_planar_profile(&p).is_err());
        let p = ellipse_profile(0.3, TAU - 1e-8);
        assert_ne!(parameters(&dxf(&p)), (0., TAU));
        assert!(dxf(&p).contains("0\nLINE\n"));
    }

    #[test]
    fn ellipse_is_exact_in_section_outer_and_hole_layers() {
        let p = SectionProfile {
            frame: frame(),
            bodies: vec![SectionBodyProfile {
                solid_id: 2,
                loops: vec![
                    SectionLoop {
                        component_id: 3,
                        is_outer: true,
                        segments: ellipse_profile(0.3, -TAU).loops.remove(0).segments,
                    },
                    SectionLoop {
                        component_id: 3,
                        is_outer: false,
                        segments: vec![ProfileSegment::Ellipse {
                            center_mm: [2., -3.],
                            major_axis_mm: [0.3, 0.4],
                            ratio: 0.25,
                            start_param_rad: 0.,
                            sweep_param_rad: TAU,
                        }],
                    },
                ],
            }],
        };
        let output = String::from_utf8(export_section_dxf(&p).unwrap()).unwrap();
        assert_eq!(
            groups(&output)
                .iter()
                .filter(|g| **g == (0, "ELLIPSE"))
                .count(),
            2
        );
        assert!(output.contains("8\nBODY_2_COMPONENT_3_OUTER\n"));
        assert!(output.contains("8\nBODY_2_COMPONENT_3_HOLE_1\n"));
        assert!(!output.contains("0\nLINE\n"));
    }

    #[test]
    #[ignore = "set FABSTEP_DXF_TEST_ARTIFACT_DIR to emit independent-reader fixtures"]
    fn emit_ellipse_reference_artifacts() {
        let destination = std::path::PathBuf::from(
            std::env::var_os("FABSTEP_DXF_TEST_ARTIFACT_DIR").expect("artifact destination"),
        );
        std::fs::create_dir_all(&destination).unwrap();
        for (name, sweep) in [
            ("full-ccw", TAU),
            ("full-cw", -TAU),
            ("ccw90", FRAC_PI_2),
            ("cw90", -FRAC_PI_2),
            ("ccw270", 1.5 * PI),
            ("cw270", -1.5 * PI),
        ] {
            let p = ellipse_profile(0.3, sweep);
            std::fs::write(
                destination.join(format!("core-ellipse-{name}.dxf")),
                export_planar_face_dxf(&p).unwrap(),
            )
            .unwrap();
            let p = SectionProfile {
                frame: p.frame,
                bodies: vec![SectionBodyProfile {
                    solid_id: 1,
                    loops: vec![SectionLoop {
                        component_id: 0,
                        is_outer: true,
                        segments: p.loops.into_iter().next().unwrap().segments,
                    }],
                }],
            };
            std::fs::write(
                destination.join(format!("core-section-ellipse-{name}.dxf")),
                export_section_dxf(&p).unwrap(),
            )
            .unwrap();
        }
    }

    fn section() -> SectionProfile {
        let boundary = profile().loops.remove(0);
        SectionProfile {
            frame: frame(),
            bodies: vec![
                SectionBodyProfile {
                    solid_id: 0,
                    loops: vec![
                        SectionLoop {
                            component_id: 0,
                            is_outer: true,
                            segments: boundary.segments,
                        },
                        SectionLoop {
                            component_id: 0,
                            is_outer: false,
                            segments: vec![ProfileSegment::Circle {
                                center_mm: [3., 4.],
                                radius_mm: 1.25,
                            }],
                        },
                        SectionLoop {
                            component_id: 7,
                            is_outer: true,
                            segments: vec![ProfileSegment::Circle {
                                center_mm: [25., 4.],
                                radius_mm: 2.,
                            }],
                        },
                    ],
                },
                SectionBodyProfile {
                    solid_id: 2,
                    loops: vec![],
                },
                SectionBodyProfile {
                    solid_id: 4,
                    loops: vec![SectionLoop {
                        component_id: 0,
                        is_outer: true,
                        segments: polygon(&[[30., 0.], [35., 0.], [35., 5.], [30., 5.]], true)
                            .segments,
                    }],
                },
            ],
        }
    }

    #[test]
    fn section_preserves_empty_bodies_disconnected_components_and_holes() {
        let p = section();
        validate_section_profile(&p).unwrap();
        let output = String::from_utf8(export_section_dxf(&p).unwrap()).unwrap();
        for layer in [
            "BODY_0_COMPONENT_0_OUTER",
            "BODY_0_COMPONENT_0_HOLE_1",
            "BODY_0_COMPONENT_7_OUTER",
            "BODY_4_COMPONENT_0_OUTER",
        ] {
            assert!(output.contains(&format!("2\n{layer}\n")));
            assert!(output.contains(&format!("8\n{layer}\n")));
        }
        assert!(!output.contains("BODY_2_"));
        assert_eq!(p.bodies[1].solid_id, 2);
        assert!(p.bodies[1].loops.is_empty());
        assert_eq!(
            groups(&output)
                .iter()
                .filter(|g| **g == (0, "LINE"))
                .count(),
            8
        );
        assert_eq!(
            groups(&output)
                .iter()
                .filter(|g| **g == (0, "CIRCLE"))
                .count(),
            2
        );
        assert_eq!(
            export_section_dxf(&p).unwrap(),
            export_section_dxf(&p).unwrap()
        );
    }

    #[test]
    fn section_empty_is_valid_for_preview_and_refused_for_export() {
        let mut p = section();
        for body in &mut p.bodies {
            body.loops.clear();
        }
        validate_section_profile(&p).unwrap();
        assert_eq!(
            export_section_dxf(&p).unwrap_err().code,
            "dxf.section_empty"
        );
        assert_eq!(p.bodies.len(), 3);
    }

    #[test]
    fn section_request_requires_sorted_unique_bounded_body_ids() {
        for ids in [vec![0], vec![31], (0..32).collect()] {
            validate_section_request(&SectionRequest {
                frame: frame(),
                solid_ids: ids,
            })
            .unwrap();
        }
        for ids in [vec![], vec![1, 1], vec![1, 0], vec![32]] {
            assert_eq!(
                validate_section_request(&SectionRequest {
                    frame: frame(),
                    solid_ids: ids
                })
                .unwrap_err()
                .code,
                "dxf.invalid_profile"
            );
        }
        assert_eq!(
            validate_section_request(&SectionRequest {
                frame: frame(),
                solid_ids: (0..33).collect()
            })
            .unwrap_err()
            .code,
            "dxf.too_complex"
        );
    }

    #[test]
    fn section_profile_preserves_request_body_identity_contract() {
        let mut p = section();
        p.bodies[1].solid_id = 0;
        assert_eq!(
            validate_section_profile(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
        p.bodies[1].solid_id = 31;
        assert!(validate_section_profile(&p).is_err());
        p.bodies[1].solid_id = 32;
        assert!(validate_section_profile(&p).is_err());
        p.bodies.clear();
        assert!(validate_section_profile(&p).is_err());
    }

    #[test]
    fn section_plane_errors_have_distinct_stable_code() {
        let bad = ProfileFrame {
            normal: [-1., 0., 0.],
            ..frame()
        };
        assert_eq!(
            validate_profile_frame(&bad).unwrap_err().code,
            "dxf.section_invalid_plane"
        );
        assert_eq!(
            validate_section_request(&SectionRequest {
                frame: bad.clone(),
                solid_ids: vec![0]
            })
            .unwrap_err()
            .code,
            "dxf.section_invalid_plane"
        );
        let mut p = section();
        p.frame = bad;
        assert_eq!(
            validate_section_profile(&p).unwrap_err().code,
            "dxf.section_invalid_plane"
        );
        // Existing DXF1 errors keep their earlier contract.
        let mut face = profile();
        face.frame = p.frame;
        assert_eq!(
            validate_planar_profile(&face).unwrap_err().code,
            "dxf.invalid_profile"
        );
    }

    #[test]
    fn section_requires_exactly_one_outer_per_component() {
        let mut p = section();
        p.bodies[0].loops[1].is_outer = true;
        assert!(validate_section_profile(&p).is_err());
        let mut p = section();
        p.bodies[0].loops[2].is_outer = false;
        assert!(validate_section_profile(&p).is_err());
        let mut p = section();
        p.bodies[0].loops[1].component_id = 8;
        assert!(validate_section_profile(&p).is_err());
    }

    #[test]
    fn section_reuses_strict_wire_and_curve_validation() {
        let mut p = section();
        p.bodies[0].loops[0].segments.pop();
        assert_eq!(
            export_section_dxf(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
        let mut p = section();
        p.bodies[0].loops[1].segments = vec![ProfileSegment::Circle {
            center_mm: [0., f64::NAN],
            radius_mm: 1.,
        }];
        assert_eq!(
            validate_section_profile(&p).unwrap_err().code,
            "dxf.invalid_profile"
        );
    }

    #[test]
    fn section_loop_budget_is_shared_across_bodies() {
        let mut p = section();
        p.bodies = [0, 1]
            .into_iter()
            .map(|solid_id| SectionBodyProfile {
                solid_id,
                loops: (0..MAX_LOOPS / 2)
                    .map(|component_id| SectionLoop {
                        component_id,
                        is_outer: true,
                        segments: vec![ProfileSegment::Circle {
                            center_mm: [0., 0.],
                            radius_mm: 1.,
                        }],
                    })
                    .collect(),
            })
            .collect();
        validate_section_profile(&p).unwrap();
        let extra = p.bodies[1].loops[0].clone();
        p.bodies[1].loops.push(extra);
        assert_eq!(
            validate_section_profile(&p).unwrap_err().code,
            "dxf.too_complex"
        );
    }

    #[test]
    fn section_segment_budget_is_shared_across_bodies() {
        let mut p = section();
        p.bodies = [0, 1]
            .into_iter()
            .map(|solid_id| SectionBodyProfile {
                solid_id,
                loops: vec![SectionLoop {
                    component_id: 0,
                    is_outer: true,
                    segments: (0..MAX_SEGMENTS / 2)
                        .map(|i| {
                            let (a, b) = if i % 2 == 0 {
                                ([0., 0.], [1., 1.])
                            } else {
                                ([1., 1.], [0., 0.])
                            };
                            ProfileSegment::Line {
                                start_mm: a,
                                end_mm: b,
                            }
                        })
                        .collect(),
                }],
            })
            .collect();
        validate_section_profile(&p).unwrap();
        let extra = p.bodies[1].loops[0].segments[0].clone();
        p.bodies[1].loops[0].segments.push(extra);
        assert_eq!(
            validate_section_profile(&p).unwrap_err().code,
            "dxf.too_complex"
        );
    }

    #[test]
    fn section_output_budget_is_shared_across_bodies() {
        let mut p = section();
        p.bodies = [0, 1]
            .into_iter()
            .map(|solid_id| SectionBodyProfile {
                solid_id,
                loops: vec![SectionLoop {
                    component_id: 0,
                    is_outer: true,
                    segments: (0..10_000)
                        .map(|i| {
                            let (a, b) = if i % 2 == 0 {
                                (1e300, -1e300)
                            } else {
                                (-1e300, 1e300)
                            };
                            ProfileSegment::Line {
                                start_mm: [a, a],
                                end_mm: [b, b],
                            }
                        })
                        .collect(),
                }],
            })
            .collect();
        validate_section_profile(&p).unwrap();
        let single_body = SectionProfile {
            frame: p.frame.clone(),
            bodies: vec![p.bodies[0].clone()],
        };
        assert!(export_section_dxf(&single_body).is_ok());
        assert_eq!(export_section_dxf(&p).unwrap_err().code, "dxf.too_complex");
    }

    #[test]
    #[ignore = "set FABSTEP_DXF_TEST_ARTIFACT_DIR to emit independent-reader fixtures"]
    fn emit_section_reference_artifacts() {
        let destination = std::path::PathBuf::from(
            std::env::var_os("FABSTEP_DXF_TEST_ARTIFACT_DIR").expect("artifact destination"),
        );
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(
            destination.join("core-section-bodies.dxf"),
            export_section_dxf(&section()).unwrap(),
        )
        .unwrap();
        let mut p = section();
        let center = [34., 4.];
        let radius = 2.5;
        let start = 0.3;
        let sweep = -1.5 * PI;
        p.bodies[2].loops[0].segments = vec![
            ProfileSegment::Arc {
                center_mm: center,
                radius_mm: radius,
                start_angle_rad: start,
                sweep_angle_rad: sweep,
            },
            ProfileSegment::Line {
                start_mm: circle_point(center, radius, start + sweep),
                end_mm: circle_point(center, radius, start),
            },
        ];
        std::fs::write(
            destination.join("core-section-cw270.dxf"),
            export_section_dxf(&p).unwrap(),
        )
        .unwrap();
    }

    /// Opt-in evidence for an independent DXF reader; production code has no IO.
    #[test]
    #[ignore = "set FABSTEP_DXF_TEST_ARTIFACT_DIR to emit independent-reader fixtures"]
    fn emit_reference_artifacts() {
        let destination = std::path::PathBuf::from(
            std::env::var_os("FABSTEP_DXF_TEST_ARTIFACT_DIR").expect("artifact destination"),
        );
        std::fs::create_dir_all(&destination).unwrap();
        let mut p = profile();
        p.loops.push(PlanarProfileLoop {
            is_outer: false,
            segments: vec![ProfileSegment::Circle {
                center_mm: [3., 4.],
                radius_mm: 1.25,
            }],
        });
        std::fs::write(
            destination.join("core-rectangle-hole.dxf"),
            export_planar_face_dxf(&p).unwrap(),
        )
        .unwrap();
        for (name, sweep) in [
            ("ccw90", FRAC_PI_2),
            ("cw90", -FRAC_PI_2),
            ("ccw270", 1.5 * PI),
            ("cw270", -1.5 * PI),
        ] {
            let mut p = profile();
            let center = [4., -3.];
            let radius = 2.5;
            let start = 0.3;
            p.loops[0].segments = vec![
                ProfileSegment::Arc {
                    center_mm: center,
                    radius_mm: radius,
                    start_angle_rad: start,
                    sweep_angle_rad: sweep,
                },
                ProfileSegment::Line {
                    start_mm: circle_point(center, radius, start + sweep),
                    end_mm: circle_point(center, radius, start),
                },
            ];
            std::fs::write(
                destination.join(format!("core-{name}.dxf")),
                export_planar_face_dxf(&p).unwrap(),
            )
            .unwrap();
        }
    }
}
