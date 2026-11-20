//! A translated section with one polynomial graph and three straight sides.
//! Abscissa along the measured chord is the affine Bernstein parameter.
use crate::math::{cross, dot, norm, scale, sub, Sum, V};
use crate::recognition::{fit_circle, validate_analysis, Work};
use crate::{Analysis, Diagnostic, FabError, Options, PlanarFace, RecognitionReport, Severity};
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CubicPrism {
    pub origin: V,
    pub x_axis: V,
    pub axis: V,
    pub width_mm: f64,
    pub depth_mm: f64,
    pub height_mm: f64,
    /// Ordinates at Bernstein poles 1 and 2; the endpoints lie on the chord.
    pub ordinates_mm: [f64; 2],
    /// Local x, y and radius of an optional through hole.
    pub bore: Option<[f64; 3]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CubicPrismRecognition {
    pub model: CubicPrism,
    /// Bottom, top, cubic, right, back, left, optional bore.
    pub triangle_faces: Vec<usize>,
    pub report: RecognitionReport,
}

fn graph(t: f64, [a, b]: [f64; 2]) -> f64 {
    3. * t * (1. - t) * ((1. - t) * a + t * b)
}

impl CubicPrism {
    fn project(&self, p: V) -> V {
        let d = sub(p, self.origin);
        [
            dot(d, self.x_axis),
            dot(d, cross(self.axis, self.x_axis)),
            dot(d, self.axis),
        ]
    }
}

fn fit_graph(points: &[V], width: f64, tol: f64) -> Option<[f64; 2]> {
    let eval = |p: V| (p[0] / width, p[1]);
    // Fix the two free controls with widely separated measured abscissae.
    let witness = |at: f64| {
        points
            .iter()
            .copied()
            .map(eval)
            .min_by(|p, q| (p.0 - at).abs().total_cmp(&(q.0 - at).abs()))
    };
    let (a, u) = witness(1. / 3.)?;
    let (b, v) = witness(2. / 3.)?;
    if a < 0.125
        || b - a < 0.125
        || b > 0.875
        || !points
            .iter()
            .copied()
            .map(eval)
            .any(|(t, _)| [0., a, b, 1.].iter().all(|q| (t - q).abs() > 1. / 32.))
    {
        return None;
    }
    let (u, v) = (u / (3. * a * (1. - a)), v / (3. * b * (1. - b)));
    let slope = (v - u) / (b - a);
    let s = u - a * slope;
    let c = [s, s + slope];
    // Candidate for an actual cubic graph. A straight or quadratic graph
    // and numerically unresolved inflections are outside this recognition.
    if (c[0] - c[1]).abs() <= 16. * tol
        || c.iter().any(|p| !p.is_finite())
        || points.windows(2).any(|p| p[1][0] <= p[0][0])
        || points
            .iter()
            .copied()
            .map(eval)
            .any(|(t, y)| (y - graph(t, c)).abs() > tol)
    {
        return None;
    }
    Some(c)
}

fn measure_cap(a: &Analysis, face: &PlanarFace, o: &Options) -> Option<CubicPrism> {
    let outer = face.loops.first()?;
    if !(9..=512).contains(&outer.len()) || face.loops.len() > 2 {
        return None;
    }
    let ring: Vec<_> = outer.iter().rev().map(|&i| a.mesh.vertices[i]).collect();
    let n = ring.len();
    let axis = scale(face.normal, -1. / norm(face.normal));
    let corners: Vec<_> = (0..n)
        .filter(|&i| {
            let u = sub(ring[i], ring[(i + n - 1) % n]);
            let v = sub(ring[(i + 1) % n], ring[i]);
            dot(u, v) < 0.8 * norm(u) * norm(v)
        })
        .collect();
    if corners.len() != 4 {
        return None;
    }
    let runs: Vec<_> = (0..4)
        .map(|i| (corners[(i + 1) % 4] + n - corners[i]) % n)
        .collect();
    let curved = runs.iter().position(|&count| count >= 6)?;
    if runs
        .iter()
        .enumerate()
        .any(|(i, &count)| i != curved && count != 1)
    {
        return None;
    }
    let corner = |offset: usize| ring[corners[(curved + offset) % 4]];
    let chord = sub(corner(1), corner(0));
    let width = norm(chord);
    let tol = o.cylinder_fit_tolerance_mm;
    if width < 64. * tol {
        return None;
    }
    let mut m = CubicPrism {
        origin: corner(0),
        x_axis: scale(chord, 1. / width),
        axis,
        width_mm: width,
        depth_mm: 0.,
        height_mm: 0.,
        ordinates_mm: [0.; 2],
        bore: None,
    };
    let end = m.project(corner(2));
    let start = m.project(corner(3));
    let depth = (start[1] + end[1]) * 0.5;
    if end[2].abs().max(start[2].abs()) > tol
        || (end[0] - width).abs().max(start[0].abs()) > tol
        || (start[1] - end[1]).abs() > tol
        || depth <= 16. * tol
    {
        return None;
    }
    let curve: Vec<_> = (0..=runs[curved])
        .map(|j| m.project(ring[(corners[curved] + j) % n]))
        .collect();
    m.ordinates_mm = fit_graph(&curve, width, tol)?;
    m.depth_mm = depth;
    let top_bound = m.ordinates_mm.into_iter().fold(0., f64::max);
    if top_bound + 16. * tol >= depth {
        return None;
    }
    if let Some(lp) = face.loops.get(1) {
        if lp.len() > 512 {
            return None;
        }
        let points: Vec<_> = lp
            .iter()
            .map(|&i| {
                let [x, y, _] = m.project(a.mesh.vertices[i]);
                [x, y]
            })
            .collect();
        let c = fit_circle(&points, tol)?;
        let [x, y] = c.center;
        let margin = c.radius + 16. * tol;
        if x <= margin || width - x <= margin || y - top_bound <= margin || depth - y <= margin {
            return None;
        }
        m.bore = Some([x, y, c.radius]);
    }
    Some(m)
}

fn delta(t: f64) -> f64 {
    t - TAU * (t / TAU).round()
}

/// Each wall admits one nonfolding chart on (profile parameter, height).
/// Positive degree of the triangulated cap and both rims seals its strip.
/// The polynomial interpolation bound applies inside every source triangle.
fn certify(a: &Analysis, mut m: CubicPrism, o: &Options) -> Option<CubicPrismRecognition> {
    let p: Vec<_> = a.mesh.vertices.iter().map(|&p| m.project(p)).collect();
    let h = p.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
    let tol = o.cylinder_fit_tolerance_mm;
    if h <= 16. * tol || p.iter().any(|p| p[2] < -tol) {
        return None;
    }
    // Both measured sections must agree independently, including an optional
    // bore. Looking along the opposite cap normal reverses the chord axis.
    let other = a.model.faces.iter().find_map(|face| {
        if dot(face.normal, m.axis) < 1. - 1e-10 {
            return None;
        }
        let tip = *face.loops.first()?.first()?;
        if (p[tip][2] - h).abs() > tol {
            return None;
        }
        measure_cap(a, face, o)
    })?;
    let start = m.project(other.origin);
    if (start[0] - m.width_mm).hypot(start[1]).hypot(start[2] - h) > tol
        || (other.depth_mm - m.depth_mm).abs() > tol
        || (other.width_mm - m.width_mm).abs() > tol
        || norm(crate::math::sub(other.x_axis, scale(m.x_axis, -1.))) * m.width_mm > tol
        || (other.ordinates_mm[1] - m.ordinates_mm[0]).abs() > tol
        || (other.ordinates_mm[0] - m.ordinates_mm[1]).abs() > tol
    {
        return None;
    }
    match (m.bore, other.bore) {
        (Some([x, y, r]), Some([ox, oy, or]))
            if (ox + x - m.width_mm).hypot(oy - y) <= tol && (or - r).abs() <= tol => {}
        (None, None) => {}
        _ => return None,
    }
    m.height_mm = h;
    let w = m.width_mm;
    let depth = m.depth_mm;
    let [c1, c2] = m.ordinates_mm;
    let second = |t: f64| 6. * ((1. - t) * (c2 - 2. * c1) + t * (c1 - 2. * c2)).abs();
    let mut areas: [Sum; 7] = std::array::from_fn(|_| Sum::default());
    let mut owners = Vec::with_capacity(a.mesh.triangles.len());
    let mut chord = 0f64;
    let mut error = 0f64;
    for &tri in &a.mesh.triangles {
        let q = tri.map(|i| p[i]);
        let max = |sample: &dyn Fn(V) -> f64| q.iter().copied().map(sample).fold(0., f64::max);
        let cap = if max(&|p| p[2].abs()) <= tol {
            Some(0)
        } else if max(&|p| (p[2] - h).abs()) <= tol {
            Some(1)
        } else {
            None
        };
        let (id, uv, res, curve_error) = if let Some(i) = cap {
            (
                i,
                q.map(|[x, y, _]| [if i == 0 { -x } else { x }, y]),
                max(&|p| (p[2] - if i == 0 { 0. } else { h }).abs()),
                0.,
            )
        } else {
            let mut found = None;
            for i in 2..6 + usize::from(m.bore.is_some()) {
                let mut uv = [[0.; 2]; 3];
                let mut r = 0f64;
                for (uv, &[x, y, z]) in uv.iter_mut().zip(&q) {
                    let (u, res) = match i {
                        2 => (x / w, y - graph(x / w, m.ordinates_mm)),
                        3 => (y, x - w),
                        4 => (-x, y - depth),
                        5 => (-y, x),
                        _ => {
                            let [cx, cy, radius] = m.bore?;
                            (-((y - cy).atan2(x - cx)), (x - cx).hypot(y - cy) - radius)
                        }
                    };
                    *uv = [u, z];
                    r = r.max(res.abs());
                }
                if r > tol {
                    continue;
                }
                if i == 2 && uv.iter().any(|p| p[0] < -tol / w || p[0] > 1. + tol / w) {
                    continue;
                }
                if i == 6 {
                    for j in 1..3 {
                        uv[j][0] = uv[0][0] + delta(uv[j][0] - uv[0][0]);
                    }
                }
                let span = uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
                    - uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                let extra = match i {
                    2 => uv.iter().map(|p| second(p[0])).fold(0., f64::max) * span.powi(2) / 8.,
                    6 if span <= PI / 4. => m.bore?[2] * 2. * (span / 4.).sin().powi(2),
                    6 => return None,
                    _ => 0.,
                };
                if found.replace((i, uv, r, extra)).is_some() {
                    return None;
                }
            }
            found?
        };
        let [s, t, u] = uv;
        let twice = (t[0] - s[0]) * (u[1] - s[1]) - (u[0] - s[0]) * (t[1] - s[1]);
        if twice <= 0. || !twice.is_finite() {
            return None;
        }
        areas[id].add(twice * 0.5);
        owners.push(id);
        error = error.max(res);
        chord = chord.max(curve_error);
    }
    // Signed projected areas of the planar source caps can differ from the
    // analytic section by the enclosed chord strips. Both caps must also
    // be whole and incident to the matching sides (verified closed source).
    let expected_section = w * (depth - (c1 + c2) / 4.) - m.bore.map_or(0., |c| PI * c[2] * c[2]);
    let perimeter =
        2. * (w + depth) + 2. * (c1.abs() + c2.abs()) + m.bore.map_or(0., |c| TAU * c[2]);
    let bound = o.weld_tolerance_mm + error + chord + 1e-6;
    if !bound.is_finite() || bound > o.max_deviation_mm || expected_section <= 0. {
        return None;
    }
    if areas[..2]
        .iter()
        .any(|a| (a.get() - expected_section).abs() > (error + chord + tol) * perimeter)
    {
        return None;
    }
    for (i, length) in [1., depth, w, depth, TAU]
        .into_iter()
        .enumerate()
        .take(4 + usize::from(m.bore.is_some()))
    {
        let expected = length * h;
        if (areas[i + 2].get() - expected).abs() > expected * 1e-6 + tol * (length + h) * 4. {
            return None;
        }
    }
    let holes = usize::from(m.bore.is_some());
    let assumption = Diagnostic {
        code: "recognition.cubic_extrusion_assumed".into(),
        severity: Severity::Warning,
        count: 1,
        message: "Le flanc est interprété comme l’extrusion droite du profil cubique mesuré aux deux extrémités.".into(),
    };
    Some(CubicPrismRecognition {
        model: m,
        triangle_faces: owners,
        report: RecognitionReport {
            cylinders: holes,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            revolutions: 0,
            max_radial_residual_mm: error,
            max_wall_residual_mm: error,
            max_chord_error_mm: chord,
            max_deviation_bound_mm: bound,
            expected_volume_mm3: expected_section * h,
            diagnostics: vec![assumption],
        },
    })
}

pub fn recognize_cubic_prism(
    a: &Analysis,
    o: &Options,
) -> Result<Option<CubicPrismRecognition>, FabError> {
    if a.mesh.triangles.len() > 32_000
        || a.model.faces.len() > 8192
        || !a
            .model
            .faces
            .iter()
            .any(|p| p.loops.first().is_some_and(|r| r.len() >= 9))
    {
        return Ok(None);
    }
    if [
        o.cylinder_fit_tolerance_mm,
        o.plane_tolerance_mm,
        o.weld_tolerance_mm,
        o.max_deviation_mm,
    ]
    .into_iter()
    .any(|v| !v.is_finite() || v <= 0.)
    {
        return Err(FabError::new(
            "options.invalid_tolerance",
            "Tolérances invalides.",
        ));
    }
    validate_analysis(a)?;
    let mut work = Work::default();
    for cap in &a.model.faces {
        if let Some(m) = measure_cap(a, cap, o) {
            if !work.reserve_optional(a.mesh.triangles.len() * 32 + a.mesh.vertices.len() * 4) {
                return Ok(None);
            }
            if let Some(hit) = certify(a, m, o) {
                // A direct caller can supply coincident vertices with different
                // indices. Their areas agree even when the source seam is open.
                crate::pipeline::validate_topology(&a.mesh)?;
                return Ok(Some(hit));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_geometrically_coincident_open_seam_cannot_certify_a_prism() {
        let options = Options::default();
        let mut analysis = crate::analyze_stl(
            include_bytes!("../../../fixtures/cubic_extrusion/cam-coarse.stl"),
            &options,
        )
        .unwrap();
        assert!(recognize_cubic_prism(&analysis, &options)
            .unwrap()
            .is_some());

        // Keep the fit, normals and projected areas identical, but detach one
        // corner of a triangle from its neighbours.
        let vertex = analysis.mesh.triangles[0][0];
        let duplicate = analysis.mesh.vertices[vertex];
        analysis.mesh.triangles[0][0] = analysis.mesh.vertices.len();
        analysis.mesh.vertices.push(duplicate);
        analysis.model.vertices.push(duplicate);
        assert_eq!(
            recognize_cubic_prism(&analysis, &options).unwrap_err().code,
            "mesh.open"
        );
    }
}
