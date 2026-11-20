//! A frustum or spherical zone between two parallel caps and an oblique cut.
//! Both branches of the cut and ownership of all four faces are certified.
use crate::math::{cross, dot, sub, V};
use crate::oblique_lip::{
    coverage::{self, Chart},
    detect, ObliqueLip,
};
use crate::{Analysis, Diagnostic, FabError, Options, RecognitionReport, Severity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Cone,
    Sphere,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObliqueBand {
    pub frame: ObliqueLip,
    pub family: Family,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObliqueBandRecognition {
    pub model: ObliqueBand,
    /// Bottom cap, top cap, oblique cutting plane, trimmed analytic support.
    pub triangle_faces: Vec<usize>,
    pub report: RecognitionReport,
}

impl ObliqueBand {
    fn center_radius(&self) -> (f64, f64) {
        let m = &self.frame;
        let h = m.height_mm;
        let rb = m.outer_radius_mm;
        let rt = m.major_radius_mm;
        let z = (rt * rt - rb * rb + h * h) / (2. * h);
        (z, rb.hypot(z))
    }
    fn radius(&self, z: f64) -> f64 {
        let m = &self.frame;
        match self.family {
            Family::Cone => {
                m.outer_radius_mm + (m.major_radius_mm - m.outer_radius_mm) * z / m.height_mm
            }
            Family::Sphere => {
                let (c, r) = self.center_radius();
                (r * r - (z - c).powi(2)).sqrt()
            }
        }
    }
    fn hessian(&self) -> Option<[f64; 3]> {
        let m = &self.frame;
        let h = m.height_mm;
        let min = m.major_radius_mm.min(m.outer_radius_mm);
        let (max, dr, d2r) = match self.family {
            Family::Cone => (
                m.outer_radius_mm,
                (m.outer_radius_mm - m.major_radius_mm).abs() / h,
                0.,
            ),
            Family::Sphere => {
                let (c, r) = self.center_radius();
                (r, c.abs().max((h - c).abs()) / min, r * r / min.powi(3))
            }
        };
        let q = m
            .cut_offset_mm
            .abs()
            .max((m.cut_offset_mm + m.slope * h).abs())
            / min;
        if !(q < 0.97 && min > 0. && h > 0.) {
            return None;
        }
        let q1 = (m.slope.abs() + q * dr) / min;
        let q2 = (q * d2r + 2. * q1 * dr) / min;
        let root = (1. - q * q).sqrt();
        let span = (-q).acos();
        let ds = q1 / root;
        let d2s = q2 / root + q * q1 * q1 / root.powi(3);
        Some([
            max * span * span,
            dr * span + max * (span + 1.) * ds,
            d2r + 2. * dr * ds + max * (ds * ds + d2s),
        ])
    }
    fn cut_chord(&self, z0: f64, z1: f64) -> f64 {
        let m = &self.frame;
        let slope = m.slope;
        let cut = m.cut_offset_mm;
        let (a, b, c) = match self.family {
            Family::Cone => {
                let k = (m.major_radius_mm - m.outer_radius_mm) / m.height_mm;
                (
                    k * k - slope * slope,
                    2. * (m.outer_radius_mm * k - cut * slope),
                    m.outer_radius_mm.powi(2) - cut * cut,
                )
            }
            Family::Sphere => {
                let (zc, r) = self.center_radius();
                (
                    -1. - slope * slope,
                    2. * (zc - cut * slope),
                    r * r - zc * zc - cut * cut,
                )
            }
        };
        // On the cut plane y=±sqrt(Q(z)) and x is affine in z.
        let q = |z: f64| (a * z + b) * z + c;
        let mut minimum = q(z0).min(q(z1));
        if a > 0. {
            minimum = minimum.min(q((-b / (2. * a)).clamp(z0.min(z1), z0.max(z1))));
        }
        if minimum <= 0. {
            return f64::INFINITY;
        }
        (4. * a * c - b * b).abs() * (z1 - z0).powi(2) / (32. * minimum.powf(1.5))
    }
    pub fn volume(&self) -> f64 {
        let m = &self.frame;
        // Independent disk segment integral, compared against OCCT volume.
        let area = |z: f64| {
            let r = self.radius(z);
            let q = (m.cut_offset_mm + m.slope * z) / r;
            r * r * ((-q).acos() + q * (1. - q * q).sqrt())
        };
        const N: usize = 512;
        let dz = m.height_mm / N as f64;
        let mut result = area(0.) + area(m.height_mm);
        for i in 1..N {
            result += area(i as f64 * dz) * if i % 2 == 0 { 2. } else { 4. };
        }
        result * dz / 3.
    }
}
pub fn recognize(a: &Analysis, o: &Options) -> Result<Option<ObliqueBandRecognition>, FabError> {
    if !(32..=10_000).contains(&a.mesh.triangles.len())
        || a.model
            .faces
            .iter()
            .filter(|f| matches!(f.loops.as_slice(),[r] if r.len()>=8))
            .take(2)
            .count()
            < 2
    {
        return Ok(None);
    }
    if [
        o.plane_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.max_deviation_mm,
        o.weld_tolerance_mm,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(FabError::new(
            "options.invalid_tolerance",
            "Les tolérances doivent être finies et strictement positives.",
        ));
    }
    crate::recognition::validate_analysis(a)?;
    Ok(prove(a, o))
}
fn prove(a: &Analysis, o: &Options) -> Option<ObliqueBandRecognition> {
    const TRANSFER_MM: f64 = 2e-6;
    let limit = o.max_deviation_mm - o.weld_tolerance_mm - TRANSFER_MM;
    let fit = o.cylinder_fit_tolerance_mm.min(1e-5);
    if limit <= fit || fit <= 0. || a.mesh.vertices.iter().flatten().any(|v| v.abs() > 1e4) {
        return None;
    }
    let plane = o.plane_tolerance_mm.min(fit);
    let (frame, mut owners) = detect::band_supports(a, fit, plane)?;
    if frame.major_radius_mm <= 1e-4
        || frame.outer_radius_mm - frame.major_radius_mm <= 1e-5
        || frame.height_mm <= 1e-4
        || !(1e-3..4.).contains(&frame.slope.abs())
    {
        return None;
    }
    let y = cross(frame.axis, frame.radial);
    let xyz: Vec<V> = a
        .mesh
        .vertices
        .iter()
        .map(|&world| {
            let p = sub(world, frame.origin);
            [dot(p, frame.radial), dot(p, y), dot(p, frame.axis)]
        })
        .collect();
    let mut masks = vec![0u8; xyz.len()];
    let mut edges = BTreeMap::new();
    let mut borders = Vec::new();
    for (i, (tri, id)) in a.mesh.triangles.iter().zip(&mut owners).enumerate() {
        if *id == usize::MAX {
            *id = 3;
        }
        for (u, v) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            masks[u] |= 1 << *id;
            let key = (u.min(v), u.max(v));
            if let Some((j, forward)) = edges.remove(&key) {
                if forward == (u < v) {
                    return None;
                }
                borders.push((i, j, u, v));
            } else {
                edges.insert(key, (i, u < v));
            }
        }
    }
    if !edges.is_empty() {
        return None;
    }
    let mut candidates = [Family::Cone, Family::Sphere]
        .into_iter()
        .filter_map(|family| {
            let model = ObliqueBand {
                family,
                frame: frame.clone(),
            };
            let max = xyz
                .iter()
                .zip(&masks)
                .filter(|(_, mask)| **mask & 8 != 0)
                .map(|(&[x, y, z], _)| {
                    let r = model.radius(z);
                    if r.is_finite() {
                        (x.hypot(y) - r).abs()
                    } else {
                        f64::INFINITY
                    }
                })
                .fold(0., f64::max);
            (max <= fit).then_some((model, max))
        });
    let (model, residual) = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }
    if model.family == Family::Cone
        && frame.slope.abs()
            >= (frame.outer_radius_mm - frame.major_radius_mm) / frame.height_mm - 1e-4
    {
        return None;
    }
    let cut_chord = |v, w| model.cut_chord(v, w);
    let chart = Chart {
        frame: &frame,
        profile: &|z| (model.radius(z), z),
        ordinate: &|p| p[2],
        hessian: model.hessian()?,
        limits: (0, 1, frame.height_mm),
        radial_slope: (model.family == Family::Cone)
            .then_some((frame.major_radius_mm - frame.outer_radius_mm) / frame.height_mm),
        cut_chord: Some(&cut_chord),
        depth: 6,
    };
    let cert = coverage::prove_chart(
        &chart,
        &xyz,
        &a.mesh.triangles,
        &owners,
        &masks,
        &borders,
        3,
        fit,
        limit,
        &mut 0,
    )?;

    let mut checks = 0;
    let mut max = cert.bound;
    for face in 0..3 {
        max = max.max(coverage::plane(
            &frame,
            &a.mesh,
            &xyz,
            &owners,
            &borders,
            face,
            plane,
            fit,
            cert.rim[face],
            &mut checks,
        )?);
    }
    if max > limit || !max.is_finite() {
        return None;
    }
    let family = model.family;

    let report = RecognitionReport {
        cylinders: 0,
        tori: 0,
        toroidal_fillets: 0,
        cones: usize::from(family == Family::Cone),
        spheres: usize::from(family == Family::Sphere),
        revolutions: 0,
        max_radial_residual_mm: residual,
        max_wall_residual_mm: cert.drift,
        max_chord_error_mm: cert.chord,
        max_deviation_bound_mm: max + o.weld_tolerance_mm + TRANSFER_MM,
        expected_volume_mm3: model.volume(),
        diagnostics: vec![Diagnostic {
            code: "recognition.oblique_band_fitted".into(),
            severity: Severity::Info,
            count: 1,
            message: "La paroi courbe et les deux branches de sa coupe oblique ont été vérifiées."
                .into(),
        }],
    };
    Some(ObliqueBandRecognition {
        model,
        triangle_faces: owners,
        report,
    })
}
