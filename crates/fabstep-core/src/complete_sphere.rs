//! Complete spheres are a terminal, explicit-mode interpretation. The app
//! tries this only after all historical recognition paths found no candidate.
use crate::math::*;
use crate::recognition::{validate_analysis, Work};
use crate::{
    Analysis, Diagnostic, FabError, MixedModel, MixedSphere, Options, RecognitionReport,
    ReconstructionMode, RevolvedModel, RoundedFaceRef, RoundedModel, RoundedRecognition, Severity,
};
use std::f64::consts::PI;

fn unproven(message: impl std::fmt::Display) -> FabError {
    FabError::new(
        "recognition.surface_unproven",
        format!("Sphère entière non prouvée : {message}. Aucun résultat partiel proposé."),
    )
}

/// Recognize one complete exterior sphere without asserting source poles or
/// boundaries. Call only after historical interpretations returned unsupported;
/// a cospherical set of vertices can also belong to a legitimate trimmed body.
pub fn recognize_complete_sphere(
    a: &Analysis,
    o: &Options,
) -> Result<RoundedRecognition, FabError> {
    if o.reconstruction != ReconstructionMode::Surfaces {
        return Err(FabError::new(
            "recognition.disabled",
            "Activer explicitement le mode Surfaces.",
        ));
    }
    for tolerance in [
        o.weld_tolerance_mm,
        o.plane_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.max_deviation_mm,
    ] {
        if !tolerance.is_finite() || tolerance <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Les tolérances doivent être finies et strictement positives.",
            ));
        }
    }
    validate_analysis(a)?;
    let mut work = Work::default();
    work.spend(a.mesh.triangles.len().saturating_mul(12) + a.mesh.vertices.len())?;
    // This also proves connectedness, opposite edge incidences, and a single
    // disk-like link at every vertex, excluding pinched pseudomanifolds.
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let vertices = a.mesh.vertices.len();
    let triangles = a.mesh.triangles.len();
    let edges = triangles * 3 / 2;
    if vertices as isize - edges as isize + triangles as isize != 2 {
        return Err(unproven(
            "la surface fermée n'a pas la topologie d'une sphère (χ ≠ 2)",
        ));
    }
    // A flat closing disk triangulated using only rim vertices is cospherical
    // too. Preserve demonstrated planar regions. Four-vertex planar quads are
    // allowed: they occur naturally between equal-angle latitude samples.
    let segmentation = crate::planar::build_with_provenance(
        &a.mesh,
        &adjacency,
        o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm),
    )?;
    for plane in &segmentation.model.faces {
        work.spend(plane.loops.iter().map(Vec::len).sum())?;
        if plane.loops.iter().any(|lp| lp.len() >= 5) {
            return Err(unproven(
                "une région plane source interdit de remplacer le corps par une sphère entière",
            ));
        }
    }
    let (center, radius) = fit_sphere(&a.mesh.vertices, o.cylinder_fit_tolerance_mm, &mut work)?;
    if radius <= 4. * o.weld_tolerance_mm.max(o.cylinder_fit_tolerance_mm) {
        return Err(unproven("rayon non résolu par les tolérances actives"));
    }
    let mut radial: f64 = 0.;
    let mut projected = Vec::with_capacity(vertices);
    for &point in &a.mesh.vertices {
        work.spend(1)?;
        let delta = sub(point, center);
        let length = norm(delta);
        if !length.is_finite() || length == 0. {
            return Err(unproven("sommet au centre ou projection non finie"));
        }
        let ideal = scale(delta, radius / length);
        // Use the vector residual of the actual final center/radius, not the
        // algebraic least-squares objective or the pre-unscaling estimate.
        let residual = norm(sub(delta, ideal));
        if residual > o.cylinder_fit_tolerance_mm {
            return Err(unproven(format!(
                "résidu sphérique {residual:.9} mm supérieur à la tolérance"
            )));
        }
        radial = radial.max(residual);
        projected.push(ideal);
    }
    let mut area = Sum::default();
    let mut chord: f64 = 0.;
    for triangle in &a.mesh.triangles {
        work.spend(1)?;
        let points = triangle.map(|v| projected[v]);
        let unit = points.map(|p| scale(p, 1. / radius));
        for i in 0..3 {
            if dot(unit[i], unit[(i + 1) % 3]) < (PI / 4.).cos() {
                return Err(unproven(
                    "triangle sphérique trop peu échantillonné (angle central > 45°)",
                ));
            }
        }
        let determinant = dot(unit[0], cross(unit[1], unit[2]));
        if !determinant.is_finite() || determinant <= 1e-15 {
            return Err(unproven(
                "projection radiale repliée, inversée ou dégénérée",
            ));
        }
        let denominator =
            1. + dot(unit[0], unit[1]) + dot(unit[1], unit[2]) + dot(unit[2], unit[0]);
        area.add(2. * determinant.atan2(denominator));
        chord =
            chord.max(radius - crate::rounded::min_triangle_norm(points[0], points[1], points[2]));
    }
    if (area.get() - 4. * PI).abs() > 1e-6 {
        return Err(unproven(format!(
            "couverture orientée {} stéradians au lieu de 4π",
            area.get()
        )));
    }
    // Radial projection is continuous over this closed manifold, has positive
    // orientation on every triangle and degree one (area 4π). Thus every sphere
    // point is covered once. The exact minimum triangle norm bounds the radial
    // sagitta in both directions; source-to-ideal interpolation adds max fit.
    let bound = o.weld_tolerance_mm + radial + chord;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            format!(
                "Borne composée de la sphère {bound:.9} mm supérieure au budget {:.9} mm.",
                o.max_deviation_mm
            ),
        ));
    }
    let volume = 4. * PI / 3. * radius.powi(3);
    if !volume.is_finite() || volume <= 0. {
        return Err(unproven("volume sphérique non fini ou non positif"));
    }
    Ok(RoundedRecognition {
        model: RoundedModel {
            revolved: RevolvedModel {
                mixed: MixedModel { vertices: a.mesh.vertices.clone(), rings: vec![], planes: vec![], cylinders: vec![], arcs: vec![], partial_cylinders: vec![] },
                cones: vec![],
            },
            spheres: vec![MixedSphere { rings: None, center, radius_mm: radius, inner: false }],
            tori: vec![],
        },
        triangle_faces: vec![RoundedFaceRef::Sphere { index: 0 }; triangles],
        report: RecognitionReport {
            cylinders: 0, cones: 0, spheres: 1, tori: 0, toroidal_fillets: 0,
            max_radial_residual_mm: radial, max_chord_error_mm: chord, max_wall_residual_mm: 0.,
            max_deviation_bound_mm: bound, expected_volume_mm3: volume,
            diagnostics: vec![
                Diagnostic { code: "recognition.rounded_intent_assumed".into(), severity: Severity::Warning, count: 1, message: "Le corps admissible est interprété comme une sphère entière dans le mode explicitement activé.".into() },
                Diagnostic { code: "recognition.rounded_coverage".into(), severity: Severity::Info, count: triangles, message: "Tous les triangles couvrent une sphère entière orientée une seule fois (4π), sans anneau ni pôle source ajouté.".into() },
                Diagnostic { code: "recognition.bound_scope".into(), severity: Severity::Info, count: 1, message: "Borne composée : soudage, résidu de projection et flèche sphérique des triangles. Validation native et audit bidirectionnel requis.".into() },
            ],
        },
    })
}

/// Linear sphere fit in bounded local coordinates. Compensated moments avoid
/// a dependence on absolute translation or accumulation drift. Partial-pivot
/// elimination is bounded to four unknowns; ill-conditioning is a refusal.
fn fit_sphere(points: &[V], tolerance: f64, work: &mut Work) -> Result<(V, f64), FabError> {
    let bbox = bounds(points);
    let origin = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let span = (0..3).map(|i| bbox[1][i] - bbox[0][i]).fold(0., f64::max);
    let world = points.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if !span.is_finite() || span <= 0. || world * f64::EPSILON * 32. > tolerance {
        return Err(unproven(
            "échelle ou translation insuffisamment résolue en f64",
        ));
    }
    let mut moments: [[Sum; 5]; 4] =
        std::array::from_fn(|_| std::array::from_fn(|_| Sum::default()));
    for &point in points {
        work.spend(20)?;
        let q = scale(sub(point, origin), 1. / span);
        let row = [2. * q[0], 2. * q[1], 2. * q[2], 1.];
        let square = dot(q, q);
        for i in 0..4 {
            for j in 0..4 {
                moments[i][j].add(row[i] * row[j]);
            }
            moments[i][4].add(row[i] * square);
        }
    }
    let mut system: [[f64; 5]; 4] =
        std::array::from_fn(|i| std::array::from_fn(|j| moments[i][j].get() / points.len() as f64));
    for column in 0..4 {
        let pivot = (column..4)
            .max_by(|&i, &j| system[i][column].abs().total_cmp(&system[j][column].abs()))
            .unwrap();
        if system[pivot][column].abs() < 1e-10 {
            return Err(unproven(
                "ajustement sphérique singulier ou mal conditionné",
            ));
        }
        system.swap(pivot, column);
        let divisor = system[column][column];
        for j in column..5 {
            system[column][j] /= divisor;
        }
        for i in 0..4 {
            if i != column {
                let multiplier = system[i][column];
                for j in column..5 {
                    system[i][j] -= multiplier * system[column][j];
                }
            }
        }
    }
    let local = [system[0][4], system[1][4], system[2][4]];
    let radius = (dot(local, local) + system[3][4]).sqrt() * span;
    let center = std::array::from_fn(|i| origin[i] + span * local[i]);
    if !radius.is_finite()
        || radius <= 0.
        || center.iter().any(|x| !x.is_finite())
        || radius * f64::EPSILON * 128. > tolerance
    {
        return Err(unproven(
            "paramètres sphériques non finis ou précision insuffisante",
        ));
    }
    Ok((center, radius))
}
