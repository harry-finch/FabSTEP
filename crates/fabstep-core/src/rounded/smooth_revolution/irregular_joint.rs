//! A tangent joint between measured heights. Four independent height/radius
//! witnesses bound each one-sided cubic and its first two derivatives.
use super::*;

struct Polynomial {
    nodes: [(f64, f64); 4],
}
struct At {
    value: [f64; 3],
    error: [f64; 3],
}

impl Polynomial {
    fn new(part: &[(f64, f64)]) -> Option<Self> {
        let (a, b) = (part[0].0, part.last()?.0);
        let nodes = std::array::from_fn(|i| {
            let goal = a + (b - a) * i as f64 / 3.;
            *part
                .iter()
                .min_by(|p, q| (p.0 - goal).abs().total_cmp(&(q.0 - goal).abs()))
                .unwrap()
        });
        nodes
            .windows(2)
            .all(|p| p[1].0 - p[0].0 >= (b - a) / 8.)
            .then_some(Self { nodes })
    }
    fn at(&self, t: f64, tol: f64) -> At {
        let mut value = [0.; 3];
        let mut error = [0.; 3];
        for (i, &(x, r)) in self.nodes.iter().enumerate() {
            let a = self.nodes[(i + 1) % 4].0;
            let b = self.nodes[(i + 2) % 4].0;
            let c = self.nodes[(i + 3) % 4].0;
            let den = (x - a) * (x - b) * (x - c);
            let basis = [
                (t - a) * (t - b) * (t - c) / den,
                ((t - a) * (t - b) + (t - a) * (t - c) + (t - b) * (t - c)) / den,
                2. * (3. * t - a - b - c) / den,
            ];
            for j in 0..3 {
                value[j] += basis[j] * r;
                error[j] += tol * basis[j].abs();
            }
        }
        At { value, error }
    }
    fn compatible(&self, part: &[(f64, f64)], tol: f64) -> bool {
        part.iter().all(|&(x, r)| {
            let v = self.at(x, tol);
            (v.value[0] - r).abs() <= tol + v.error[0]
        })
    }
}

fn difference(left: &Polynomial, right: &Polynomial, x: f64, tol: f64) -> At {
    let a = left.at(x, tol);
    let b = right.at(x, tol);
    At {
        value: std::array::from_fn(|i| a.value[i] - b.value[i]),
        error: std::array::from_fn(|i| a.error[i] + b.error[i]),
    }
}
fn sign(at: &At, k: usize) -> i32 {
    if at.value[k] > at.error[k] {
        1
    } else if at.value[k] < -at.error[k] {
        -1
    } else {
        0
    }
}

pub(super) fn fit(
    samples: &[Node],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Profile>, FabError> {
    let first = &samples[0];
    let last = samples.last().unwrap();
    let height = last.z - first.z;
    let data: Vec<_> = samples
        .iter()
        .map(|p| ((p.z - first.z) / height, p.radius))
        .collect();
    let scale = samples.iter().map(|p| p.radius).fold(1., f64::max);
    // Include arithmetic slack on the selected physical radii in the
    // interpolation envelopes. It never enlarges final vertex acceptance.
    let tol = o.cylinder_fit_tolerance_mm + 512. * f64::EPSILON * scale;
    let mut result = None;
    for split in 6..samples.len().saturating_sub(7) {
        let left = &data[..=split];
        let right = &data[split + 1..];
        let mut lo = left.last().unwrap().0.max(0.125);
        let mut hi = right[0].0.min(0.875);
        if lo >= hi {
            continue;
        }
        work.spend(64 * (data.len() + 64))?;
        let (Some(p), Some(q)) = (Polynomial::new(left), Polynomial::new(right)) else {
            return Ok(None);
        };
        if !p.compatible(left, tol) || !q.compatible(right, tol) {
            continue;
        }
        let a = difference(&p, &q, lo, tol);
        let b = difference(&p, &q, hi, tol);
        let curvature = sign(&a, 2);
        // Each possible curvature difference is affine in t, so strict,
        // identical endpoint signs imply a monotone derivative difference
        // throughout the bracket, even when each witness varies by tol.
        if curvature == 0 || curvature != sign(&b, 2) {
            return Ok(None);
        }
        let sa = sign(&a, 1);
        let sb = sign(&b, 1);
        if sa == 0 || sb == 0 {
            return Ok(None);
        }
        if sa == sb {
            continue;
        }
        if result.is_some() {
            return Ok(None);
        }
        for _ in 0..48 {
            let mid = (lo + hi) / 2.;
            if difference(&p, &q, mid, tol).value[1] * (sa as f64) > 0. {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let k = (lo + hi) / 2.;
        let a = p.at(k, tol);
        let b = q.at(k, tol);
        let r = (a.value[0] + b.value[0]) / 2.;
        let slope = (a.value[1] + b.value[1]) / 2.;
        if (a.value[0] - b.value[0]).abs() > tol {
            return Ok(None);
        }
        let lower = [
            first.radius,
            first.radius + k * p.at(0., tol).value[1] / 3.,
            r - k * slope / 3.,
            r,
        ];
        let upper = [
            r,
            r + (1. - k) * slope / 3.,
            last.radius - (1. - k) * q.at(1., tol).value[1] / 3.,
            last.radius,
        ];
        if lower
            .iter()
            .chain(&upper)
            .any(|x| !x.is_finite() || *x <= 8. * tol)
            || left
                .iter()
                .any(|&(t, r)| (radius(&lower, t / k) - r).abs() > o.cylinder_fit_tolerance_mm)
            || right.iter().any(|&(t, r)| {
                (radius(&upper, (t - k) / (1. - k)) - r).abs() > o.cylinder_fit_tolerance_mm
            })
        {
            return Ok(None);
        }
        result = Some(Profile {
            first: lower,
            second: Some((
                split,
                CubicRevolutionSpan {
                    start_fraction: k,
                    next_radii_mm: [upper[1], upper[2], upper[3]],
                },
            )),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_derivative_sign_indeterminate_at_a_bracket_boundary_never_selects_a_joint() {
        let k = 0.375;
        let nodes: Vec<_> = (0..=64)
            .map(|i| {
                let t = i as f64 / 64.;
                let r = if t < k {
                    radius(&[6., 4.5, 7.5, 8.], t / k)
                } else {
                    radius(&[8., 8. + 5. / 6., 4., 6.], (t - k) / (1. - k))
                };
                Node {
                    radius: r,
                    z: t * 24.,
                    vertices: vec![i],
                }
            })
            .collect();
        assert!(fit(&nodes, &Options::default(), &mut Work::default())
            .unwrap()
            .is_none());
    }
}
