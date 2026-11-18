//! Joint semicircular cylinder/cone chains. All bands, cap arcs and straight
//! side rails must be witnessed by the same connected source strip. This is
//! deliberately narrower than arbitrary partial surfaces of revolution.
use super::*;
use crate::{MixedPartialCone, RoundedFaceRef, RoundedRecognition};

fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}

struct Band {
    triangles: BTreeSet<usize>,
    rails: [(Vec<usize>, usize); 2],
    cylindrical: bool,
}
struct Chain {
    caps: [usize; 2],
    cap_chains: [Vec<usize>; 2],
    arcs: Vec<CircularArc>,
    bands: Vec<Band>,
    inner: bool,
    radial: f64,
    chord: f64,
}

pub(crate) fn augment(
    a: &Analysis,
    result: &mut RoundedRecognition,
    o: &Options,
    scope: Option<&BTreeSet<usize>>,
) -> Result<(), FabError> {
    let model = &result.model.revolved.mixed;
    let mut work = Work::default();
    let candidates = match candidates(model, o, &mut work) {
        Ok(c) => c,
        Err(e) if e.code == "recognition.partial_unproven" => return Ok(()),
        Err(e) => return Err(e),
    };
    if candidates.len() < 2 {
        return Ok(());
    }
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let mut edges: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    let mut vertex_triangles = vec![Vec::new(); a.mesh.vertices.len()];
    for (t, &[a, b, c]) in a.mesh.triangles.iter().enumerate() {
        work.spend(3)?;
        for v in [a, b, c] {
            vertex_triangles[v].push(t);
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            edges.entry(edge(u, v)).or_default().push(t);
        }
    }
    let owners: Vec<_> = result
        .triangle_faces
        .iter()
        .map(|f| {
            Some(match f {
                RoundedFaceRef::Plane { index } => MixedFaceRef::Plane { index: *index },
                _ => MixedFaceRef::Cylinder { index: usize::MAX },
            })
        })
        .collect();
    let mut chains = vec![];
    for i in 0..candidates.len() {
        for j in i + 1..candidates.len() {
            work.spend(1)?;
            if let Some(chain) = prove(
                a,
                model,
                &owners,
                [&candidates[i], &candidates[j]],
                &adjacency,
                &edges,
                &vertex_triangles,
                o,
                &mut work,
            )? {
                if scope.is_none_or(|s| {
                    chain
                        .bands
                        .iter()
                        .any(|b| b.triangles.iter().any(|t| s.contains(t)))
                }) {
                    chains.push(chain);
                }
            }
        }
    }
    // Ambiguous competing strips are not arbitrarily prioritized.
    let sets: Vec<BTreeSet<_>> = chains
        .iter()
        .map(|c| {
            c.bands
                .iter()
                .flat_map(|b| b.triangles.iter().copied())
                .collect()
        })
        .collect();
    let consumed_planes: Vec<BTreeSet<_>> = sets
        .iter()
        .map(|triangles| {
            triangles
                .iter()
                .filter_map(|&t| match owners[t] {
                    Some(MixedFaceRef::Plane { index }) => Some(index),
                    _ => None,
                })
                .collect()
        })
        .collect();
    let support_planes: Vec<BTreeSet<_>> = chains
        .iter()
        .map(|c| {
            c.caps
                .into_iter()
                .chain(c.bands.iter().flat_map(|b| b.rails.iter().map(|(_, p)| *p)))
                .collect()
        })
        .collect();
    let mut ambiguous = BTreeSet::new();
    for i in 0..chains.len() {
        for j in i + 1..chains.len() {
            work.spend(sets[i].len().min(sets[j].len()))?;
            work.spend(support_planes[i].len() + support_planes[j].len())?;
            if !sets[i].is_disjoint(&sets[j])
                || !support_planes[i].is_disjoint(&consumed_planes[j])
                || !support_planes[j].is_disjoint(&consumed_planes[i])
            {
                ambiguous.insert(i);
                ambiguous.insert(j);
            }
        }
    }
    let mut replacements: BTreeMap<usize, Vec<Replacement>> = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    let mut radial = 0_f64;
    let mut chord = 0_f64;
    let mut count = 0;
    let model = &mut result.model.revolved.mixed;
    for (i, c) in chains.into_iter().enumerate() {
        if ambiguous.contains(&i) {
            continue;
        }
        count += 1;
        let first = model.arcs.len();
        for end in 0..2 {
            let index = if end == 0 {
                first
            } else {
                first + c.arcs.len() - 1
            };
            let arc = &c.arcs[index - first];
            replacements
                .entry(c.caps[end])
                .or_default()
                .push(Replacement {
                    chain: c.cap_chains[end].clone(),
                    segment: MixedSegment::Arc {
                        arc: index,
                        reversed: c.cap_chains[end][0] != arc.vertices[0],
                    },
                });
        }
        for (band_index, b) in c.bands.into_iter().enumerate() {
            let arcs = [first + band_index, first + band_index + 1];
            let owner = if b.cylindrical {
                let index = model.partial_cylinders.len();
                model.partial_cylinders.push(MixedPartialCylinder {
                    elliptic_end: false,
                    arcs,
                    inner: c.inner,
                });
                RoundedFaceRef::PartialCylinder { index }
            } else {
                let index = result.model.partial_cones.len();
                result.model.partial_cones.push(MixedPartialCone {
                    arcs,
                    inner: c.inner,
                });
                RoundedFaceRef::PartialCone { index }
            };
            for t in b.triangles {
                if let RoundedFaceRef::Plane { index } = result.triangle_faces[t] {
                    deleted.insert(index);
                } else {
                    return Err(unproven());
                }
                result.triangle_faces[t] = owner;
            }
            for (chain, plane) in b.rails {
                replacements.entry(plane).or_default().push(Replacement {
                    segment: MixedSegment::Line {
                        vertices: [chain[0], *chain.last().unwrap()],
                    },
                    chain,
                });
            }
        }
        model.arcs.extend(c.arcs);
        radial = radial.max(c.radial);
        chord = chord.max(c.chord);
    }
    if count == 0 {
        return Ok(());
    }
    for (plane, replacements) in replacements {
        if deleted.contains(&plane) {
            return Err(unproven());
        }
        for lp in &mut model.planes[plane].loops {
            if let MixedLoop::Polygon { vertices } = lp {
                if let Some(segments) = replace_loop(vertices, &replacements, &mut work)? {
                    *lp = MixedLoop::Segments { segments };
                }
            }
        }
    }
    let mut mapping = vec![None; model.planes.len()];
    let mut kept = vec![];
    for (i, p) in std::mem::take(&mut model.planes).into_iter().enumerate() {
        if !deleted.contains(&i) {
            mapping[i] = Some(kept.len());
            kept.push(p);
        }
    }
    model.planes = kept;
    for f in &mut result.triangle_faces {
        if let RoundedFaceRef::Plane { index } = f {
            *index = mapping[*index].ok_or_else(unproven)?;
        }
    }
    result.report.cylinders = model.cylinders.len() + model.partial_cylinders.len();
    result.report.cones = result.model.revolved.cones.len() + result.model.partial_cones.len();
    result.report.max_radial_residual_mm = result.report.max_radial_residual_mm.max(radial);
    result.report.max_chord_error_mm = result.report.max_chord_error_mm.max(chord);
    result.report.max_wall_residual_mm = result.report.max_wall_residual_mm.max(NATIVE_TOL * 2.);
    result.report.diagnostics.push(crate::Diagnostic{code:"recognition.partial_conical_chain".into(),severity:crate::Severity::Warning,count,message:"Des chaînes semi-circulaires cylindre/cône sont reconstruites conjointement ; l’intention analytique est déduite du maillage.".into()});
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn prove(
    a: &Analysis,
    model: &MixedModel,
    owners: &[Option<MixedFaceRef>],
    caps: [&Candidate; 2],
    adjacency: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    vertex_triangles: &[Vec<usize>],
    o: &Options,
    work: &mut Work,
) -> Result<Option<Chain>, FabError> {
    let [lo, hi] = caps;
    let tol = o.cylinder_fit_tolerance_mm;
    if lo.plane == hi.plane
        || (lo.radius - hi.radius).abs() <= tol
        || dot(lo.normal, hi.normal).abs() < 1. - 1e-10
    {
        return Ok(None);
    }
    let midpoint = |c: &Candidate| {
        scale(
            add(
                a.mesh.vertices[c.chain[0]],
                a.mesh.vertices[*c.chain.last().unwrap()],
            ),
            0.5,
        )
    };
    let center = midpoint(lo);
    let upper = midpoint(hi);
    // First scope: two semicircular caps. Using their exact diameter midpoint
    // is a constrained fit checked against ALL source vertices, not snapping.
    if norm(sub(center, lo.center)) > tol || norm(sub(upper, hi.center)) > tol {
        return Ok(None);
    }
    let delta = sub(upper, center);
    let h = dot(delta, lo.normal);
    if h.abs() <= tol * 4. {
        return Ok(None);
    }
    let axis = scale(lo.normal, h.signum());
    let height = h.abs();
    if norm(sub(delta, scale(axis, height))) > NATIVE_TOL {
        return Ok(None);
    }
    let x = scale(
        sub(a.mesh.vertices[lo.chain[0]], center),
        1. / norm(sub(a.mesh.vertices[lo.chain[0]], center)),
    );
    if dot(x, axis).abs() > 1e-10 {
        return Ok(None);
    }
    let mut y = cross(axis, x);
    if dot(sub(a.mesh.vertices[lo.chain[1]], center), y) < 0. {
        y = scale(y, -1.);
    }
    // Keep a direct axis frame; reverse bottom source order when necessary.
    let mut lower_chain = lo.chain.clone();
    if dot(cross(x, y), axis) < 0. {
        lower_chain.reverse();
    }
    let start = scale(
        sub(a.mesh.vertices[lower_chain[0]], center),
        1. / norm(sub(a.mesh.vertices[lower_chain[0]], center)),
    );
    let frame = Frame {
        origin: center,
        axis,
        x: start,
        y: cross(axis, start),
    };
    work.spend(a.mesh.vertices.len())?;
    let projected: Vec<_> = a.mesh.vertices.iter().map(|&p| frame.project(p)).collect();
    let angle = |p: P2| {
        let x = p[1].atan2(p[0]);
        if x < -1e-7 {
            x + TAU
        } else {
            x
        }
    };
    let mut upper_chain = hi.chain.clone();
    if angle(projected[upper_chain[0]].0) > PI / 2. {
        upper_chain.reverse();
    }
    let r0 = norm(sub(a.mesh.vertices[lower_chain[0]], center));
    let r1 = norm(sub(a.mesh.vertices[upper_chain[0]], upper));
    for (chain, r, z) in [(&lower_chain, r0, 0.), (&upper_chain, r1, height)] {
        let mut last = -1.;
        for &v in chain {
            work.spend(1)?;
            let (p, pz) = projected[v];
            let theta = angle(p);
            if (p[0].hypot(p[1]) - r).abs() > tol
                || (pz - z).abs() > NATIVE_TOL
                || theta < last
                || theta > PI + 1e-7
            {
                return Ok(None);
            }
            last = theta;
        }
        if norm(sub(
            a.mesh.vertices[chain[0]],
            add(center, add(scale(axis, z), scale(start, r))),
        )) > NATIVE_TOL
            || norm(sub(
                a.mesh.vertices[*chain.last().unwrap()],
                add(center, add(scale(axis, z), scale(start, -r))),
            )) > NATIVE_TOL
        {
            return Ok(None);
        }
    }
    let cap_edges: BTreeSet<_> = lower_chain
        .windows(2)
        .chain(upper_chain.windows(2))
        .map(|p| edge(p[0], p[1]))
        .collect();
    let first = edges[&edge(lower_chain[0], lower_chain[1])]
        .iter()
        .copied()
        .find(|&t| !is_plane(&owners[t], lo.plane))
        .ok_or_else(unproven)?;
    let xyz = a.mesh.triangles[first].map(|v| a.mesh.vertices[v]);
    let normal = cross(sub(xyz[1], xyz[0]), sub(xyz[2], xyz[0]));
    let inner = dot(normal, sub(xyz[0], center)) < 0.;
    let sign = if inner { -1. } else { 1. };
    let mut selected = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([first]);
    while let Some(t) = queue.pop_front() {
        if !visited.insert(t) {
            continue;
        }
        work.spend(1)?;
        if !matches!(owners[t], Some(MixedFaceRef::Plane { .. }))
            || is_plane(&owners[t], lo.plane)
            || is_plane(&owners[t], hi.plane)
        {
            continue;
        }
        let ids = a.mesh.triangles[t];
        let xyz = ids.map(|v| a.mesh.vertices[v]);
        let n = cross(sub(xyz[1], xyz[0]), sub(xyz[2], xyz[0]));
        if ids.iter().any(|&v| {
            let (p, z) = projected[v];
            let r = p[0].hypot(p[1]);
            let theta = angle(p);
            r < r0.min(r1) - tol
                || r > r0.max(r1) + tol
                || z < -NATIVE_TOL
                || z > height + NATIVE_TOL
                || !(-1e-7..=PI + 1e-7).contains(&theta)
                || sign * dot(n, add(scale(frame.x, p[0]), scale(frame.y, p[1])))
                    <= norm(n) * r * 1e-10
        }) {
            continue;
        }
        selected.insert(t);
        queue.extend(adjacency[t].iter().copied());
    }
    let mut vertices: BTreeSet<_> = selected.iter().flat_map(|&t| a.mesh.triangles[t]).collect();
    if vertices.is_empty() {
        return Ok(None);
    }
    let mut levels: Vec<f64> = vertices.iter().map(|&v| projected[v].1).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() <= NATIVE_TOL);
    if !(4..=9).contains(&levels.len()) {
        return Ok(None);
    }
    let mut layers = vec![Vec::new(); levels.len()];
    for v in std::mem::take(&mut vertices) {
        let Some(i) = levels
            .iter()
            .position(|z| (projected[v].1 - z).abs() <= NATIVE_TOL)
        else {
            return Ok(None);
        };
        layers[i].push(v);
    }
    let mut arcs = vec![];
    let mut radial = 0_f64;
    for (i, layer) in layers.iter_mut().enumerate() {
        layer.sort_by(|&a, &b| angle(projected[a].0).total_cmp(&angle(projected[b].0)));
        if layer.len() < MIN_POINTS {
            return Ok(None);
        }
        let endpoints = [layer[0], *layer.last().unwrap()];
        let radius = projected[endpoints[0]].0[0];
        if radius <= tol * 4. {
            return Ok(None);
        }
        let arc = CircularArc {
            center: add(center, scale(axis, levels[i])),
            axis,
            radius_mm: radius,
            sweep_rad: PI,
            vertices: endpoints,
        };
        for (v, dir) in endpoints.into_iter().zip([1., -1.]) {
            if norm(sub(
                a.mesh.vertices[v],
                add(arc.center, scale(start, radius * dir)),
            )) > NATIVE_TOL
            {
                return Ok(None);
            }
        }
        for &v in layer.iter() {
            let p = projected[v].0;
            radial = radial.max((p[0].hypot(p[1]) - radius).abs());
        }
        arcs.push(arc);
    }
    // Rigid transforms can introduce different last bits for the same radius.
    // Canonicalize only roundoff-scale equality, then repeat the endpoint and
    // complete radial witnesses. Material radius differences remain conical.
    for i in 1..arcs.len() {
        let numerical = 128. * f64::EPSILON * (1. + norm(center) + arcs[i].radius_mm);
        if (arcs[i].radius_mm - arcs[i - 1].radius_mm).abs() <= numerical {
            arcs[i].radius_mm = arcs[i - 1].radius_mm;
        }
    }
    radial = 0.;
    for (arc, layer) in arcs.iter().zip(&layers) {
        for (v, dir) in arc.vertices.into_iter().zip([1., -1.]) {
            if norm(sub(
                a.mesh.vertices[v],
                add(arc.center, scale(start, arc.radius_mm * dir)),
            )) > NATIVE_TOL
            {
                return Ok(None);
            }
        }
        for &v in layer {
            let p = projected[v].0;
            radial = radial.max((p[0].hypot(p[1]) - arc.radius_mm).abs());
        }
    }
    if radial > tol {
        return Ok(None);
    }
    // Outer source cap chains must coincide exactly, not merely approximately.
    if layers[0] != lower_chain || *layers.last().unwrap() != upper_chain {
        return Ok(None);
    }
    let mut groups = vec![BTreeSet::new(); levels.len() - 1];
    for &t in &selected {
        work.spend(levels.len())?;
        let zs = a.mesh.triangles[t].map(|v| projected[v].1);
        let Some(band) = (0..groups.len()).find(|&i| {
            zs.iter()
                .all(|z| *z >= levels[i] - NATIVE_TOL && *z <= levels[i + 1] + NATIVE_TOL)
        }) else {
            return Ok(None);
        };
        groups[band].insert(t);
    }
    let mut bands = vec![];
    let mut chord = 0_f64;
    let mut cylinder_count = 0;
    let mut cone_count = 0;
    for (i, triangles) in groups.into_iter().enumerate() {
        if triangles.is_empty() {
            return Ok(None);
        }
        let low = &arcs[i];
        let high = &arcs[i + 1];
        let h = levels[i + 1] - levels[i];
        if h <= tol * 4. {
            return Ok(None);
        }
        let cylindrical = (high.radius_mm - low.radius_mm).abs() <= NATIVE_TOL;
        // Cylinders use one exact radius in native construction.
        if cylindrical && high.radius_mm != low.radius_mm {
            return Ok(None);
        }
        if cylindrical {
            cylinder_count += 1;
        } else {
            cone_count += 1;
        }
        let mut area = Sum::default();
        let mut boundary = BTreeSet::new();
        for &t in &triangles {
            work.spend(3)?;
            let ids = a.mesh.triangles[t];
            let uv = ids.map(|v| [angle(projected[v].0), projected[v].1]);
            let signed = (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1])
                - (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]);
            if sign * signed <= 0. {
                return Ok(None);
            }
            area.add(sign * signed * 0.5);
            let span = uv.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
                - uv.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
            if span > PI / 4. + 1e-12 {
                return Ok(None);
            }
            chord = chord.max(2. * low.radius_mm.max(high.radius_mm) * (span / 4.).sin().powi(2));
            for v in ids {
                let (p, z) = projected[v];
                let expected =
                    low.radius_mm + (high.radius_mm - low.radius_mm) * (z - levels[i]) / h;
                let residual = (p[0].hypot(p[1]) - expected).abs();
                if residual > tol {
                    return Ok(None);
                }
                radial = radial.max(residual);
            }
            for (u, v) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
                let e = edge(u, v);
                if edges[&e].iter().any(|t| !triangles.contains(t)) {
                    boundary.insert(e);
                }
            }
        }
        if (area.get() - PI * h).abs() > 8. * NATIVE_TOL * (PI + h / low.radius_mm) + PI * h * 1e-10
        {
            return Ok(None);
        }
        for layer in [&layers[i], &layers[i + 1]] {
            for p in layer.windows(2) {
                if !boundary.remove(&edge(p[0], p[1])) {
                    return Ok(None);
                }
            }
        }
        let Some(rail0) = rail(
            low.vertices[0],
            high.vertices[0],
            &mut boundary,
            &triangles,
            owners,
            edges,
            vertex_triangles,
            &a.mesh.vertices,
            work,
        )?
        else {
            return Ok(None);
        };
        let Some(rail1) = rail(
            low.vertices[1],
            high.vertices[1],
            &mut boundary,
            &triangles,
            owners,
            edges,
            vertex_triangles,
            &a.mesh.vertices,
            work,
        )?
        else {
            return Ok(None);
        };
        if !boundary.is_empty() {
            return Ok(None);
        }
        bands.push(Band {
            triangles,
            rails: [rail0, rail1],
            cylindrical,
        });
    }
    if cylinder_count == 0
        || cone_count == 0
        || radial + chord + o.weld_tolerance_mm + 2. * NATIVE_TOL > o.max_deviation_mm
    {
        return Ok(None);
    }
    let consumed: BTreeSet<_> = selected
        .iter()
        .filter_map(|&t| match owners[t] {
            Some(MixedFaceRef::Plane { index }) => Some(index),
            _ => None,
        })
        .collect();
    for (t, owner) in owners.iter().enumerate() {
        work.spend(1)?;
        if let Some(MixedFaceRef::Plane { index }) = owner {
            if consumed.contains(index) && !selected.contains(&t) {
                return Ok(None);
            }
        }
    }
    // Converting a polygon support to an analytic-wire face makes the whole
    // contour native-constrained. Check every retained support vertex, not only
    // the new arc/rail; safe_planes cannot subdivide a curved contour later.
    let support_planes: BTreeSet<_> = [lo.plane, hi.plane]
        .into_iter()
        .chain(bands.iter().flat_map(|b| b.rails.iter().map(|(_, p)| *p)))
        .collect();
    for index in support_planes {
        let p = &model.planes[index];
        let normal = scale(p.normal, 1. / norm(p.normal));
        let origin = loop_origin(model, &p.loops[0]);
        let circle_residual = |center: V, axis: V, radius: f64| {
            dot(sub(center, origin), normal).abs()
                + radius * norm(cross(normal, scale(axis, 1. / norm(axis))))
        };
        for lp in &p.loops {
            let residual = match lp {
                MixedLoop::Polygon { vertices } => {
                    work.spend(vertices.len())?;
                    vertices
                        .iter()
                        .map(|&v| dot(sub(model.vertices[v], origin), normal).abs())
                        .fold(0_f64, f64::max)
                }
                MixedLoop::Circle { ring } => {
                    work.spend(1)?;
                    let ring = &model.rings[*ring];
                    circle_residual(ring.center, ring.axis, ring.radius_mm)
                }
                MixedLoop::Segments { segments } => {
                    work.spend(segments.len())?;
                    segments
                        .iter()
                        .map(|s| match s {
                            MixedSegment::Line { vertices } => vertices
                                .iter()
                                .map(|&v| dot(sub(model.vertices[v], origin), normal).abs())
                                .fold(0_f64, f64::max),
                            MixedSegment::EllipticArc { arc, .. } => {
                                let arc = &model.elliptic_arcs[*arc];
                                circle_residual(arc.center, arc.axis, arc.major_mm)
                            }
                            MixedSegment::Arc { arc, .. } => {
                                let arc = &model.arcs[*arc];
                                circle_residual(arc.center, arc.axis, arc.radius_mm)
                            }
                        })
                        .fold(0_f64, f64::max)
                }
            };
            if residual > 5e-8 {
                return Ok(None);
            }
        }
    }
    for e in cap_edges {
        if edges[&e].iter().filter(|t| selected.contains(t)).count() != 1 {
            return Ok(None);
        }
    }
    Ok(Some(Chain {
        caps: [lo.plane, hi.plane],
        cap_chains: [lower_chain, upper_chain],
        arcs,
        bands,
        inner,
        radial,
        chord,
    }))
}

pub(crate) fn cone_flux(model: &MixedModel, cone: &MixedPartialCone, reference: V) -> f64 {
    let a = &model.arcs[cone.arcs[0]];
    let b = &model.arcs[cone.arcs[1]];
    let h = dot(sub(b.center, a.center), a.axis);
    let x = scale(
        sub(model.vertices[a.vertices[0]], a.center),
        1. / a.radius_mm,
    );
    let y = cross(a.axis, x);
    let integral = add(
        scale(x, a.sweep_rad.sin()),
        scale(y, 1. - a.sweep_rad.cos()),
    );
    let slope = (b.radius_mm - a.radius_mm) / h;
    let offset = sub(a.center, reference);
    (if cone.inner { -1. } else { 1. })
        * h
        * (a.radius_mm + b.radius_mm)
        * 0.5
        * (dot(offset, integral) + a.sweep_rad * (a.radius_mm - slope * dot(offset, a.axis)))
        / 3.
}
