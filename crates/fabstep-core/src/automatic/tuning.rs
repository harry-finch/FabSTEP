//! Screen optional tolerances on the same welded mesh and closed adjacency.
//! Only the caller can certify the winning geometry after native STEP transfer.
use super::{recognize_with_adjacency, validate};
use crate::{
    math::{dot, sub},
    Analysis, FabError, Options, ReconstructionMode, RoundedFaceRef, RoundedRecognition,
};
use std::collections::BTreeSet;

/// A pure recognition on unchanged source triangles, pending the native audit.
pub struct AutomaticCandidate {
    pub options: Options,
    pub recognition: RoundedRecognition,
}

// Bound the total number of triangles presented to optional recognitions.
// Each individual pass also retains the recognizers' ordinary work limits.
const TRIANGLE_TRIAL_BUDGET: usize = 100_000;
const DENSE_THRESHOLD: usize = 8_192;
const FACTORS: [(f64, f64); 5] = [(5., 1.), (1., 2.), (10., 1.), (5., 2.), (10., 5.)];

/// Shared across the document, including bodies on which no candidate succeeds.
#[derive(Default)]
pub struct AutomaticTuningBudget {
    trials: usize,
    triangle_evaluations: usize,
}

impl AutomaticTuningBudget {
    fn can_try(&self, triangles: usize) -> bool {
        self.trials < FACTORS.len()
            && triangles <= TRIANGLE_TRIAL_BUDGET - self.triangle_evaluations
    }
}

/// Return the ordinary recognition and the best optional simplification found
/// under the document budget. Explicit numeric tolerances disable tuning,
/// including for old callers which do not supply `auto_tune`.
pub fn recognize_automatic_candidates(
    a: &Analysis,
    options: &Options,
    budget: &mut AutomaticTuningBudget,
    check_current: &mut impl FnMut() -> Result<(), FabError>,
) -> Result<(RoundedRecognition, Option<AutomaticCandidate>), FabError> {
    check_current()?;
    validate(a, options)?;
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let baseline = recognize_with_adjacency(a, options, None, &adjacency)?;
    check_current()?;
    let triangles = a.mesh.triangles.len();
    let defaults = Options::default();
    if !options.auto_tune
        || options.reconstruction != ReconstructionMode::Auto
        || options.plane_tolerance_mm != defaults.plane_tolerance_mm
        || options.cylinder_fit_tolerance_mm != defaults.cylinder_fit_tolerance_mm
        || options.weld_tolerance_mm != defaults.weld_tolerance_mm
        || options.max_deviation_mm != defaults.max_deviation_mm
        || !budget.can_try(triangles)
        || !curved_residual(&baseline, &adjacency)
    {
        return Ok((baseline, None));
    }

    let mut best = None;
    let ceiling = options.max_deviation_mm.min(0.01);
    // For denser inputs, require measured support/rim evidence before paying
    // for a single optional recognition. No retry per pair or per boundary.
    let dense_factors;
    let factors = if triangles > DENSE_THRESHOLD {
        dense_factors = dense_nomination(a, &baseline, &adjacency, options);
        if dense_factors.is_none() {
            check_current()?;
            return Ok((baseline, None));
        }
        dense_factors.as_slice()
    } else {
        &FACTORS
    };
    let mut face_limit = distinct_faces(&baseline);
    for &(plane_factor, radial_factor) in factors {
        if !budget.can_try(triangles) {
            break;
        }
        let plane = options.plane_tolerance_mm * plane_factor;
        let radial = options.cylinder_fit_tolerance_mm * radial_factor;
        if (plane_factor != 1. && plane > ceiling) || (radial_factor != 1. && radial > ceiling) {
            continue;
        }
        check_current()?;
        let trial = Options {
            plane_tolerance_mm: plane,
            cylinder_fit_tolerance_mm: radial,
            ..options.clone()
        };
        budget.trials += 1;
        budget.triangle_evaluations += triangles;
        let candidate = recognize_with_adjacency(a, &trial, None, &adjacency);
        check_current()?;
        let Ok(candidate) = candidate else {
            // An optional proof can refuse this partition or exhaust its budget.
            continue;
        };
        if !improved_coverage(&baseline, &candidate)
            || !candidate.report.max_deviation_bound_mm.is_finite()
            || candidate.report.max_deviation_bound_mm > options.max_deviation_mm
        {
            continue;
        }
        let faces = distinct_faces(&candidate);
        if faces < face_limit {
            face_limit = faces;
            // With no remaining shallow planar interfaces, the original aim is met.
            let complete = !curved_residual(&candidate, &adjacency);
            best = Some(AutomaticCandidate {
                options: trial,
                recognition: candidate,
            });
            if complete {
                break;
            }
        }
    }
    Ok((baseline, best))
}

fn dense_nomination(
    a: &Analysis,
    base: &RoundedRecognition,
    adjacency: &[Vec<usize>],
    options: &Options,
) -> Option<(f64, f64)> {
    let mixed = &base.model.revolved.mixed;
    let plane = options.plane_tolerance_mm;
    let radial = options.cylinder_fit_tolerance_mm;
    let mut plane_residual = 0_f64;
    let signed_offset = |i: usize, j: usize, normal| {
        let origin = a.mesh.vertices[a.mesh.triangles[i][0]];
        a.mesh.triangles[j]
            .iter()
            .map(|v| dot(sub(a.mesh.vertices[*v], origin), normal).abs())
            .fold(0_f64, f64::max)
    };
    for (i, neighbors) in adjacency.iter().enumerate() {
        let RoundedFaceRef::Plane { index: l } = base.triangle_faces[i] else {
            continue;
        };
        for &j in neighbors {
            if j <= i {
                continue;
            }
            let RoundedFaceRef::Plane { index: r } = base.triangle_faces[j] else {
                continue;
            };
            if l == r {
                continue;
            }
            let (n, m) = (mixed.planes[l].normal, mixed.planes[r].normal);
            if dot(n, m) < 1. - 1e-10 {
                continue;
            }
            let measured = signed_offset(i, j, n).max(signed_offset(j, i, m));
            if measured <= 10. * plane {
                plane_residual = plane_residual.max(measured);
            }
        }
    }

    let mut radial_residual = 0_f64;
    let mut loop_vertices = DENSE_THRESHOLD;
    'planes: for face in &mixed.planes {
        for lp in &face.loops {
            let crate::MixedLoop::Polygon { vertices } = lp else {
                continue;
            };
            if !(12..=DENSE_THRESHOLD).contains(&vertices.len()) {
                continue;
            }
            if vertices.len() > loop_vertices {
                break 'planes;
            }
            loop_vertices -= vertices.len();
            if let Some((_, _, fit)) =
                crate::rounded::fit_planar_ring(&mixed.vertices, vertices, face.normal, 5. * radial)
            {
                if fit.chord + fit.radial + options.weld_tolerance_mm <= options.max_deviation_mm {
                    radial_residual = radial_residual.max(fit.radial);
                }
            }
        }
    }
    let plane_factor = if plane_residual <= plane {
        1.
    } else if plane_residual <= 5. * plane {
        5.
    } else {
        10.
    };
    let radial_factor = if radial_residual <= radial {
        1.
    } else if radial_residual <= 2. * radial {
        2.
    } else {
        5.
    };
    (plane_factor > 1. || radial_factor > 1.).then_some((plane_factor, radial_factor))
}

fn distinct_faces(r: &RoundedRecognition) -> usize {
    r.triangle_faces.iter().collect::<BTreeSet<_>>().len()
}

// A planar region between two distinct, gently tilted planar neighbours is
// a possible curved strip. Three unrelated bevels do not form such a strip.
fn curved_residual(r: &RoundedRecognition, neighbors: &[Vec<usize>]) -> bool {
    let planes = &r.model.revolved.mixed.planes;
    if planes.len() < 3 {
        return false;
    }
    let mut first_neighbor = vec![usize::MAX; planes.len()];
    for (i, row) in neighbors.iter().enumerate() {
        let RoundedFaceRef::Plane { index: left } = r.triangle_faces[i] else {
            continue;
        };
        for &j in row {
            if j <= i {
                continue;
            }
            let RoundedFaceRef::Plane { index: right } = r.triangle_faces[j] else {
                continue;
            };
            if left == right {
                continue;
            }
            let cosine = dot(planes[left].normal, planes[right].normal);
            if (0.94..0.999999).contains(&cosine) {
                for (center, neighbor) in [(left, right), (right, left)] {
                    if first_neighbor[center] != usize::MAX && first_neighbor[center] != neighbor {
                        return true;
                    }
                    first_neighbor[center] = neighbor;
                }
            }
        }
    }
    false
}

fn improved_coverage(before: &RoundedRecognition, after: &RoundedRecognition) -> bool {
    let mut gained = false;
    for (a, b) in before.triangle_faces.iter().zip(&after.triangle_faces) {
        if matches!(a, RoundedFaceRef::Plane { .. }) {
            gained |= !matches!(b, RoundedFaceRef::Plane { .. });
        } else if std::mem::discriminant(a) != std::mem::discriminant(b) {
            return false;
        }
    }
    gained && before.triangle_faces.len() == after.triangle_faces.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze_stl;

    fn options() -> Options {
        Options {
            reconstruction: ReconstructionMode::Auto,
            ..Default::default()
        }
    }

    // R-117 translated in the input f32 representation: weld and the original
    // source indices stay fixed, but quantization changes the support residual.
    fn octant(options: &Options) -> Analysis {
        analyze_stl(
            include_bytes!("../../../../fixtures/r133-translated-octant.stl"),
            options,
        )
        .unwrap()
    }

    #[test]
    fn quantized_supports_recover_the_sphere_under_the_same_budget() {
        let opts = options();
        let source = octant(&opts);
        let mut budget = AutomaticTuningBudget::default();
        let (baseline, tuned) =
            recognize_automatic_candidates(&source, &opts, &mut budget, &mut || Ok(())).unwrap();
        assert!(baseline.model.octants.is_empty());
        let tuned = tuned.expect("octant proof");
        assert_eq!(tuned.recognition.model.octants.len(), 1);
        assert_eq!(distinct_faces(&tuned.recognition), 4);
        assert_eq!(tuned.options.max_deviation_mm, opts.max_deviation_mm);
        assert_eq!(tuned.options.weld_tolerance_mm, opts.weld_tolerance_mm);
        assert_eq!(
            tuned.options.cylinder_fit_tolerance_mm,
            opts.cylinder_fit_tolerance_mm
        );
        assert_eq!(
            tuned.options.plane_tolerance_mm,
            opts.plane_tolerance_mm * 5.
        );
        assert_eq!(budget.trials, 1);
    }

    #[test]
    fn explicit_limits_and_closed_planar_solids_skip_all_trials() {
        let mut budget = AutomaticTuningBudget::default();
        for opts in [
            Options {
                auto_tune: false,
                ..options()
            },
            Options {
                plane_tolerance_mm: 1e-7,
                ..options()
            },
        ] {
            let (_, tuned) =
                recognize_automatic_candidates(&octant(&opts), &opts, &mut budget, &mut || Ok(()))
                    .unwrap();
            assert!(tuned.is_none());
            assert_eq!(budget.trials, 0);
        }
        let opts = options();
        let cube = analyze_stl(include_bytes!("../../../../fixtures/cube.stl"), &opts).unwrap();
        let (_, tuned) =
            recognize_automatic_candidates(&cube, &opts, &mut budget, &mut || Ok(())).unwrap();
        assert!(tuned.is_none());
        assert_eq!(budget.trials, 0);
    }

    #[test]
    fn cancellation_after_a_probe_discards_its_winner() {
        let opts = options();
        let mut calls = 0;
        let mut budget = AutomaticTuningBudget::default();
        let result =
            recognize_automatic_candidates(&octant(&opts), &opts, &mut budget, &mut || {
                calls += 1;
                if calls == 4 {
                    Err(FabError::new("engine.stale", "cancelled"))
                } else {
                    Ok(())
                }
            });
        assert!(matches!(result, Err(e) if e.code == "engine.stale"));
        assert_eq!(budget.trials, 1);
    }

    #[test]
    fn document_limits_apply_even_to_later_bodies() {
        let opts = options();
        let source = octant(&opts);
        for mut budget in [
            AutomaticTuningBudget {
                trials: FACTORS.len(),
                triangle_evaluations: 0,
            },
            AutomaticTuningBudget {
                trials: 0,
                triangle_evaluations: TRIANGLE_TRIAL_BUDGET - source.mesh.triangles.len() + 1,
            },
        ] {
            let (_, tuned) =
                recognize_automatic_candidates(&source, &opts, &mut budget, &mut || Ok(()))
                    .unwrap();
            assert!(tuned.is_none());
        }
    }
    // Each cap has an internal polar grid. Both quadrangular walls and dense
    // caps stay in the same closed, consistently oriented welded mesh.
    fn dense_ring(radial_noise: f64) -> crate::Mesh {
        use std::f64::consts::TAU;
        const SIDES: usize = 64;
        const RINGS: usize = 240;
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let cap_size = 1 + SIDES * RINGS;
        for layer in 0..2 {
            let base = vertices.len();
            vertices.push([0., 0., f64::from(layer)]);
            for ring in 0..RINGS {
                for j in 0..SIDES {
                    let angle = j as f64 * TAU / SIDES as f64;
                    let radius =
                        (3. + if j % 2 == 0 {
                            radial_noise
                        } else {
                            -radial_noise
                        }) * (ring + 1) as f64
                            / RINGS as f64;
                    vertices.push([radius * angle.cos(), radius * angle.sin(), f64::from(layer)]);
                    let cur = base + 1 + ring * SIDES + j;
                    let next = base + 1 + ring * SIDES + (j + 1) % SIDES;
                    let mut add = |mut t: [usize; 3]| {
                        if layer == 0 {
                            t.swap(1, 2);
                        }
                        triangles.push(t);
                    };
                    if ring == 0 {
                        add([base, cur, next]);
                    } else {
                        add([cur - SIDES, cur, next]);
                        add([cur - SIDES, next, next - SIDES]);
                    }
                }
            }
        }
        for j in 0..SIDES {
            let last = cap_size - SIDES;
            let cur = last + j;
            let next = last + (j + 1) % SIDES;
            triangles.push([cur, next, next + cap_size]);
            triangles.push([cur, next + cap_size, cur + cap_size]);
        }
        crate::Mesh {
            vertices,
            triangles,
        }
    }

    #[test]
    #[ignore = "release qualification of optional screening on 61,440 triangles"]
    fn large_ring_screens_once_and_skips_out_of_range_facets() {
        for (noise, expected) in [(1.5e-5, true), (3e-4, false)] {
            let opts = options();
            let raw = dense_ring(noise);
            assert_eq!(raw.triangles.len(), 61_440);
            let a = crate::analyze_mesh(&raw, &opts).unwrap();
            let mut budget = AutomaticTuningBudget::default();
            let t = std::time::Instant::now();
            let mut previous = t;
            let mut times = vec![];
            let (original, candidate) =
                recognize_automatic_candidates(&a, &opts, &mut budget, &mut || {
                    let now = std::time::Instant::now();
                    times.push(now - previous);
                    previous = now;
                    Ok(())
                })
                .unwrap();
            eprintln!(
                "noise {noise}: {} faces, {} optional passes, checkpoints {times:?}, total {:?}",
                distinct_faces(&original),
                budget.trials,
                t.elapsed()
            );
            assert_eq!(budget.trials, usize::from(expected));
            assert_eq!(candidate.is_some(), expected);
            if let Some(best) = candidate {
                assert_eq!(best.recognition.report.cylinders, 1);
                assert_eq!(distinct_faces(&best.recognition), 3);
            }
        }
    }
}
