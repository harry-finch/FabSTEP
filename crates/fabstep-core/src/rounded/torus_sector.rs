//! One annular sweep, with a quarter round at its upper exterior edge.
//! The four source planes seed the frame. The five swept strips and both
//! radial caps must tile oriented disks with the exact shared boundary graph.
use super::*;
use crate::{CircularArc, MixedPartialCylinder, MixedSegment, MixedTorusSector};
use std::f64::consts::FRAC_PI_2;

const START: usize = 5;
const END: usize = 6;
// Allow two STEP transfers and physical trimming checks. Each canonical
// length/direction comparison uses the native 5e-8 mm guard.
const NATIVE_MM: f64 = 1e-5;
const ROUNDING_LIMIT_MM: f64 = 1e-7;
type Sections = [[f64; 2]; 5];

fn reject() -> FabError {
    unproven("secteur annulaire ou couverture de ses limites indéterminés")
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn unit(v: V) -> Result<V, FabError> {
    let n = norm(v);
    if n > 0. && n.is_finite() {
        Ok(scale(v, 1. / n))
    } else {
        Err(reject())
    }
}
fn signed(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

pub(super) fn volume(s: &MixedTorusSector) -> f64 {
    let (r, major, h) = (s.minor_radius_mm, s.major_radius_mm, s.height_mm);
    let inner = s.inner_radius_mm;
    let outer = major + r;
    // Positive contributions avoid subtracting nearly equal full cylinders
    // for a thin annulus.
    let straight = (h - r) * (outer - inner) * (outer + inner);
    let shoulder = r * (major - inner) * (major + inner);
    s.sweep_rad / 2. * (straight + shoulder + PI * major * r * r / 2. + 2. * r.powi(3) / 3.)
}

struct Sector {
    frame: Frame,
    shape: MixedTorusSector,
    corners: Sections,
    /// Region indices for bottom, top, start, end.
    planes: [usize; 4],
}

impl Sector {
    fn local(&self, p: V) -> [f64; 3] {
        let [x, y, z] = self.frame.project(p);
        [y.atan2(x), x.hypot(y), z]
    }
    fn radial(&self, u: f64) -> V {
        add(scale(self.frame.x, u.cos()), scale(self.frame.y, u.sin()))
    }
    fn point(&self, u: f64, rho: f64, z: f64) -> V {
        add(
            self.frame.origin,
            add(scale(self.radial(u), rho), scale(self.frame.axis, z)),
        )
    }
    fn coords(&self, j: usize, rho: f64, z: f64) -> f64 {
        let [ra, za] = self.corners[j];
        let [rb, zb] = self.corners[(j + 1) % 5];
        match j {
            0 | 3 => (rho - ra) / (rb - ra),
            1 | 4 => (z - za) / (zb - za),
            _ => (z - za).atan2(rho - rb) / FRAC_PI_2,
        }
    }
    fn section(&self, j: usize, t: f64) -> [f64; 2] {
        if t == 0. {
            return self.corners[j];
        }
        if t == 1. {
            return self.corners[(j + 1) % 5];
        }
        let [a, b] = self.corners[j];
        let [c, d] = self.corners[(j + 1) % 5];
        if j == 2 {
            let angle = FRAC_PI_2 * t;
            let r = self.shape.minor_radius_mm;
            [c + r * angle.cos(), b + r * angle.sin()]
        } else {
            [a + (c - a) * t, b + (d - b) * t]
        }
    }
}
fn seed(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    work: &mut Work,
) -> Result<Sector, FabError> {
    let mesh = &a.mesh;
    let supports: Vec<_> = s
        .model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.loops.len() == 1 && p.loops[0].len() >= 8)
        .take(5)
        .map(|(i, p)| (i, &p.loops[0]))
        .collect();
    // Require exactly four simple extended boundaries; another substantial
    // support or curved portion of the object is outside this whole-body case.
    if supports.len() != 4 {
        return Err(reject());
    }
    work.spend(supports.iter().map(|s| s.1.len() * 2).sum())?;
    let planes = supports
        .iter()
        .map(|(i, lp)| {
            let base = mesh.vertices[lp[0]];
            let mut n = [0.; 3];
            for k in 0..lp.len() {
                n = add(
                    n,
                    cross(
                        sub(mesh.vertices[lp[k]], base),
                        sub(mesh.vertices[lp[(k + 1) % lp.len()]], base),
                    ),
                )
            }
            let mut n = unit(n)?;
            if dot(n, s.model.faces[*i].normal) < 0. {
                n = scale(n, -1.);
            }
            Ok((n, base))
        })
        .collect::<Result<Vec<_>, FabError>>()?;
    let pairs: Vec<_> = (0..4)
        .flat_map(|i| (i + 1..4).map(move |j| (i, j)))
        .filter(|&(i, j)| dot(planes[i].0, planes[j].0) < -0.99999999)
        .collect();
    let &[(bottom, top)] = pairs.as_slice() else {
        return Err(reject());
    };
    for (b, t) in [(bottom, top), (top, bottom)] {
        let axis = planes[t].0;
        let other: Vec<_> = (0..4).filter(|&i| i != t && i != b).collect();
        for (start, end) in [(other[0], other[1]), (other[1], other[0])] {
            let (n0, p0) = planes[start];
            let (n1, p1) = planes[end];
            if dot(n0, axis).abs().max(dot(n1, axis).abs()) > 1e-6 {
                continue;
            }
            let anchor = planes[b].1;
            let x = unit(cross(axis, n0))?;
            let y = cross(axis, x);
            let dx = dot(n1, x);
            if dx.abs() < 0.1 {
                continue;
            }
            let dy = dot(sub(p0, anchor), n0) / dot(n0, y);
            let ox = (dot(sub(p1, anchor), n1) - dot(n1, y) * dy) / dx;
            let origin = add(anchor, add(scale(x, ox), scale(y, dy)));
            let frame = Frame { origin, axis, x, y };
            let end_vector = unit(cross(n1, axis))?;
            let sweep = dot(y, end_vector).atan2(dot(x, end_vector));
            if !(0.1..PI - 0.1).contains(&sweep) {
                continue;
            }
            let height = dot(sub(planes[t].1, origin), axis);
            let radii = |i: usize| {
                supports[i]
                    .1
                    .iter()
                    .map(|&v| {
                        let [x, y, _] = frame.project(mesh.vertices[v]);
                        x.hypot(y)
                    })
                    .fold([f64::INFINITY, 0_f64], |r, x| [r[0].min(x), r[1].max(x)])
            };
            let [inner, outer] = radii(b);
            let [_, major] = radii(t);
            let minor = outer - major;
            if !(minor > 0. && height > minor && major > inner && inner > 0. && major > minor) {
                continue;
            }
            let shape = MixedTorusSector {
                center: add(origin, scale(axis, height - minor)),
                axis,
                start_direction: x,
                major_radius_mm: major,
                minor_radius_mm: minor,
                height_mm: height,
                inner_radius_mm: inner,
                sweep_rad: sweep,
                inner: false,
                arc_uses: [(2, false), (6, false), (3, true), (5, true)],
            };
            return Ok(Sector {
                frame,
                shape,
                corners: [
                    [inner, 0.],
                    [outer, 0.],
                    [outer, height - minor],
                    [major, height],
                    [inner, height],
                ],
                planes: [b, t, start, end].map(|i| supports[i].0),
            });
        }
    }
    Err(reject())
}

struct Chart {
    u: f64,
    rho: f64,
    z: f64,
}
impl Chart {
    fn t(&self, f: &Sector, j: usize) -> f64 {
        f.coords(j, self.rho, self.z)
    }
}

fn classify(f: &Sector, p: &[[f64; 3]], t: [usize; 3], tol: f64) -> Result<usize, FabError> {
    let candidates: Vec<_> = [1, 2, 4]
        .into_iter()
        .filter(|&j| {
            t.iter().all(|&k| {
                let [_, rho, z] = p[k];
                let [r, h] = f.section(j, f.coords(j, rho, z).clamp(0., 1.));
                (rho - r).hypot(z - h) <= tol
            })
        })
        .collect();
    if let [j] = candidates.as_slice() {
        Ok(*j)
    } else {
        Err(reject())
    }
}

#[allow(clippy::too_many_arguments)]
fn prove(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
    f: Sector,
) -> Result<RoundedRecognition, FabError> {
    let mesh = &a.mesh;
    let shape = &f.shape;
    let scale_mm = mesh
        .vertices
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(shape.height_mm.max(shape.major_radius_mm), f64::max);
    let rounding = scale_mm * 2048. * f64::EPSILON;
    let tol = o.cylinder_fit_tolerance_mm;
    if rounding > ROUNDING_LIMIT_MM
        || shape
            .minor_radius_mm
            .min(shape.inner_radius_mm)
            .min(shape.height_mm - shape.minor_radius_mm)
            .min(shape.major_radius_mm - shape.inner_radius_mm)
            <= 16. * tol
    {
        return Err(reject());
    }
    work.spend(mesh.vertices.len() * 6 + mesh.triangles.len() * 12)?;
    let local: Vec<_> = mesh.vertices.iter().map(|&p| f.local(p)).collect();
    let mut masks = vec![0u8; local.len()];
    let mut owners = Vec::with_capacity(mesh.triangles.len());
    for (triangle, &t) in mesh.triangles.iter().enumerate() {
        let id = match f
            .planes
            .iter()
            .position(|&r| r == s.triangle_regions[triangle])
        {
            Some(i) => [0, 3, START, END][i],
            None => classify(&f, &local, t, tol)?,
        };
        for v in t {
            masks[v] |= 1 << id;
        }
        owners.push(id);
    }
    let mut cuts: BTreeMap<Edge, Vec<Edge>> = BTreeMap::new();
    for (&pair, ts) in edges {
        let [a, b] = ts.as_slice() else {
            return Err(reject());
        };
        if owners[*a] != owners[*b] {
            cuts.entry(edge(owners[*a], owners[*b]))
                .or_default()
                .push(pair);
        }
    }
    let mut expected = BTreeSet::new();
    for j in 0..5 {
        for k in [(j + 1) % 5, START, END] {
            expected.insert(edge(j, k));
        }
    }
    if cuts.keys().copied().collect::<BTreeSet<_>>() != expected {
        return Err(reject());
    }
    block::disks::<7>(a, &owners, adj, &cuts, work)?;
    let mut params = Vec::with_capacity(local.len());
    let mut ends = [[usize::MAX; 5]; 2];
    let mut positions = mesh.vertices.clone();
    let mut residual = 0_f64;
    let mut moved = 0;
    let mut max_plane = 0_f64;
    for (i, (&[u, rho, z], &mask)) in local.iter().zip(&masks).enumerate() {
        let caps = mask >> START;
        if mask == 0 || caps == 3 {
            return Err(reject());
        }
        let u = match caps {
            1 => 0.,
            2 => shape.sweep_rad,
            _ => u.clamp(0., shape.sweep_rad),
        };
        let bands: Vec<_> = (0..5).filter(|j| mask & (1 << j) != 0).collect();
        let corner = (0..5)
            .find(|&j| bands.len() == 2 && bands.contains(&j) && bands.contains(&((j + 4) % 5)));
        let [radius, h] = match (bands.as_slice(), corner) {
            ([], _) => [rho, z],
            ([j], _) => f.section(*j, f.coords(*j, rho, z).clamp(0., 1.)),
            (_, Some(j)) => {
                if caps != 0 {
                    let endpoint = &mut ends[(caps - 1) as usize][j];
                    if *endpoint != usize::MAX {
                        return Err(reject());
                    }
                    *endpoint = i;
                }
                f.corners[j]
            }
            _ => return Err(reject()),
        };
        let q = f.point(u, radius, h);
        let error = norm(sub(mesh.vertices[i], q));
        residual = residual.max(error);
        if caps != 0 {
            let n = cross(f.frame.axis, f.radial(u));
            max_plane = max_plane.max(dot(sub(mesh.vertices[i], q), n).abs())
        }
        if bands.contains(&0) || bands.contains(&3) {
            max_plane = max_plane.max((h - z).abs());
        }
        if error > tol || max_plane > o.plane_tolerance_mm {
            return Err(reject());
        }
        if corner.is_some() && caps != 0 {
            moved += usize::from(q != positions[i]);
            positions[i] = q;
        }
        params.push(Chart {
            u,
            rho: radius,
            z: h,
        });
    }
    if ends.iter().flatten().any(|&v| v == usize::MAX) {
        return Err(reject());
    }

    // Each cut is a single chain with strictly monotone parameters. Together
    // with positive triangle Jacobians and disk topology this gives degree 1
    // on each rectangular chart and on each chordal planar end polygon.
    for (&(a, b), es) in &cuts {
        let chain = block::ordered_chain(es)?;
        let varying_angle = b < START;
        let sequence = chain
            .iter()
            .map(|&v| {
                if varying_angle {
                    params[v].u
                } else {
                    params[v].t(&f, a)
                }
            })
            .collect::<Vec<_>>();
        let direction = (sequence[sequence.len() - 1] - sequence[0]).signum();
        if sequence
            .windows(2)
            .any(|w| direction * (w[1] - w[0]) <= 128. * f64::EPSILON)
        {
            return Err(reject());
        }
    }
    let mut chord = 0_f64;
    for (&t, &face) in mesh.triangles.iter().zip(&owners) {
        let chart = t.map(|i| {
            let p = &params[i];
            if face < START {
                [p.u, p.t(&f, face)]
            } else {
                [p.rho, p.z]
            }
        });
        let det = signed(chart[0], chart[1], chart[2]) * if face == END { -1. } else { 1. };
        let widths = [0, 1].map(|j| {
            chart.iter().map(|p| p[j]).fold(f64::NEG_INFINITY, f64::max)
                - chart.iter().map(|p| p[j]).fold(f64::INFINITY, f64::min)
        });
        if det <= 512. * f64::EPSILON * (widths[0] * widths[1]).max(f64::MIN_POSITIVE) {
            return Err(reject());
        }
        if face >= START {
            continue;
        }
        let du = widths[0];
        let dt = widths[1];
        let radius = f.corners[face][0].max(f.corners[(face + 1) % 5][0]);
        let curv = radius * du * du / 8.;
        let err = match face {
            2 => {
                let dv = dt * FRAC_PI_2;
                curv + shape.minor_radius_mm * (du * dv / 4. + dv * dv / 8.)
            }
            0 | 3 => {
                curv + (f.corners[face][0] - f.corners[(face + 1) % 5][0]).abs() * du * dt / 4.
            }
            _ => curv,
        };
        chord = chord.max(err);
    }
    // Planar end disks are bounded by the inscribed meridian polygon and its
    // exact arc. The strip's meridian sagitta above bounds their symmetric
    // difference too. Source-to-corrected vertex residual covers all charts.
    let bound = o.weld_tolerance_mm + residual + chord + rounding + NATIVE_MM;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le secteur torique dépasse le budget d’écart.",
        ));
    }
    let mut result = assemble(f, ends, positions, owners, residual, chord, bound);
    if moved > 0 {
        result.report.diagnostics.push(Diagnostic {
            code: "recognition.toric_sector_endpoints_projected".into(),
            severity: Severity::Info,
            count: moved,
            message: "Les extrémités partagées sont projetées sur les arcs du secteur. Leur déplacement est inclus dans la borne.".into(),
        });
    }
    Ok(result)
}

pub(super) fn candidate(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    if !(10..=8192).contains(&s.model.faces.len()) || a.mesh.triangles.len() > 32_000 {
        return Ok(None);
    }
    let sector = match seed(a, s, work) {
        Ok(f) => f,
        Err(e) if e.code == "recognition.surface_unproven" => return Ok(None),
        Err(e) => return Err(e),
    };
    match prove(a, s, adj, edges, o, work, sector) {
        Ok(result) => Ok(Some(result)),
        Err(e) if e.code == "recognition.surface_unproven" => Ok(None),
        Err(e) => Err(e),
    }
}

fn assemble(
    f: Sector,
    ends: [[usize; 5]; 2],
    vertices: Vec<V>,
    owners: Vec<usize>,
    residual: f64,
    chord: f64,
    bound: f64,
) -> RoundedRecognition {
    let shape = &f.shape;
    let mut arcs: Vec<_> = f
        .corners
        .iter()
        .enumerate()
        .map(|(j, &[radius, z])| CircularArc {
            center: f.point(0., 0., z),
            axis: shape.axis,
            radius_mm: radius,
            sweep_rad: shape.sweep_rad,
            vertices: [ends[0][j], ends[1][j]],
        })
        .collect();
    for (k, u) in [0., shape.sweep_rad].into_iter().enumerate() {
        arcs.push(CircularArc {
            center: f.point(
                u,
                shape.major_radius_mm,
                shape.height_mm - shape.minor_radius_mm,
            ),
            axis: cross(f.radial(u), shape.axis),
            radius_mm: shape.minor_radius_mm,
            sweep_rad: FRAC_PI_2,
            vertices: [ends[k][2], ends[k][3]],
        });
    }
    let arc = |arc, reversed| MixedSegment::Arc { arc, reversed };
    let line = |a, b| MixedSegment::Line { vertices: [a, b] };
    let plane = |normal, segments| MixedPlane {
        normal,
        loops: vec![MixedLoop::Segments { segments }],
    };
    let mut planes = Vec::new();
    for j in [0, 3] {
        let k = j + 1;
        planes.push(plane(
            scale(shape.axis, if j == 0 { -1. } else { 1. }),
            vec![
                arc(j, false),
                line(ends[1][j], ends[1][k]),
                arc(k, true),
                line(ends[0][k], ends[0][j]),
            ],
        ));
    }
    for (k, u) in [0., shape.sweep_rad].into_iter().enumerate() {
        let e = ends[k];
        let segments = if k == 0 {
            vec![
                line(e[0], e[1]),
                line(e[1], e[2]),
                arc(5, false),
                line(e[3], e[4]),
                line(e[4], e[0]),
            ]
        } else {
            vec![
                line(e[0], e[4]),
                line(e[4], e[3]),
                arc(6, true),
                line(e[2], e[1]),
                line(e[1], e[0]),
            ]
        };
        planes.push(plane(
            scale(
                cross(f.radial(u), shape.axis),
                if k == 0 { 1. } else { -1. },
            ),
            segments,
        ));
    }
    let mixed = MixedModel {
        vertices,
        rings: vec![],
        planes,
        cylinders: vec![],
        arcs,
        elliptic_arcs: vec![],
        partial_cylinders: vec![
            MixedPartialCylinder {
                elliptic_end: false,
                arcs: [1, 2],
                inner: false,
            },
            MixedPartialCylinder {
                elliptic_end: false,
                arcs: [0, 4],
                inner: true,
            },
        ],
    };
    let expected_volume_mm3 = volume(shape);
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed,
            cones: vec![],
        },
        torus_sectors: vec![f.shape],
        tori: vec![],
        spheres: vec![],
        partial_cones: vec![],
        octants: vec![],
        linear_blend: None,
        unequal_pocket: None,
        cross_drill: None,
        smooth_revolution: None,
    };
    let triangle_faces = owners
        .iter()
        .map(|i| match i {
            0 => RoundedFaceRef::Plane { index: 0 },
            3 => RoundedFaceRef::Plane { index: 1 },
            5 => RoundedFaceRef::Plane { index: 2 },
            6 => RoundedFaceRef::Plane { index: 3 },
            1 => RoundedFaceRef::PartialCylinder { index: 0 },
            4 => RoundedFaceRef::PartialCylinder { index: 1 },
            2 => RoundedFaceRef::TorusSector { index: 0 },
            _ => unreachable!(),
        })
        .collect();
    let interpretation = Diagnostic {
        code: "recognition.toric_sector_fitted".into(),
        severity: Severity::Warning,
        count: 1,
        message: "Une bande du maillage est interprétée comme un secteur de tore. Les limites sont ajustées dans la borne annoncée.".into(),
    };
    let report = RecognitionReport {
        cylinders: 2,
        cones: 0,
        spheres: 0,
        tori: 1,
        toroidal_fillets: 1,
        revolutions: 0,
        max_radial_residual_mm: residual,
        max_chord_error_mm: chord,
        max_wall_residual_mm: 0.,
        max_deviation_bound_mm: bound,
        expected_volume_mm3,
        diagnostics: vec![interpretation],
    };
    RoundedRecognition {
        model,
        report,
        triangle_faces,
    }
}
