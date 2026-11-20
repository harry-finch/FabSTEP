//! A clipped cylindrical blank tangent to a convex toroidal lip. Estimate its
//! supports from three planar disks, then prove the five entire trimmed faces.
use crate::math::{cross, dot, norm, scale, sub, V};
use crate::{Analysis, Diagnostic, FabError, Options, RecognitionReport, Severity};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::FRAC_PI_2,
};

pub(crate) mod coverage;
pub(crate) mod detect;

const BOTTOM: usize = 0;
const TOP: usize = 1;
const CUT: usize = 2;
const CYLINDER: usize = 3;
const TORUS: usize = 4;
const NATIVE_ALLOWANCE: f64 = 2e-5;
type UV = [f64; 2];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObliqueLip {
    pub origin: V,
    pub axis: V,
    pub radial: V,
    pub outer_radius_mm: f64,
    pub major_radius_mm: f64,
    pub height_mm: f64,
    pub cut_offset_mm: f64,
    pub slope: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObliqueLipRecognition {
    pub model: ObliqueLip,
    /// Bottom, top, cut plane, outer cylinder, toroidal lip.
    pub triangle_faces: Vec<usize>,
    pub report: RecognitionReport,
}
fn unit(v: V) -> Option<V> {
    let n = norm(v);
    (n > 1e-12 && n.is_finite()).then(|| scale(v, 1. / n))
}
fn plus(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
impl ObliqueLip {
    fn minor(&self) -> f64 {
        self.outer_radius_mm - self.major_radius_mm
    }
    fn shoulder(&self) -> f64 {
        self.height_mm - self.minor()
    }
    fn local(&self, p: V) -> V {
        let p = sub(p, self.origin);
        [
            dot(p, self.radial),
            dot(p, cross(self.axis, self.radial)),
            dot(p, self.axis),
        ]
    }
    fn profile(&self, f: usize, v: f64) -> (f64, f64) {
        if f == CYLINDER {
            (self.outer_radius_mm, v)
        } else {
            (
                self.major_radius_mm + self.minor() * v.cos(),
                self.shoulder() + self.minor() * v.sin(),
            )
        }
    }
    fn cut(&self, z: f64) -> f64 {
        self.cut_offset_mm + self.slope * z
    }
    fn span(&self, f: usize, v: f64) -> f64 {
        let (r, z) = self.profile(f, v);
        (-self.cut(z) / r).acos()
    }
    fn point(&self, f: usize, [s, v]: UV) -> V {
        let (r, z) = self.profile(f, v);
        let u = s * self.span(f, v);
        [-r * u.cos(), -r * u.sin(), z]
    }
    // For q=cut(z)/rho, bound its first two derivatives using q*rho=cut(z).
    // The denominator also keeps both intersection branches away from tangency.
    fn hessian(&self, f: usize) -> Option<[f64; 3]> {
        let (lo, hi, r, dr, dz) = if f == TORUS {
            (
                self.shoulder(),
                self.height_mm,
                self.major_radius_mm,
                self.minor(),
                self.minor(),
            )
        } else {
            (0., self.shoulder(), self.outer_radius_mm, 0., 1.)
        };
        let q = self.cut(lo).abs().max(self.cut(hi).abs()) / r;
        if q >= 0.97 {
            return None;
        }
        let q1 = (self.slope.abs() * dz + q * dr) / r;
        let q2 = (self.slope.abs() + q + 2. * q1) * dr / r;
        let root = (1. - q * q).sqrt();
        let a = (-q).acos();
        let a1 = q1 / root;
        let a2 = q2 / root + q * q1 * q1 / root.powi(3);
        Some([
            self.outer_radius_mm * a * a,
            dr * a + self.outer_radius_mm * (a + 1.) * a1,
            dr + 2. * dr * a1 + self.outer_radius_mm * (a1 * a1 + a2),
        ])
    }
    fn volume(&self) -> f64 {
        // Integrate the disk segment along the cylinder axis. The angular
        // fillet coordinate removes the square-root endpoint from quadrature.
        let integral = |f, hi: f64| {
            let area = |v| {
                let (r, z) = self.profile(f, v);
                let q = self.cut(z) / r;
                let a = r * r * ((-q).acos() + q * (1. - q * q).sqrt());
                if f == TORUS {
                    a * self.minor() * v.cos()
                } else {
                    a
                }
            };
            const N: usize = 512;
            let h = hi / N as f64;
            let mut sum = area(0.) + area(hi);
            for i in 1..N {
                sum += area(h * i as f64) * if i % 2 == 0 { 2. } else { 4. };
            }
            sum * h / 3.
        };
        integral(CYLINDER, self.shoulder()) + integral(TORUS, FRAC_PI_2)
    }
}
pub fn recognize(a: &Analysis, o: &Options) -> Result<Option<ObliqueLipRecognition>, FabError> {
    if !(32..=10_000).contains(&a.mesh.triangles.len())
        || a.model
            .faces
            .iter()
            .filter(|p| matches!(p.loops.as_slice(), [r] if r.len()>=8))
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
fn prove(a: &Analysis, o: &Options) -> Option<ObliqueLipRecognition> {
    let available = o.max_deviation_mm - o.weld_tolerance_mm - NATIVE_ALLOWANCE;
    if available <= 0. || a.mesh.vertices.iter().flatten().any(|v| v.abs() > 1e4) {
        return None;
    }
    let fit = o.cylinder_fit_tolerance_mm.min(1e-5);
    let plane = o.plane_tolerance_mm.min(fit);
    let (model, mut owners) = detect::supports(a, fit, plane)?;
    if model.major_radius_mm <= model.minor() || model.minor() <= 1e-4 || model.shoulder() <= 1e-4 {
        return None;
    }
    let xyz: Vec<_> = a
        .mesh
        .vertices
        .iter()
        .copied()
        .map(|p| model.local(p))
        .collect();
    let mut masks = vec![0u8; xyz.len()];
    let mut edges = BTreeMap::new();
    let mut borders = Vec::new();
    for (i, (&t, id)) in a.mesh.triangles.iter().zip(&mut owners).enumerate() {
        if *id == usize::MAX {
            *id = if t.iter().all(|&j| {
                let [x, y, z] = xyz[j];
                (x.hypot(y) - model.outer_radius_mm).abs() <= fit && z <= model.shoulder() + fit
            }) {
                CYLINDER
            } else {
                TORUS
            };
        }
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
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
    let mut work = 0;
    let charts = [CYLINDER, TORUS].map(|f| {
        coverage::chart(
            &model,
            &xyz,
            &a.mesh.triangles,
            &owners,
            &masks,
            &borders,
            f,
            fit,
            available,
            &mut work,
        )
    });
    let [Some(cyl), Some(tor)] = charts else {
        return None;
    };
    let mut bound = cyl.bound.max(tor.bound);
    let mut checks = 0;
    for f in [BOTTOM, TOP, CUT] {
        let rim = cyl.rim[f].max(tor.rim[f]);
        bound = bound.max(coverage::plane(
            &model,
            &a.mesh,
            &xyz,
            &owners,
            &borders,
            f,
            plane,
            fit,
            rim,
            &mut checks,
        )?);
    }
    if !(0.0..=available).contains(&bound) {
        return None;
    }
    let volume = model.volume();
    let report=RecognitionReport {
        cylinders:1,tori:1,toroidal_fillets:1,cones:0,spheres:0,revolutions:0,
        max_radial_residual_mm:cyl.drift.max(tor.drift), max_chord_error_mm:cyl.chord.max(tor.chord),max_wall_residual_mm:bound,
        max_deviation_bound_mm:bound+NATIVE_ALLOWANCE+o.weld_tolerance_mm,expected_volume_mm3:volume,
        diagnostics:vec![Diagnostic{code:"recognition.oblique_lip_fitted".into(),severity:Severity::Info,count:1,
            message:"La paroi cylindrique et son congé torique ont été reconstruits avec les deux branches de leur coupe oblique.".into()}]
    };
    Some(ObliqueLipRecognition {
        model,
        triangle_faces: owners,
        report,
    })
}
