use crate::math::*;
use crate::{
    Analysis, AnalyticExtrusion, Diagnostic, FabError, Options, ProfileLoop, Recognition,
    RecognitionReport, ReconstructionMode, Severity,
};
use std::f64::consts::{PI, TAU};

type P2 = [f64; 2];
const MAX_WORK: usize = 10_000_000;
const MIN_CIRCLE_VERTICES: usize = 12;

#[derive(Default)]
pub(crate) struct Work(usize);
impl Work {
    pub(crate) fn spend(&mut self, count: usize) -> Result<(), FabError> {
        self.0 = self.0.saturating_add(count);
        if self.0 > MAX_WORK {
            Err(FabError::new(
                "recognition.too_complex",
                "Budget de reconnaissance dépassé. Utiliser le mode planaire.",
            ))
        } else {
            Ok(())
        }
    }
}
struct Frame {
    origin: V,
    axis: V,
    x: V,
    y: V,
}
impl Frame {
    fn project(&self, p: V) -> (P2, f64) {
        let p = sub(p, self.origin);
        ([dot(p, self.x), dot(p, self.y)], dot(p, self.axis))
    }
}
pub(crate) struct CircleFit {
    pub center: P2,
    pub radius: f64,
    pub radial: f64,
    pub chord: f64,
}

/// Recognizes complete straight extrusions in explicit cylinder mode.
/// All lateral triangles, including intermediate axial rings, are verified.
/// Source BRep validation remains required: this is not a self-intersection test.
pub fn recognize_extrusion(
    analysis: &Analysis,
    options: &Options,
) -> Result<Recognition, FabError> {
    if options.reconstruction != ReconstructionMode::Cylinders {
        return Err(FabError::new(
            "recognition.disabled",
            "Activer explicitement le mode cylindres pour cette reconstruction.",
        ));
    }
    for tolerance in [
        options.weld_tolerance_mm,
        options.plane_tolerance_mm,
        options.cylinder_fit_tolerance_mm,
        options.max_deviation_mm,
    ] {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    validate_analysis(analysis)?;
    let adjacency = crate::pipeline::validate_topology(&analysis.mesh)?;
    let segmentation = crate::planar::build_with_provenance(
        &analysis.mesh,
        &adjacency,
        options
            .plane_tolerance_mm
            .max(options.cylinder_fit_tolerance_mm),
    )?;
    let mut work = Work::default();
    let mut budget_error = None;
    for (index, face) in segmentation.model.faces.iter().enumerate() {
        if !face.loops.iter().any(|lp| lp.len() >= MIN_CIRCLE_VERTICES) {
            continue;
        }
        work.spend(analysis.mesh.vertices.len())?;
        match candidate(analysis, &segmentation, index, options, &mut work) {
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {}
            Err(error) if error.code == "recognition.deviation_exceeded" => {
                budget_error = Some(error)
            }
            Err(error) => return Err(error),
        }
    }
    Err(budget_error.unwrap_or_else(||FabError::new("recognition.unsupported",
        "Aucune extrusion droite complète à profil constant et cercle suffisamment échantillonné n'a été prouvée. Utiliser le mode planaire.")))
}

pub(crate) fn validate_analysis(a: &Analysis) -> Result<(), FabError> {
    let valid = !a.mesh.triangles.is_empty()
        && a.mesh.triangles.len() <= crate::stl::MAX_TRIANGLES
        && a.mesh.vertices == a.model.vertices
        && a.mesh.vertices.iter().flatten().all(|x| x.is_finite())
        && a.mesh
            .triangles
            .iter()
            .flatten()
            .all(|&v| v < a.mesh.vertices.len())
        && a.model.faces.iter().all(|f| {
            norm(f.normal).is_finite()
                && norm(f.normal) > 0.
                && !f.loops.is_empty()
                && f.loops
                    .iter()
                    .all(|lp| lp.len() >= 3 && lp.iter().all(|&v| v < a.mesh.vertices.len()))
        });
    if valid {
        Ok(())
    } else {
        Err(FabError::new(
            "recognition.invalid_analysis",
            "Analyse source invalide ou incohérente.",
        ))
    }
}

fn candidate(
    a: &Analysis,
    segmentation: &crate::planar::Segmentation,
    bottom: usize,
    o: &Options,
    work: &mut Work,
) -> Result<Option<Recognition>, FabError> {
    let cap = &segmentation.model.faces[bottom];
    let axis = scale(cap.normal, -1. / norm(cap.normal));
    let least = (0..3)
        .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
        .unwrap();
    let mut helper = [0.; 3];
    helper[least] = 1.;
    let x = cross(helper, axis);
    let x = scale(x, 1. / norm(x));
    let frame = Frame {
        origin: a.mesh.vertices[cap.loops[0][0]],
        axis,
        x,
        y: cross(axis, x),
    };
    let projected: Vec<(P2, f64)> = a.mesh.vertices.iter().map(|&p| frame.project(p)).collect();
    let height = projected
        .iter()
        .map(|(_, z)| *z)
        .fold(f64::NEG_INFINITY, f64::max);
    let tolerance = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    if !height.is_finite()
        || height <= tolerance * 4.
        || projected.iter().any(|(_, z)| *z < -tolerance)
    {
        return Ok(None);
    }

    // Reverse the outward bottom loops: outer CCW and holes CW in this frame.
    let loops: Vec<Vec<P2>> = cap
        .loops
        .iter()
        .map(|lp| lp.iter().rev().map(|&v| projected[v].0).collect())
        .collect();
    if loops
        .iter()
        .enumerate()
        .any(|(i, lp)| (signed_area(lp) > 0.) != (i == 0))
    {
        return Ok(None);
    }
    let fits: Vec<Option<CircleFit>> = loops
        .iter()
        .map(|lp| fit_circle(lp, o.cylinder_fit_tolerance_mm))
        .collect();
    if fits.iter().all(Option::is_none) {
        return Ok(None);
    }
    let mut cap_residual: f64 = 0.;
    for lp in &cap.loops {
        for &v in lp {
            cap_residual = cap_residual.max(projected[v].1.abs());
        }
    }
    if cap_residual > tolerance {
        return Ok(None);
    }
    let mut top = None;
    let mut correspondence_residual: f64 = 0.;
    for (index, face) in segmentation.model.faces.iter().enumerate() {
        work.spend(1)?;
        if index == bottom
            || face.loops.len() != loops.len()
            || dot(face.normal, axis) / norm(face.normal) < 1. - 1e-10
        {
            continue;
        }
        if face
            .loops
            .iter()
            .flatten()
            .any(|&v| (projected[v].1 - height).abs() > tolerance)
        {
            continue;
        }
        let upper: Vec<Vec<P2>> = face
            .loops
            .iter()
            .map(|lp| lp.iter().map(|&v| projected[v].0).collect())
            .collect();
        if let Some(residual) = match_loops(&loops, &upper, tolerance, work)? {
            top = Some(index);
            correspondence_residual = residual;
            break;
        }
    }
    if top.is_none() {
        return Ok(None);
    }

    // Simplification belongs to the separate extrusion model, never to the
    // source planar BRep. All source vertices are retained in Analysis.
    let mut polygons = Vec::new();
    for lp in &loops {
        polygons.push(simplify_collinear(lp, work)?);
    }
    let segments: Vec<(P2, P2)> = polygons
        .iter()
        .flat_map(|lp| (0..lp.len()).map(|i| (lp[i], lp[(i + 1) % lp.len()])))
        .collect();
    let mut wall_residual = correspondence_residual.max(cap_residual);
    let mut wall_triangles = 0;
    for (triangle_index, triangle) in a.mesh.triangles.iter().enumerate() {
        let positions = triangle.map(|i| projected[i]);
        // Membership, never altitude alone: tiny near-cap flanges are walls
        // and must not disappear inside the cap tolerance band.
        if segmentation.triangle_regions[triangle_index] == bottom {
            for (_, z) in positions {
                if z.abs() > tolerance {
                    return Ok(None);
                }
                wall_residual = wall_residual.max(z.abs());
                cap_residual = cap_residual.max(z.abs());
            }
            continue;
        }
        if Some(segmentation.triangle_regions[triangle_index]) == top {
            for (_, z) in positions {
                if (z - height).abs() > tolerance {
                    return Ok(None);
                }
                wall_residual = wall_residual.max((z - height).abs());
                cap_residual = cap_residual.max((z - height).abs());
            }
            continue;
        }
        wall_triangles += 1;
        let [p, q, r] = triangle.map(|i| a.mesh.vertices[i]);
        let triangle_normal = cross(sub(q, p), sub(r, p));
        let mut best = f64::INFINITY;
        for &(start, end) in &segments {
            work.spend(1)?;
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let outward = [
                frame.x[0] * dy - frame.y[0] * dx,
                frame.x[1] * dy - frame.y[1] * dx,
                frame.x[2] * dy - frame.y[2] * dx,
            ];
            if dot(triangle_normal, outward) <= 0. {
                continue;
            }
            let residual = positions
                .iter()
                .map(|(p, z)| {
                    let axial = (-*z).max(*z - height).max(0.);
                    distance_segment(*p, start, end).hypot(axial)
                })
                .fold(0., f64::max);
            best = best.min(residual);
        }
        if best > tolerance {
            return Ok(None);
        }
        wall_residual = wall_residual.max(best);
    }
    if wall_triangles == 0 {
        return Ok(None);
    }
    // Axial cap error and in-plane cap correspondence error may coexist.
    // Sum their measured maxima into the extrusion bound rather than silently
    // assuming those errors occur in the same direction or at the same point.
    wall_residual += cap_residual + correspondence_residual;
    let mut cylinders = 0;
    let mut radial: f64 = 0.;
    let mut chord: f64 = 0.;
    let mut output = Vec::new();
    for (polygon, fit) in polygons.into_iter().zip(fits) {
        output.push(if let Some(fit) = fit {
            cylinders += 1;
            radial = radial.max(fit.radial);
            chord = chord.max(fit.chord);
            ProfileLoop::Circle {
                center: fit.center,
                radius_mm: fit.radius,
            }
        } else {
            ProfileLoop::Polygon { points: polygon }
        });
    }
    if !profiles_are_disjoint(&output, work)? {
        return Ok(None);
    }
    let expected_area = loop_area(&output[0]) - output[1..].iter().map(loop_area).sum::<f64>();
    let expected_volume = expected_area * height;
    // Distance to each associated extruded segment is convex: checking every
    // vertex bounds its entire source triangle. Add the circle/chord and weld
    // allowances; this is not a global bidirectional Hausdorff computation.
    let deviation = o.weld_tolerance_mm + wall_residual + radial + chord;
    if !expected_volume.is_finite() || expected_volume <= 0. || !deviation.is_finite() {
        return Ok(None);
    }
    if deviation > o.max_deviation_mm {
        return Err(FabError::new("recognition.deviation_exceeded",format!(
            "Borne d'écart {:.6} mm supérieure au budget {:.6} mm (corde, ajustement, extrusion et soudage). Augmenter explicitement le budget ou utiliser le mode planaire.",deviation,o.max_deviation_mm)));
    }
    let outer = output.remove(0);
    Ok(Some(Recognition{
        extrusion:AnalyticExtrusion{origin:frame.origin,axis:frame.axis,x_axis:frame.x,height_mm:height,outer,holes:output},
        report:RecognitionReport{cylinders,cones:0,spheres:0,tori:0,toroidal_fillets:0,max_radial_residual_mm:radial,max_chord_error_mm:chord,
            max_wall_residual_mm:wall_residual,max_deviation_bound_mm:deviation,expected_volume_mm3:expected_volume,
            diagnostics:vec![Diagnostic{code:"recognition.circular_intent_assumed".into(),severity:Severity::Warning,
                message:"Le mode explicite interprète des profils polygonaux suffisamment circulaires comme des cercles. L'intention de conception originale n'est pas récupérable du STL.".into(),count:cylinders},
                Diagnostic{code:"recognition.bound_scope".into(),severity:Severity::Info,
                message:"Borne composée : soudage, distance des triangles aux parois extrudées associées, résidu radial et flèche des cordes. Ce n'est pas une mesure Hausdorff globale ; la coque source et le résultat doivent être validés par le noyau CAO.".into(),count:1}]}
    }))
}

pub(crate) fn match_loops(
    lower: &[Vec<P2>],
    upper: &[Vec<P2>],
    tol: f64,
    work: &mut Work,
) -> Result<Option<f64>, FabError> {
    let mut used = vec![false; upper.len()];
    let mut residual: f64 = 0.;
    for (i, lp) in lower.iter().enumerate() {
        let mut matched = None;
        for (j, other) in upper.iter().enumerate() {
            if used[j] || (i == 0) != (j == 0) || lp.len() != other.len() {
                continue;
            }
            for shift in 0..other.len() {
                work.spend(1)?;
                if distance(lp[0], other[shift]) > tol {
                    continue;
                }
                let mut error: f64 = 0.;
                for k in 0..lp.len() {
                    work.spend(1)?;
                    error = error.max(distance(lp[k], other[(k + shift) % lp.len()]));
                }
                if error <= tol {
                    matched = Some((j, error));
                    break;
                }
            }
            if matched.is_some() {
                break;
            }
        }
        if let Some((j, error)) = matched {
            used[j] = true;
            residual = residual.max(error);
        } else {
            return Ok(None);
        }
    }
    Ok(Some(residual))
}

/// Centered, normalized algebraic fit with strict maximum radial residual.
/// No discarded outliers. Complete angular coverage and density are required.
pub(crate) fn fit_circle(points: &[P2], tolerance: f64) -> Option<CircleFit> {
    if points.len() < MIN_CIRCLE_VERTICES {
        return None;
    }
    let reference = points[0];
    let mut sums = [Sum::default(), Sum::default()];
    for p in points {
        for i in 0..2 {
            sums[i].add(p[i] - reference[i]);
        }
    }
    let mean = [
        reference[0] + sums[0].get() / points.len() as f64,
        reference[1] + sums[1].get() / points.len() as f64,
    ];
    let extent = points.iter().map(|p| distance(*p, mean)).fold(0., f64::max);
    if !extent.is_finite() || extent <= tolerance {
        return None;
    }
    let mut xx = Sum::default();
    let mut xy = Sum::default();
    let mut yy = Sum::default();
    let mut xq = Sum::default();
    let mut yq = Sum::default();
    for p in points {
        let x = (p[0] - mean[0]) / extent;
        let y = (p[1] - mean[1]) / extent;
        let q = x * x + y * y;
        xx.add(x * x);
        xy.add(x * y);
        yy.add(y * y);
        xq.add(x * q * 0.5);
        yq.add(y * q * 0.5);
    }
    let (xx, xy, yy, xq, yq) = (xx.get(), xy.get(), yy.get(), xq.get(), yq.get());
    let determinant = xx * yy - xy * xy;
    if determinant <= (xx + yy).powi(2) * 1e-12 {
        return None;
    }
    let center = [
        mean[0] + extent * (xq * yy - yq * xy) / determinant,
        mean[1] + extent * (yq * xx - xq * xy) / determinant,
    ];
    let mut radius_sum = Sum::default();
    for p in points {
        radius_sum.add(distance(*p, center));
    }
    let radius = radius_sum.get() / points.len() as f64;
    if !radius.is_finite() || radius <= tolerance {
        return None;
    }
    let radial = points
        .iter()
        .map(|p| (distance(*p, center) - radius).abs())
        .fold(0., f64::max);
    if radial > tolerance {
        return None;
    }
    let direction = if signed_area(points) > 0. { 1. } else { -1. };
    let mut total = Sum::default();
    let mut maximum: f64 = 0.;
    for i in 0..points.len() {
        let p = [points[i][0] - center[0], points[i][1] - center[1]];
        let q = [
            points[(i + 1) % points.len()][0] - center[0],
            points[(i + 1) % points.len()][1] - center[1],
        ];
        let angle = direction * (p[0] * q[1] - p[1] * q[0]).atan2(p[0] * q[0] + p[1] * q[1]);
        if angle <= 0. || angle > PI / 4. + 1e-12 {
            return None;
        }
        total.add(angle);
        maximum = maximum.max(angle);
    }
    if (total.get() - TAU).abs() > 1e-8 {
        return None;
    }
    let chord = 2. * radius * (maximum / 4.).sin().powi(2);
    Some(CircleFit {
        center,
        radius,
        radial,
        chord,
    })
}

fn simplify_collinear(points: &[P2], work: &mut Work) -> Result<Vec<P2>, FabError> {
    let mut result = points.to_vec();
    let extent = points
        .iter()
        .map(|p| distance(*p, points[0]))
        .fold(0., f64::max);
    let epsilon = extent * f64::EPSILON * 128.;
    loop {
        let mut remove = None;
        for i in 0..result.len() {
            work.spend(1)?;
            if result.len() > 3
                && distance_segment(
                    result[i],
                    result[(i + result.len() - 1) % result.len()],
                    result[(i + 1) % result.len()],
                ) <= epsilon
            {
                remove = Some(i);
                break;
            }
        }
        if let Some(i) = remove {
            result.remove(i);
        } else {
            break;
        }
    }
    Ok(result)
}
fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
pub(crate) fn distance_segment(p: P2, a: P2, b: P2) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let length = d[0].hypot(d[1]);
    if length == 0. {
        return distance(p, a);
    }
    let unit = [d[0] / length, d[1] / length];
    let t = ((p[0] - a[0]) * unit[0] + (p[1] - a[1]) * unit[1]).clamp(0., length);
    distance(p, [a[0] + unit[0] * t, a[1] + unit[1] * t])
}
pub(crate) fn signed_area(points: &[P2]) -> f64 {
    let origin = points[0];
    let mut sum = Sum::default();
    for i in 0..points.len() {
        let p = [points[i][0] - origin[0], points[i][1] - origin[1]];
        let q = [
            points[(i + 1) % points.len()][0] - origin[0],
            points[(i + 1) % points.len()][1] - origin[1],
        ];
        sum.add(p[0] * q[1] - p[1] * q[0]);
    }
    sum.get() * 0.5
}
fn loop_area(profile: &ProfileLoop) -> f64 {
    match profile {
        ProfileLoop::Circle { radius_mm, .. } => PI * radius_mm * radius_mm,
        ProfileLoop::Polygon { points } => signed_area(points).abs(),
    }
}
fn point_inside(p: P2, polygon: &[P2]) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1])
        {
            inside = !inside;
        }
    }
    inside
}
fn boundary_distance(p: P2, polygon: &[P2]) -> f64 {
    (0..polygon.len())
        .map(|i| distance_segment(p, polygon[i], polygon[(i + 1) % polygon.len()]))
        .fold(f64::INFINITY, f64::min)
}
fn profiles_are_disjoint(profiles: &[ProfileLoop], work: &mut Work) -> Result<bool, FabError> {
    // Source polygon loops already passed analyze_stl. Circle replacements
    // must not introduce contacts, crossings, or changes in inclusion.
    for hole in &profiles[1..] {
        work.spend(profile_size(&profiles[0]) + profile_size(hole))?;
        let contained = match (&profiles[0], hole) {
            (
                ProfileLoop::Circle {
                    center: a,
                    radius_mm: ra,
                },
                ProfileLoop::Circle {
                    center: b,
                    radius_mm: rb,
                },
            ) => distance(*a, *b) + rb < *ra,
            (ProfileLoop::Polygon { points }, ProfileLoop::Circle { center, radius_mm }) => {
                point_inside(*center, points) && boundary_distance(*center, points) > *radius_mm
            }
            (ProfileLoop::Circle { center, radius_mm }, ProfileLoop::Polygon { points }) => {
                points.iter().all(|p| distance(*p, *center) < *radius_mm)
            }
            _ => true,
        };
        if !contained {
            return Ok(false);
        }
    }
    for i in 1..profiles.len() {
        for j in i + 1..profiles.len() {
            work.spend(profile_size(&profiles[i]) + profile_size(&profiles[j]))?;
            let disjoint = match (&profiles[i], &profiles[j]) {
                (
                    ProfileLoop::Circle {
                        center: a,
                        radius_mm: ra,
                    },
                    ProfileLoop::Circle {
                        center: b,
                        radius_mm: rb,
                    },
                ) => distance(*a, *b) > ra + rb,
                (ProfileLoop::Circle { center, radius_mm }, ProfileLoop::Polygon { points })
                | (ProfileLoop::Polygon { points }, ProfileLoop::Circle { center, radius_mm }) => {
                    !point_inside(*center, points)
                        && boundary_distance(*center, points) > *radius_mm
                }
                _ => true,
            };
            if !disjoint {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn profile_size(profile: &ProfileLoop) -> usize {
    match profile {
        ProfileLoop::Circle { .. } => 1,
        ProfileLoop::Polygon { points } => points.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn samples(count: usize, angle: f64) -> Vec<P2> {
        (0..count)
            .map(|i| {
                let a = angle * i as f64 / count as f64;
                [5. + 10. * a.cos(), -8. + 10. * a.sin()]
            })
            .collect()
    }
    #[test]
    fn circle_fit_requires_complete_well_sampled_evidence() {
        assert!(fit_circle(&samples(8, TAU), 1e-5).is_none());
        assert!(fit_circle(&samples(64, PI / 6.), 1e-5).is_none());
        let fit = fit_circle(&samples(64, TAU), 1e-5).unwrap();
        assert!(distance(fit.center, [5., -8.]) < 1e-12);
        assert!((fit.radius - 10.).abs() < 1e-12);
        let mut noisy = samples(64, TAU);
        noisy[5][0] += 0.01;
        assert!(fit_circle(&noisy, 1e-5).is_none());
    }
    #[test]
    fn fitted_holes_must_remain_disjoint_and_inside() {
        let outer = ProfileLoop::Polygon {
            points: vec![[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]],
        };
        let hole = ProfileLoop::Circle {
            center: [0., 0.],
            radius_mm: 1.,
        };
        assert!(
            profiles_are_disjoint(&[outer.clone(), hole.clone()], &mut Work::default()).unwrap()
        );
        let outside = ProfileLoop::Circle {
            center: [1.5, 0.],
            radius_mm: 1.,
        };
        assert!(!profiles_are_disjoint(&[outer.clone(), outside], &mut Work::default()).unwrap());
        let touching = ProfileLoop::Circle {
            center: [1., 0.],
            radius_mm: 0.5,
        };
        assert!(!profiles_are_disjoint(&[outer, hole, touching], &mut Work::default()).unwrap());
        assert_eq!(
            Work(MAX_WORK).spend(1).unwrap_err().code,
            "recognition.too_complex"
        );
    }
}
