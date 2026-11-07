//! Isolated uniformly rounded cuboids, anchored by six actual rectangular
//! source regions. No bounding-box-only fit and no SDF-only acceptance.
use crate::math::*;
use crate::recognition::{validate_analysis, Work};
use crate::{
    Analysis, Diagnostic, FabError, Options, RecognitionReport, ReconstructionMode,
    RoundedBoxFaceRef, RoundedBoxModel, RoundedBoxRecognition, Severity,
};
use std::f64::consts::PI;

fn unsupported() -> FabError {
    FabError::new(
        "recognition.unsupported",
        "Aucun ensemble de six supports rectangulaires dominants pour un boîtier arrondi.",
    )
}
fn unproven(detail: impl std::fmt::Display) -> FabError {
    FabError::new(
        "recognition.surface_unproven",
        format!("Boîtier arrondi non prouvé : {detail}. Aucun résultat partiel proposé."),
    )
}
fn sign(positive: bool) -> f64 {
    if positive {
        1.
    } else {
        -1.
    }
}
fn transverse(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}
fn face(index: usize) -> RoundedBoxFaceRef {
    if index < 6 {
        RoundedBoxFaceRef::Plane {
            axis: index / 2,
            positive: index % 2 == 1,
        }
    } else if index < 18 {
        let i = index - 6;
        RoundedBoxFaceRef::Cylinder {
            axis: i / 4,
            positive: [(i % 4) & 1 != 0, (i % 4) & 2 != 0],
        }
    } else {
        let i = index - 18;
        RoundedBoxFaceRef::Sphere {
            positive: [i & 1 != 0, i & 2 != 0, i & 4 != 0],
        }
    }
}
fn project(p: V, center: V, axes: [V; 3]) -> V {
    let q = sub(p, center);
    axes.map(|axis| dot(q, axis))
}
fn lift(p: V, center: V, axes: [V; 3]) -> V {
    std::array::from_fn(|i| center[i] + axes[0][i] * p[0] + axes[1][i] * p[1] + axes[2][i] * p[2])
}

pub fn recognize_rounded_box(a: &Analysis, o: &Options) -> Result<RoundedBoxRecognition, FabError> {
    if o.reconstruction != ReconstructionMode::Surfaces {
        return Err(FabError::new(
            "recognition.disabled",
            "Activer explicitement le mode Surfaces.",
        ));
    }
    for t in [
        o.weld_tolerance_mm,
        o.plane_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.max_deviation_mm,
    ] {
        if !t.is_finite() || t <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    validate_analysis(a)?;
    let mut work = Work::default();
    work.spend(a.mesh.triangles.len().saturating_mul(12) + a.mesh.vertices.len())?;
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let tol = o.cylinder_fit_tolerance_mm;
    let segmentation =
        crate::planar::build_with_provenance(&a.mesh, &adjacency, o.plane_tolerance_mm.max(tol))?;
    // Top seven only: no combinatorial search through arbitrary plane sets.
    // A separated six-plane hypothesis is only a seed; all subsequent geometry
    // and all source triangles must independently prove the rounded cuboid.
    let mut largest: Vec<(f64, usize)> = Vec::new();
    for (index, region) in segmentation.model.faces.iter().enumerate() {
        let mut area = Sum::default();
        let origin = a.mesh.vertices[region.loops[0][0]];
        for lp in &region.loops {
            work.spend(lp.len().saturating_mul(7))?;
            for i in 0..lp.len() {
                area.add(
                    dot(
                        cross(
                            sub(a.mesh.vertices[lp[i]], origin),
                            sub(a.mesh.vertices[lp[(i + 1) % lp.len()]], origin),
                        ),
                        region.normal,
                    ) * 0.5,
                );
            }
        }
        largest.push((area.get(), index));
        largest.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
        largest.truncate(7);
    }
    if largest.len() < 6
        || largest[5].0 <= 0.
        || (largest.len() == 7 && largest[5].0 < 4. * largest[6].0)
    {
        return Err(unsupported());
    }
    if a.mesh.vertices.len() as isize - (a.mesh.triangles.len() * 3 / 2) as isize
        + a.mesh.triangles.len() as isize
        != 2
    {
        return Err(unproven(
            "surface fermée de topologie différente d'une sphère (χ ≠ 2)",
        ));
    }
    let chosen: Vec<_> = largest[..6]
        .iter()
        .map(|&(_, i)| &segmentation.model.faces[i])
        .collect();
    if chosen.iter().any(|p| p.loops.len() != 1) {
        return Err(unproven("support plan percé ou multiple"));
    }
    let x = chosen[0].normal;
    let y0 = chosen
        .iter()
        .map(|p| p.normal)
        .min_by(|u, v| dot(*u, x).abs().total_cmp(&dot(*v, x).abs()))
        .unwrap();
    let y = sub(y0, scale(x, dot(y0, x)));
    if norm(y) < 0.5 {
        return Err(unproven("normales des supports non orthogonales"));
    }
    let y = scale(y, 1. / norm(y));
    let axes = [x, y, cross(x, y)];
    let bounds = bounds(&a.mesh.vertices);
    // This anchor only conditions arithmetic; no extent is inferred from it.
    let anchor = std::array::from_fn(|i| bounds[0][i] + (bounds[1][i] - bounds[0][i]) * 0.5);
    let magnitude = bounds.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if magnitude * f64::EPSILON * 32. > tol {
        return Err(unproven(
            "translation trop grande pour résoudre les tolérances",
        ));
    }
    let mut supports: [[Option<f64>; 2]; 3] = [[None; 2]; 3];
    let mut assignments = Vec::new();
    for region in &chosen {
        let dots = axes.map(|axis| dot(axis, region.normal));
        let axis = (0..3)
            .max_by(|&i, &j| dots[i].abs().total_cmp(&dots[j].abs()))
            .unwrap();
        if dots[axis].abs() < 0.999 {
            return Err(unproven("normales des six supports incompatibles"));
        }
        let positive = dots[axis] > 0.;
        let slot = &mut supports[axis][usize::from(positive)];
        if slot.is_some() {
            return Err(unproven("supports opposés manquants ou répétés"));
        }
        let lp = &region.loops[0];
        let mut sum = Sum::default();
        work.spend(lp.len())?;
        for &v in lp {
            sum.add(project(a.mesh.vertices[v], anchor, axes)[axis]);
        }
        *slot = Some(sum.get() / lp.len() as f64);
        assignments.push((axis, positive));
    }
    if supports.iter().flatten().any(Option::is_none) {
        return Err(unproven("six plans support non appariés"));
    }
    let halves: V =
        std::array::from_fn(|i| (supports[i][1].unwrap() - supports[i][0].unwrap()) * 0.5);
    let local_center: V =
        std::array::from_fn(|i| (supports[i][1].unwrap() + supports[i][0].unwrap()) * 0.5);
    let center = lift(local_center, anchor, axes);
    let mut radius = Sum::default();
    for (region, &(axis, _)) in chosen.iter().zip(&assignments) {
        work.spend(region.loops[0].len())?;
        let points: Vec<_> = region.loops[0]
            .iter()
            .map(|&v| project(a.mesh.vertices[v], center, axes))
            .collect();
        let limits = bounds_of(&points);
        for other in transverse(axis) {
            radius.add(halves[other] - (limits[1][other] - limits[0][other]) * 0.5);
        }
    }
    let radius = radius.get() / 12.;
    let core = halves.map(|h| h - radius);
    let resolution = o.weld_tolerance_mm.max(o.plane_tolerance_mm).max(tol) * 8.;
    if !radius.is_finite()
        || radius <= resolution
        || core.iter().any(|&h| !h.is_finite() || h <= resolution)
    {
        return Err(unproven("rayon ou dimensions du noyau non résolus"));
    }
    // All six loops must be the actual rectangular source boundaries, with
    // all four corners witnessed. No synthetic rectangle closes a missing face.
    for (region, &(axis, positive)) in chosen.iter().zip(&assignments) {
        let others = transverse(axis);
        let mut corners = [false; 4];
        work.spend(region.loops[0].len().saturating_mul(4))?;
        for &v in &region.loops[0] {
            let p = project(a.mesh.vertices[v], center, axes);
            let mut ideal = p;
            ideal[axis] = sign(positive) * halves[axis];
            let mut on_border = false;
            for k in others {
                if (p[k].abs() - core[k]).abs() <= tol {
                    ideal[k] = p[k].signum() * core[k];
                    on_border = true;
                }
                if p[k].abs() > core[k] + tol {
                    return Err(unproven("contour plan incompatible avec le retrait commun"));
                }
            }
            if !on_border || norm(sub(a.mesh.vertices[v], lift(ideal, center, axes))) > tol {
                return Err(unproven(
                    "frontière source non rectangulaire ou rayon variable",
                ));
            }
            for (i, witness) in corners.iter_mut().enumerate() {
                if others
                    .iter()
                    .enumerate()
                    .all(|(j, &k)| (p[k] - sign(i & (1 << j) != 0) * core[k]).abs() <= tol)
                {
                    *witness = true;
                }
            }
        }
        if corners.iter().any(|&b| !b) {
            return Err(unproven("coin d'un rectangle support absent de la source"));
        }
    }
    let mut projected = Vec::with_capacity(a.mesh.vertices.len());
    let mut residual: f64 = 0.;
    for &source in &a.mesh.vertices {
        work.spend(4)?;
        let mut q = project(source, center, axes);
        for i in 0..3 {
            if (q[i].abs() - core[i]).abs() <= tol {
                q[i] = q[i].signum() * core[i];
            }
        }
        let foot: V = std::array::from_fn(|i| q[i].clamp(-core[i], core[i]));
        let delta = sub(q, foot);
        let length = norm(delta);
        if length <= 0. {
            return Err(unproven("sommet à l'intérieur du noyau"));
        }
        let ideal: V = std::array::from_fn(|i| foot[i] + radius * delta[i] / length);
        let error = norm(sub(source, lift(ideal, center, axes)));
        if !error.is_finite() || error > tol {
            return Err(unproven(format!(
                "résidu vectoriel {error:.9} mm supérieur à la tolérance"
            )));
        }
        residual = residual.max(error);
        projected.push(ideal);
    }
    prove(
        a, o, center, axes, core, radius, projected, residual, &mut work,
    )
}
fn bounds_of(points: &[V]) -> [V; 2] {
    bounds(points)
}

fn contains(p: V, f: RoundedBoxFaceRef, b: V, r: f64) -> bool {
    match f {
        RoundedBoxFaceRef::Plane { axis, positive } => {
            p[axis] == sign(positive) * (b[axis] + r)
                && transverse(axis).into_iter().all(|i| p[i].abs() <= b[i])
        }
        RoundedBoxFaceRef::Cylinder { axis, positive } => {
            p[axis].abs() <= b[axis]
                && transverse(axis)
                    .into_iter()
                    .zip(positive)
                    .all(|(i, s)| sign(s) * p[i] >= b[i])
        }
        RoundedBoxFaceRef::Sphere { positive } => (0..3).all(|i| sign(positive[i]) * p[i] >= b[i]),
    }
}
fn outward(p: V, f: RoundedBoxFaceRef, b: V) -> V {
    match f {
        RoundedBoxFaceRef::Plane { axis, positive } => {
            let mut n = [0.; 3];
            n[axis] = sign(positive);
            n
        }
        RoundedBoxFaceRef::Cylinder { axis, positive } => {
            let mut n = [0.; 3];
            for (i, s) in transverse(axis).into_iter().zip(positive) {
                n[i] = p[i] - sign(s) * b[i];
            }
            n
        }
        RoundedBoxFaceRef::Sphere { positive } => {
            std::array::from_fn(|i| p[i] - sign(positive[i]) * b[i])
        }
    }
}
// P(q)=clamp_B(q)+R*(q-clamp_B(q))/|q-clamp_B(q)| is continuous and
// agrees on every shared seam. Each ideal triangle stays in one convex
// coordinate domain. P is homotopic to identity outside B, which contains
// the origin. The radial degree of the closed ideal mesh therefore proves
// coverage of ALL of the convex rounded-box boundary, not just its vertices.
#[allow(clippy::too_many_arguments)]
fn prove(
    a: &Analysis,
    o: &Options,
    center: V,
    axes: [V; 3],
    core: V,
    radius: f64,
    points: Vec<V>,
    residual: f64,
    work: &mut Work,
) -> Result<RoundedBoxRecognition, FabError> {
    let mut owners = Vec::with_capacity(a.mesh.triangles.len());
    let mut counts = [0usize; 26];
    let mut witnesses = [0usize; 26];
    let mut vertex_masks = vec![0u32; points.len()];
    let mut area = Sum::default();
    let mut chord: f64 = 0.;
    for triangle in &a.mesh.triangles {
        work.spend(90)?;
        let q = triangle.map(|v| points[v]);
        let mut selected = None;
        for i in 0..26 {
            if q.into_iter().all(|p| contains(p, face(i), core, radius)) {
                if selected.is_some() {
                    return Err(unproven("triangle ambigu sur plusieurs domaines"));
                }
                selected = Some(i);
            }
        }
        let index = selected
            .ok_or_else(|| unproven("triangle franchissant une couture sans frontière source"))?;
        let patch = face(index);
        counts[index] += 1;
        owners.push(patch);
        let normal = cross(sub(q[1], q[0]), sub(q[2], q[0]));
        let local = q.map(|p| outward(p, patch, core));
        // These are the exact signs of the planar, cylindrical and spherical
        // projection Jacobians. For cylinders the expression is affine in q,
        // so positivity at all three vertices proves positivity everywhere.
        if local.into_iter().any(|n| dot(normal, n) <= 0.) {
            return Err(unproven("projection locale repliée ou inversée"));
        }
        if !matches!(patch, RoundedBoxFaceRef::Plane { .. }) {
            let unit = local.map(|v| scale(v, 1. / norm(v)));
            for i in 0..3 {
                if dot(unit[i], unit[(i + 1) % 3]) < (PI / 4.).cos() {
                    return Err(unproven(
                        "courbure insuffisamment échantillonnée (pas >45°)",
                    ));
                }
            }
            chord =
                chord.max(radius - crate::rounded::min_triangle_norm(local[0], local[1], local[2]));
            for (&v, n) in triangle.iter().zip(local) {
                let curved_interior = match patch {
                    RoundedBoxFaceRef::Cylinder { axis, .. } => transverse(axis)
                        .into_iter()
                        .all(|i| n[i].abs() > 4. * o.cylinder_fit_tolerance_mm),
                    RoundedBoxFaceRef::Sphere { .. } => n
                        .into_iter()
                        .all(|x| x.abs() > 4. * o.cylinder_fit_tolerance_mm),
                    _ => false,
                };
                if curved_interior && vertex_masks[v] & (1 << index) == 0 {
                    vertex_masks[v] |= 1 << index;
                    witnesses[index] += 1;
                }
            }
        }
        let unit = q.map(|p| scale(p, 1. / norm(p)));
        let determinant = dot(unit[0], cross(unit[1], unit[2]));
        if !determinant.is_finite() || determinant <= 1e-15 {
            return Err(unproven("projection radiale globale repliée ou dégénérée"));
        }
        area.add(
            2. * determinant
                .atan2(1. + dot(unit[0], unit[1]) + dot(unit[1], unit[2]) + dot(unit[2], unit[0])),
        );
    }
    if counts.contains(&0)
        || witnesses[6..18].iter().any(|&n| n < 8)
        || witnesses[18..].iter().any(|&n| n < 6)
    {
        return Err(unproven(
            "les 26 domaines et leurs courbures intérieures ne sont pas tous prouvés",
        ));
    }
    if (area.get() - 4. * PI).abs() > 1e-6 {
        return Err(unproven(format!(
            "couverture orientée {} au lieu de 4π",
            area.get()
        )));
    }
    let bound = o.weld_tolerance_mm + residual + chord;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            format!(
                "Borne du boîtier arrondi {bound:.9} mm supérieure au budget {:.9} mm.",
                o.max_deviation_mm
            ),
        ));
    }
    let [x, y, z] = core.map(|v| v * 2.);
    let volume = x * y * z
        + 2. * radius * (x * y + x * z + y * z)
        + PI * radius * radius * (x + y + z)
        + 4. * PI / 3. * radius.powi(3);
    if !volume.is_finite() || volume <= 0. {
        return Err(unproven("volume analytique non positif ou non fini"));
    }
    Ok(RoundedBoxRecognition {model:RoundedBoxModel{vertices:a.mesh.vertices.clone(),center,axes,core_half_extents:core,radius_mm:radius},triangle_faces:owners,
        report:RecognitionReport{cylinders:12,cones:0,spheres:8,tori:0,toroidal_fillets:0,max_radial_residual_mm:residual,max_chord_error_mm:chord,max_wall_residual_mm:0.,max_deviation_bound_mm:bound,expected_volume_mm3:volume,
        diagnostics:vec![Diagnostic{code:"recognition.rounded_box_intent_assumed".into(),severity:Severity::Warning,count:1,message:"Les six supports rectangulaires et leurs raccords sont interprétés comme un boîtier convexe à rayon uniforme dans le mode explicitement activé.".into()},Diagnostic{code:"recognition.rounded_box_coverage".into(),severity:Severity::Info,count:a.mesh.triangles.len(),message:"Tous les triangles sont attribués aux 6 plans, 12 quarts de cylindre et 8 octants sphériques ; coutures communes, orientation et couverture de degré un prouvées.".into()},Diagnostic{code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,message:"Borne composée : soudage, déplacements vectoriels des sommets et flèche exacte par domaine. Validation native et audit bidirectionnel requis.".into()}]}})
}
