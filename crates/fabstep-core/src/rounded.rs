//! Explicit rounded-surface recognition. Complete circular boundaries cut
//! topological patches; no correspondence between mesh latitude samples is
//! required. Every triangle has an oriented, degree-one parameter image.
use crate::math::*;
use crate::recognition::{fit_circle, signed_area, validate_analysis, Work};
use crate::{
    Analysis, CircularRing, Diagnostic, FabError, MixedCone, MixedCylinder, MixedFaceRef,
    MixedLoop, MixedModel, MixedPlane, MixedSphere, MixedTorus, Options, RecognitionReport,
    ReconstructionMode, RevolvedModel, RoundedFaceRef, RoundedModel, RoundedRecognition, Severity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};
type Edge = (usize, usize);
fn edge(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn boundary(v: &[usize]) -> BTreeSet<Edge> {
    (0..v.len())
        .map(|i| edge(v[i], v[(i + 1) % v.len()]))
        .collect()
}
fn unproven(s: impl std::fmt::Display) -> FabError {
    FabError::new(
        "recognition.surface_unproven",
        format!("Surface arrondie non prouvée : {s}. Aucun résultat partiel proposé."),
    )
}
fn unsupported() -> FabError {
    FabError::new(
        "recognition.unsupported",
        "Aucune sphère ou raccord torique admissible détecté dans les contours source.",
    )
}
#[derive(Clone)]
struct Frame {
    origin: V,
    axis: V,
    x: V,
    y: V,
}
impl Frame {
    fn new(origin: V, normal: V) -> Self {
        let mut axis = scale(normal, 1. / norm(normal));
        let major = (0..3)
            .max_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
            .unwrap();
        if axis[major] < 0. {
            axis = scale(axis, -1.);
        }
        let least = (0..3)
            .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
            .unwrap();
        let mut helper = [0.; 3];
        helper[least] = 1.;
        let x = cross(helper, axis);
        let x = scale(x, 1. / norm(x));
        Self {
            origin,
            axis,
            x,
            y: cross(axis, x),
        }
    }
    fn project(&self, p: V) -> V {
        let p = sub(p, self.origin);
        [dot(p, self.x), dot(p, self.y), dot(p, self.axis)]
    }
    fn lift(&self, p: V) -> V {
        std::array::from_fn(|i| {
            self.origin[i] + self.x[i] * p[0] + self.y[i] * p[1] + self.axis[i] * p[2]
        })
    }
}
#[derive(Clone)]
struct Rim {
    vertices: Vec<usize>,
    region: usize,
    center: V,
    axis: V,
    radius: f64,
}
#[derive(Clone)]
struct Node {
    vertices: Vec<usize>,
    z: f64,
    radius: f64,
}
#[derive(Clone, Copy)]
enum Shape {
    Cylinder,
    Cone,
    Torus {
        center_z: f64,
        major: f64,
        minor: f64,
        v: [f64; 2],
    },
}
struct Section {
    ends: [usize; 2],
    shape: Shape,
    triangles: Vec<usize>,
}
#[derive(Clone, Copy)]
struct SphericalCap {
    boundary: usize,
    pole: usize,
    center_z: f64,
    radius: f64,
}
#[derive(Default)]
struct Errors {
    radial: f64,
    chord: f64,
    wall: f64,
}
impl Errors {
    fn merge(&mut self, e: &Self) {
        self.radial = self.radial.max(e.radial);
        self.chord = self.chord.max(e.chord);
        self.wall = self.wall.max(e.wall);
    }
}
struct Patch {
    frame: Frame,
    nodes: Vec<Node>,
    sections: Vec<Section>,
    sphere: Option<SphericalCap>,
    triangles: Vec<usize>,
}

pub fn recognize_rounded(a: &Analysis, o: &Options) -> Result<RoundedRecognition, FabError> {
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
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let segmentation = crate::planar::build_with_provenance(&a.mesh, &adjacency, tol)?;
    let mut work = Work::default();
    let mut edges: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    for (t, &[x, y, z]) in a.mesh.triangles.iter().enumerate() {
        for (u, v) in [(x, y), (y, z), (z, x)] {
            edges.entry(edge(u, v)).or_default().push(t);
        }
    }
    let mut rims = Vec::new();
    let mut cuts = BTreeSet::new();
    for (region, face) in segmentation.model.faces.iter().enumerate() {
        for lp in &face.loops {
            if lp.len() < 12 {
                continue;
            }
            work.spend(lp.len())?;
            let frame = Frame::new(a.mesh.vertices[lp[0]], face.normal);
            let points: Vec<_> = lp
                .iter()
                .map(|&v| {
                    let p = frame.project(a.mesh.vertices[v]);
                    [p[0], p[1]]
                })
                .collect();
            let Some(fit) = fit_circle(&points, o.cylinder_fit_tolerance_mm) else {
                continue;
            };
            cuts.extend(boundary(lp));
            rims.push(Rim {
                vertices: lp.clone(),
                region,
                center: frame.lift([fit.center[0], fit.center[1], 0.]),
                axis: frame.axis,
                radius: fit.radius,
            });
        }
    }
    if rims.is_empty() {
        return Err(unsupported());
    }
    if rims.len() > 128 {
        return Err(FabError::new(
            "recognition.too_complex",
            "Plus de 128 contours circulaires candidats.",
        ));
    }
    let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for rim in &rims {
        work.spend(components.len())?;
        let key = edge(rim.vertices[0], rim.vertices[1]);
        let Some(&start) = edges[&key]
            .iter()
            .find(|&&t| segmentation.triangle_regions[t] != rim.region)
        else {
            return Err(unproven("contour sans paroi adjacente"));
        };
        if components.values().any(|c| c.binary_search(&start).is_ok()) {
            continue;
        }
        let mut selected = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(t) = pending.pop() {
            work.spend(1)?;
            if !selected.insert(t) {
                continue;
            }
            let [x, y, z] = a.mesh.triangles[t];
            for (u, v) in [(x, y), (y, z), (z, x)] {
                let k = edge(u, v);
                if !cuts.contains(&k) {
                    pending.extend(edges[&k].iter().copied());
                }
            }
        }
        let key = *selected.first().unwrap();
        components.insert(key, selected.into_iter().collect());
    }
    let mut patches = Vec::new();
    let mut rounded = false;
    let mut failure = None;
    for triangles in components.into_values() {
        work.spend(triangles.len() + rims.iter().map(|r| r.vertices.len()).sum::<usize>())?;
        let selected: BTreeSet<_> = triangles.iter().copied().collect();
        let actual = patch_boundary(a, &triangles, &selected, &edges);
        let boundary_rims: Vec<_> = rims
            .iter()
            .filter(|r| boundary(&r.vertices).is_subset(&actual))
            .cloned()
            .collect();
        let expected: BTreeSet<_> = boundary_rims
            .iter()
            .flat_map(|r| boundary(&r.vertices))
            .collect();
        if expected != actual || boundary_rims.is_empty() || boundary_rims.len() > 2 {
            failure = Some(unproven("frontières de patch non circulaires ou ambiguës"));
            continue;
        }
        match classify(a, triangles, &boundary_rims, &edges, o, &mut work) {
            Ok(p) => {
                rounded |= p.sphere.is_some()
                    || p.sections
                        .iter()
                        .any(|s| matches!(s.shape, Shape::Torus { .. }))
                    // Only the newly proved opposed pair opts into this UV
                    // path without a rounded face. Older single-cone sources
                    // retain their historical Primitives fallback.
                    || p.sections.iter().filter(|s| matches!(s.shape, Shape::Cone)).count() == 2;
                patches.push(p);
            }
            Err(e) if e.code == "recognition.surface_unproven" => failure = Some(e),
            Err(e) => return Err(e),
        }
    }
    if !rounded {
        if let Some(e) = failure {
            return Err(e);
        }
        return Err(unsupported());
    }
    if let Some(e) = failure {
        return Err(e);
    }
    let mut model = RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                vertices: a.mesh.vertices.clone(),
                rings: Vec::new(),
                planes: Vec::new(),
                cylinders: Vec::new(),
                arcs: Vec::new(),
                partial_cylinders: Vec::new(),
            },
            cones: Vec::new(),
        },
        spheres: Vec::new(),
        tori: Vec::new(),
    };
    let mut owners = vec![None; a.mesh.triangles.len()];
    let mut ring_map = BTreeMap::new();
    let mut errors = Errors::default();
    for patch in patches {
        let offset = model.revolved.mixed.rings.len();
        for (n, node) in patch.nodes.iter().enumerate() {
            model.revolved.mixed.rings.push(CircularRing {
                center: patch.frame.lift([0., 0., node.z]),
                axis: patch.frame.axis,
                radius_mm: node.radius,
            });
            if node.vertices.len() > 1 {
                ring_map.insert(boundary(&node.vertices), offset + n);
            }
        }
        if let Some(sphere) = patch.sphere {
            let (e, inner) = prove_sphere(a, &patch, sphere, &edges, o, &mut work)?;
            errors.merge(&e);
            let index = model.spheres.len();
            let mut ends = [sphere.boundary, sphere.pole];
            if patch.nodes[ends[0]].z > patch.nodes[ends[1]].z {
                ends.swap(0, 1);
            }
            model.spheres.push(MixedSphere {
                rings: Some(ends.map(|n| offset + n)),
                center: patch.frame.lift([0., 0., sphere.center_z]),
                radius_mm: sphere.radius,
                inner,
            });
            for &t in &patch.triangles {
                if owners[t]
                    .replace(RoundedFaceRef::Sphere { index })
                    .is_some()
                {
                    return Err(unproven("triangle attribué à plusieurs patchs"));
                }
            }
        } else {
            for section in &patch.sections {
                let (e, inner) = prove_section(a, &patch, section, &edges, o, &mut work)?;
                errors.merge(&e);
                let rings = section.ends.map(|n| offset + n);
                let owner = match section.shape {
                    Shape::Cylinder => {
                        let index = model.revolved.mixed.cylinders.len();
                        model
                            .revolved
                            .mixed
                            .cylinders
                            .push(MixedCylinder { rings, inner });
                        RoundedFaceRef::Cylinder { index }
                    }
                    Shape::Cone => {
                        let index = model.revolved.cones.len();
                        model.revolved.cones.push(MixedCone { rings, inner });
                        RoundedFaceRef::Cone { index }
                    }
                    Shape::Torus {
                        center_z,
                        major,
                        minor,
                        v,
                    } => {
                        let index = model.tori.len();
                        model.tori.push(MixedTorus {
                            rings: Some(rings),
                            axis: None,
                            center: patch.frame.lift([0., 0., center_z]),
                            major_radius_mm: major,
                            minor_radius_mm: minor,
                            meridian_rad: v,
                            inner,
                        });
                        RoundedFaceRef::Torus { index }
                    }
                };
                for &t in &section.triangles {
                    if owners[t].replace(owner.clone()).is_some() {
                        return Err(unproven("triangle attribué à plusieurs surfaces"));
                    }
                }
            }
        }
    }
    // Source facets left outside the proven full circular patches are retained.
    // The existing partial-cylinder proof can then operate on those real planes.
    let mut counts = vec![(0_usize, 0_usize); segmentation.model.faces.len()];
    for (t, &r) in segmentation.triangle_regions.iter().enumerate() {
        counts[r].0 += 1;
        counts[r].1 += usize::from(owners[t].is_some());
    }
    let mut plane_map = BTreeMap::new();
    for (region, face) in segmentation.model.faces.iter().enumerate() {
        let (total, used) = counts[region];
        if used == total {
            continue;
        }
        if used != 0 {
            return Err(unproven(format!(
                "région plane {region} partiellement remplacée"
            )));
        }
        let index = model.revolved.mixed.planes.len();
        plane_map.insert(region, index);
        let loops = face
            .loops
            .iter()
            .map(|lp| {
                if let Some(&ring) = ring_map.get(&boundary(lp)) {
                    MixedLoop::Circle { ring }
                } else {
                    MixedLoop::Polygon {
                        vertices: lp.clone(),
                    }
                }
            })
            .collect();
        model.revolved.mixed.planes.push(MixedPlane {
            normal: face.normal,
            loops,
        });
    }
    let mut partial_owners = Vec::with_capacity(owners.len());
    for (t, owner) in owners.iter().enumerate() {
        partial_owners.push(Some(if owner.is_some() {
            MixedFaceRef::Cylinder { index: usize::MAX }
        } else {
            MixedFaceRef::Plane {
                index: plane_map[&segmentation.triangle_regions[t]],
            }
        }));
    }
    let partial = crate::partial::augment(
        a,
        &mut model.revolved.mixed,
        &mut partial_owners,
        o,
        &mut work,
    )?;
    errors.radial = errors.radial.max(partial.radial);
    errors.chord = errors.chord.max(partial.chord);
    errors.wall = errors.wall.max(partial.wall);
    for (t, p) in partial_owners.into_iter().enumerate() {
        match p.unwrap() {
            MixedFaceRef::Cylinder { index } if index == usize::MAX => {}
            MixedFaceRef::Plane { index } => {
                owners[t] = Some(RoundedFaceRef::Plane { index });
                let plane = &model.revolved.mixed.planes[index];
                let normal = scale(plane.normal, 1. / norm(plane.normal));
                let origin = crate::partial::loop_origin(&model.revolved.mixed, &plane.loops[0]);
                for &v in &a.mesh.triangles[t] {
                    errors.wall = errors
                        .wall
                        .max(dot(sub(a.mesh.vertices[v], origin), normal).abs());
                }
            }
            MixedFaceRef::PartialCylinder { index } => {
                owners[t] = Some(RoundedFaceRef::PartialCylinder { index })
            }
            MixedFaceRef::Cylinder { .. } => {
                return Err(unproven("attribution de cylindre incohérente"))
            }
        }
    }
    // Internal latitude samples are proof data, not BRep boundaries. Remove rings
    // with no analytic/plane incidence and remap every retained reference.
    compact_rings(&mut model)?;
    let bound = o.weld_tolerance_mm + errors.radial + errors.chord + errors.wall;
    if !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            format!(
                "Borne composée des surfaces {bound:.9} mm supérieure au budget {:.9} mm.",
                o.max_deviation_mm
            ),
        ));
    }
    let volume = volume(&model)?;
    let spheres = model.spheres.len();
    let tori = model.tori.len();
    let cones = model.revolved.cones.len();
    let cylinders =
        model.revolved.mixed.cylinders.len() + model.revolved.mixed.partial_cylinders.len();
    let planes = model.revolved.mixed.planes.len();
    let intent = if spheres + tori == 0 {
        Diagnostic { code: "recognition.conic_intent_assumed".into(), severity: Severity::Warning,
            count: cones, message: "Les bandes circulaires à rayon affine sont interprétées comme cônes dans le mode explicitement activé.".into() }
    } else {
        Diagnostic { code: "recognition.rounded_intent_assumed".into(), severity: Severity::Warning,
            count: spheres+tori, message: "Les patchs admissibles sont interprétés comme sphères ou raccords toriques dans le mode explicitement activé.".into() }
    };
    Ok(RoundedRecognition{model,triangle_faces:owners.into_iter().map(Option::unwrap).collect(),report:RecognitionReport{cylinders,cones,spheres,tori,toroidal_fillets:tori,max_radial_residual_mm:errors.radial,max_chord_error_mm:errors.chord,max_wall_residual_mm:errors.wall,max_deviation_bound_mm:bound,expected_volume_mm3:volume,diagnostics:vec![intent,Diagnostic{code:"recognition.rounded_coverage".into(),severity:Severity::Info,count:a.mesh.triangles.len(),message:format!("Tous les triangles sont attribués : {spheres} sphères, {tori} tores, {cones} cônes, {cylinders} cylindres et {planes} plans. Frontières, raccords et pôles issus de la source.")},Diagnostic{code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,message:"Borne composée : soudage, résidus de projection, flèche sphérique ou borne paramétrique des triangles. Validation native et audit bidirectionnel requis ; pas une mesure Hausdorff globale.".into()}]}})
}
fn patch_boundary(
    a: &Analysis,
    triangles: &[usize],
    selected: &BTreeSet<usize>,
    edges: &BTreeMap<Edge, Vec<usize>>,
) -> BTreeSet<Edge> {
    let mut out = BTreeSet::new();
    for &t in triangles {
        let [x, y, z] = a.mesh.triangles[t];
        for (u, v) in [(x, y), (y, z), (z, x)] {
            let k = edge(u, v);
            if edges[&k].iter().any(|i| !selected.contains(i)) {
                out.insert(k);
            }
        }
    }
    out
}
fn topology(
    a: &Analysis,
    triangles: &[usize],
    rings: &[&Node],
    edges: &BTreeMap<Edge, Vec<usize>>,
) -> Result<(), FabError> {
    let selected: BTreeSet<_> = triangles.iter().copied().collect();
    let actual = patch_boundary(a, triangles, &selected, edges);
    let expected: BTreeSet<_> = rings
        .iter()
        .filter(|n| n.radius > 0.)
        .flat_map(|n| boundary(&n.vertices))
        .collect();
    if actual != expected {
        return Err(unproven(
            "frontière triangulaire différente des anneaux prouvés",
        ));
    }
    let vertices: BTreeSet<_> = triangles
        .iter()
        .flat_map(|&t| a.mesh.triangles[t])
        .collect();
    let all_edges: BTreeSet<_> = triangles
        .iter()
        .flat_map(|&t| {
            let [x, y, z] = a.mesh.triangles[t];
            [edge(x, y), edge(y, z), edge(z, x)]
        })
        .collect();
    let loops = rings.iter().filter(|n| n.radius > 0.).count();
    let chi = vertices.len() as isize - all_edges.len() as isize + triangles.len() as isize;
    if chi != 2 - loops as isize {
        return Err(unproven(
            "patch avec trou ou topologie autre qu'un disque/anneau",
        ));
    }
    Ok(())
}

fn classify(
    a: &Analysis,
    triangles: Vec<usize>,
    rims: &[Rim],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Patch, FabError> {
    let tol = o.plane_tolerance_mm.max(o.cylinder_fit_tolerance_mm);
    let frame = Frame::new(rims[0].center, rims[0].axis);
    let vertices: BTreeSet<_> = triangles
        .iter()
        .flat_map(|&t| a.mesh.triangles[t])
        .collect();
    work.spend(vertices.len() + triangles.len())?;
    let projected: BTreeMap<_, _> = vertices
        .iter()
        .map(|&v| (v, frame.project(a.mesh.vertices[v])))
        .collect();
    let mut nodes: Vec<_> = rims
        .iter()
        .map(|r| {
            let p = frame.project(r.center);
            if p[0].hypot(p[1]) > tol || dot(r.axis, frame.axis) < 1. - 1e-10 {
                return Err(unproven("frontières non coaxiales"));
            }
            Ok(Node {
                vertices: r.vertices.clone(),
                z: p[2],
                radius: r.radius,
            })
        })
        .collect::<Result<_, _>>()?;
    if nodes.len() == 1 {
        let base = nodes[0].z;
        let tip = vertices
            .iter()
            .copied()
            .filter(|v| projected[v][0].hypot(projected[v][1]) <= tol)
            .max_by(|x, y| {
                (projected[x][2] - base)
                    .abs()
                    .total_cmp(&(projected[y][2] - base).abs())
            });
        if let Some(tip) = tip {
            let z = projected[&tip][2];
            let radius = (z - base).abs();
            let residual = projected
                .values()
                .map(|p| (p[0].hypot(p[1]).hypot(p[2] - base) - radius).abs())
                .fold(0., f64::max);
            if radius > tol * 4.
                && (radius - nodes[0].radius).abs() <= o.cylinder_fit_tolerance_mm
                && residual <= o.cylinder_fit_tolerance_mm
                && projected
                    .values()
                    .all(|p| (p[2] - base) * (z - base) >= -tol * radius)
            {
                nodes[0].radius = radius;
                nodes.push(Node {
                    vertices: vec![tip],
                    z,
                    radius: 0.,
                });
                return Ok(Patch {
                    frame,
                    nodes,
                    sections: Vec::new(),
                    sphere: Some(SphericalCap {
                        boundary: 0,
                        pole: 1,
                        center_z: base,
                        radius,
                    }),
                    triangles,
                });
            }
            // A planar circle and a real axial pole determine a unique sphere:
            // R=(a²+h²)/(2h), C_z=P_z-sign(h)*R. Keep both demonstrated
            // boundaries exact instead of introducing independently fitted
            // radii that would leave native sewing gaps. The existing
            // hemisphere interpretation above remains first for compatibility.
            let height = (z - base).abs();
            let sign = (z - base).signum();
            let rim_radius = nodes[0].radius;
            let cap_radius = 0.5 * (height + rim_radius * (rim_radius / height));
            let center_z = base + sign * (height - cap_radius);
            if height > 4. * tol
                && cap_radius.is_finite()
                && center_z.is_finite()
                && cap_radius * f64::EPSILON * 128. <= o.cylinder_fit_tolerance_mm
                && projected.values().all(|p| (p[2] - base) * sign >= -tol)
            {
                work.spend(projected.len().saturating_mul(4))?;
                // Ring/pole samples alone also describe a shallow cone.
                // Require resolved interior curvature at multiple heights,
                // never infer a spherical cap from its boundary alone.
                let mut evidence = 0;
                let mut lowest = f64::INFINITY;
                let mut highest = f64::NEG_INFINITY;
                let cone_normalization = 1_f64.hypot(rim_radius / height);
                for p in projected.values() {
                    let depth = (p[2] - base) * sign;
                    let conical_radius = rim_radius * (1. - depth / height);
                    if depth > 4. * tol
                        && height - depth > 4. * tol
                        && (p[0].hypot(p[1]) - conical_radius).abs() / cone_normalization
                            > o.cylinder_fit_tolerance_mm
                    {
                        evidence += 1;
                        lowest = lowest.min(depth);
                        highest = highest.max(depth);
                    }
                }
                let height_range = highest - lowest;
                let residual = projected
                    .values()
                    .map(|p| (p[0].hypot(p[1]).hypot(p[2] - center_z) - cap_radius).abs())
                    .fold(0., f64::max);
                if evidence >= 12
                    && height_range > 4. * tol
                    && residual <= o.cylinder_fit_tolerance_mm
                {
                    nodes.push(Node {
                        vertices: vec![tip],
                        z,
                        radius: 0.,
                    });
                    return Ok(Patch {
                        frame,
                        nodes,
                        sections: Vec::new(),
                        sphere: Some(SphericalCap {
                            boundary: 0,
                            pole: 1,
                            center_z,
                            radius: cap_radius,
                        }),
                        triangles,
                    });
                }
            }
            nodes.push(Node {
                vertices: vec![tip],
                z,
                radius: 0.,
            });
        } else {
            return Err(unproven("patch à un anneau sans pôle source coaxial"));
        }
    }
    // Discover genuine complete intermediate circles from horizontal source edges.
    let selected: BTreeSet<_> = triangles.iter().copied().collect();
    let mut incident: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    work.spend(edges.len())?;
    for (&(u, v), uses) in edges {
        if uses.iter().any(|t| selected.contains(t))
            && projected.contains_key(&u)
            && projected.contains_key(&v)
            && (projected[&u][2] - projected[&v][2]).abs() <= tol
        {
            incident.entry(u).or_default().push(v);
            incident.entry(v).or_default().push(u);
        }
    }
    let mut visited = BTreeSet::new();
    for &start in incident.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            work.spend(1)?;
            if !visited.insert(v) {
                continue;
            }
            comp.push(v);
            stack.extend(incident[&v].iter().copied());
        }
        if comp.len() < 12 || comp.iter().any(|v| incident[v].len() != 2) {
            continue;
        }
        let mut lp = vec![start];
        let mut previous = start;
        let mut current = incident[&start][0];
        while current != start {
            work.spend(1)?;
            lp.push(current);
            if lp.len() > comp.len() {
                return Err(unproven("cycle circulaire incohérent"));
            }
            let next = *incident[&current].iter().find(|&&v| v != previous).unwrap();
            previous = current;
            current = next;
        }
        let points: Vec<_> = lp
            .iter()
            .map(|v| [projected[v][0], projected[v][1]])
            .collect();
        let Some(fit) = fit_circle(&points, o.cylinder_fit_tolerance_mm) else {
            continue;
        };
        if fit.center[0].hypot(fit.center[1]) > tol {
            continue;
        }
        let z = lp.iter().map(|v| projected[v][2]).sum::<f64>() / lp.len() as f64;
        if lp.iter().any(|v| (projected[v][2] - z).abs() > tol) {
            continue;
        }
        let radius = lp
            .iter()
            .map(|v| projected[v][0].hypot(projected[v][1]))
            .sum::<f64>()
            / lp.len() as f64;
        work.spend(nodes.len() + lp.len())?;
        if nodes
            .iter()
            .any(|n| (n.z - z).abs() <= tol && (n.radius - radius).abs() <= tol)
        {
            continue;
        }
        nodes.push(Node {
            vertices: lp,
            z,
            radius,
        });
    }
    nodes.sort_by(|a, b| a.z.total_cmp(&b.z));
    if nodes.windows(2).any(|w| w[1].z - w[0].z <= 4. * tol) {
        return Err(unproven("niveaux source ambigus ou trop rapprochés"));
    }
    let mut breaks = BTreeSet::from([0, nodes.len() - 1]);
    let mut runs = Vec::new();
    let mut i = 0;
    while i + 1 < nodes.len() {
        if (nodes[i + 1].radius - nodes[i].radius).abs() <= 2. * o.cylinder_fit_tolerance_mm {
            let start = i;
            let mut end = i + 1;
            while end + 1 < nodes.len()
                && (nodes[end + 1].radius - nodes[start].radius).abs()
                    <= 2. * o.cylinder_fit_tolerance_mm
            {
                end += 1;
            }
            let radius = (nodes[start].radius + nodes[end].radius) * 0.5;
            let z0 = nodes[start].z;
            let z1 = nodes[end].z;
            work.spend(projected.len())?;
            let fits = projected
                .values()
                .filter(|p| p[2] >= z0 - tol && p[2] <= z1 + tol)
                .all(|p| (p[0].hypot(p[1]) - radius).abs() <= o.cylinder_fit_tolerance_mm);
            if fits {
                breaks.insert(start);
                breaks.insert(end);
                runs.push([start, end]);
                for n in &mut nodes[start..=end] {
                    n.radius = radius;
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    let split: Vec<_> = breaks.into_iter().collect();
    let mut sections = Vec::new();
    for pair in split.windows(2) {
        work.spend(triangles.len() + runs.len())?;
        let ends = [pair[0], pair[1]];
        let low = &nodes[ends[0]];
        let high = &nodes[ends[1]];
        let z0 = low.z;
        let z1 = high.z;
        let height = z1 - z0;
        let selected_triangles: Vec<_> = triangles
            .iter()
            .copied()
            .filter(|&t| {
                a.mesh.triangles[t]
                    .iter()
                    .all(|v| projected[v][2] >= z0 - tol && projected[v][2] <= z1 + tol)
            })
            .collect();
        let ids: BTreeSet<_> = selected_triangles
            .iter()
            .flat_map(|&t| a.mesh.triangles[t])
            .collect();
        if selected_triangles.is_empty() {
            return Err(unproven("portion sans triangles source"));
        }
        let linear = ids
            .iter()
            .map(|v| {
                let p = projected[v];
                (p[0].hypot(p[1])
                    - (low.radius + (high.radius - low.radius) * (p[2] - z0) / height))
                    .abs()
            })
            .fold(0., f64::max);
        let shape = if runs.contains(&ends) {
            Shape::Cylinder
        } else if linear <= o.cylinder_fit_tolerance_mm {
            Shape::Cone
        } else {
            let lower_cylinder = runs.iter().any(|r| r[1] == ends[0]);
            let upper_cylinder = runs.iter().any(|r| r[0] == ends[1]);
            let (major, center_z, v) =
                if lower_cylinder && ends[1] == nodes.len() - 1 && low.radius > high.radius {
                    (high.radius, z0, [0., PI / 2.])
                } else if upper_cylinder && ends[0] == 0 && low.radius < high.radius {
                    (low.radius, z1, [-PI / 2., 0.])
                } else {
                    return Err(unproven(
                        "méridienne non affine sans raccord cylindre-plan démontré",
                    ));
                };
            let minor = height;
            if major <= minor
                || ((low.radius - high.radius).abs() - minor).abs()
                    > 2. * o.cylinder_fit_tolerance_mm
            {
                return Err(unproven(
                    "rayons incompatibles avec un raccord torique tangent",
                ));
            }
            // Tangency is constrained by actual shared boundary circles/planes, never
            // by a rounded nominal design parameter. All resulting movements are fit residuals.
            let cylinder_end = if lower_cylinder { ends[0] } else { ends[1] };
            let run = runs
                .iter()
                .find(|r| r[0] == cylinder_end || r[1] == cylinder_end)
                .copied()
                .unwrap();
            for n in &mut nodes[run[0]..=run[1]] {
                n.radius = major + minor;
            }
            let residual = ids
                .iter()
                .map(|v| {
                    let p = projected[v];
                    ((p[0].hypot(p[1]) - major).hypot(p[2] - center_z) - minor).abs()
                })
                .fold(0., f64::max);
            if residual > o.cylinder_fit_tolerance_mm {
                return Err(unproven(format!(
                    "résidu du raccord torique {residual:.9} mm"
                )));
            }
            if ids
                .iter()
                .map(|v| projected[v][2].to_bits())
                .collect::<BTreeSet<_>>()
                .len()
                < 4
            {
                return Err(unproven(
                    "raccord torique insuffisamment échantillonné en méridienne",
                ));
            }
            Shape::Torus {
                center_z,
                major,
                minor,
                v,
            }
        };
        sections.push(Section {
            ends,
            shape,
            triangles: selected_triangles,
        });
    }
    work.spend(sections.len())?;
    let cones = sections
        .iter()
        .filter(|s| matches!(s.shape, Shape::Cone))
        .count();
    let tori = sections
        .iter()
        .filter(|s| matches!(s.shape, Shape::Torus { .. }))
        .count();
    let double_chamfer = if let [left, land, right] = sections.as_slice() {
        let profile = [left.ends[0], land.ends[0], land.ends[1], right.ends[1]]
            .map(|n| [nodes[n].z, nodes[n].radius]);
        matches!(left.shape, Shape::Cone)
            && matches!(land.shape, Shape::Cylinder)
            && matches!(right.shape, Shape::Cone)
            && crate::revolved::resolved_double_chamfer(
                profile,
                o.cylinder_fit_tolerance_mm
                    .max(o.plane_tolerance_mm)
                    .max(o.weld_tolerance_mm),
            )
    } else {
        false
    };
    if (cones > 1 && !double_chamfer) || tori > 1 {
        return Err(unproven(
            "plusieurs portions coniques ou toriques sans deux pentes opposées et portée cylindrique résolue",
        ));
    }
    let assigned: Vec<_> = sections
        .iter()
        .flat_map(|s| s.triangles.iter().copied())
        .collect();
    if assigned.len() != triangles.len()
        || assigned.iter().copied().collect::<BTreeSet<_>>().len() != triangles.len()
    {
        return Err(unproven("triangles traversant une frontière de raccord"));
    }
    Ok(Patch {
        frame,
        nodes,
        sections,
        sphere: None,
        triangles,
    })
}

fn prove_sphere(
    a: &Analysis,
    p: &Patch,
    cap: SphericalCap,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<(Errors, bool), FabError> {
    work.spend(p.triangles.len())?;
    let SphericalCap {
        boundary,
        pole,
        center_z,
        radius,
    } = cap;
    topology(
        a,
        &p.triangles,
        &[&p.nodes[boundary], &p.nodes[pole]],
        edges,
    )?;
    let center = [0., 0., center_z];
    let rim = &p.nodes[boundary];
    let boundary_z = rim.z - center_z;
    let pole_vertex = p.nodes[pole].vertices[0];
    let boundary_vertices: BTreeSet<_> = rim.vertices.iter().copied().collect();
    let sign = (p.nodes[pole].z - center_z).signum();
    let mut projected = BTreeMap::new();
    let mut errors = Errors::default();
    for v in p
        .triangles
        .iter()
        .flat_map(|&t| a.mesh.triangles[t])
        .collect::<BTreeSet<_>>()
    {
        work.spend(1)?;
        let actual = sub(p.frame.project(a.mesh.vertices[v]), center);
        let length = norm(actual);
        if length == 0. || (actual[2] - boundary_z) * sign < -o.cylinder_fit_tolerance_mm {
            return Err(unproven("sommet hors du domaine de la calotte sphérique"));
        }
        let ideal = project_cap_vertex(
            actual,
            radius,
            [rim.radius, boundary_z],
            sign,
            v == pole_vertex,
            boundary_vertices.contains(&v),
        )?;
        let residual = norm(sub(actual, ideal));
        if residual > o.cylinder_fit_tolerance_mm {
            return Err(unproven(format!(
                "sommet sphérique {v} : résidu {residual:.9} mm"
            )));
        }
        errors.radial = errors.radial.max(residual);
        projected.insert(v, ideal);
    }
    let mut area = Sum::default();
    let mut inner = None;
    for &t in &p.triangles {
        work.spend(1)?;
        let [u, v, w] = a.mesh.triangles[t].map(|i| projected[&i]);
        let normalized = [u, v, w].map(|x| scale(x, 1. / radius));
        for i in 0..3 {
            let cosine = dot(normalized[i], normalized[(i + 1) % 3]).clamp(-1., 1.);
            if cosine < (PI / 4.).cos() {
                return Err(unproven("triangle sphérique trop peu échantillonné"));
            }
        }
        let triple = dot(normalized[0], cross(normalized[1], normalized[2]));
        if !triple.is_finite() || triple.abs() < 1e-15 {
            return Err(unproven("triangle sphérique dégénéré ou méridien replié"));
        }
        let current = triple < 0.;
        if inner.is_some_and(|old| old != current) {
            return Err(unproven("projection sphérique repliée"));
        }
        inner = Some(current);
        let denominator = 1.
            + dot(normalized[0], normalized[1])
            + dot(normalized[1], normalized[2])
            + dot(normalized[2], normalized[0]);
        area.add(2. * triple.abs().atan2(denominator));
        errors.chord = errors.chord.max(radius - min_triangle_norm(u, v, w));
    }
    // The radial image of a mesh boundary follows great-circle arcs, not
    // the analytic cap's small circle. Its degree-one reference area is the
    // geodesic fan from the demonstrated pole to the actual circular rim.
    let pole_unit = [0., 0., sign];
    let mut expected = Sum::default();
    let mut winding = Sum::default();
    let mut direction = None;
    let mut fringe: f64 = 0.;
    for i in 0..rim.vertices.len() {
        work.spend(1)?;
        let u = projected[&rim.vertices[i]];
        let v = projected[&rim.vertices[(i + 1) % rim.vertices.len()]];
        let du = (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]);
        if du.abs() < 1e-12 || du.abs() > PI / 4. || direction.is_some_and(|old| old != (du < 0.)) {
            return Err(unproven(
                "frontière sphérique repliée ou insuffisamment échantillonnée",
            ));
        }
        direction = Some(du < 0.);
        winding.add(du.abs());
        let u = scale(u, 1. / radius);
        let v = scale(v, 1. / radius);
        let triple = dot(pole_unit, cross(u, v)).abs();
        expected.add(2. * triple.atan2(1. + dot(pole_unit, u) + dot(u, v) + dot(v, pole_unit)));
        // At fixed azimuth the greatest latitude drift of the geodesic rim
        // occurs at its chord midpoint. This chord distance on the sphere
        // bounds both directions between the geodesic polygon and the cap,
        // for small AND large caps. It is zero at the equator.
        let latitude = boundary_z.atan2(rim.radius);
        let geodesic_latitude = boundary_z.atan2(rim.radius * (du / 2.).cos());
        fringe = fringe.max(2. * radius * ((geodesic_latitude - latitude).abs() / 2.).sin());
    }
    if (winding.get() - TAU).abs() > 1e-6 || (area.get() - expected.get()).abs() > 1e-6 {
        return Err(unproven(format!(
            "couverture sphérique {} stéradians, polygone source {}",
            area.get(),
            expected.get()
        )));
    }
    // Radial projection covers the geodesic polygon once: the exact interior
    // sagitta bounds the ideal triangles, and triangle inequality adds the
    // boundary fringe needed to cover precisely the analytic trimmed cap.
    errors.chord += fringe;
    Ok((errors, inner.unwrap()))
}
/// Exact domain projection used by the proof; its full vector displacement
/// is accounted separately before any interior or trim-fringe bound is added.
fn project_cap_vertex(
    actual: V,
    radius: f64,
    rim: [f64; 2],
    sign: f64,
    pole: bool,
    boundary: bool,
) -> Result<V, FabError> {
    let radial = scale(actual, radius / norm(actual));
    if pole {
        return Ok([0., 0., sign * radius]);
    }
    if boundary || (radial[2] - rim[1]) * sign < 0. {
        // Clamp the projected point, not just the source point: a large
        // cap's radial projection can cross its small-circle trim.
        let rho = actual[0].hypot(actual[1]);
        if rho == 0. {
            return Err(unproven("sommet de frontière sphérique sur l'axe"));
        }
        Ok([rim[0] * actual[0] / rho, rim[0] * actual[1] / rho, rim[1]])
    } else {
        Ok(radial)
    }
}

pub(crate) fn min_triangle_norm(a: V, b: V, c: V) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let n2 = dot(n, n);
    if n2 > 0. {
        let q = scale(n, dot(n, a) / n2);
        let aq = sub(q, a);
        let aa = dot(ab, ab);
        let bb = dot(ab, ac);
        let cc = dot(ac, ac);
        let d = aa * cc - bb * bb;
        if d > 0. {
            let u = (dot(aq, ab) * cc - dot(aq, ac) * bb) / d;
            let v = (dot(aq, ac) * aa - dot(aq, ab) * bb) / d;
            if u >= 0. && v >= 0. && u + v <= 1. {
                return norm(q);
            }
        }
    }
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .map(|(p, q)| {
            let d = sub(q, p);
            let t = (-dot(p, d) / dot(d, d)).clamp(0., 1.);
            norm(std::array::from_fn(|i| p[i] + t * d[i]))
        })
        .fold(f64::INFINITY, f64::min)
}

fn prove_section(
    a: &Analysis,
    p: &Patch,
    s: &Section,
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<(Errors, bool), FabError> {
    work.spend(s.triangles.len())?;
    let low = &p.nodes[s.ends[0]];
    let high = &p.nodes[s.ends[1]];
    topology(a, &s.triangles, &[low, high], edges)?;
    let height = high.z - low.z;
    if height <= 0. {
        return Err(unproven("portion axiale inversée"));
    }
    if low.radius == 0. || high.radius == 0. {
        return prove_pointed_cone(a, p, s, o, work);
    }
    let low_ids: BTreeSet<_> = low.vertices.iter().copied().collect();
    let high_ids: BTreeSet<_> = high.vertices.iter().copied().collect();
    let (v0, v1) = match s.shape {
        Shape::Torus { v, .. } => (v[0], v[1]),
        _ => (low.z, high.z),
    };
    let mut uv = BTreeMap::new();
    let mut errors = Errors::default();
    for id in s
        .triangles
        .iter()
        .flat_map(|&t| a.mesh.triangles[t])
        .collect::<BTreeSet<_>>()
    {
        work.spend(1)?;
        let q = p.frame.project(a.mesh.vertices[id]);
        let rho = q[0].hypot(q[1]);
        if rho <= 0. {
            return Err(unproven("sommet sur l'axe hors d'un pôle démontré"));
        }
        let u = q[1].atan2(q[0]);
        let mut v = match s.shape {
            Shape::Torus {
                center_z, major, ..
            } => (q[2] - center_z).atan2(rho - major),
            _ => q[2],
        };
        if low_ids.contains(&id) {
            v = v0;
        } else if high_ids.contains(&id) {
            v = v1;
        } else {
            v = v.clamp(v0, v1);
        }
        let (radius, z) = match s.shape {
            Shape::Cylinder => (low.radius, v),
            Shape::Cone => (
                low.radius + (high.radius - low.radius) * (v - low.z) / height,
                v,
            ),
            Shape::Torus {
                center_z,
                major,
                minor,
                ..
            } => (major + minor * v.cos(), center_z + minor * v.sin()),
        };
        let ideal = [radius * u.cos(), radius * u.sin(), z];
        let residual = norm(sub(q, ideal));
        if residual > o.cylinder_fit_tolerance_mm {
            return Err(unproven(format!(
                "sommet {id} hors de sa surface : résidu {residual:.9} mm"
            )));
        }
        errors.radial = errors.radial.max(residual);
        uv.insert(id, [u, v]);
    }
    let mut area = Sum::default();
    let mut inner = None;
    for &t in &s.triangles {
        work.spend(1)?;
        let mut q = a.mesh.triangles[t].map(|v| uv[&v]);
        for i in 1..3 {
            while q[i][0] - q[0][0] > PI {
                q[i][0] -= TAU;
            }
            while q[i][0] - q[0][0] < -PI {
                q[i][0] += TAU;
            }
        }
        let du = q.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
            - q.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let dv = q.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max)
            - q.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        if du > PI / 4. + 1e-12 {
            return Err(unproven(format!(
                "triangle {t} : pas azimutal supérieur à 45°"
            )));
        }
        let determinant =
            (q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1]);
        if !determinant.is_finite() || determinant.abs() < 1e-14 {
            return Err(unproven(format!(
                "triangle {t} : image paramétrique dégénérée"
            )));
        }
        let current = determinant < 0.;
        if inner.is_some_and(|old| old != current) {
            return Err(unproven("paramétrisation de la paroi repliée"));
        }
        inner = Some(current);
        area.add(determinant.abs() * 0.5);
        let chord = match s.shape {
            Shape::Cylinder | Shape::Cone => {
                // For any barycentric point of the ideal facet, z and the
                // conical radius are affine: r(z)=sum(lambda_i*r_i). All
                // radial directions lie in an angular sector of width du.
                // Their convex sum has length at least r(z)*cos(du/2), so
                // radial projection to the cone moves it by no more than
                // r_max*(1-cos(du/2)). No UV mixed-derivative estimate is
                // asserted for these ruled surfaces.
                2. * low.radius.max(high.radius) * (du / 4.).sin().powi(2)
            }
            Shape::Torus { major, minor, .. } => {
                if dv > PI / 4. + 1e-12 {
                    return Err(unproven("pas méridien du tore supérieur à 45°"));
                }
                // Norms of S_uu, S_uv, S_vv are bounded respectively by
                // major+minor, minor, minor. Taylor's integral remainder
                // around a barycentric UV point and Var(x)<=range(x)^2/4
                // give this bound on every point of the triangle, including
                // its interior. The oriented degree-one UV coverage check
                // below supplies the corresponding analytic patch coverage.
                ((major + minor) * du * du + 2. * minor * du * dv + minor * dv * dv) / 8.
            }
        };
        errors.chord = errors.chord.max(chord);
    }
    let expected = TAU * (v1 - v0);
    if (area.get() - expected).abs() > expected.abs() * 1e-7 + 1e-9 {
        return Err(unproven(format!(
            "couverture UV {} différente de {expected}",
            area.get()
        )));
    }
    Ok((errors, inner.unwrap()))
}
fn prove_pointed_cone(
    a: &Analysis,
    p: &Patch,
    s: &Section,
    o: &Options,
    work: &mut Work,
) -> Result<(Errors, bool), FabError> {
    // Rounded mode need not reinterpret legacy pointed cones. This branch is
    // used only when a separately proved rounded patch shares the same body.
    let low = &p.nodes[s.ends[0]];
    let high = &p.nodes[s.ends[1]];
    if !matches!(s.shape, Shape::Cone) {
        return Err(unproven("pôle sur une surface non conique"));
    }
    let pole = if low.radius == 0. { low } else { high };
    let ring = if low.radius > 0. { low } else { high };
    if pole.vertices.len() != 1 {
        return Err(unproven("pointe source ambiguë"));
    }
    let apex = pole.vertices[0];
    let mut errors = Errors::default();
    let mut inner = None;
    let mut coverage = Sum::default();
    let ring_vertices: BTreeSet<_> = ring.vertices.iter().copied().collect();
    for &t in &s.triangles {
        work.spend(1)?;
        let ids = a.mesh.triangles[t];
        let mut q = ids.map(|v| p.frame.project(a.mesh.vertices[v]));
        let mut angles = Vec::new();
        for (i, &v) in ids.iter().enumerate() {
            let p = q[i];
            let ideal = if v == apex {
                [0., 0., pole.z]
            } else {
                let rho = p[0].hypot(p[1]);
                if rho == 0. {
                    return Err(unproven("sommet axial autre que la pointe source"));
                }
                let z = if ring_vertices.contains(&v) {
                    ring.z
                } else {
                    p[2].clamp(low.z, high.z)
                };
                let radius =
                    low.radius + (high.radius - low.radius) * (z - low.z) / (high.z - low.z);
                [radius * p[0] / rho, radius * p[1] / rho, z]
            };
            let residual = norm(sub(p, ideal));
            if residual > o.cylinder_fit_tolerance_mm {
                return Err(unproven("résidu de pointe conique"));
            }
            q[i] = ideal;
            errors.radial = errors.radial.max(residual);
            if v != apex {
                angles.push(p[1].atan2(p[0]));
            }
        }
        if angles.len() != 2 {
            return Err(unproven(
                "pointe conique à subdivisions non prises en charge dans ce patch",
            ));
        }
        let mut du = (angles[1] - angles[0]).abs();
        if du > PI {
            du = TAU - du;
        }
        if du > PI / 4. {
            return Err(unproven("pointe conique trop peu échantillonnée"));
        }
        coverage.add(du);
        let n = cross(sub(q[1], q[0]), sub(q[2], q[0]));
        let sense = dot(
            n,
            [
                (q[0][0] + q[1][0] + q[2][0]) / 3.,
                (q[0][1] + q[1][1] + q[2][1]) / 3.,
                0.,
            ],
        );
        if sense == 0. || inner.is_some_and(|old| old != (sense < 0.)) {
            return Err(unproven("orientation de pointe conique incohérente"));
        }
        inner = Some(sense < 0.);
        errors.chord = errors.chord.max(2. * ring.radius * (du / 4.).sin().powi(2));
    }
    if (coverage.get() - TAU).abs() > 1e-7 {
        return Err(unproven("éventail conique incomplet"));
    }
    Ok((errors, inner.unwrap()))
}

fn compact_rings(m: &mut RoundedModel) -> Result<(), FabError> {
    let n = m.revolved.mixed.rings.len();
    let mut count = vec![0_usize; n];
    let mut plane = vec![false; n];
    for p in &m.revolved.mixed.planes {
        for lp in &p.loops {
            if let MixedLoop::Circle { ring } = lp {
                count[*ring] += 1;
                plane[*ring] = true;
            }
        }
    }
    for rings in m
        .revolved
        .mixed
        .cylinders
        .iter()
        .map(|s| s.rings)
        .chain(m.revolved.cones.iter().map(|s| s.rings))
        .chain(m.spheres.iter().filter_map(|s| s.rings))
        .chain(m.tori.iter().filter_map(|s| s.rings))
    {
        for i in rings {
            count[i] += 1;
        }
    }
    let mut mapping = vec![usize::MAX; n];
    let mut rings = Vec::new();
    for (i, r) in m.revolved.mixed.rings.iter().enumerate() {
        if count[i] == 0 {
            continue;
        }
        if count[i] != if r.radius_mm == 0. { 1 } else { 2 } || (r.radius_mm == 0. && plane[i]) {
            return Err(unproven(format!(
                "anneau {i} : incidence {} invalide",
                count[i]
            )));
        }
        mapping[i] = rings.len();
        rings.push(r.clone());
    }
    for p in &mut m.revolved.mixed.planes {
        for lp in &mut p.loops {
            if let MixedLoop::Circle { ring } = lp {
                *ring = mapping[*ring];
            }
        }
    }
    for c in &mut m.revolved.mixed.cylinders {
        c.rings = c.rings.map(|i| mapping[i]);
    }
    for c in &mut m.revolved.cones {
        c.rings = c.rings.map(|i| mapping[i]);
    }
    for c in &mut m.spheres {
        c.rings = c.rings.map(|rings| rings.map(|i| mapping[i]));
    }
    for c in &mut m.tori {
        c.rings = c.rings.map(|rings| rings.map(|i| mapping[i]));
    }
    m.revolved.mixed.rings = rings;
    Ok(())
}
fn volume(m: &RoundedModel) -> Result<f64, FabError> {
    let mixed = &m.revolved.mixed;
    let bbox = bounds(&mixed.vertices);
    let reference = std::array::from_fn(|i| bbox[0][i] + (bbox[1][i] - bbox[0][i]) * 0.5);
    let mut sum = Sum::default();
    for plane in &mixed.planes {
        let normal = scale(plane.normal, 1. / norm(plane.normal));
        let origin = crate::partial::loop_origin(mixed, &plane.loops[0]);
        let frame = Frame::new(origin, normal);
        let mut area = Sum::default();
        for (i, lp) in plane.loops.iter().enumerate() {
            let a = match lp {
                MixedLoop::Polygon { vertices } => signed_area(
                    &vertices
                        .iter()
                        .map(|&v| {
                            let p = frame.project(mixed.vertices[v]);
                            [p[0], p[1]]
                        })
                        .collect::<Vec<_>>(),
                )
                .abs(),
                MixedLoop::Circle { ring } => PI * mixed.rings[*ring].radius_mm.powi(2),
                MixedLoop::Segments { segments } => {
                    crate::partial::segmented_area(mixed, segments, normal).abs()
                }
            };
            area.add(a * if i == 0 { 1. } else { -1. });
        }
        if !area.get().is_finite() || area.get() <= 0. {
            return Err(unproven("aire plane non positive"));
        }
        sum.add(dot(sub(origin, reference), normal) * area.get() / 3.);
    }
    for (rings, inner) in mixed
        .cylinders
        .iter()
        .map(|c| (c.rings, c.inner))
        .chain(m.revolved.cones.iter().map(|c| (c.rings, c.inner)))
    {
        let a = &mixed.rings[rings[0]];
        let b = &mixed.rings[rings[1]];
        let h = dot(sub(b.center, a.center), a.axis);
        let z = dot(sub(a.center, reference), a.axis);
        let flux = PI / 3.
            * (h * (a.radius_mm.powi(2) + a.radius_mm * b.radius_mm)
                + z * (a.radius_mm.powi(2) - b.radius_mm.powi(2)));
        sum.add(flux * if inner { -1. } else { 1. });
    }
    for c in &mixed.partial_cylinders {
        sum.add(crate::partial::cylinder_flux(mixed, c, reference));
    }
    for s in &m.spheres {
        let Some(rings) = s.rings else {
            return Err(unproven("sphère entière dans un modèle de patchs"));
        };
        let a = &mixed.rings[rings[0]];
        let b = &mixed.rings[rings[1]];
        let r = s.radius_mm;
        let low = (dot(sub(a.center, s.center), a.axis) / r).clamp(-1., 1.);
        let high = (dot(sub(b.center, s.center), a.axis) / r).clamp(-1., 1.);
        let z = dot(sub(s.center, reference), a.axis);
        let flux =
            TAU / 3. * (r.powi(3) * (high - low) + z * r * r * 0.5 * (high * high - low * low));
        sum.add(flux * if s.inner { -1. } else { 1. });
    }
    for t in &m.tori {
        let Some(rings) = t.rings else {
            return Err(unproven("tore entier dans un modèle de patchs"));
        };
        let axis = mixed.rings[rings[0]].axis;
        let z = dot(sub(t.center, reference), axis);
        let big = t.major_radius_mm;
        let small = t.minor_radius_mm;
        let integral = |v: f64| {
            (big * big + small * small) * v.sin() + big * small * (1.5 * v + 0.25 * (2. * v).sin())
                - z * big * v.cos()
                + 0.5 * z * small * v.sin().powi(2)
        };
        let flux = TAU * small / 3. * (integral(t.meridian_rad[1]) - integral(t.meridian_rad[0]));
        sum.add(flux * if t.inner { -1. } else { 1. });
    }
    let value = sum.get();
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(unproven("volume analytique non positif"))
    }
}

#[cfg(test)]
mod trim_regression {
    use super::*;

    #[test]
    fn large_cap_projection_clamps_a_source_point_inside_the_trim() {
        let radius = 6.;
        let rim = [20_f64.sqrt(), -4.];
        let actual = [rim[0] - 6e-6, 0., rim[1] + 2e-6];
        let radial = scale(actual, radius / norm(actual));
        assert!(actual[2] > rim[1]);
        assert!(radial[2] < rim[1]);
        let projected = project_cap_vertex(actual, radius, rim, 1., false, false).unwrap();
        assert_eq!(projected, [rim[0], 0., rim[1]]);
        assert!((norm(sub(actual, projected)) - 40_f64.sqrt() * 1e-6).abs() < 1e-14);
        // Explicit boundary and pole projections also count axial and lateral
        // noise, even when a radial-only fit would hide part of the movement.
        let boundary = [rim[0], 0., rim[1] + 2e-6];
        let ideal = project_cap_vertex(boundary, radius, rim, 1., false, true).unwrap();
        assert!((norm(sub(boundary, ideal)) - 2e-6).abs() < 1e-14);
        let pole = [2e-6, 0., radius];
        let ideal = project_cap_vertex(pole, radius, rim, 1., true, false).unwrap();
        assert_eq!(norm(sub(pole, ideal)), 2e-6);
    }

    #[test]
    fn hemisphere_residual_includes_equator_and_pole_displacements() {
        let options = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        };
        let original = crate::analyze_stl(
            include_bytes!("../../../fixtures/m5_hemisphere.stl"),
            &options,
        )
        .unwrap();
        let equator = original
            .model
            .faces
            .iter()
            .find(|f| {
                f.normal[2] < -0.99
                    && f.loops
                        .iter()
                        .flatten()
                        .all(|&v| original.mesh.vertices[v][2].abs() < 1e-12)
            })
            .unwrap()
            .loops[0]
            .clone();
        let pole = original
            .mesh
            .vertices
            .iter()
            .position(|p| (p[2] - 6.).abs() < 1e-12)
            .unwrap();
        let triangles: Vec<_> = original
            .mesh
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                t.iter()
                    .any(|&v| original.mesh.vertices[v][2] > 1e-12)
                    .then_some(i)
            })
            .collect();
        let mut edges: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
        for (t, &[a, b, c]) in original.mesh.triangles.iter().enumerate() {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                edges.entry(edge(u, v)).or_default().push(t);
            }
        }
        let patch = Patch {
            frame: Frame::new([0.; 3], [0., 0., 1.]),
            nodes: vec![
                Node {
                    vertices: equator.clone(),
                    z: 0.,
                    radius: 6.,
                },
                Node {
                    vertices: vec![pole],
                    z: 6.,
                    radius: 0.,
                },
            ],
            sections: Vec::new(),
            sphere: Some(SphericalCap {
                boundary: 0,
                pole: 1,
                center_z: 0.,
                radius: 6.,
            }),
            triangles,
        };
        let displacement = 2e-6;
        for (vertex, axis, delta) in [(equator[0], 2, -displacement), (pole, 0, displacement)] {
            let mut source = original.clone();
            source.mesh.vertices[vertex][axis] += delta;
            let (errors, _) = prove_sphere(
                &source,
                &patch,
                patch.sphere.unwrap(),
                &edges,
                &options,
                &mut Work::default(),
            )
            .unwrap();
            assert!(errors.radial >= displacement * (1. - 1e-8));
        }
    }
}
