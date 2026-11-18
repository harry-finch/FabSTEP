//! Two disjoint exterior rolling cones on opposite, parallel box corners.
use super::*;

// A four-sided cap has lost both opposite corners. Each of the four side
// planes is adjacent to both caps and one other side. Restore only the
// second missing adjacency to identify the first corner with the single
// blend frame routine. The actual mesh incidence is verified below.
fn frames(
    top: &[(f64, usize)],
    neighbors: [u8; 6],
    a: &Analysis,
    s: &crate::planar::Segmentation,
    work: &mut Work,
) -> Result<Option<[Block; 2]>, FabError> {
    let mut missing = Vec::new();
    for i in 0..6 {
        for j in i + 1..6 {
            if neighbors[i].count_ones() == 3
                && neighbors[j].count_ones() == 3
                && neighbors[i] & (1 << j) == 0
                && dot(
                    s.model.faces[top[i].1].normal,
                    s.model.faces[top[j].1].normal,
                )
                .abs()
                    < 1e-8
            {
                missing.push((i, j));
            }
        }
    }
    let [_, (i, j)] = missing.as_slice() else {
        return Ok(None);
    };
    let mut single = neighbors;
    single[*i] |= 1 << j;
    single[*j] |= 1 << i;
    let Some(f) = frame(top, single, a, s, work)? else {
        return Ok(None);
    };
    let opposite = Block {
        origin: f.world([f.lengths[0], f.lengths[1], 0.]),
        axes: [scale(f.axes[0], -1.), scale(f.axes[1], -1.), f.axes[2]],
        regions: [3, 4, 2, 0, 1, 5].map(|i| f.regions[i]),
        lengths: f.lengths,
    };
    Ok(Some([f, opposite]))
}

fn partition(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    f: &Block,
    adj: &[Vec<usize>],
    work: &mut Work,
) -> Result<Vec<usize>, FabError> {
    work.spend(s.model.faces.len() + 2 * a.mesh.triangles.len())?;
    let mut by_region = vec![usize::MAX; s.model.faces.len()];
    for (owner, r) in f.regions.into_iter().enumerate() {
        by_region[r] = owner;
    }
    let mut owners: Vec<_> = s.triangle_regions.iter().map(|&r| by_region[r]).collect();
    let mut patches = 0;
    for start in 0..owners.len() {
        if owners[start] != usize::MAX {
            continue;
        }
        patches += 1;
        if patches > 2 {
            return Err(unproven("plus de deux parois entre les supports"));
        }
        let pending = CONE + 2;
        let mut region = vec![start];
        owners[start] = pending;
        let mut i = 0;
        let mut mask = 0u8;
        while i < region.len() {
            let t = region[i];
            i += 1;
            work.spend(5)?;
            for &n in &adj[t] {
                let other = owners[n];
                if other == usize::MAX {
                    owners[n] = pending;
                    region.push(n);
                } else if other < CONE {
                    mask |= 1 << other;
                }
            }
        }
        let owner = match mask {
            0b100111 => CONE,
            0b111100 => CONE + 1,
            _ => return Err(unproven("supports du deuxième cône indéterminés")),
        };
        for t in region {
            owners[t] = owner;
        }
    }
    if patches != 2 {
        return Err(unproven("deux parois disjointes attendues"));
    }
    Ok(owners)
}

fn all_chains(
    a: &Analysis,
    owners: &[usize],
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    work: &mut Work,
) -> Result<Chains, FabError> {
    let mut cuts: BTreeMap<Edge, Vec<Edge>> = BTreeMap::new();
    for (&e, inc) in edges {
        work.spend(1)?;
        let &[i, j] = inc.as_slice() else {
            return Err(unproven("incidence du bloc ouverte"));
        };
        if owners[i] != owners[j] {
            cuts.entry(edge(owners[i], owners[j])).or_default().push(e);
        }
    }
    let mut wanted = BTreeSet::new();
    for i in 0..6 {
        for j in i + 1..6 {
            if i % 3 != j % 3 && (i, j) != (0, 1) && (i, j) != (3, 4) {
                wanted.insert((i, j));
            }
        }
    }
    wanted.extend([0, 1, 2, 5].map(|i| (i, CONE)));
    wanted.extend([3, 4, 2, 5].map(|i| (i, CONE + 1)));
    if wanted != cuts.keys().copied().collect() {
        return Err(unproven("limites des congés non indépendantes"));
    }
    disks::<8>(a, owners, adj, &cuts, work)?;
    cuts.into_iter()
        .map(|(key, es)| {
            work.spend(9 * es.len() + 1)?;
            Ok((key, ordered_chain(&es)?))
        })
        .collect()
}

fn straight_boundaries(
    f: &Block,
    local: &[V],
    chains: &Chains,
    chart: &mut Chart,
    proof: &mut ErrorBound,
    fit: f64,
    work: &mut Work,
) -> Result<(), FabError> {
    for (&(i, j), chain) in chains {
        if j >= CONE {
            continue;
        }
        work.spend(chain.len() * 7)?;
        let along = 3 - i % 3 - j % 3;
        let mut last = f64::NEG_INFINITY;
        let forward = local[chain[0]][along] < local[chain[chain.len() - 1]][along];
        for k in 0..chain.len() {
            let v = chain[if forward { k } else { chain.len() - 1 - k }];
            let p = local[v];
            let end = k == 0 || k + 1 == chain.len();
            let mut ideal = chart.boundary[v].unwrap_or(p);
            if end && chart.boundary[v].is_none() {
                ideal[along] = if p[along] < f.lengths[along] / 2. {
                    0.
                } else {
                    f.lengths[along]
                };
            }
            ideal[i % 3] = f.height(i);
            ideal[j % 3] = f.height(j);
            if ideal[along] < 0. || ideal[along] > f.lengths[along] || ideal[along] <= last {
                return Err(unproven("bord droit replié ou hors bloc"));
            }
            let tolerance = if end && chart.boundary[v].is_none() {
                fit.min(NATIVE / 4.)
            } else {
                fit
            };
            proof.record(p, ideal, tolerance)?;
            if chart.boundary[v].is_some_and(|prior| norm(sub(prior, ideal)) > NATIVE / 8.) {
                return Err(unproven("sommet incompatible entre les limites"));
            }
            chart.boundary[v] = Some(ideal);
            last = ideal[along];
        }
        chart
            .trims
            .insert((i, j), [chain[0], chain[chain.len() - 1]]);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn candidate(
    top: &[(f64, usize)],
    neighbors: [u8; 6],
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let Some([f, other]) = frames(top, neighbors, a, s, work)? else {
        return Ok(None);
    };
    if !f.has_taper(a, s, o, work)? || !other.has_taper(a, s, o, work)? {
        return Ok(None);
    }
    let owners = partition(a, s, &f, adj, work)?;
    let cs = all_chains(a, &owners, adj, edges, work)?;
    let mut proof = ErrorBound::default();
    let fit = o.cylinder_fit_tolerance_mm;
    let mut vertices = a.mesh.vertices.clone();
    let mut arcs = Vec::with_capacity(4);
    let mut trims = BTreeMap::new();
    let mut prove_corner = |i: usize, frame: &Block, face_ids: [usize; 4]| -> Result<_, FabError> {
        work.spend(7 * a.mesh.vertices.len() + 4 * a.mesh.triangles.len())?;
        let local: Vec<_> = a.mesh.vertices.iter().map(|&p| frame.local(p)).collect();
        let mut curve_chains = Chains::new();
        for (face, global) in [0, 1, 2, 5].into_iter().zip(face_ids) {
            curve_chains.insert((face, CONE), cs[&(global, CONE + i)].clone());
        }
        let g = Envelope::new(frame, &local, &mut curve_chains, o)?;
        let chart = boundaries(frame, &g, &local, &curve_chains, fit, &mut proof, work)?;
        for (face, global) in [0, 1, 2, 5].into_iter().zip(face_ids) {
            trims.insert((global, CONE + i), chart.trims[&(face, CONE)]);
        }
        let own: Vec<_> = owners
            .iter()
            .map(|&v| if v == CONE + i { CONE } else { usize::MAX })
            .collect();
        triangles(
            a, frame, &g, &own, &local, &chart, true, o, &mut proof, work,
        )?;
        let setback = g.radii[0].max(g.radii[1]);
        let (domain, changes) =
            append_domain(frame, &g, &mut vertices, &mut arcs, &mut proof, fit)?;
        Ok((local, chart, domain, setback, changes))
    };
    let (local, mut chart, first, r1, n1) = prove_corner(0, &f, [0, 1, 2, 5])?;
    let (_, c2, second, r2, n2) = prove_corner(1, &other, [3, 4, 2, 5])?;
    let moved = n1 + n2;
    if r1 + r2 + 8. * fit >= f.lengths[0].min(f.lengths[1]) {
        return Err(unproven("les domaines des deux congés ne sont pas séparés"));
    }
    for (vertex, p) in c2.boundary.iter().enumerate() {
        if let Some(point) = p {
            if chart.boundary[vertex].is_some() {
                return Err(unproven("sommet partagé entre deux congés"));
            }
            chart.boundary[vertex] = Some(f.local(other.world(*point)));
        }
    }
    straight_boundaries(&f, &local, &cs, &mut chart, &mut proof, fit, work)?;
    trims.extend(
        chart
            .trims
            .iter()
            .filter(|(edge, _)| edge.1 < CONE)
            .map(|(k, v)| (*k, *v)),
    );
    // The same triangle projection used for a single blend must also hold on
    // the two caps with both cuts and on their four shortened side faces.
    for (&ids, &owner) in a.mesh.triangles.iter().zip(&owners) {
        if owner < CONE {
            work.spend(16)?;
            prove_plane_triangle(ids, owner, &f, &local, &chart, o, &mut proof)?;
        }
    }
    let mut planes = Vec::new();
    for (i, r) in f.regions.into_iter().enumerate() {
        let cap = i % 3 == 2;
        let segments = super::super::block::plane_segments(
            &s.model.faces[r].loops[0],
            i,
            if cap { 6 } else { 4 },
            edges,
            &owners,
            work,
            |ids, j| {
                let ends = trims[&edge(i, j)];
                let reversed = ids == [ends[1], ends[0]];
                if !reversed && ids != ends {
                    return Err(unproven("extrémité commune non prouvée"));
                }
                Ok(if cap && j >= CONE {
                    if reversed != (i == 2) {
                        return Err(unproven("sens de l’ellipse incohérent"));
                    }
                    MixedSegment::EllipticArc {
                        arc: 2 * (j - CONE) + usize::from(i == 5),
                        reversed,
                    }
                } else {
                    MixedSegment::Line { vertices: ids }
                })
            },
        )?;
        planes.push(MixedPlane {
            normal: f.normal(i),
            loops: vec![MixedLoop::Segments { segments }],
        });
    }
    let model = RoundedModel {
        revolved: RevolvedModel {
            mixed: MixedModel {
                vertices,
                planes,
                elliptic_arcs: arcs,
                arcs: vec![],
                rings: vec![],
                cylinders: vec![],
                partial_cylinders: vec![],
            },
            cones: vec![],
        },
        linear_blend: Some(Box::new(crate::LinearBlends {
            first,
            second: Some(second),
        })),
        smooth_revolution: None,
        torus_sectors: vec![],
        octants: vec![],
        partial_cones: vec![],
        tori: vec![],
        spheres: vec![],
        unequal_pocket: None,
        cross_drill: None,
    };
    let wall = 2. * proof.plane + proof.projection;
    let deviation = o.weld_tolerance_mm + wall + proof.projection + proof.chord;
    if !deviation.is_finite() || deviation > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Les congés linéaires dépassent la borne d’écart.",
        ));
    }
    let volume = volume(&model)?;
    let mut diagnostics = vec![
        Diagnostic {
            code: "recognition.conic_intent_assumed".into(),
            severity: Severity::Warning,
            count: 2,
            message: "Deux portions de cône inclinées représentent les congés à rayon linéaire."
                .into(),
        },
        Diagnostic {
            code: "recognition.rounded_coverage".into(),
            severity: Severity::Info,
            count: owners.len(),
            message: "Chaque triangle appartient à un des six plans ou à un des deux cônes.".into(),
        },
        Diagnostic {
            code:"recognition.bound_scope".into(),severity:Severity::Info,count:1,
            message:"La borne couvre les deux trapèzes paramétriques, les six plans et leurs limites partagées.".into(),
        },
    ];
    if moved > 0 {
        diagnostics.push(Diagnostic {
            code: "recognition.linear_blend_endpoints_projected".into(),
            severity: Severity::Info,
            count: moved,
            message: "Les extrémités des ellipses ont été projetées dans la borne d’écart.".into(),
        });
    }
    let report = RecognitionReport {
        revolutions: 0,
        cylinders: 0,
        cones: 2,
        spheres: 0,
        tori: 0,
        toroidal_fillets: 0,
        max_radial_residual_mm: proof.projection,
        max_wall_residual_mm: wall,
        max_chord_error_mm: proof.chord,
        max_deviation_bound_mm: deviation,
        expected_volume_mm3: volume,
        diagnostics,
    };
    let triangle_faces = owners
        .into_iter()
        .map(|i| {
            if i < CONE {
                RoundedFaceRef::Plane { index: i }
            } else {
                RoundedFaceRef::LinearBlend { index: i - CONE }
            }
        })
        .collect();
    Ok(Some(RoundedRecognition {
        model,
        report,
        triangle_faces,
    }))
}
