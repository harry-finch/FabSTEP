//! Concave pocket fillets bounded by a right stop and an elliptic arc.
//! Each ellipse can belong to an oblique plane or another fillet. A U has two
//! distinct elliptic edges incident on its central wall. Four fillets close
//! a rectangular circuit around a measured planar floor.
//! The eleven planar neighbors are seeded from the largest source triangles. Only
//! the fully certified partition (including the pocket opening) is returned.
use crate::math::{cross, dot, norm, scale, sub, Sum, V};
use crate::recognition::Work;
use crate::{
    Analysis, CircularArc, Diagnostic, EllipticArc, FabError, Mesh, MixedLoop, MixedModel,
    MixedPartialCylinder, MixedPlane, MixedSegment, Options, RecognitionReport, RevolvedModel,
    RoundedFaceRef, RoundedModel, RoundedRecognition, Severity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::FRAC_PI_2;

#[path = "adjacent_pocket.rs"]
mod adjacent;
mod network;
#[path = "unequal_pocket.rs"]
mod unequal;
mod wall;
const WALL: usize = 11;
type Edge = (usize, usize);
type Cuts = BTreeMap<Edge, Vec<usize>>;

fn pair(a: usize, b: usize) -> Edge {
    (a.min(b), a.max(b))
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn unit(v: V) -> Option<V> {
    let length = norm(v);
    (length > 0. && length.is_finite()).then(|| scale(v, 1. / length))
}
fn unsupported() -> FabError {
    FabError::new(
        "recognition.oblique_pocket_unproven",
        "Les contours du congé de poche oblique ne sont pas démontrés.",
    )
}
fn checked(condition: bool) -> Result<(), FabError> {
    if condition {
        Ok(())
    } else {
        Err(unsupported())
    }
}
#[derive(Clone, Copy)]
struct Plane {
    origin: V,
    normal: V,
}
impl Plane {
    fn distance(self, p: V) -> f64 {
        dot(sub(p, self.origin), self.normal)
    }
}
struct Patch {
    support: Plane,
    triangles: BTreeSet<usize>,
    area: f64,
}
struct Partition {
    planes: Vec<Patch>,
    owners: Vec<usize>,
    cuts: Cuts,
    // Boundary edges with directed orientation as in the source triangle.
    neighbor: BTreeMap<(usize, usize), usize>,
}

// For a prescribed normal, the midplane of the extreme projections
// minimizes the maximum point residual, including all interior vertices.
fn center_plane(mesh: &Mesh, patch: &Patch, normal: V) -> Plane {
    let base = patch.support.origin;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for &t in &patch.triangles {
        for &v in &mesh.triangles[t] {
            let h = dot(sub(mesh.vertices[v], base), normal);
            low = low.min(h);
            high = high.max(h);
        }
    }
    Plane {
        origin: add(base, scale(normal, (low + high) / 2.)),
        normal,
    }
}

fn refit_supports(mesh: &Mesh, part: &mut Partition) -> Result<(), FabError> {
    // The seed triangle suffices for discovery. Binary STL rounding makes
    // its normal less reliable than the oriented area of the entire patch.
    for patch in &mut part.planes {
        let mut vector = [0.; 3];
        for &t in &patch.triangles {
            let [a, b, c] = mesh.triangles[t].map(|v| mesh.vertices[v]);
            vector = add(vector, cross(sub(b, a), sub(c, a)));
        }
        patch.support.normal = unit(vector).ok_or_else(unsupported)?;
    }
    Ok(())
}

fn partition(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    plane_tolerance: f64,
    work: &mut Work,
) -> Result<Partition, FabError> {
    let triangles = &mesh.triangles;
    let count = triangles.len();
    work.spend(count * 32)?;
    let mut normals = Vec::with_capacity(count);
    let mut areas = Vec::with_capacity(count);
    for indices in triangles {
        let [p, q, r] = indices.map(|v| mesh.vertices[v]);
        let n = cross(sub(q, p), sub(r, p));
        areas.push(norm(n));
        normals.push(unit(n).ok_or_else(unsupported)?);
    }
    let mut seeds: Vec<_> = (0..count).collect();
    seeds.sort_unstable_by(|&i, &j| areas[j].total_cmp(&areas[i]).then_with(|| i.cmp(&j)));
    let mut assigned = vec![false; count];
    let mut patches = Vec::new();
    for seed in seeds {
        if assigned[seed] {
            continue;
        }
        let support = Plane {
            origin: mesh.vertices[triangles[seed][0]],
            normal: normals[seed],
        };
        let mut stack = vec![seed];
        assigned[seed] = true;
        let mut region = BTreeSet::new();
        let mut area = Sum::default();
        while let Some(t) = stack.pop() {
            area.add(areas[t]);
            region.insert(t);
            work.spend(16)?;
            for &next in &adjacency[t] {
                if !assigned[next]
                    && dot(normals[next], support.normal) > 0.
                    && triangles[next]
                        .iter()
                        .all(|&v| support.distance(mesh.vertices[v]).abs() <= plane_tolerance)
                {
                    assigned[next] = true;
                    stack.push(next);
                }
            }
        }
        patches.push(Patch {
            support,
            triangles: region,
            area: area.get(),
        });
    }
    patches.sort_by(|a, b| b.area.total_cmp(&a.area));
    checked(patches.len() > WALL && patches[WALL - 1].area > 2. * patches[WALL].area)?;
    patches.truncate(WALL);
    let mut owners = vec![WALL; count];
    for (i, patch) in patches.iter().enumerate() {
        for &t in &patch.triangles {
            owners[t] = i;
        }
    }
    let contours = prove_open_patch(mesh, adjacency, &owners, WALL)?;
    // Before splitting a U, its common planar stop has two disconnected
    // interfaces with the unlabelled curved strip. Only the dedicated
    // recognizer can certify those cuts after assigning all three walls.
    let (cuts, neighbor) = if contours == 2 {
        // The four unlabelled walls form a single annulus around a real floor.
        // Ordered interfaces exist only after splitting that annulus.
        (Cuts::new(), Neighbors::new())
    } else {
        collect_interfaces(mesh, &owners, true)?
    };
    Ok(Partition {
        planes: patches,
        owners,
        cuts,
        neighbor,
    })
}

fn prove_disk(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    owners: &[usize],
    id: usize,
) -> Result<(), FabError> {
    checked(prove_open_patch(mesh, adjacency, owners, id)? == 1)
}

fn prove_open_patch(
    mesh: &Mesh,
    adjacency: &[Vec<usize>],
    owners: &[usize],
    id: usize,
) -> Result<usize, FabError> {
    let first = owners
        .iter()
        .position(|&i| i == id)
        .ok_or_else(unsupported)?;
    let mut wall = BTreeSet::from([first]);
    let mut stack = vec![first];
    while let Some(t) = stack.pop() {
        for &other in &adjacency[t] {
            if owners[other] == id && wall.insert(other) {
                stack.push(other);
            }
        }
    }
    checked(wall.len() == owners.iter().filter(|&&v| v == id).count())?;
    let wall_loops = crate::planar::boundaries(mesh, &wall)?;
    checked((1..=2).contains(&wall_loops.len()))?;
    let vertices: BTreeSet<_> = wall.iter().flat_map(|&t| mesh.triangles[t]).collect();
    let boundary = wall_loops.iter().map(Vec::len).sum::<usize>();
    checked(
        2 * vertices.len() + 2 * wall.len()
            == 2 * (2 - wall_loops.len()) + 3 * wall.len() + boundary,
    )?;
    Ok(wall_loops.len())
}

type Neighbors = BTreeMap<(usize, usize), usize>;
fn interfaces(mesh: &Mesh, owners: &[usize]) -> Result<(Cuts, Neighbors), FabError> {
    collect_interfaces(mesh, owners, false)
}
fn collect_interfaces(
    mesh: &Mesh,
    owners: &[usize],
    discover_network: bool,
) -> Result<(Cuts, Neighbors), FabError> {
    let triangles = &mesh.triangles;
    let mut directed = BTreeMap::new();
    for (t, &owner) in owners.iter().enumerate() {
        let [a, b, c] = triangles[t];
        for (p, q) in [(a, b), (b, c), (c, a)] {
            checked(directed.insert((p, q), owner).is_none())?;
        }
    }
    let mut by_pair = BTreeMap::<Edge, Vec<Edge>>::new();
    let mut neighbor = BTreeMap::new();
    for (&(p, q), &owner) in &directed {
        let &other = directed.get(&(q, p)).ok_or_else(unsupported)?;
        if owner != other {
            neighbor.insert((p, q), other);
            if owner < other {
                by_pair.entry((owner, other)).or_default().push((p, q));
            }
        }
    }
    let mut cuts = Cuts::new();
    let mut split_stop = false;
    for (key, edges) in by_pair {
        match crate::rounded::block::ordered_chain(&edges) {
            Ok(chain) => {
                cuts.insert(key, chain);
            }
            Err(_) if discover_network && key.1 == WALL && !split_stop => {
                // Exactly four endpoints on this one support are necessary
                // for two independent quarter-circle stops. After splitting
                // the strip, all interfaces must again be single chains.
                let mut degree = BTreeMap::<usize, usize>::new();
                for (a, b) in edges {
                    *degree.entry(a).or_default() += 1;
                    *degree.entry(b).or_default() += 1;
                }
                checked(
                    degree.values().all(|&d| d <= 2)
                        && degree.values().filter(|&&d| d == 1).count() == 4,
                )?;
                split_stop = true;
            }
            Err(_) => return Err(unsupported()),
        }
    }
    if split_stop {
        Ok((Cuts::new(), Neighbors::new()))
    } else {
        Ok((cuts, neighbor))
    }
}

fn intersection(planes: [Plane; 3], near: V) -> Option<V> {
    let [a, b, c] = planes.map(|p| p.normal);
    let reciprocal = [cross(b, c), cross(c, a), cross(a, b)];
    let det = dot(a, reciprocal[0]);
    if det.abs() < 0.01 {
        return None;
    }
    let rhs = planes.map(|p| -p.distance(near) / det);
    Some((0..3).fold(near, |p, i| add(p, scale(reciprocal[i], rhs[i]))))
}

fn straight(chain: &[usize], raw: &[V], projected: &[V], tol: f64) -> Option<f64> {
    let a = projected[chain[0]];
    let b = projected[*chain.last()?];
    let d = sub(b, a);
    let length = norm(d);
    let dir = unit(d)?;
    let mut previous = f64::NEG_INFINITY;
    let mut residual: f64 = 0.;
    for (i, &v) in chain.iter().enumerate() {
        let h = if i == 0 {
            0.
        } else if i + 1 == chain.len() {
            length
        } else {
            dot(sub(raw[v], a), dir)
        };
        if h <= previous || h > length {
            return None;
        }
        previous = h;
        residual = residual.max(norm(sub(raw[v], add(a, scale(dir, h)))));
    }
    (residual <= tol).then_some(residual)
}

fn plane_loops(
    mesh: &Mesh,
    part: &Partition,
    geoms: &[wall::Geometry],
    projected: &[V],
    work: &mut Work,
    o: &Options,
) -> Result<(Vec<MixedPlane>, f64), FabError> {
    let mut faces = Vec::new();
    let mut checks = 0;
    let mut residual = 0_f64;
    for (i, patch) in part.planes.iter().enumerate() {
        let p = patch.support;
        // A consistent positive projected Jacobian and simple oriented contours
        // give degree one on the planar domain, also for the top face with a hole.
        for &t in &patch.triangles {
            let [a, b, c] = mesh.triangles[t].map(|v| mesh.vertices[v]);
            checked(dot(cross(sub(b, a), sub(c, a)), p.normal) > 0.)?;
            for v in [a, b, c] {
                residual = residual.max(p.distance(v).abs());
            }
        }
        checked(residual <= o.plane_tolerance_mm)?;
        let previous_checks = checks;
        let loops = crate::planar::validate_loops(
            mesh,
            crate::planar::boundaries(mesh, &patch.triangles)?,
            p.origin,
            p.normal,
            &mut checks,
        )?;
        work.spend(checks - previous_checks)?;
        let mut result = Vec::new();
        for vertices in loops {
            work.spend(4 * vertices.len())?;
            let ns: Vec<_> = (0..vertices.len())
                .map(|k| part.neighbor[&(vertices[k], vertices[(k + 1) % vertices.len()])])
                .collect();
            let breaks: Vec<_> = (0..ns.len())
                .filter(|&k| ns[k] != ns[(k + ns.len() - 1) % ns.len()])
                .collect();
            checked(breaks.len() >= 3)?;
            let mut segments = Vec::new();
            for (idx, &k) in breaks.iter().enumerate() {
                let j = breaks[(idx + 1) % breaks.len()];
                let ids = [vertices[k], vertices[j]];
                let other = ns[k];
                let cut = &part.cuts[&pair(i, other)];
                let ordered = [cut[0], *cut.last().unwrap()];
                checked(
                    (ids == ordered || ids == [ordered[1], ordered[0]])
                        && (j + ns.len() - k) % ns.len() + 1 == cut.len(),
                )?;
                if let Some((index, geom)) = geoms
                    .iter()
                    .enumerate()
                    .find(|(_, g)| other == g.id && i == g.circle_plane)
                {
                    let reversed = ids[0] != geom.circle.vertices[0];
                    segments.push(if geom.near_ellipse.is_some() {
                        MixedSegment::EllipticArc { arc: 0, reversed }
                    } else {
                        MixedSegment::Arc {
                            arc: index,
                            reversed,
                        }
                    });
                } else if let Some(geom) =
                    geoms.iter().find(|g| other == g.id && i == g.ellipse_plane)
                {
                    segments.push(MixedSegment::EllipticArc {
                        arc: usize::from(geom.near_ellipse.is_some()),
                        reversed: ids[0] != geom.ellipse.vertices[0],
                    });
                } else {
                    checked(
                        p.distance(projected[ids[0]]).abs() < 1e-8
                            && p.distance(projected[ids[1]]).abs() < 1e-8,
                    )?;
                    segments.push(MixedSegment::Line { vertices: ids });
                }
            }
            result.push(MixedLoop::Segments { segments });
        }
        faces.push(MixedPlane {
            normal: p.normal,
            loops: result,
        });
    }
    checked(
        faces.iter().filter(|f| f.loops.len() == 2).count() == 1
            && faces.iter().all(|f| f.loops.len() <= 2),
    )?;
    Ok((faces, residual))
}

struct Projected {
    vertices: Vec<V>,
    displaced: usize,
    max_corner: f64,
    line_residual: f64,
}
fn project(
    mesh: &Mesh,
    p: &Partition,
    geoms: &[wall::Geometry],
    o: &Options,
) -> Result<Projected, FabError> {
    let mut corners = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (&(i, j), cut) in &p.cuts {
        for &id in [cut.first().unwrap(), cut.last().unwrap()] {
            let planes = corners.entry(id).or_default();
            if i < WALL {
                planes.insert(i);
            }
            if j < WALL {
                planes.insert(j);
            }
        }
    }
    let mut vertices = mesh.vertices.clone();
    let mut max_corner = 0_f64;
    let mut displaced = 0;
    for (v, faces) in corners {
        let point = if let Some(corner) = geoms.iter().find_map(|g| g.corner(v)) {
            // The meeting point of two floor rails has only the floor
            // as a planar incident face. Each isolated end meets two planes.
            checked(
                faces.len() == 2
                    || (faces.len() == 1
                        && geoms.iter().filter(|g| g.corner(v).is_some()).count() == 2),
            )?;
            checked(
                geoms
                    .iter()
                    .filter_map(|g| g.corner(v))
                    .all(|q| norm(sub(q, corner)) < 1e-8),
            )?;
            corner
        } else {
            let supports: Vec<_> = faces.iter().map(|&i| p.planes[i].support).collect();
            intersection(supports.try_into().map_err(|_| unsupported())?, vertices[v])
                .ok_or_else(unsupported)?
        };
        checked(
            faces
                .iter()
                .all(|&i| p.planes[i].support.distance(point).abs() < 1e-8),
        )?;
        let distance = norm(sub(vertices[v], point));
        checked(distance <= o.cylinder_fit_tolerance_mm.min(1e-5))?;
        max_corner = max_corner.max(distance);
        if point != vertices[v] {
            displaced += 1;
        }
        vertices[v] = point;
    }
    let mut line_residual = 0_f64;
    for (&(i, j), cut) in &p.cuts {
        if geoms
            .iter()
            .any(|g| [pair(g.circle_plane, g.id), pair(g.ellipse_plane, g.id)].contains(&(i, j)))
        {
            continue;
        }
        let error = straight(
            cut,
            &mesh.vertices,
            &vertices,
            o.cylinder_fit_tolerance_mm.min(1e-5),
        )
        .ok_or_else(unsupported)?;
        line_residual = line_residual.max(error);
    }
    Ok(Projected {
        vertices,
        displaced,
        max_corner,
        line_residual,
    })
}

fn candidate(
    a: &Analysis,
    o: &Options,
    adjacency: &[Vec<usize>],
    work: &mut Work,
    automatic: bool,
    mut part: Partition,
) -> Result<RoundedRecognition, FabError> {
    let mesh = &a.mesh;
    let neighbor_count = part.cuts.keys().filter(|(_, j)| *j == WALL).count();
    let geoms = if part.cuts.is_empty() {
        network::fit(mesh, adjacency, &mut part, o, work)?
    } else if neighbor_count == 5 {
        // The small fillet's side shares both a straight rail and the
        // continuation of the large cylinder. Only one entire side interface
        // stays straight. Equal-radius corners have two straight interfaces.
        let tol = o.cylinder_fit_tolerance_mm.min(1e-5);
        let straight_sides = part
            .cuts
            .iter()
            .filter(|((_, j), cut)| {
                *j == WALL && straight(cut, &mesh.vertices, &mesh.vertices, tol).is_some()
            })
            .count();
        if straight_sides == 1 {
            return unequal::candidate(a, o, adjacency, work, part);
        }
        adjacent::fit(mesh, adjacency, &mut part, o, work)?
    } else {
        vec![wall::fit(mesh, &mut part, o)?]
    };
    let mut residual = 0_f64;
    let mut chord = 0_f64;
    for g in &geoms {
        let error = wall::prove(mesh, &part, g, o, work)?;
        residual = residual.max(error.residual);
        chord = chord.max(error.chord);
    }
    let points = project(mesh, &part, &geoms, o)?;
    let (planes, plane_error) = plane_loops(mesh, &part, &geoms, &points.vertices, work, o)?;
    // Winding bounds the difference of each planar trimmed domain by the
    // boundary homotopy, including its circular/elliptical chordal intervals.
    let rounding = mesh
        .vertices
        .iter()
        .flatten()
        .copied()
        .map(f64::abs)
        .fold(1., f64::max)
        * 128.
        * f64::EPSILON;
    checked(rounding < 1e-8)?;
    let trim = points.max_corner.max(points.line_residual) + 2. * plane_error + rounding + 1e-5;
    let bound = o.weld_tolerance_mm + residual + chord + trim;
    if bound > o.max_deviation_mm {
        if automatic && geoms[0].near_ellipse.is_some() {
            return Err(unsupported());
        }
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "La paroi cylindrique et son contour elliptique dépassent la borne d’écart.",
        ));
    }
    let cylinders = geoms.len();
    let adjacent = cylinders == 2;
    let double_stop = geoms[0].near_ellipse.is_some();
    let (fitted, adjusted, description) = if cylinders == 4 {
        (
            "recognition.closed_pocket_fitted",
            "recognition.adjacent_pocket_adjusted",
            "Les quatre congés de la poche partagent un fond plan et quatre arêtes elliptiques.",
        )
    } else if cylinders == 3 {
        (
            "recognition.u_pocket_fitted",
            "recognition.adjacent_pocket_adjusted",
            "Les trois congés de la poche en U partagent deux arêtes elliptiques mesurées.",
        )
    } else if double_stop {
        (
            "recognition.double_oblique_pocket_fitted",
            "recognition.oblique_pocket_adjusted",
            "Le congé cylindrique est délimité par deux butées elliptiques mesurées.",
        )
    } else if adjacent {
        ("recognition.adjacent_pocket_fitted","recognition.adjacent_pocket_adjusted","Les deux congés adjacents sont interprétés comme des quarts de cylindre à arête elliptique commune.")
    } else {
        ("recognition.oblique_pocket_fitted","recognition.oblique_pocket_adjusted","Le congé de fond de poche est interprété comme un quart de cylindre arrêté par un plan oblique.")
    };
    let mut diagnostics = vec![Diagnostic {
        code: fitted.into(),
        severity: Severity::Warning,
        count: cylinders,
        message: description.into(),
    }];
    if points.displaced > 0 {
        diagnostics.push(Diagnostic{
            code:adjusted.into(),severity:Severity::Info,count:points.displaced,
            message:"Les sommets de jonction ont été ajustés aux supports mesurés dans la borne d’écart.".into(),
        });
    }
    // For a U, store both seams in the central chart's orientation. Each
    // flank recovers its own orientation from its cylinder axis.
    let chart = if cylinders == 3 { &geoms[2] } else { &geoms[0] };
    let ellipses = if cylinders == 4 {
        geoms
            .iter()
            .map(|g| g.near_ellipse.clone().ok_or_else(unsupported))
            .collect::<Result<_, _>>()?
    } else {
        chart
            .near_ellipse
            .iter()
            .chain([&chart.ellipse])
            .cloned()
            .collect()
    };
    let mixed = MixedModel {
        vertices: points.vertices,
        planes,
        rings: vec![],
        cylinders: vec![],
        elliptic_arcs: ellipses,
        arcs: geoms
            .into_iter()
            .filter(|g| g.near_ellipse.is_none())
            .map(|g| g.circle)
            .collect(),
        partial_cylinders: (0..cylinders)
            .map(|index| MixedPartialCylinder {
                arcs: if cylinders == 4 {
                    [index, (index + 1) % 4]
                } else if cylinders == 3 {
                    if index == 2 {
                        [0, 1]
                    } else {
                        [index, index]
                    }
                } else {
                    [index, usize::from(double_stop)]
                },
                elliptic_start: double_stop || index == 2,
                inner: true,
                elliptic_end: true,
            })
            .collect(),
    };
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed,
            cones: vec![],
        },
        spheres: vec![],
        tori: vec![],
        partial_cones: vec![],
        torus_sectors: vec![],
        smooth_revolution: None,
        unequal_pocket: None,
        cross_drill: None,
        linear_blend: None,
        octants: vec![],
    };
    let volume = crate::rounded::volume(&model)?;
    let triangle_faces = part
        .owners
        .into_iter()
        .map(|i| {
            if i >= WALL {
                RoundedFaceRef::PartialCylinder { index: i - WALL }
            } else {
                RoundedFaceRef::Plane { index: i }
            }
        })
        .collect();
    Ok(RoundedRecognition {
        model,
        triangle_faces,
        report: RecognitionReport {
            revolutions: 0,
            cylinders,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            max_radial_residual_mm: residual,
            max_chord_error_mm: chord,
            max_wall_residual_mm: trim,
            max_deviation_bound_mm: bound,
            expected_volume_mm3: volume,
            diagnostics,
        },
    })
}

// Cheap necessary condition before repeating patch discovery on rounded STL
// coordinates. The outer rim of a rectangular block supplies the frame even
// if the STL subdivides each of its sides into many collinear edges.
fn rectangular_supports(model: &crate::PlanarModel) -> bool {
    let Some(rim) = model.faces.iter().find(|face| face.loops.len() == 2) else {
        return false;
    };
    let outline = &rim.loops[0];
    let rail = outline
        .iter()
        .zip(outline.iter().cycle().skip(1))
        .map(|(&a, &b)| sub(model.vertices[b], model.vertices[a]))
        .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))
        .and_then(unit);
    let Some(x) = rail else {
        return false;
    };
    let axes = [x, cross(rim.normal, x), rim.normal];
    axes.into_iter().all(|axis| {
        [-1., 1.].into_iter().all(|sign| {
            model
                .faces
                .iter()
                .any(|f| sign * dot(f.normal, axis) > 1. - 1e-6)
        })
    })
}

fn inconclusive(error: &FabError) -> bool {
    matches!(
        error.code.as_str(),
        "recognition.oblique_pocket_unproven" | "planar.invalid_contour"
    )
}

pub(crate) fn recognize(
    a: &Analysis,
    segmented: &crate::PlanarModel,
    o: &Options,
    adjacency: &[Vec<usize>],
    work: &mut Work,
    automatic: bool,
) -> Result<Option<RoundedRecognition>, FabError> {
    if !(24..=4096).contains(&a.mesh.triangles.len()) {
        return Ok(None);
    }
    let mut result = partition(&a.mesh, adjacency, o.plane_tolerance_mm.min(1e-5), work)
        .and_then(|part| candidate(a, o, adjacency, work, automatic, part));
    let discovery_tol = o.cylinder_fit_tolerance_mm.min(1e-5);
    if result.as_ref().is_err_and(inconclusive)
        && o.plane_tolerance_mm < discovery_tol
        && rectangular_supports(segmented)
    {
        // Only the discovery radius changes. The final plane distances,
        // incidence, cylindrical fits and deviation use the requested limits.
        result = partition(&a.mesh, adjacency, discovery_tol, work).and_then(|part| {
            checked(
                part.cuts.is_empty() || part.cuts.keys().filter(|(_, j)| *j == WALL).count() == 5,
            )?;
            candidate(a, o, adjacency, work, automatic, part)
        });
    }
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if inconclusive(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Outward radial flux on the strip between two inclined planes. Integration
/// in z contributes their height difference h + c*cos(t) + s*sin(t).
pub(crate) fn double_stop_flux(
    model: &MixedModel,
    wall: &MixedPartialCylinder,
    reference: V,
) -> f64 {
    let [lo, hi] = wall.arcs.map(|i| &model.elliptic_arcs[i]);
    let separation = sub(hi.center, lo.center);
    let h = norm(separation);
    let axis = scale(separation, 1. / h);
    let first = sub(model.vertices[lo.vertices[0]], lo.center);
    let radial = sub(first, scale(axis, dot(first, axis)));
    let x = scale(radial, 1. / norm(radial));
    let y = cross(axis, x);
    let r = lo.minor_mm;
    let gradient = sub(
        scale(lo.axis, r / dot(lo.axis, axis)),
        scale(hi.axis, r / dot(hi.axis, axis)),
    );
    let (c, s) = (dot(gradient, x), dot(gradient, y));
    // Quarter turn [0, pi/2]; both squared trigonometric integrals equal pi/4.
    let half_sweep = FRAC_PI_2 / 2.;
    let offset = sub(lo.center, reference);
    let flux = r * (h * FRAC_PI_2 + c + s)
        + dot(offset, x) * (h + c * half_sweep + s / 2.)
        + dot(offset, y) * (h + c / 2. + s * half_sweep);
    (if wall.inner { -1. } else { 1. }) * r * flux / 3.
}
