//! One or two cubic Bernstein spans over a strictly increasing axial coordinate.
//! A one-span meridian may start at a witnessed axial pole. Its oriented
//! triangle fan covers the first disk, meeting the annulus on its first ring.
//! Recognition already checked a connected, oriented, closed two-manifold
//! (including vertex links). The segmentation gives the actual boundary of
//! each cap as one simple cycle. Removing both end disks (or a disk and
//! the first fan) gives two boundary cycles. Each remaining lateral triangle has
//! positive degree in the periodic chart and each end cycle maps once around
//! it. Hence the sum of local degrees at any regular interior point is one.
//! A hole would have degree zero, a second sheet degree at least two. Shared
//! edges have consistent lifts and nonzero triangles, extending coverage to
//! boundaries and vertices. The same argument on each cap's cyclic polygon
//! proves its planar interior before extending the inscribed rim to the disk.
mod irregular;
mod irregular_joint;
pub(super) mod returning;
use super::*;
use crate::{planar::Segmentation, CubicRevolutionSpan, SmoothRevolution};

// 20 times the native 5e-8 mm guard: both serializations, then the final
// stored support/trim domains and numerical slack (ADR-0039).
const NATIVE_RESERVE_MM: f64 = 1e-6;
const MAX_TRIANGLES: usize = 32_000;
const MAX_REGIONS: usize = 8192;

fn radius(b: &[f64; 4], t: f64) -> f64 {
    let s = 1. - t;
    s * s * s * b[0] + 3. * s * s * t * b[1] + 3. * s * t * t * b[2] + t * t * t * b[3]
}

/// Endpoints and two interior measurements determine the four Bernstein
/// coefficients. Require at least one independent, well-separated check.
fn fit(nodes: &[Node], o: &Options) -> Option<[f64; 4]> {
    let first = &nodes[0];
    let last = nodes.last()?;
    let height = last.z - first.z;
    let tol = o.cylinder_fit_tolerance_mm.max(o.plane_tolerance_mm);
    let t = |n: &Node| (n.z - first.z) / height;
    let closest = |target: f64| {
        nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (t(a) - target).abs().total_cmp(&(t(b) - target).abs()))
            .map(|(i, _)| i)
    };
    let a = closest(1. / 3.)?;
    let b = closest(2. / 3.)?;
    let (u, v) = (t(&nodes[a]), t(&nodes[b]));
    if u < 0.125 || v - u < 0.125 || 1. - v < 0.125 || height * 0.125 <= 8. * tol {
        return None;
    }
    let rhs = |n: &Node| {
        let q = t(n);
        (n.radius - (1. - q).powi(3) * first.radius - q.powi(3) * last.radius) / (3. * q * (1. - q))
    };
    // rhs(t) = (1-t) b1 + t b2.
    let slope = (rhs(&nodes[b]) - rhs(&nodes[a])) / (v - u);
    let b1 = rhs(&nodes[a]) - u * slope;
    let poles = [first.radius, b1, b1 + slope, last.radius];
    if poles
        .iter()
        .skip(usize::from(first.radius == 0.))
        .any(|&r| !r.is_finite() || r <= 8. * tol)
        || !nodes
            .iter()
            .any(|n| [0., u, v, 1.].iter().all(|&q| (t(n) - q).abs() > 1. / 64.))
    {
        return None;
    }
    let mut non_affine = 0;
    for n in nodes {
        let q = t(n);
        if (n.radius - radius(&poles, q)).abs() > o.cylinder_fit_tolerance_mm {
            return None;
        }
        let linear = (1. - q) * first.radius + q * last.radius;
        if (n.radius - linear).abs() > 8. * tol {
            non_affine += 1;
        }
    }
    (non_affine >= 2).then_some(poles)
}

/// A circular meridian would also define a toroidal or spherical support.
/// A nonzero determinant of [1,z,r,z²+r²] excludes concyclicity. Use real
/// source vertices, retaining a conservative perturbation radius in (z,r).
fn excludes_circle(a: &Analysis, nodes: &[Node], frame: &Frame, tolerance: f64) -> bool {
    let count = nodes.len().min(9);
    let samples: Vec<_> = (0..count)
        .map(|i| {
            let node = &nodes[i * (nodes.len() - 1) / (count - 1)];
            let [x, y, z] = frame.project(a.mesh.vertices[node.vertices[0]]);
            [z, x.hypot(y)]
        })
        .collect();
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in &samples {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let extent = (hi[0] - lo[0]).hypot(hi[1] - lo[1]);
    let uncertainty = tolerance / extent;
    if !uncertainty.is_finite() || uncertainty > 0.01 {
        return false;
    }
    // All observed 2D points have norm <=1 after normalization. Perturbing
    // a point by d<=0.01 changes its four-coordinate row norm by <=2.25*d.
    // Every perturbed row has norm <1.8. Multilinearity + Hadamard thus
    // bound the determinant change by 4*2.25*1.8³*d < 53*d.
    let threshold = 54. * uncertainty + 512. * f64::EPSILON;
    let rows: Vec<V> = samples
        .iter()
        .map(|p| {
            let z = (p[0] - lo[0]) / extent;
            let r = (p[1] - lo[1]) / extent;
            [z, r, z * z + r * r]
        })
        .collect();
    for i in 0..count - 3 {
        for j in i + 1..count - 2 {
            for k in j + 1..count - 1 {
                let normal = cross(sub(rows[j], rows[i]), sub(rows[k], rows[i]));
                if rows[k + 1..]
                    .iter()
                    .any(|&p| dot(normal, sub(p, rows[i])).abs() > threshold)
                {
                    return true;
                }
            }
        }
    }
    false
}

struct Profile {
    first: [f64; 4],
    second: Option<(usize, CubicRevolutionSpan)>,
}

fn fit_profile(nodes: &[Node], o: &Options, work: &mut Work) -> Result<Option<Profile>, FabError> {
    if let Some(first) = fit(nodes, o) {
        return Ok(Some(Profile {
            first,
            second: None,
        }));
    }
    if nodes[0].radius == 0. {
        return Ok(None);
    }
    // At least seven complete rings per span, including the measured joint.
    // Trying every eligible source ring also detects non-unique partitions.
    let z0 = nodes[0].z;
    let height = nodes.last().unwrap().z - z0;
    let mut result = None;
    for joint in 6..nodes.len().saturating_sub(6) {
        let knot = (nodes[joint].z - z0) / height;
        if !(0.125..=0.875).contains(&knot) {
            continue;
        }
        work.spend(10 * nodes.len())?;
        let (Some(mut left), Some(mut right)) = (fit(&nodes[..=joint], o), fit(&nodes[joint..], o))
        else {
            continue;
        };
        // Inserting the internal double knot in a degree-three B-spline
        // gives the shared Bernstein control (1-k) left[2] + k right[1].
        // Project those two controls onto this C1 constraint while keeping
        // the witnessed common radius fixed, then recheck every sample.
        let w = 1. - knot;
        let mismatch = w * left[2] + knot * right[1] - left[3];
        let correction = mismatch / (w * w + knot * knot);
        left[2] -= w * correction;
        right[1] -= knot * correction;
        let fits = |span: &[Node], controls: &[f64; 4]| {
            let h = span.last().unwrap().z - span[0].z;
            controls.iter().all(|r| {
                r.is_finite() && *r > 8. * o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm)
            }) && span.iter().all(|n| {
                (radius(controls, (n.z - span[0].z) / h) - n.radius).abs()
                    <= o.cylinder_fit_tolerance_mm
            })
        };
        if !fits(&nodes[..=joint], &left) || !fits(&nodes[joint..], &right) {
            continue;
        }
        if result.is_some() {
            return Ok(None);
        }
        result = Some(Profile {
            first: left,
            second: Some((
                joint,
                CubicRevolutionSpan {
                    start_fraction: knot,
                    next_radii_mm: [right[1], right[2], right[3]],
                },
            )),
        });
    }
    Ok(result)
}

fn spans(f: &SmoothRevolution) -> impl Iterator<Item = (f64, f64, [f64; 4])> + '_ {
    let end = f.second_span.as_ref().map_or(1., |s| s.start_fraction);
    std::iter::once((0., end, f.radii_mm)).chain(f.second_span.iter().map(|s| {
        let [a, b, c] = s.next_radii_mm;
        (s.start_fraction, 1., [f.radii_mm[3], a, b, c])
    }))
}

pub(super) fn volume(f: &SmoothRevolution) -> Result<f64, FabError> {
    if let Some([q1, q2]) = f.axial_controls_mm {
        // Signed volume pi * integral r(t)^2 z'(t) dt. Both factors
        // are quadratic Bernstein polynomials in the affine radial chart.
        let (a, b) = (f.radii_mm[0], f.radii_mm[3]);
        let r2 = [a * a, a * b, b * b];
        let dz = [3. * q1, 3. * (q2 - q1), 3. * (f.height_mm - q2)];
        let binom2 = [1., 2., 1.];
        let binom4 = [1., 4., 6., 4., 1.];
        let mut sum = Sum::default();
        for i in 0..3 {
            for j in 0..3 {
                sum.add(r2[i] * dz[j] * binom2[i] * binom2[j] / (5. * binom4[i + j]));
            }
        }
        let v = PI * sum.get();
        return if v.is_finite() && v > 0. {
            Ok(v)
        } else {
            Err(unproven("volume du retour axial indéterminé"))
        };
    }
    let c3 = [1., 3., 3., 1.];
    let c6 = [1., 6., 15., 20., 15., 6., 1.];
    let mut sum = Sum::default();
    for (lo, hi, controls) in spans(f) {
        for (i, &a) in controls.iter().enumerate() {
            for (j, &b) in controls.iter().enumerate() {
                sum.add((hi - lo) * a * b * c3[i] * c3[j] / (7. * c6[i + j]));
            }
        }
    }
    let v = PI * f.height_mm * sum.get();
    if v.is_finite() && v > 0. {
        Ok(v)
    } else {
        Err(unproven("volume cubique indéterminé"))
    }
}

#[derive(Clone, Copy, Default)]
struct Point {
    theta: f64,
    level: Option<usize>,
    /// Corrected planar coordinates, also defining the cap polygons.
    xy: [f64; 2],
}
fn delta(a: f64, b: f64) -> f64 {
    let d = b - a;
    if d > PI {
        d - TAU
    } else if d < -PI {
        d + TAU
    } else {
        d
    }
}
struct Bound {
    residual: f64,
    curvature: f64,
}
impl Bound {
    fn charge_projection(&mut self, error: f64, tol: f64) -> Result<(), FabError> {
        if !error.is_finite() || error > tol {
            return Err(unproven("sommet hors de sa méridienne ou de son disque"));
        }
        self.residual = self.residual.max(error);
        Ok(())
    }
}

fn ring_points(
    a: &Analysis,
    frame: &Frame,
    nodes: &[Node],
    f: &SmoothRevolution,
    tol: f64,
    bound: &mut Bound,
) -> Result<Vec<Point>, FabError> {
    let mut points = vec![Point::default(); a.mesh.vertices.len()];
    for (level, n) in nodes.iter().enumerate() {
        let t = (n.z - nodes[0].z) / f.height_mm;
        let (lo, hi, controls) = spans(f)
            .find(|(_, hi, _)| t <= *hi)
            .ok_or_else(|| unproven("niveau hors profil"))?;
        let r = radius(&controls, (t - lo) / (hi - lo));
        if n.radius == 0. {
            if level != 0 || n.vertices.len() != 1 || r != 0. {
                return Err(unproven("pôle de révolution ambigu"));
            }
            let v = n.vertices[0];
            let [x, y, z] = frame.project(a.mesh.vertices[v]);
            bound.charge_projection(x.hypot(y).hypot(z - n.z), tol)?;
            points[v] = Point {
                level: Some(0),
                ..Point::default()
            };
            continue;
        }
        for &v in &n.vertices {
            let [x, y, z] = frame.project(a.mesh.vertices[v]);
            let rho = x.hypot(y);
            bound.charge_projection((rho - r).hypot(z - n.z), tol)?;
            if points[v].level.is_some() || rho <= tol {
                return Err(unproven("témoins de niveaux confondus"));
            }
            points[v] = Point {
                theta: y.atan2(x),
                level: Some(level),
                xy: [r * x / rho, r * y / rho],
            };
        }
        // A cycle is an actual set of mesh edges. Its map to the angular
        // parameter is strictly monotonic of degree one, including the seam.
        let mut turn = Sum::default();
        let mut orientation = 0.;
        for (i, &v) in n.vertices.iter().enumerate() {
            let next = n.vertices[(i + 1) % n.vertices.len()];
            let w = delta(points[v].theta, points[next].theta);
            if i == 0 {
                orientation = w.signum();
            }
            if orientation * w <= 1e-10 || w.abs() > PI / 4. + 1e-12 {
                return Err(unproven("anneau angulaire incomplet ou replié"));
            }
            turn.add(w);
            if level == 0 || level + 1 == nodes.len() {
                // Each chord is paired with its full circle arc. Extending
                // radially from the disk center gives the same cap bound.
                bound.curvature = bound.curvature.max(r * w * w / 8.);
            }
        }
        if (turn.get().abs() - TAU).abs() > 1e-8 {
            return Err(unproven("degré angulaire du contour distinct de un"));
        }
    }
    Ok(points)
}

#[derive(Default)]
struct Derivatives {
    r: f64,
    first: f64,
    second: f64,
}
impl Derivatives {
    fn new(b: [f64; 4]) -> Self {
        Self {
            r: b.into_iter().fold(0., f64::max),
            first: 3. * b.windows(2).map(|p| (p[1] - p[0]).abs()).fold(0., f64::max),
            second: 6.
                * b.windows(3)
                    .map(|p| (p[2] - 2. * p[1] + p[0]).abs())
                    .fold(0., f64::max),
        }
    }
    fn triangle(&self, points: [Point; 3], nodes: &[Node], height: f64) -> Result<f64, FabError> {
        let [Some(i), Some(j), Some(k)] = points.map(|p| p.level) else {
            return Err(unproven("sommet de paroi sans anneau source complet"));
        };
        let lo = i.min(j).min(k);
        let hi = i.max(j).max(k);
        if hi != lo + 1 {
            return Err(unproven(
                "triangle hors de la bande entre anneaux consécutifs",
            ));
        }
        let [a, b, c] = points.map(|p| p.theta);
        let (u, v) = (delta(a, b), delta(a, c));
        let w = 0_f64.max(u).max(v) - 0_f64.min(u).min(v);
        if w >= PI / 2. {
            return Err(unproven("triangle traversant une couture ambiguë"));
        }
        // Exactly two vertices lie at the same level. Scaling z by the strip
        // height makes this determinant purely angular and well conditioned.
        let det = u * (k as f64 - i as f64) - v * (j as f64 - i as f64);
        if det <= 1e-10 {
            return Err(unproven("triangle angle-hauteur replié"));
        }
        // All edges on adjacent triangles have the same unique shortest lift.
        // Positive local degree and a single degree-one circle at each end
        // give multiplicity one throughout the annulus (boundary degree).
        let dt = (nodes[hi].z - nodes[lo].z) / height;
        // Second-order barycentric interpolation: Var(t)<=dt²/4,
        // Var(theta)<=w²/4 and |Cov|<=dt*w/4. Bound both surface directions
        // and the mixed Hessian term before adding the 3D vertex residual.
        Ok((self.second * dt * dt + 2. * self.first * dt * w + self.r * w * w) / 8.)
    }
}

fn cap_triangle(
    local: [V; 3],
    points: [Point; 3],
    end: usize,
    level: usize,
    z: f64,
    tol: f64,
    bound: &mut Bound,
) -> Result<(), FabError> {
    let mut xy = [[0.; 2]; 3];
    for i in 0..3 {
        let [x, y, h] = local[i];
        // Cap flatness has its own tolerance even when joint segmentation
        // used the larger circular-fit tolerance. This includes rim vertices.
        bound.charge_projection((h - z).abs(), tol)?;
        match points[i].level {
            Some(n) if n == level => xy[i] = points[i].xy,
            Some(_) => return Err(unproven("disque en contact avec un autre niveau")),
            None => {
                xy[i] = [x, y];
            }
        }
    }
    // The cap's single boundary is the convex cyclic polygon of the
    // corrected rim. Every projected triangle has the outward orientation,
    // hence degree one and no folds or uncovered disk interior.
    let [a, b, c] = xy;
    let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let span = (b[0] - a[0]).hypot(b[1] - a[1]) * (c[0] - a[0]).hypot(c[1] - a[1]);
    let sign = if end == 0 { -1. } else { 1. };
    if sign * det <= 1e-12 * span {
        return Err(unproven("triangle de disque indéterminé ou replié"));
    }
    Ok(())
}

pub(super) fn candidate(
    a: &Analysis,
    s: &Segmentation,
    rims: &[Rim],
    nodes: &[Node],
    frame: &Frame,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let pole = nodes.first().is_some_and(|n| n.radius == 0.);
    if a.mesh.triangles.len() > MAX_TRIANGLES
        || s.model.faces.len() > MAX_REGIONS
        || !(2..=128).contains(&nodes.len())
        || rims.len() != if pole { 1 } else { 2 }
    {
        return Ok(None);
    }
    work.spend(nodes.len() * 8 + rims.iter().map(|r| r.vertices.len()).sum::<usize>())?;
    let last = nodes.len() - 1;
    let mut cap_regions = [usize::MAX; 2];
    for (end, level) in [0, last].into_iter().enumerate().skip(usize::from(pole)) {
        let Some(rim) = rims.iter().find(|r| r.vertices == nodes[level].vertices) else {
            return Ok(None);
        };
        let face = &s.model.faces[rim.region];
        let expected = if end == 0 { -1. } else { 1. };
        if face.loops.len() != 1
            || face.loops[0] != rim.vertices
            || expected * dot(frame.axis, face.normal) < 1. - 1e-10
        {
            return Ok(None);
        }
        cap_regions[end] = rim.region;
    }
    if cap_regions[0] == cap_regions[1] {
        return Ok(None);
    }
    let structured = if nodes.len() >= 5 {
        certify(a, s, nodes, frame, o, work, cap_regions, None)
    } else {
        Ok(None)
    };
    match &structured {
        Ok(Some(_)) => return structured,
        Err(e)
            if !matches!(
                e.code.as_str(),
                "recognition.surface_unproven" | "recognition.deviation_exceeded"
            ) =>
        {
            return structured
        }
        _ => (),
    }
    if pole {
        return structured;
    }
    let Some(samples) = irregular::samples::<17>(a, s, nodes, cap_regions, frame, work)? else {
        return structured;
    };
    certify(a, s, nodes, frame, o, work, cap_regions, Some(&samples))
}

#[allow(clippy::too_many_arguments)]
fn certify(
    a: &Analysis,
    s: &Segmentation,
    nodes: &[Node],
    frame: &Frame,
    o: &Options,
    work: &mut Work,
    cap_regions: [usize; 2],
    samples: Option<&[Node]>,
) -> Result<Option<RoundedRecognition>, FabError> {
    let last = nodes.len() - 1;
    let pole = nodes[0].radius == 0.;
    let dense;
    let mut fitting = samples.unwrap_or(nodes);
    let profile = if samples.is_some() {
        if let Some(first) = fit(fitting, o) {
            Some(Profile {
                first,
                second: None,
            })
        } else if let Some(measured) =
            irregular::samples::<65>(a, s, nodes, cap_regions, frame, work)?
        {
            dense = measured;
            fitting = &dense;
            irregular_joint::fit(fitting, o, work)?
        } else {
            None
        }
    } else {
        fit_profile(nodes, o, work)?
    };
    let Some(profile) = profile else {
        return Ok(None);
    };
    let b = profile.first;
    let origin = frame.lift([0., 0., nodes[0].z]);
    let height = nodes[last].z - nodes[0].z;
    let extent = origin
        .into_iter()
        .chain(b)
        .chain(profile.second.iter().flat_map(|(_, s)| s.next_radii_mm))
        .chain([height])
        .map(f64::abs)
        .fold(1., f64::max);
    let roundoff = 512. * f64::EPSILON * extent;
    if roundoff > o.cylinder_fit_tolerance_mm.min(5e-8) {
        return Err(unproven("précision de placement insuffisante"));
    }
    work.spend(9 * 9 * 9)?;
    if !excludes_circle(a, fitting, frame, o.cylinder_fit_tolerance_mm + roundoff) {
        return Ok(None);
    }
    if let Some((split, _)) = &profile.second {
        work.spend(2 * 9 * 9 * 9)?;
        if !excludes_circle(
            a,
            &fitting[..=*split],
            frame,
            o.cylinder_fit_tolerance_mm + roundoff,
        ) || !excludes_circle(
            a,
            &fitting[*split + usize::from(samples.is_some())..],
            frame,
            o.cylinder_fit_tolerance_mm + roundoff,
        ) {
            return Ok(None);
        }
    }
    let f = SmoothRevolution {
        origin,
        axis: frame.axis,
        height_mm: height,
        radii_mm: b,
        axial_controls_mm: None,
        second_span: profile.second.map(|(_, s)| s),
        pole_vertex: pole.then(|| nodes[0].vertices[0]),
        rings: if pole { [0, 0] } else { [0, 1] },
        witness_tolerance_mm: 0.,
    };
    work.spend(a.mesh.vertices.len() * 4 + a.mesh.triangles.len() * 8)?;
    let mut bound = Bound {
        residual: 0.,
        curvature: 0.,
    };
    let tol = o.cylinder_fit_tolerance_mm;
    let points = ring_points(a, frame, nodes, &f, tol, &mut bound)?;
    let irregular_points = if samples.is_some() {
        Some(irregular::corrected(
            a,
            s,
            cap_regions,
            &points,
            nodes,
            frame,
            &f,
            tol,
            &mut bound,
        )?)
    } else {
        None
    };
    let available =
        o.max_deviation_mm - o.weld_tolerance_mm - bound.residual - roundoff - NATIVE_RESERVE_MM;
    let mut derivatives = Derivatives::default();
    for (lo, hi, controls) in spans(&f) {
        let local = Derivatives::new(controls);
        let width = hi - lo;
        // The common C1 derivative is Lipschitz across the knot with the
        // maximum of the two per-span second derivative bounds in global t.
        derivatives.r = derivatives.r.max(local.r);
        derivatives.first = derivatives.first.max(local.first / width);
        derivatives.second = derivatives.second.max(local.second / width.powi(2));
    }
    // A genuine source fan has one triangle per edge of the next circle.
    // Its boundary is checked without assuming that triangle IDs or ring
    // vertices start at the meridian seam.
    let mut fan_edges = BTreeSet::new();
    let mut owners = Vec::with_capacity(a.mesh.triangles.len());
    for (&ids, &region) in a.mesh.triangles.iter().zip(&s.triangle_regions) {
        let mapped = ids.map(|i| points[i]);
        if let Some(end) = cap_regions.iter().position(|&r| r == region) {
            let level = if end == 0 { 0 } else { last };
            cap_triangle(
                ids.map(|i| frame.project(a.mesh.vertices[i])),
                mapped,
                end,
                level,
                nodes[level].z,
                o.plane_tolerance_mm,
                &mut bound,
            )?;
            owners.push(RoundedFaceRef::Plane {
                index: end - usize::from(pole),
            });
        } else if let Some(i) = f
            .pole_vertex
            .and_then(|v| ids.iter().position(|&id| id == v))
        {
            let (b, c) = ((i + 1) % 3, (i + 2) % 3);
            let w = delta(mapped[b].theta, mapped[c].theta);
            if mapped[b].level != Some(1)
                || mapped[c].level != Some(1)
                || !(-PI / 4. - 1e-12..-1e-10).contains(&w)
                || !fan_edges.insert(edge(ids[b], ids[c]))
            {
                return Err(unproven("éventail du pôle incomplet ou replié"));
            }
            // Pair a point of the first band with the same axial fraction
            // of the flat source fan. Radial interpolation contributes at
            // most |r''| dt²/8; the angular chord adds r(first ring)*w²/8.
            let dt = (nodes[1].z - nodes[0].z) / height;
            let chord = (derivatives.second * dt * dt + radius(&f.radii_mm, dt) * w * w) / 8.;
            bound.curvature = bound.curvature.max(chord);
            owners.push(RoundedFaceRef::SmoothRevolution { index: 0 });
        } else {
            let chord = if let Some(projected) = &irregular_points {
                irregular::triangle(ids.map(|i| projected[i]), &f, available, work)?
            } else {
                derivatives.triangle(mapped, nodes, height)?
            };
            bound.curvature = bound.curvature.max(chord);
            owners.push(RoundedFaceRef::SmoothRevolution { index: 0 });
        }
    }
    if pole && fan_edges != boundary(&nodes[1].vertices) {
        return Err(unproven("le pôle ne couvre pas le premier anneau"));
    }
    let (fitted_message, bound_message) = if irregular_points.is_some() {
        (
            if f.second_span.is_some() {
                "Une méridienne à deux travées cubiques tangentes est ajustée aux sommets de la paroi entre les disques mesurés."
            } else {
                "La méridienne cubique est ajustée aux sommets de la paroi entre les deux disques mesurés."
            },
            "La borne couvre les extrémités et chaque triangle, avec soudage, projection radiale continue et réserve native.",
        )
    } else {
        (
            "Une méridienne cubique ajustée aux anneaux mesurés représente la paroi de révolution.",
            "La borne couvre les extrémités et chaque triangle de la paroi, avec soudage, interpolation angle–hauteur et réserve native.",
        )
    };
    finish(
        a,
        o,
        nodes,
        frame,
        bound,
        roundoff,
        f,
        owners,
        (fitted_message, bound_message),
    )
}

#[allow(clippy::too_many_arguments)]
fn finish(
    a: &Analysis,
    o: &Options,
    nodes: &[Node],
    frame: &Frame,
    bound: Bound,
    roundoff: f64,
    mut f: SmoothRevolution,
    owners: Vec<RoundedFaceRef>,
    (fitted_message, bound_message): (&str, &str),
) -> Result<Option<RoundedRecognition>, FabError> {
    let last = nodes.len() - 1;
    let pole = f.pole_vertex.is_some();
    f.witness_tolerance_mm = bound.residual + roundoff;
    let wall = roundoff + NATIVE_RESERVE_MM;
    let deviation = o.weld_tolerance_mm + bound.residual + bound.curvature + wall;
    if !deviation.is_finite() || deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "La révolution ajustée dépasse le budget d’écart.",
        ));
    }
    let v = volume(&f)?;
    let rings = [0, last]
        .into_iter()
        .skip(usize::from(pole))
        .map(|i| CircularRing {
            center: frame.lift([0., 0., nodes[i].z]),
            axis: frame.axis,
            radius_mm: nodes[i].radius,
        })
        .collect();
    let planes = (usize::from(pole)..2)
        .map(|i| MixedPlane {
            normal: scale(frame.axis, if i == 0 { -1. } else { 1. }),
            loops: vec![MixedLoop::Circle {
                ring: i - usize::from(pole),
            }],
        })
        .collect();
    Ok(Some(RoundedRecognition {
        triangle_faces: owners,
        model: RoundedModel {
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices: a.mesh.vertices.clone(),
                    rings,
                    planes,
                    cylinders: vec![],
                    arcs: vec![],
                    elliptic_arcs: vec![],
                    partial_cylinders: vec![],
                },
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
            partial_cones: vec![],
            octants: vec![],
            linear_blend: None,
            unequal_pocket: None,
            drilled_shaft: None,
            cross_drill: None,
            torus_sectors: vec![],
            smooth_revolution: Some(Box::new(f)),
        },
        report: RecognitionReport {
            cylinders: 0,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            revolutions: 1,
            max_radial_residual_mm: bound.residual,
            max_wall_residual_mm: wall,
            max_chord_error_mm: bound.curvature,
            max_deviation_bound_mm: deviation,
            expected_volume_mm3: v,
            diagnostics: vec![
                Diagnostic {
                    code: "reconstruction.fitted_revolution".into(),
                    severity: Severity::Warning,
                    count: 1,
                    message: fitted_message.into(),
                },
                Diagnostic {
                    code: "recognition.bound_scope".into(),
                    severity: Severity::Info,
                    count: 1,
                    message: bound_message.into(),
                },
            ],
        },
    }))
}
