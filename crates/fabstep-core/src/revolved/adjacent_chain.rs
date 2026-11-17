//! Search measured source-ring profiles only after the historical recognizers
//! decline them. Every returned span still needs the caller's wall proof.

use super::resolved_run_samples;
use crate::recognition::Work;
use crate::{FabError, Options};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub ends: [usize; 2],
    pub conic: bool,
}

#[derive(Clone, Copy)]
struct Candidate {
    span: Span,
    slope: f64,
    uncertainty: f64,
}

struct Search<'a> {
    profile: &'a [[f64; 2]],
    epsilon: f64,
    fit: f64,
    work: &'a mut Work,
    path: Vec<Candidate>,
    solution: Option<(Vec<Span>, bool)>,
    ambiguous: bool,
    inconclusive: bool,
}

impl Search<'_> {
    fn visit(&mut self, start: usize, cones: usize, adjacent: bool) -> Result<(), FabError> {
        if start == self.profile.len() - 1 {
            if cones >= 2 {
                if self.solution.is_some() {
                    self.ambiguous = true;
                } else {
                    self.solution = Some((self.path.iter().map(|c| c.span).collect(), adjacent));
                }
            }
            return Ok(());
        }

        let [z0, r0] = self.profile[start];
        let first_change = self.profile[start + 1][1] - r0;
        let conic = first_change.abs() > 2. * self.fit;
        let previous = self.path.last().copied();
        if previous.is_some_and(|p| !p.span.conic && !conic) {
            return Ok(());
        }
        // Evaluate conical candidates from their own endpoints. A slope
        // interval computed from intermediate f64 radii can exclude a valid
        // endpoint fit after interpolation rounds back to those same radii.
        for end in start + 1..self.profile.len() {
            self.work.spend(1)?;
            let [z1, r1] = self.profile[end];
            let dz = z1 - z0;
            let dr = r1 - r0;
            if !dz.is_finite() {
                break;
            }
            let change = r1 - self.profile[end - 1][1];
            if (change.abs() > 2. * self.fit) != conic
                || (conic && change.is_sign_positive() != first_change.is_sign_positive())
            {
                break;
            }
            if !conic && dr.abs() > 2. * self.fit {
                break;
            }
            if end - start < 3 || dz <= 8. * self.epsilon {
                continue;
            }
            let slope = dr / dz;
            // Bound signed slope movement when each endpoint coordinate can
            // vary by epsilon, with a strictly positive adjusted height.
            let uncertainty =
                (2. * self.epsilon + 2. * self.epsilon * slope.abs()) / (dz - 2. * self.epsilon);
            if !slope.is_finite()
                || !uncertainty.is_finite()
                || (conic && slope.abs() <= uncertainty)
                || previous
                    .is_some_and(|p| (p.slope - slope).abs() <= 4. * (p.uncertainty + uncertainty))
            {
                continue;
            }
            let points = &self.profile[start..=end];
            self.work.spend(points.len().saturating_mul(2))?;
            if !resolved_run_samples(z0, z1, points.iter().map(|p| p[0]), self.epsilon)
                || points.iter().any(|&[z, radius]| {
                    let fit = if conic {
                        r0 + dr * (z - z0) / dz
                    } else {
                        (r0 + r1) * 0.5
                    };
                    let residual = (radius - fit).abs();
                    !residual.is_finite() || residual > self.fit
                })
            {
                continue;
            }
            let next_cones = cones + usize::from(conic);
            if next_cones > 8 || self.path.len() == 17 {
                // A longer admissible prefix exhausts this grammar's scope.
                // Discard the whole search, including any earlier solution.
                self.inconclusive = true;
                return Ok(());
            }
            self.path.push(Candidate {
                span: Span {
                    ends: [start, end],
                    conic,
                },
                slope,
                uncertainty,
            });
            self.visit(
                end,
                next_cones,
                adjacent || (conic && previous.is_some_and(|p| p.span.conic)),
            )?;
            self.path.pop();
            if self.ambiguous || self.inconclusive {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) fn classify(
    profile: &[[f64; 2]],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Vec<Span>>, FabError> {
    let fit = o.cylinder_fit_tolerance_mm;
    let epsilon = fit.max(o.plane_tolerance_mm).max(o.weld_tolerance_mm);
    // The two-cone-only case already has a priority path. The smallest new
    // grammar includes a third span sharing its endpoint with each neighbor.
    if profile.len() < 10 || !epsilon.is_finite() || epsilon <= 0. || !fit.is_finite() || fit <= 0.
    {
        return Ok(None);
    }
    work.spend(profile.len().saturating_mul(2))?;
    if profile
        .iter()
        .any(|p| !p[0].is_finite() || !p[1].is_finite() || p[1] <= epsilon)
        || profile.windows(2).any(|w| {
            let height = w[1][0] - w[0][0];
            !height.is_finite() || height <= 4. * epsilon
        })
    {
        return Ok(None);
    }
    let mut search = Search {
        profile,
        epsilon,
        fit,
        work,
        path: Vec::with_capacity(17),
        solution: None,
        ambiguous: false,
        inconclusive: false,
    };
    search.visit(0, 0, false)?;
    match search.solution {
        Some((spans, true)) if !search.ambiguous && !search.inconclusive => Ok(Some(spans)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Vec<[f64; 2]> {
        let ends: [[f64; 2]; 4] = [[0., 10.], [3., 13.], [6., 15.94], [9., 14.]];
        let mut rings = Vec::new();
        for pair in ends.windows(2) {
            for t in [0., 1. / 3., 2. / 3.] {
                rings.push(std::array::from_fn(|i| {
                    pair[0][i] + t * (pair[1][i] - pair[0][i])
                }));
            }
        }
        rings.push(ends[3]);
        rings
    }

    #[test]
    fn two_nearby_breaks_each_fit_and_make_the_complete_path_ambiguous() {
        let mut rings = profile();
        let o = Options::default();
        let accepted = classify(&rings, &o, &mut Work::default()).unwrap().unwrap();
        assert_eq!(
            accepted.iter().map(|span| span.ends).collect::<Vec<_>>(),
            [[0, 3], [3, 6], [6, 9]]
        );
        rings.insert(4, [3.00008, 13. + 0.98 * 0.00008]);
        let mut work = Work::default();
        let mut search = Search {
            profile: &rings,
            epsilon: o.cylinder_fit_tolerance_mm,
            fit: o.cylinder_fit_tolerance_mm,
            work: &mut work,
            path: Vec::new(),
            solution: None,
            ambiguous: false,
            inconclusive: false,
        };
        search.visit(0, 0, false).unwrap();
        assert!(search.ambiguous);
        assert!(classify(&rings, &o, &mut Work::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn rounded_intermediate_radii_do_not_exclude_a_valid_endpoint_fit() {
        let ends = [100_000_000., 100_000_000.07, 100_000_000.61, 100_000_000.24];
        let mut profile = Vec::new();
        for (i, pair) in ends.windows(2).enumerate() {
            for j in 0..3 {
                let radius = pair[0] + (pair[1] - pair[0]) * f64::from(j) / 3.;
                profile.push([(3 * i) as f64 + f64::from(j), radius]);
            }
        }
        profile.push([9., ends[3]]);
        let fit = 1e-12;
        assert!((profile[1][1] - ends[0]) + 2. * fit < ((profile[2][1] - ends[0]) - 2. * fit) / 2.);
        let options = Options {
            cylinder_fit_tolerance_mm: fit,
            plane_tolerance_mm: fit,
            weld_tolerance_mm: fit,
            ..Options::default()
        };
        let spans = classify(&profile, &options, &mut Work::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            spans.iter().map(|s| s.ends).collect::<Vec<_>>(),
            [[0, 3], [3, 6], [6, 9]]
        );
    }

    #[test]
    fn the_first_candidate_is_unusable_when_the_uniqueness_budget_expires() {
        let rings = profile();
        let o = Options::default();
        let mut full = Work::default();
        classify(&rings, &o, &mut full).unwrap().unwrap();
        let mut saw_first_without_full_search = false;
        for remaining in 0..full.used_for_test() {
            let mut work = Work::default();
            work.spend(10_000_000 - remaining).unwrap();
            let mut search = Search {
                profile: &rings,
                epsilon: o.cylinder_fit_tolerance_mm,
                fit: o.cylinder_fit_tolerance_mm,
                work: &mut work,
                path: Vec::new(),
                solution: None,
                ambiguous: false,
                inconclusive: false,
            };
            let result = search.visit(0, 0, false);
            if let (Some(_), Err(error)) = (search.solution, result) {
                assert_eq!(error.code, "recognition.too_complex");
                let mut work = Work::default();
                work.spend(10_000_000 - remaining - 2 * rings.len())
                    .unwrap();
                assert_eq!(
                    classify(&rings, &o, &mut work).unwrap_err().code,
                    "recognition.too_complex"
                );
                saw_first_without_full_search = true;
                break;
            }
        }
        assert!(saw_first_without_full_search);
    }
}
