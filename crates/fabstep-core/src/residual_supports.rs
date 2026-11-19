//! R-117 qualification of supports on residual bands. A nomination carries
//! neither a trim nor a BRep ownership token. Construction requires separate
//! proofs of the common curves, their collars and global chart coverage.
use crate::math::{cross, dot, norm, scale, sub, V};
use crate::recognition::Work;
use crate::{FabError, Mesh, Options};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Support {
    Sphere { center: V, radius: f64 },
    Cone { apex: V, axis: V, slope: f64 },
}

#[derive(Debug, PartialEq)]
struct Nomination {
    support: Support,
    triangles: Vec<usize>,
    vertex_residual_mm: f64,
    triangle_bound_mm: f64,
}

fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}

fn unit(v: V) -> Option<V> {
    let length = norm(v);
    (length.is_finite() && length > 0.).then(|| v.map(|x| x / length))
}

/// Small least-squares systems, with column scaling and a rank guard. A
/// singular ray/normal configuration nominates nothing.
struct System<const N: usize> {
    gram: [[f64; N]; N],
    rhs: [f64; N],
}
impl<const N: usize> System<N> {
    fn new() -> Self {
        Self {
            gram: [[0.; N]; N],
            rhs: [0.; N],
        }
    }

    fn row(&mut self, row: [f64; N], value: f64) {
        for (i, &coefficient) in row.iter().enumerate() {
            self.rhs[i] += coefficient * value;
            for (entry, v) in self.gram[i].iter_mut().zip(row) {
                *entry += coefficient * v;
            }
        }
    }

    fn solve(mut self) -> Option<[f64; N]> {
        let scales = std::array::from_fn::<_, N, _>(|i| self.gram[i][i].sqrt());
        if scales.iter().any(|s| !s.is_finite() || *s == 0.) {
            return None;
        }
        for i in 0..N {
            self.rhs[i] /= scales[i];
            for j in 0..N {
                self.gram[i][j] /= scales[i] * scales[j];
            }
        }
        for j in 0..N {
            let p =
                (j..N).max_by(|&a, &b| self.gram[a][j].abs().total_cmp(&self.gram[b][j].abs()))?;
            self.gram.swap(j, p);
            self.rhs.swap(j, p);
            let pivot = self.gram[j][j];
            if !pivot.is_finite() || pivot.abs() < 1e-10 {
                return None;
            }
            let row = self.gram[j].map(|v| v / pivot);
            let rhs = self.rhs[j] / pivot;
            for i in 0..N {
                if i != j {
                    let factor = self.gram[i][j];
                    self.rhs[i] -= factor * rhs;
                    for (v, r) in self.gram[i].iter_mut().zip(row) {
                        *v -= factor * r;
                    }
                }
            }
            self.gram[j] = row;
            self.rhs[j] = rhs;
        }
        let solution = std::array::from_fn(|i| self.rhs[i] / scales[i]);
        solution.iter().all(|x| x.is_finite()).then_some(solution)
    }
}

fn sphere(samples: &[(V, V)], points: &[V]) -> Option<Support> {
    // Orthogonal projections onto the planes normal to the measured rays.
    let mut rays = System::new();
    for &(p, n) in samples {
        for i in 0..3 {
            let mut row = scale(n, -n[i]);
            row[i] += 1.;
            rays.row(row, dot(row, p));
        }
    }
    let mut center = rays.solve()?;
    let mut radius =
        points.iter().map(|&p| norm(sub(p, center))).sum::<f64>() / points.len() as f64;
    // Refine the actual radial residual at the original vertices.
    for _ in 0..5 {
        let mut fitting = System::new();
        for &p in points {
            let delta = sub(p, center);
            let [x, y, z] = unit(delta)?;
            fitting.row([x, y, z, 1.], norm(delta) - radius);
        }
        let [x, y, z, r] = fitting.solve()?;
        center = add(center, [x, y, z]);
        radius += r;
    }
    (radius.is_finite() && radius > 0.).then_some(Support::Sphere { center, radius })
}

fn cone(samples: &[(V, V)], points: &[V]) -> Option<Support> {
    // Tangent planes meet at the apex. Normal tips lie on an offset plane
    // whose normal is the cone axis; a cylinder/planar strip is rank deficient.
    let mut tangent_planes = System::new();
    let mut gauss_plane = System::new();
    for &(p, n) in samples {
        tangent_planes.row(n, dot(n, p));
        gauss_plane.row(n, 1.);
    }
    let mut apex = tangent_planes.solve()?;
    let mut axis = unit(gauss_plane.solve()?)?;
    if dot(axis, sub(points[0], apex)) < 0. {
        axis = scale(axis, -1.);
    }
    let mut cosine = 0.;
    for &p in points {
        cosine += dot(unit(sub(p, apex))?, axis) / points.len() as f64;
    }
    for _ in 0..6 {
        let i = (0..3).min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))?;
        let mut e = [0.; 3];
        e[i] = 1.;
        let x = unit(cross(axis, e))?;
        let y = cross(axis, x);
        let mut fitting = System::new();
        for &p in points {
            let v = sub(p, apex);
            let direction = unit(v)?;
            let projection = dot(direction, axis);
            let [a, b, c] = scale(sub(scale(direction, projection), axis), 1. / norm(v));
            fitting.row(
                [a, b, c, dot(direction, x), dot(direction, y), -1.],
                cosine - projection,
            );
        }
        let [a, b, c, u, v, k] = fitting.solve()?;
        apex = add(apex, [a, b, c]);
        axis = unit(add(axis, add(scale(x, u), scale(y, v))))?;
        cosine += k;
    }
    let angle = cosine.acos();
    (angle.is_finite() && (3_f64.to_radians()..80_f64.to_radians()).contains(&angle)).then_some(
        Support::Cone {
            apex,
            axis,
            slope: angle.tan(),
        },
    )
}

impl Support {
    // The signed radial error equals norm(q) minus an affine function.
    // Projection along q preserves axial height on cones.
    fn evaluate(self, p: V) -> Option<(V, f64, V)> {
        match self {
            Self::Sphere { center, radius } => {
                let q = sub(p, center);
                Some((q, norm(q) - radius, unit(q)?))
            }
            Self::Cone { apex, axis, slope } => {
                let v = sub(p, apex);
                let z = dot(v, axis);
                if z <= 0. {
                    return None;
                }
                let q = sub(v, scale(axis, z));
                Some((
                    q,
                    norm(q) - slope * z,
                    unit(sub(unit(q)?, scale(axis, slope)))?,
                ))
            }
        }
    }
    fn translate(self, offset: V) -> Self {
        match self {
            Self::Sphere { center, radius } => Self::Sphere {
                center: add(center, offset),
                radius,
            },
            Self::Cone { apex, axis, slope } => Self::Cone {
                apex: add(apex, offset),
                axis,
                slope,
            },
        }
    }
}

fn check_patch(
    mesh: &Mesh,
    ids: &[usize],
    normals: &[V],
    s: Support,
    o: &Options,
) -> Option<(f64, f64)> {
    let mut bound = 0_f64;
    let mut residual = 0_f64;
    let mut orientation = None;
    for &t in ids {
        let [a, b, c] = mesh.triangles[t].map(|i| s.evaluate(mesh.vertices[i]));
        let [(a, ra, na), (b, rb, nb), (c, rc, nc)] = [a?, b?, c?];
        let mut local = 0_f64;
        for (r, n) in [(ra, na), (rb, nb), (rc, nc)] {
            let cosine = dot(normals[t], n);
            let positive = cosine > 0.;
            if cosine.abs() < 0.9 || *orientation.get_or_insert(positive) != positive {
                return None;
            }
            local = local.max(r.abs());
        }
        if local > o.cylinder_fit_tolerance_mm {
            return None;
        }
        residual = residual.max(local);
        // norm(q) is convex and bounded below by dot(d,q). For any
        // barycentric weights, its interpolation deficit is therefore at
        // most max_i(norm(q_i) - dot(d,q_i)). The half-space also keeps the
        // radial projection defined over the entire source triangle.
        let d = unit(add(add(unit(a)?, unit(b)?), unit(c)?))?;
        let mut deficit = 0_f64;
        for q in [a, b, c] {
            if dot(q, d) <= 0. {
                return None;
            }
            let delta = sub(unit(q)?, d);
            deficit = deficit.max(norm(q) * (0.5 * dot(delta, delta) + 128. * f64::EPSILON));
        }
        bound = bound.max(local + deficit);
        if bound > o.max_deviation_mm {
            return None;
        }
    }
    Some((residual, bound))
}

fn discover(
    mesh: &Mesh,
    residual: &[bool],
    options: &Options,
    work: &mut Work,
) -> Result<Vec<Nomination>, FabError> {
    // This initial domain is for small connected strips. Large inputs remain
    // with the existing reconstructors; never truncate a strip to fit a budget.
    let count = mesh.triangles.len();
    if count > 4096
        || mesh.vertices.len() > 3 * 4096
        || residual.len() != count
        || !options.cylinder_fit_tolerance_mm.is_finite()
        || options.cylinder_fit_tolerance_mm <= 0.
        || !options.max_deviation_mm.is_finite()
        || options.max_deviation_mm <= 0.
    {
        return Ok(vec![]);
    }
    work.spend(count * 64)?;
    let mut normals = vec![[0.; 3]; count];
    let mut neighbors = vec![Vec::new(); count];
    let mut edges = BTreeMap::new();
    for (i, &triangle) in mesh
        .triangles
        .iter()
        .enumerate()
        .filter(|(i, _)| residual[*i])
    {
        let [a, b, c] = triangle.map(|v| mesh.vertices.get(v).copied());
        let (Some(a), Some(b), Some(c)) = (a, b, c) else {
            return Ok(vec![]);
        };
        let Some(n) = unit(cross(sub(b, a), sub(c, a))) else {
            return Ok(vec![]);
        };
        normals[i] = n;
        for j in 0..3 {
            let (v, w) = (triangle[j], triangle[(j + 1) % 3]);
            let entry = edges.entry((v.min(w), v.max(w))).or_insert((i, v, false));
            if entry.0 == i {
                continue;
            }
            if entry.1 == v || entry.2 {
                return Ok(vec![]);
            }
            entry.2 = true;
            if dot(n, normals[entry.0]) > 20_f64.to_radians().cos() {
                neighbors[i].push(entry.0);
                neighbors[entry.0].push(i);
            }
        }
    }
    let mut seen = vec![false; count];
    let mut results = Vec::new();
    for seed in 0..count {
        if !residual[seed] || seen[seed] {
            continue;
        }
        seen[seed] = true;
        let mut ids = vec![seed];
        let mut index = 0;
        while index < ids.len() {
            for &t in &neighbors[ids[index]] {
                if !seen[t] {
                    seen[t] = true;
                    ids.push(t);
                }
            }
            index += 1;
        }
        if ids.len() < 16 {
            continue;
        }
        ids.sort_unstable();
        let vertices = ids
            .iter()
            .flat_map(|&i| mesh.triangles[i])
            .collect::<BTreeSet<_>>();
        let origin = mesh.vertices[*vertices.first().unwrap()];
        let points: Vec<_> = vertices
            .into_iter()
            .map(|v| sub(mesh.vertices[v], origin))
            .collect();
        let samples: Vec<_> = ids
            .iter()
            .map(|&i| {
                let [a, b, c] = mesh.triangles[i].map(|v| sub(mesh.vertices[v], origin));
                (scale(add(add(a, b), c), 1. / 3.), normals[i])
            })
            .collect();
        // Debit bounded normal-equation rows (both candidates, all fixed
        // refinement iterations) and verification under a single shared Work.
        work.spend(samples.len() * 120 + points.len() * 480 + 6000)?;
        let mut valid = [sphere(&samples, &points), cone(&samples, &points)]
            .into_iter()
            .flatten()
            .map(|s| s.translate(origin))
            .filter_map(|s| check_patch(mesh, &ids, &normals, s, options).map(|e| (s, e)));
        if let Some((support, (vertex_residual_mm, triangle_bound_mm))) = valid.next() {
            if valid.next().is_none() {
                results.push(Nomination {
                    support,
                    triangles: ids,
                    vertex_residual_mm,
                    triangle_bound_mm,
                });
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFSET: V = [32., -12., 9.];
    fn rotate([x, y, z]: V) -> V {
        [0.8 * x - 0.6 * z, y, 0.6 * x + 0.8 * z]
    }

    // Nominal drawings only generate independent f32 samples. Both open
    // boundaries are oblique in parameter space; there is no circular ring.
    fn band(surface: impl Fn(f64, f64) -> V, warp: bool) -> Mesh {
        let (n, m) = (20, 8);
        let mut mesh = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        for j in 0..=m {
            for i in 0..=n {
                let v = j as f64 / m as f64;
                let u = i as f64 / n as f64;
                let theta = -0.9 + 1.8 * u + if warp { 0.03 * (v + u).sin() } else { 0. };
                let h = v + 0.12 * (1.2 * theta + 0.3 * v).sin();
                let p = add(rotate(surface(theta, h)), OFFSET);
                mesh.vertices.push(p.map(|x| x as f32 as f64));
            }
        }
        for j in 0..m {
            for i in 0..n {
                let a = j * (n + 1) + i;
                let (b, c, d) = (a + 1, a + n + 1, a + n + 2);
                if (i + j) % 2 == 0 {
                    mesh.triangles.extend([[a, b, d], [a, d, c]])
                } else {
                    mesh.triangles.extend([[a, b, c], [b, d, c]])
                }
            }
        }
        mesh
    }
    fn conic(t: f64, h: f64) -> V {
        let z = 7. + 3. * h;
        [0.5 * z * t.cos(), 0.5 * z * t.sin(), z]
    }
    fn spherical(t: f64, h: f64) -> V {
        let v = 0.12 + 0.6 * h;
        [8. * v.cos() * t.cos(), 8. * v.cos() * t.sin(), 8. * v.sin()]
    }
    fn options() -> Options {
        Options {
            cylinder_fit_tolerance_mm: 1e-5,
            ..Options::default()
        }
    }

    #[test]
    fn open_f32_caps_and_cones_nominate_the_drawn_support_in_a_rotated_frame() {
        for (is_sphere, mesh) in [(true, band(spherical, true)), (false, band(conic, true))] {
            let input = mesh.vertices.clone();
            let mask = vec![true; mesh.triangles.len()];
            let mut work = Work::default();
            let found = discover(&mesh, &mask, &options(), &mut work).unwrap();
            assert_eq!(found.len(), 1, "sphere={is_sphere}");
            println!(
                "sphere={is_sphere}, {} triangles, {} work units, vertex residual {:.9} mm, radial-image bound {:.9} mm",
                found[0].triangles.len(), work.used_for_test(), found[0].vertex_residual_mm, found[0].triangle_bound_mm
            );
            assert_eq!(
                found[0].triangles,
                (0..mesh.triangles.len()).collect::<Vec<_>>()
            );
            assert!(found[0].vertex_residual_mm < options().cylinder_fit_tolerance_mm);
            assert!(
                found[0].triangle_bound_mm > 0.
                    && found[0].triangle_bound_mm <= options().max_deviation_mm
            );
            match found[0].support {
                Support::Sphere { center, radius } => {
                    assert!(
                        is_sphere && norm(sub(center, OFFSET)) < 1e-5 && (radius - 8.).abs() < 1e-5
                    );
                }
                Support::Cone { apex, axis, slope } => {
                    assert!(
                        !is_sphere
                            && norm(sub(apex, OFFSET)) < 1e-5
                            && norm(sub(axis, rotate([0., 0., 1.]))) < 1e-5
                            && (slope - 0.5).abs() < 1e-5
                    );
                }
            }
            // Independently sample each source triangle using the nominal
            // drawing in its original frame. The tiny allowance covers the
            // difference between the fitted f32 support and the drawing.
            for triangle in &mesh.triangles {
                let [a, b, c] = triangle.map(|i| mesh.vertices[i]);
                for i in 0..=8 {
                    for j in 0..=8 - i {
                        let p = sub(
                            add(
                                a,
                                add(
                                    scale(sub(b, a), i as f64 / 8.),
                                    scale(sub(c, a), j as f64 / 8.),
                                ),
                            ),
                            OFFSET,
                        );
                        let q = [0.8 * p[0] + 0.6 * p[2], p[1], -0.6 * p[0] + 0.8 * p[2]];
                        let distance = if is_sphere {
                            (norm(q) - 8.).abs()
                        } else {
                            (q[0].hypot(q[1]) - 0.5 * q[2]).abs()
                        };
                        assert!(distance <= found[0].triangle_bound_mm + 1e-5);
                    }
                }
            }
            assert_eq!(
                found,
                discover(&mesh, &mask, &options(), &mut Work::default()).unwrap()
            );
            let mut tight = options();
            tight.max_deviation_mm = 0.0001;
            assert!(discover(&mesh, &mask, &tight, &mut Work::default())
                .unwrap()
                .is_empty());
            assert_eq!(mesh.vertices, input);
        }
    }

    #[test]
    fn interrupted_bands_taken_from_independent_closed_stls_recover_their_supports() {
        for (is_cone, bytes) in [
            (
                true,
                include_bytes!("../../../fixtures/m4_frustum.stl").as_slice(),
            ),
            (
                false,
                include_bytes!("../../../fixtures/m5b_small_cap.stl").as_slice(),
            ),
        ] {
            let mesh = crate::analyze_stl(bytes, &options()).unwrap().mesh;
            // An oblique window excludes the caps and the back of the part.
            // Discovery receives only the residual mask, never its cutting equation.
            let mask: Vec<_> = mesh
                .triangles
                .iter()
                .map(|t| {
                    let p = t.map(|i| mesh.vertices[i]);
                    p.iter().all(|q| q[0] > 0.4 + 0.09 * q[2])
                        && p.iter().any(|q| (q[2] - p[0][2]).abs() > 0.0001)
                })
                .collect();
            let found = discover(&mesh, &mask, &options(), &mut Work::default()).unwrap();
            assert_eq!(found.len(), 1, "cone={is_cone}");
            assert_eq!(
                found[0].triangles.len(),
                mask.iter().filter(|x| **x).count()
            );
            match found[0].support {
                Support::Cone { apex, axis, slope } => assert!(
                    is_cone
                        && norm(sub(apex, [0., 0., 14.])) < 1e-5
                        && norm(sub(axis, [0., 0., -1.])) < 1e-5
                        && (slope - 0.5).abs() < 1e-5
                ),
                Support::Sphere { center, radius } => assert!(
                    !is_cone
                        && norm(sub(center, [0., 0., -4.])) < 1e-5
                        && (radius - 6.).abs() < 1e-5
                ),
            }
        }
    }

    #[test]
    fn planes_tori_elliptic_cylinders_cubics_and_outliers_produce_no_support() {
        let mut noisy = band(spherical, true);
        noisy.vertices[95][1] += 0.03;
        let elliptic = band(|t, h| [5. * t.cos(), 4.5 * t.sin(), 3. * h], false);
        let cubic = band(
            |t, h| {
                let r = 4. + h + h.powi(3);
                [r * t.cos(), r * t.sin(), 4. * h]
            },
            false,
        );
        let torus = band(
            |t, h| {
                let r = 7. + 2. * h.cos();
                [r * t.cos(), r * t.sin(), 2. * h.sin()]
            },
            false,
        );
        let plane = band(|t, h| [t, h, 0.], false);
        let stairs = band(|t, h| [t, h, (h * 4.).floor() * 0.03], false);
        for mesh in [elliptic, cubic, torus, plane, stairs, noisy] {
            let result = discover(
                &mesh,
                &vec![true; mesh.triangles.len()],
                &options(),
                &mut Work::default(),
            )
            .unwrap();
            assert!(result.is_empty(), "{result:?}");
        }
    }

    #[test]
    fn normals_do_not_cross_reserved_triangles_or_spend_another_recognizers_budget() {
        let mesh = band(conic, false);
        let residual: Vec<_> = (0..mesh.triangles.len())
            .map(|i| (i / 2) % 20 != 10)
            .collect();
        let result = discover(&mesh, &residual, &options(), &mut Work::default()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.iter().map(|r| r.triangles.len()).sum::<usize>(),
            residual.iter().filter(|x| **x).count()
        );
        assert!(result
            .iter()
            .all(|r| r.triangles.iter().all(|&t| residual[t])));
        let mut work = Work::default();
        work.spend(9_990_000).unwrap();
        assert_eq!(
            discover(&mesh, &residual, &options(), &mut work)
                .unwrap_err()
                .code,
            "recognition.too_complex"
        );
    }
}
