//! Complete ring tori: a bounded terminal interpretation of a closed genus-one
//! source. Fitting supplies only a candidate; every vertex and triangle must
//! subsequently prove the oriented, degree-one periodic parameter covering.
use crate::math::*;
use crate::recognition::{validate_analysis, Work};
use crate::{
    Analysis, Diagnostic, FabError, MixedModel, MixedTorus, Options, RecognitionReport,
    ReconstructionMode, RevolvedModel, RoundedFaceRef, RoundedModel, RoundedRecognition, Severity,
};
use std::f64::consts::{PI, TAU};

const TERMS: usize = 13;

fn unproven(detail: impl std::fmt::Display) -> FabError {
    FabError::new(
        "recognition.surface_unproven",
        format!("Tore entier non prouvé : {detail}. Aucun résultat partiel proposé."),
    )
}

#[derive(Clone, Copy)]
struct Torus {
    center: V,
    axis: V,
    major: f64,
    minor: f64,
}

/// Called only after the historical rounded/conical/cylindrical paths found
/// no candidate. A different genus is unsupported; a failed genus-one proof
/// is blocking and must never be retried as a convenient different primitive.
pub fn recognize_complete_torus(a: &Analysis, o: &Options) -> Result<RoundedRecognition, FabError> {
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
    let triangle_count = a.mesh.triangles.len();
    work.spend(triangle_count.saturating_mul(12) + a.mesh.vertices.len())?;
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let edge_count = triangle_count * 3 / 2;
    if a.mesh.vertices.len() as isize - edge_count as isize + triangle_count as isize != 0 {
        return Err(FabError::new(
            "recognition.unsupported",
            "Le corps fermé n'a pas la topologie d'un tore entier (χ ≠ 0).",
        ));
    }
    let segmentation = crate::planar::build_with_provenance(
        &a.mesh,
        &adjacency,
        o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm),
    )?;
    for plane in &segmentation.model.faces {
        work.spend(plane.loops.iter().map(Vec::len).sum())?;
        if plane.loops.iter().any(|lp| lp.len() >= 5) {
            return Err(unproven(
                "une région plane source interdit de remplacer le corps par un tore entier",
            ));
        }
    }
    let torus = fit_torus(&a.mesh.vertices, o.cylinder_fit_tolerance_mm, &mut work)?;
    let resolution = o.weld_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    if torus.minor <= 4. * resolution || torus.major - torus.minor <= 4. * resolution {
        return Err(unproven(
            "rayon du tube ou ouverture centrale non résolu ; tore annulaire R > r requis",
        ));
    }
    let helper_index = (0..3)
        .min_by(|&i, &j| torus.axis[i].abs().total_cmp(&torus.axis[j].abs()))
        .unwrap();
    let mut helper = [0.; 3];
    helper[helper_index] = 1.;
    let x = cross(helper, torus.axis);
    let x = scale(x, 1. / norm(x));
    let y = cross(torus.axis, x);
    let mut parameters = Vec::with_capacity(a.mesh.vertices.len());
    let mut residual: f64 = 0.;
    for &point in &a.mesh.vertices {
        work.spend(1)?;
        let delta = sub(point, torus.center);
        let q = [dot(delta, x), dot(delta, y), dot(delta, torus.axis)];
        let rho = q[0].hypot(q[1]);
        if rho <= resolution || (rho - torus.major).hypot(q[2]) <= resolution {
            return Err(unproven(
                "projection torique indéterminée sur l'axe ou le cercle central",
            ));
        }
        let u = q[1].atan2(q[0]);
        let v = q[2].atan2(rho - torus.major);
        let radius = torus.major + torus.minor * v.cos();
        let ideal = [radius * u.cos(), radius * u.sin(), torus.minor * v.sin()];
        let distance = norm(sub(q, ideal));
        if !distance.is_finite() || distance > o.cylinder_fit_tolerance_mm {
            return Err(unproven(format!(
                "résidu torique {distance:.9} mm supérieur à la tolérance"
            )));
        }
        residual = residual.max(distance);
        parameters.push([u, v]);
    }
    let mut area = Sum::default();
    let mut chord: f64 = 0.;
    for triangle in &a.mesh.triangles {
        work.spend(1)?;
        let mut uv = triangle.map(|i| parameters[i]);
        // Each edge has a unique short lift in both periodic coordinates.
        // Consequently neighboring triangles agree modulo 2π on their shared
        // edge; the affine local maps form a continuous map to the UV torus.
        for i in 1..3 {
            for j in 0..2 {
                let difference = uv[i][j] - uv[0][j];
                uv[i][j] = uv[0][j] + difference.sin().atan2(difference.cos());
            }
        }
        let span: [f64; 2] = std::array::from_fn(|i| {
            uv.iter().map(|p| p[i]).fold(f64::NEG_INFINITY, f64::max)
                - uv.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min)
        });
        if span.iter().any(|d| !d.is_finite() || *d > PI / 4. + 1e-12) {
            return Err(unproven(
                "triangle torique trop peu échantillonné (pas UV > 45°)",
            ));
        }
        let determinant = (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1])
            - (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]);
        if !determinant.is_finite() || determinant <= 1e-14 {
            return Err(unproven(
                "projection UV torique repliée, inversée ou dégénérée",
            ));
        }
        area.add(determinant * 0.5);
        let [du, dv] = span;
        // ||S_uu||≤R+r, ||S_uv||≤r, ||S_vv||≤r. Taylor expansion around
        // the barycentric UV point and Var≤range²/4 bound all triangle
        // interiors, not just edges. The same affine UV parameter covers the
        // corresponding analytic image, so this estimate is bidirectional.
        chord = chord.max(
            ((torus.major + torus.minor) * du * du
                + 2. * torus.minor * du * dv
                + torus.minor * dv * dv)
                / 8.,
        );
    }
    // A continuous map of closed oriented surfaces, positive on every face,
    // with area one fundamental UV domain has degree one: no missing region
    // or multiple winding is accepted. χ=0 alone would not establish this.
    if (area.get() - TAU * TAU).abs() > TAU * TAU * 1e-7 + 1e-9 {
        return Err(unproven(format!(
            "couverture UV {} différente de (2π)²",
            area.get()
        )));
    }
    let bound = o.weld_tolerance_mm + residual + chord;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            format!(
                "Borne composée du tore {bound:.9} mm supérieure au budget {:.9} mm.",
                o.max_deviation_mm
            ),
        ));
    }
    let volume = 2. * PI * PI * torus.major * torus.minor * torus.minor;
    if !volume.is_finite() || volume <= 0. {
        return Err(unproven("volume torique non fini ou non positif"));
    }
    Ok(RoundedRecognition {
        model: RoundedModel {
            revolved: RevolvedModel { mixed: MixedModel { vertices: a.mesh.vertices.clone(), rings: vec![], planes: vec![], cylinders: vec![], arcs: vec![], partial_cylinders: vec![] }, cones: vec![] },
            spheres: vec![],
            tori: vec![MixedTorus { rings: None, axis: Some(torus.axis), center: torus.center, major_radius_mm: torus.major, minor_radius_mm: torus.minor, meridian_rad: [0., TAU], inner: false }],
        },
        triangle_faces: vec![RoundedFaceRef::Torus { index: 0 }; triangle_count],
        report: RecognitionReport { cylinders: 0, cones: 0, spheres: 0, tori: 1, toroidal_fillets: 0, max_radial_residual_mm: residual, max_chord_error_mm: chord, max_wall_residual_mm: 0., max_deviation_bound_mm: bound, expected_volume_mm3: volume, diagnostics: vec![
            Diagnostic { code: "recognition.rounded_intent_assumed".into(), severity: Severity::Warning, count: 1, message: "Le corps admissible est interprété comme un tore entier dans le mode explicitement activé.".into() },
            Diagnostic { code: "recognition.rounded_coverage".into(), severity: Severity::Info, count: triangle_count, message: "Tous les triangles couvrent une fois le domaine torique périodique (2π)², sans anneau source ajouté.".into() },
            Diagnostic { code: "recognition.bound_scope".into(), severity: Severity::Info, count: 1, message: "Borne composée : soudage, résidu de projection et borne paramétrique des triangles. Validation native et audit bidirectionnel requis.".into() },
        ] },
    })
}

/// The structured quartic of a circular ring torus is
/// (|q-c|²+R²-r²)² - 4R²(|q-c|²-(a·(q-c))²)=0.
/// Its leading term |q|⁴ is known. Thirteen remaining coefficients are linear
/// unknowns, regardless of the axis or the distribution of source samples.
/// This is not a PCA assumption and the algebraic residual is NEVER the fit
/// acceptance predicate: final physical parameters must pass all proofs.
fn fit_torus(points: &[V], tolerance: f64, work: &mut Work) -> Result<Torus, FabError> {
    let bbox = bounds(points);
    let origin = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let span = (0..3).map(|i| bbox[1][i] - bbox[0][i]).fold(0., f64::max);
    let world = points.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if !span.is_finite() || span <= 0. || world * f64::EPSILON * 32. > tolerance {
        return Err(unproven(
            "échelle ou translation insuffisamment résolue en f64",
        ));
    }
    let mut moments: [[Sum; TERMS + 1]; TERMS] =
        std::array::from_fn(|_| std::array::from_fn(|_| Sum::default()));
    for &point in points {
        work.spend(TERMS * (TERMS + 1) / 2 + TERMS)?;
        let [x, y, z] = scale(sub(point, origin), 1. / span);
        let s = x * x + y * y + z * z;
        let row = [
            x * s,
            y * s,
            z * s,
            x * x,
            y * y,
            z * z,
            2. * x * y,
            2. * x * z,
            2. * y * z,
            x,
            y,
            z,
            1.,
        ];
        for i in 0..TERMS {
            for j in i..TERMS {
                moments[i][j].add(row[i] * row[j]);
            }
            moments[i][TERMS].add(-row[i] * s * s);
        }
    }
    let mut system: [[f64; TERMS + 1]; TERMS] = std::array::from_fn(|i| {
        std::array::from_fn(|j| if j < i { moments[j][i].get() } else { moments[i][j].get() } / points.len() as f64)
    });
    work.spend(TERMS * TERMS * (TERMS + 1))?;
    for column in 0..TERMS {
        let pivot = (column..TERMS)
            .max_by(|&i, &j| system[i][column].abs().total_cmp(&system[j][column].abs()))
            .unwrap();
        if !system[pivot][column].is_finite() || system[pivot][column].abs() < 1e-12 {
            return Err(unproven(
                "ajustement quartique singulier ou mal conditionné",
            ));
        }
        system.swap(pivot, column);
        let divisor = system[column][column];
        for j in column..=TERMS {
            system[column][j] /= divisor;
        }
        for i in 0..TERMS {
            if i != column {
                let multiplier = system[i][column];
                for j in column..=TERMS {
                    system[i][j] -= multiplier * system[column][j];
                }
            }
        }
    }
    let coefficients: [f64; TERMS] = std::array::from_fn(|i| system[i][TERMS]);
    let local = [
        -coefficients[0] / 4.,
        -coefficients[1] / 4.,
        -coefficients[2] / 4.,
    ];
    let mut quadratic = [
        [coefficients[3], coefficients[6], coefficients[7]],
        [coefficients[6], coefficients[4], coefficients[8]],
        [coefficients[7], coefficients[8], coefficients[5]],
    ];
    for i in 0..3 {
        for j in 0..3 {
            quadratic[i][j] -= 4. * local[i] * local[j];
        }
    }
    // Q-4ccᵀ has two equal eigenvalues s=2|c|²-2(R²+r²),
    // and one eigenvalue s+4R² along the torus axis.
    let (values, vectors) = symmetric_eigen(quadratic, work)?;
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| values[i].total_cmp(&values[j]));
    let low = (values[order[0]] + values[order[1]]) * 0.5;
    let major_squared = (values[order[2]] - low) / 4.;
    let minor_squared = dot(local, local) - low * 0.5 - major_squared;
    let major = major_squared.sqrt() * span;
    let minor = minor_squared.sqrt() * span;
    let center: V = std::array::from_fn(|i| origin[i] + span * local[i]);
    let axis = std::array::from_fn(|i| vectors[i][order[2]]);
    let mut axis = scale(axis, 1. / norm(axis));
    let dominant = (0..3)
        .max_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
        .unwrap();
    if axis[dominant] < 0. {
        axis = scale(axis, -1.);
    }
    if !major.is_finite()
        || !minor.is_finite()
        || minor <= 0.
        || major <= minor
        || center.iter().chain(&axis).any(|x| !x.is_finite())
        || major * f64::EPSILON * 128. > tolerance
    {
        return Err(unproven(
            "paramètres non finis ou incompatibles avec un tore annulaire R > r",
        ));
    }
    refine_torus(
        points,
        Torus {
            center,
            axis,
            major,
            minor,
        },
        span,
        tolerance,
        work,
    )
}

/// Deterministic Jacobi rotations of a 3×3 symmetric matrix, with a fixed
/// iteration cap. A failure to diagonalize is a refusal, never a guessed axis.
fn symmetric_eigen(
    mut matrix: [[f64; 3]; 3],
    work: &mut Work,
) -> Result<(V, [[f64; 3]; 3]), FabError> {
    let mut vectors = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    if matrix.iter().flatten().any(|x| !x.is_finite()) {
        return Err(unproven("matrice quadratique non finie"));
    }
    let scale = matrix.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    for _ in 0..32 {
        work.spend(60)?;
        let (p, q) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|&(i, j), &(k, l)| matrix[i][j].abs().total_cmp(&matrix[k][l].abs()))
            .unwrap();
        if matrix[p][q].abs() <= scale * f64::EPSILON * 16. {
            return Ok(([matrix[0][0], matrix[1][1], matrix[2][2]], vectors));
        }
        let angle = 0.5 * (2. * matrix[p][q]).atan2(matrix[q][q] - matrix[p][p]);
        let (sine, cosine) = angle.sin_cos();
        for i in 0..3 {
            let a = matrix[i][p];
            let b = matrix[i][q];
            matrix[i][p] = cosine * a - sine * b;
            matrix[i][q] = sine * a + cosine * b;
            let a = vectors[i][p];
            let b = vectors[i][q];
            vectors[i][p] = cosine * a - sine * b;
            vectors[i][q] = sine * a + cosine * b;
        }
        for j in 0..3 {
            let a = matrix[p][j];
            let b = matrix[q][j];
            matrix[p][j] = cosine * a - sine * b;
            matrix[q][j] = sine * a + cosine * b;
        }
    }
    Err(unproven("diagonalisation de l'axe torique non convergente"))
}

/// A quartic fit can bias f32 radii even when all physical residuals already
/// satisfy the fit tolerance. Refine whenever its geometric RMS is resolved
/// above roundoff, using seven increments (center, two axis tangents, R and r).
/// A previously admissible maximum residual must remain admissible; reducing
/// RMS never licenses sacrificing a source vertex to improve nominal volume.
/// No random starts, sample selection, or tolerance adaptation is performed.
fn refine_torus(
    points: &[V],
    mut torus: Torus,
    scale: f64,
    tolerance: f64,
    work: &mut Work,
) -> Result<Torus, FabError> {
    for _ in 0..6 {
        let (objective, maximum) = distance_objective(points, torus, scale, work)?;
        // Coordinate subtraction is limited by the absolute translation as
        // well as object size. This stopping floor is numerical roundoff,
        // not a replacement or adaptation of the user's fit tolerance.
        let magnitude = torus
            .center
            .iter()
            .map(|x| x.abs())
            .fold(scale.max(torus.major + torus.minor), f64::max);
        let noise = 32. * f64::EPSILON * (magnitude / scale);
        if objective <= points.len() as f64 * noise * noise {
            return Ok(torus);
        }
        let least = (0..3)
            .min_by(|&i, &j| torus.axis[i].abs().total_cmp(&torus.axis[j].abs()))
            .unwrap();
        let mut helper = [0.; 3];
        helper[least] = 1.;
        let tangent = cross(helper, torus.axis);
        let tangent = crate::math::scale(tangent, 1. / norm(tangent));
        let other = cross(torus.axis, tangent);
        let mut moments: [[Sum; 8]; 7] =
            std::array::from_fn(|_| std::array::from_fn(|_| Sum::default()));
        for &point in points {
            work.spend(35)?;
            let q = crate::math::scale(sub(point, torus.center), 1. / scale);
            let z = dot(q, torus.axis);
            let radial = sub(q, crate::math::scale(torus.axis, z));
            let rho = norm(radial);
            let major = torus.major / scale;
            let tube_radius = (rho - major).hypot(z);
            if rho <= 0. || tube_radius <= 0. {
                return Ok(torus);
            }
            let radial_normal = (rho - major) / tube_radius;
            let axial_normal = z / tube_radius;
            let mut row = [0.; 7];
            for i in 0..3 {
                row[i] = -radial_normal * radial[i] / rho - axial_normal * torus.axis[i];
            }
            let angular_derivative = z * major / (tube_radius * rho);
            row[3] = angular_derivative * dot(q, tangent);
            row[4] = angular_derivative * dot(q, other);
            row[5] = -radial_normal;
            row[6] = -1.;
            let residual = tube_radius - torus.minor / scale;
            for i in 0..7 {
                for j in i..7 {
                    moments[i][j].add(row[i] * row[j]);
                }
                moments[i][7].add(-row[i] * residual);
            }
        }
        let mut matrix: [[f64; 8]; 7] = std::array::from_fn(|i| {
            std::array::from_fn(|j| if j < i { moments[j][i].get() } else { moments[i][j].get() } / points.len() as f64)
        });
        work.spend(7 * 7 * 8)?;
        for column in 0..7 {
            let pivot = (column..7)
                .max_by(|&i, &j| matrix[i][column].abs().total_cmp(&matrix[j][column].abs()))
                .unwrap();
            if !matrix[pivot][column].is_finite() || matrix[pivot][column].abs() < 1e-12 {
                return Ok(torus);
            }
            matrix.swap(pivot, column);
            let divisor = matrix[column][column];
            for j in column..8 {
                matrix[column][j] /= divisor;
            }
            for i in 0..7 {
                if i != column {
                    let multiplier = matrix[i][column];
                    for j in column..8 {
                        matrix[i][j] -= multiplier * matrix[column][j];
                    }
                }
            }
        }
        let step: [f64; 7] = std::array::from_fn(|i| matrix[i][7]);
        if step.iter().any(|x| !x.is_finite()) {
            return Ok(torus);
        }
        let mut fraction = 1.;
        let mut accepted = None;
        for _ in 0..8 {
            let center = std::array::from_fn(|i| torus.center[i] + fraction * scale * step[i]);
            let axis: V = std::array::from_fn(|i| {
                torus.axis[i] + fraction * (step[3] * tangent[i] + step[4] * other[i])
            });
            let axis = crate::math::scale(axis, 1. / norm(axis));
            let major = torus.major + fraction * scale * step[5];
            let minor = torus.minor + fraction * scale * step[6];
            if major.is_finite()
                && minor.is_finite()
                && minor > 0.
                && major > minor
                && center.iter().chain(&axis).all(|x| x.is_finite())
            {
                let candidate = Torus {
                    center,
                    axis,
                    major,
                    minor,
                };
                let (next_objective, next_maximum) =
                    distance_objective(points, candidate, scale, work)?;
                if next_objective < objective && (maximum > tolerance || next_maximum <= tolerance)
                {
                    accepted = Some(candidate);
                    break;
                }
            }
            fraction *= 0.5;
        }
        let Some(next) = accepted else {
            return Ok(torus);
        };
        torus = next;
    }
    // Reaching the iteration cap is not success. The caller still verifies
    // every physical vertex residual and the complete topological/UV proof.
    Ok(torus)
}

fn distance_objective(
    points: &[V],
    torus: Torus,
    scale: f64,
    work: &mut Work,
) -> Result<(f64, f64), FabError> {
    let mut sum = Sum::default();
    let mut maximum: f64 = 0.;
    for &point in points {
        work.spend(1)?;
        let delta = sub(point, torus.center);
        let z = dot(delta, torus.axis);
        let rho = norm(sub(delta, crate::math::scale(torus.axis, z)));
        let distance = (rho - torus.major).hypot(z) - torus.minor;
        if !distance.is_finite() {
            return Err(unproven("raffinement géométrique non fini"));
        }
        maximum = maximum.max(distance.abs());
        sum.add((distance / scale).powi(2));
    }
    Ok((sum.get(), maximum))
}
