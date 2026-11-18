//! Deterministic automatic reconstruction: local uncertainty retains facets,
//! while source, work-budget and final native validation remain mandatory.
use crate::math::*;
use crate::*;
use std::collections::BTreeSet;

pub fn recognize_automatic(a: &Analysis, o: &Options) -> Result<RoundedRecognition, FabError> {
    recognize(a, o, None)
}
fn validate(a: &Analysis, o: &Options) -> Result<(), FabError> {
    crate::recognition::validate_analysis(a)?;
    for v in [
        o.weld_tolerance_mm,
        o.plane_tolerance_mm,
        o.cylinder_fit_tolerance_mm,
        o.max_deviation_mm,
    ] {
        if !v.is_finite() || v <= 0. {
            return Err(FabError::new(
                "options.invalid_tolerance",
                "Tolérances invalides.",
            ));
        }
    }
    Ok(())
}
fn recognize(
    a: &Analysis,
    o: &Options,
    scope: Option<&BTreeSet<usize>>,
) -> Result<RoundedRecognition, FabError> {
    validate(a, o)?;
    let active = Options {
        reconstruction: ReconstructionMode::Surfaces,
        ..o.clone()
    };
    let mut result = match crate::rounded::recognize_auto_patches(a, &active, scope) {
        Ok(r) => r,
        Err(e) if e.code == "recognition.unsupported" => {
            let mut complete = None;
            for attempt in [
                crate::recognize_complete_torus
                    as fn(&Analysis, &Options) -> Result<RoundedRecognition, FabError>,
                crate::recognize_complete_sphere,
            ] {
                match attempt(a, &active) {
                    Ok(r) => {
                        complete = Some(r);
                        break;
                    }
                    Err(e)
                        if matches!(
                            e.code.as_str(),
                            "recognition.unsupported"
                                | "recognition.surface_unproven"
                                | "recognition.deviation_exceeded"
                        ) => {}
                    Err(e) => return Err(e),
                }
            }
            match complete {
                Some(r) => r,
                None => planar(a, o)?,
            }
        }
        Err(e) => return Err(e),
    };
    if result.report.cylinders + result.report.cones + result.report.spheres + result.report.tori
        == 0
    {
        for attempt in [
            crate::recognize_complete_torus
                as fn(&Analysis, &Options) -> Result<RoundedRecognition, FabError>,
            crate::recognize_complete_sphere,
        ] {
            match attempt(a, &active) {
                Ok(r) => {
                    result = r;
                    break;
                }
                Err(e)
                    if matches!(
                        e.code.as_str(),
                        "recognition.unsupported"
                            | "recognition.surface_unproven"
                            | "recognition.deviation_exceeded"
                    ) => {}
                Err(e) => return Err(e),
            }
        }
    }
    // Local D/G blocks already own their entire source partition, including
    // the shared analytic boundaries that adjoining planes must preserve.
    if !result.model.octants.is_empty() || result.model.linear_blend.is_some() {
        refresh(a, o, &mut result)?;
        return Ok(result);
    }
    crate::partial::chain::augment(a, &mut result, o, scope)?;
    safe_planes(a, &mut result)?;
    refresh(a, o, &mut result)?;
    Ok(result)
}
fn planar(a: &Analysis, o: &Options) -> Result<RoundedRecognition, FabError> {
    let adj = crate::pipeline::validate_topology(&a.mesh)?;
    let s = crate::planar::build_with_provenance(&a.mesh, &adj, o.plane_tolerance_mm)?;
    Ok(RoundedRecognition {
        model: RoundedModel {
            linear_blend: None,
            octants: vec![],
            partial_cones: vec![],
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices: a.mesh.vertices.clone(),
                    rings: vec![],
                    planes: s
                        .model
                        .faces
                        .into_iter()
                        .map(|f| MixedPlane {
                            normal: f.normal,
                            loops: f
                                .loops
                                .into_iter()
                                .map(|vertices| MixedLoop::Polygon { vertices })
                                .collect(),
                        })
                        .collect(),
                    cylinders: vec![],
                    arcs: vec![],
                    elliptic_arcs: vec![],
                    partial_cylinders: vec![],
                },
                cones: vec![],
            },
            spheres: vec![],
            tori: vec![],
        },
        triangle_faces: s
            .triangle_regions
            .into_iter()
            .map(|index| RoundedFaceRef::Plane { index })
            .collect(),
        report: RecognitionReport {
            cylinders: 0,
            cones: 0,
            spheres: 0,
            tori: 0,
            toroidal_fillets: 0,
            max_radial_residual_mm: 0.,
            max_chord_error_mm: 0.,
            max_wall_residual_mm: 0.,
            max_deviation_bound_mm: 0.,
            expected_volume_mm3: 0.,
            diagnostics: vec![Diagnostic {
                code: "auto.faceted_base".into(),
                severity: Severity::Info,
                count: 1,
                message: "Aucune forme courbe prouvée ; les facettes source sont conservées."
                    .into(),
            }],
        },
    })
}
fn safe_planes(a: &Analysis, r: &mut RoundedRecognition) -> Result<(), FabError> {
    let old = &r.model.revolved.mixed.planes;
    let mut split = BTreeSet::new();
    for (i, p) in old.iter().enumerate() {
        if p.loops
            .iter()
            .all(|lp| matches!(lp, MixedLoop::Polygon { .. }))
        {
            let origin = crate::partial::loop_origin(&r.model.revolved.mixed, &p.loops[0]);
            if p.loops.iter().any(|lp| match lp {
                MixedLoop::Polygon { vertices } => vertices
                    .iter()
                    .any(|&v| dot(sub(a.mesh.vertices[v], origin), p.normal).abs() > 5e-8),
                _ => false,
            }) {
                split.insert(i);
            }
        }
    }
    if split.is_empty() {
        return Ok(());
    }
    let mut mapping = vec![0; old.len()];
    let mut planes = Vec::new();
    for (i, p) in old.iter().enumerate() {
        if !split.contains(&i) {
            mapping[i] = planes.len();
            planes.push(p.clone());
        }
    }
    for (t, owner) in r.triangle_faces.iter_mut().enumerate() {
        if let RoundedFaceRef::Plane { index } = owner {
            if split.contains(index) {
                let v = a.mesh.triangles[t];
                let normal = cross(
                    sub(a.mesh.vertices[v[1]], a.mesh.vertices[v[0]]),
                    sub(a.mesh.vertices[v[2]], a.mesh.vertices[v[0]]),
                );
                *index = planes.len();
                planes.push(MixedPlane {
                    normal: scale(normal, 1. / norm(normal)),
                    loops: vec![MixedLoop::Polygon {
                        vertices: v.to_vec(),
                    }],
                });
            } else {
                *index = mapping[*index];
            }
        }
    }
    r.model.revolved.mixed.planes = planes;
    r.report.diagnostics.push(Diagnostic{code:"auto.planar_regions_split".into(),severity:Severity::Info,count:split.len(),message:"Des regroupements plans non constructibles ont été remplacés par leurs triangles source, sans déplacement.".into()});
    Ok(())
}
fn refresh(a: &Analysis, o: &Options, r: &mut RoundedRecognition) -> Result<(), FabError> {
    let mut residual = 0_f64;
    let mut face_residuals = vec![(0_f64, 0_f64); r.model.revolved.mixed.planes.len()];
    for (t, owner) in r.triangle_faces.iter().enumerate() {
        if let RoundedFaceRef::Plane { index } = owner {
            let p = &r.model.revolved.mixed.planes[*index];
            let origin = crate::partial::loop_origin(&r.model.revolved.mixed, &p.loops[0]);
            for &v in &a.mesh.triangles[t] {
                let offset = sub(a.mesh.vertices[v], origin);
                let distance = dot(offset, p.normal).abs();
                residual = residual.max(distance);
                face_residuals[*index].0 = face_residuals[*index].0.max(distance);
                face_residuals[*index].1 = face_residuals[*index].1.max(norm(offset));
            }
        }
    }
    let approximate = face_residuals
        .iter()
        .filter(|(residual, extent)| *residual > extent * f64::EPSILON * 128.)
        .count();
    r.report
        .diagnostics
        .retain(|d| d.code != "guidance.plane_residual");
    if approximate > 0 {
        r.report.diagnostics.push(Diagnostic{code:"guidance.plane_residual".into(),severity:Severity::Warning,count:approximate,message:"Certaines faces planes simplifient un faible relief source ; le résidu est inclus dans la borne d’écart.".into()});
    }
    for diagnostic in &mut r.report.diagnostics {
        if diagnostic.code == "recognition.rounded_coverage" {
            diagnostic.count = r.triangle_faces.len();
            diagnostic.message = format!("Tous les triangles sont attribués : {} sphères, {} tores, {} cônes, {} cylindres et {} plans. Frontières, raccords et éventuels pôles issus de la source.", r.report.spheres, r.report.tori, r.report.cones, r.report.cylinders, r.model.revolved.mixed.planes.len());
        }
    }
    r.report.max_wall_residual_mm = r.report.max_wall_residual_mm.max(residual);
    r.report.max_deviation_bound_mm = o.weld_tolerance_mm
        + r.report.max_wall_residual_mm
        + r.report.max_radial_residual_mm
        + r.report.max_chord_error_mm;
    if r.report.max_deviation_bound_mm > o.max_deviation_mm {
        return Err(FabError::new(
            "recognition.deviation_exceeded",
            "Le résultat automatique dépasse le budget d’écart.",
        ));
    }
    if !r.model.spheres.iter().any(|s| s.rings.is_none())
        && !r.model.tori.iter().any(|t| t.rings.is_none())
    {
        r.report.expected_volume_mm3 = crate::rounded::volume(&r.model)?;
    }
    Ok(())
}

/// Pure source-triangle region, never a mesh-neighbour flood or export grant.
pub fn automatic_region(
    a: &Analysis,
    o: &Options,
    seed_triangle: usize,
) -> Result<Vec<usize>, FabError> {
    if seed_triangle >= a.mesh.triangles.len() {
        return Err(invalid());
    }
    let result = recognize_automatic(a, o)?;
    guided_rounded_region(&result, GuidanceFamily::Auto, &[seed_triangle])
}
fn invalid() -> FabError {
    FabError::new(
        "guidance.invalid_selection",
        "Sélection absente, dupliquée ou hors du corps source.",
    )
}
fn unproven() -> FabError {
    FabError::new(
        "guidance.family_unproven",
        "La sélection ne correspond pas à une zone prouvée de la famille demandée.",
    )
}
fn torus_group(r: &RoundedRecognition, owner: RoundedFaceRef) -> Option<Vec<RoundedFaceRef>> {
    let torus = match owner {
        RoundedFaceRef::Torus { index } => Some(index),
        RoundedFaceRef::Cylinder { index } => {
            let c = &r.model.revolved.mixed.cylinders[index];
            let candidates: Vec<_> = r
                .model
                .tori
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.rings
                        .is_some_and(|rings| rings.iter().any(|x| c.rings.contains(x)))
                })
                .map(|(i, _)| i)
                .collect();
            (candidates.len() == 1).then(|| candidates[0])
        }
        _ => None,
    }?;
    let mut group = vec![RoundedFaceRef::Torus { index: torus }];
    if let Some(rings) = r.model.tori[torus].rings {
        for (index, c) in r.model.revolved.mixed.cylinders.iter().enumerate() {
            if c.rings.iter().any(|x| rings.contains(x)) {
                group.push(RoundedFaceRef::Cylinder { index });
            }
        }
    }
    // The first guided torus scope is a fillet plus cylindrical supports.
    // Reject chains requiring additional cones/spheres/other curved neighbours.
    let mut boundary_rings = BTreeSet::new();
    for owner in &group {
        match owner {
            RoundedFaceRef::Torus { index } => {
                if let Some(rings) = r.model.tori[*index].rings {
                    boundary_rings.extend(rings);
                }
            }
            RoundedFaceRef::Cylinder { index } => {
                boundary_rings.extend(r.model.revolved.mixed.cylinders[*index].rings)
            }
            _ => {}
        }
    }
    if r.model
        .revolved
        .cones
        .iter()
        .any(|c| c.rings.iter().any(|i| boundary_rings.contains(i)))
        || r.model.spheres.iter().any(|s| {
            s.rings
                .is_some_and(|rings| rings.iter().any(|i| boundary_rings.contains(i)))
        })
        || r.model.tori.iter().enumerate().any(|(i, t)| {
            i != torus
                && t.rings
                    .is_some_and(|rings| rings.iter().any(|i| boundary_rings.contains(i)))
        })
        || r.model
            .revolved
            .mixed
            .cylinders
            .iter()
            .enumerate()
            .any(|(i, c)| {
                !group.contains(&RoundedFaceRef::Cylinder { index: i })
                    && c.rings.iter().any(|i| boundary_rings.contains(i))
            })
    {
        return None;
    }
    Some(group)
}
fn partial_chain_group(
    r: &RoundedRecognition,
    owner: RoundedFaceRef,
) -> Option<Vec<RoundedFaceRef>> {
    let mixed = &r.model.revolved.mixed;
    let arcs = |face| match face {
        RoundedFaceRef::PartialCylinder { index } => Some(mixed.partial_cylinders[index].arcs),
        RoundedFaceRef::PartialCone { index } => Some(r.model.partial_cones[index].arcs),
        _ => None,
    };
    arcs(owner)?;
    let mut incidence: std::collections::BTreeMap<usize, Vec<RoundedFaceRef>> =
        std::collections::BTreeMap::new();
    for face in (0..mixed.partial_cylinders.len())
        .map(|index| RoundedFaceRef::PartialCylinder { index })
        .chain((0..r.model.partial_cones.len()).map(|index| RoundedFaceRef::PartialCone { index }))
    {
        for arc in arcs(face)? {
            incidence.entry(arc).or_default().push(face);
        }
    }
    let mut seen_arcs = BTreeSet::new();
    let mut seen_faces = BTreeSet::new();
    let key = |face| match face {
        RoundedFaceRef::PartialCylinder { index } => (0, index),
        RoundedFaceRef::PartialCone { index } => (1, index),
        _ => unreachable!(),
    };
    let mut pending = vec![owner];
    let mut group = vec![];
    while let Some(face) = pending.pop() {
        if !seen_faces.insert(key(face)) {
            continue;
        }
        group.push(face);
        for arc in arcs(face)? {
            if seen_arcs.insert(arc) {
                pending.extend(incidence.get(&arc)?.iter().copied());
            }
        }
    }
    Some(group)
}
pub fn guided_rounded_region(
    r: &RoundedRecognition,
    family: GuidanceFamily,
    selected: &[usize],
) -> Result<Vec<usize>, FabError> {
    if selected.is_empty()
        || selected.iter().any(|&t| t >= r.triangle_faces.len())
        || selected.iter().collect::<BTreeSet<_>>().len() != selected.len()
    {
        return Err(invalid());
    }
    // Selection order is not a family hypothesis. A joint chain can begin
    // with its dependent band; require an actual selected witness of the
    // explicit family, then still validate EVERY selected face against it.
    let owner = match family {
        GuidanceFamily::Cylinder => selected
            .iter()
            .map(|&t| r.triangle_faces[t])
            .find(|f| {
                matches!(
                    f,
                    RoundedFaceRef::Cylinder { .. } | RoundedFaceRef::PartialCylinder { .. }
                )
            })
            .ok_or_else(unproven)?,
        GuidanceFamily::Cone => selected
            .iter()
            .map(|&t| r.triangle_faces[t])
            .find(|f| {
                matches!(
                    f,
                    RoundedFaceRef::Cone { .. } | RoundedFaceRef::PartialCone { .. }
                )
            })
            .ok_or_else(unproven)?,
        _ => r.triangle_faces[selected[0]],
    };
    let group = match family {
        GuidanceFamily::Auto => partial_chain_group(r, owner)
            .or_else(|| torus_group(r, owner))
            .unwrap_or_else(|| vec![owner]),
        GuidanceFamily::Torus => torus_group(r, owner).ok_or_else(unproven)?,
        GuidanceFamily::Cylinder
            if matches!(
                owner,
                RoundedFaceRef::Cylinder { .. } | RoundedFaceRef::PartialCylinder { .. }
            ) =>
        {
            partial_chain_group(r, owner).unwrap_or_else(|| vec![owner])
        }
        GuidanceFamily::Cone
            if matches!(
                owner,
                RoundedFaceRef::Cone { .. } | RoundedFaceRef::PartialCone { .. }
            ) =>
        {
            partial_chain_group(r, owner).unwrap_or_else(|| vec![owner])
        }
        GuidanceFamily::Plane if matches!(owner, RoundedFaceRef::Plane { .. }) => vec![owner],
        _ => return Err(unproven()),
    };
    if selected
        .iter()
        .any(|&t| !group.contains(&r.triangle_faces[t]))
    {
        return Err(unproven());
    }
    Ok(r.triangle_faces
        .iter()
        .enumerate()
        .filter_map(|(t, o)| group.contains(o).then_some(t))
        .collect())
}

pub fn recognize_automatic_guided(
    a: &Analysis,
    o: &Options,
    requests: &[(GuidanceFamily, Vec<usize>)],
) -> Result<RoundedRecognition, FabError> {
    crate::recognition::validate_guidance_size(a)?;
    validate(a, o)?;
    if requests.len() > 16 {
        return Err(FabError::new(
            "guidance.limit",
            "Au plus seize indications.",
        ));
    }
    let mut all = BTreeSet::new();
    let mut scope = BTreeSet::new();
    for (family, selection) in requests {
        if selection.is_empty() {
            return Err(invalid());
        }
        for &t in selection {
            if t >= a.mesh.triangles.len() {
                return Err(invalid());
            }
            if !all.insert(t) {
                return Err(FabError::new(
                    "guidance.overlapping_indications",
                    "Indications dupliquées ou recouvrantes.",
                ));
            }
            if *family != GuidanceFamily::Plane {
                scope.insert(t);
            }
        }
    }
    let mut result = if scope.is_empty() && o.reconstruction != ReconstructionMode::Auto {
        planar(a, o)?
    } else {
        recognize(
            a,
            o,
            (o.reconstruction != ReconstructionMode::Auto).then_some(&scope),
        )?
    };
    if !requests.is_empty()
        && (!result.model.octants.is_empty() || result.model.linear_blend.is_some())
    {
        return Err(unproven());
    }
    let mut planes: Vec<_> = requests
        .iter()
        .filter(|(f, _)| *f == GuidanceFamily::Plane)
        .map(|(_, s)| s.iter().copied().collect::<BTreeSet<_>>())
        .collect();
    planes.sort();
    let adjacency = crate::pipeline::validate_topology(&a.mesh)?;
    let mut checks = 0;
    for selected in planes {
        let mut replaced = BTreeSet::new();
        for &t in &selected {
            if let RoundedFaceRef::Plane { index } = result.triangle_faces[t] {
                replaced.insert(index);
            } else {
                return Err(unproven());
            }
        }
        if result.triangle_faces.iter().enumerate().any(|(t, owner)| {
            matches!(owner,RoundedFaceRef::Plane{index} if replaced.contains(index))
                && !selected.contains(&t)
        }) {
            return Err(FabError::new(
                "guidance.plane_incomplete",
                "Sélectionner les faces planes entières.",
            ));
        }
        if replaced.iter().any(|&i| {
            result.model.revolved.mixed.planes[i]
                .loops
                .iter()
                .any(|lp| !matches!(lp, MixedLoop::Polygon { .. }))
        }) {
            return Err(FabError::new(
                "guidance.plane_unsupported_boundary",
                "Contour courbe non pris en charge.",
            ));
        }
        let face = crate::planar::guided_face(
            &a.mesh,
            &adjacency,
            &selected,
            o.plane_tolerance_mm,
            &mut checks,
        )?;
        let mut mapping = vec![0; result.model.revolved.mixed.planes.len()];
        let mut merged = Vec::new();
        for (i, p) in result.model.revolved.mixed.planes.iter().enumerate() {
            if !replaced.contains(&i) {
                mapping[i] = merged.len();
                merged.push(p.clone());
            }
        }
        let added = merged.len();
        merged.push(MixedPlane {
            normal: face.normal,
            loops: face
                .loops
                .into_iter()
                .map(|vertices| MixedLoop::Polygon { vertices })
                .collect(),
        });
        for owner in &mut result.triangle_faces {
            if let RoundedFaceRef::Plane { index } = owner {
                *index = if replaced.contains(index) {
                    added
                } else {
                    mapping[*index]
                };
            }
        }
        result.model.revolved.mixed.planes = merged;
    }
    let mut occupied = BTreeSet::new();
    for (family, selected) in requests {
        for t in guided_rounded_region(&result, *family, selected)? {
            if !occupied.insert(t) {
                return Err(FabError::new(
                    "guidance.overlapping_indications",
                    "Les parois prouvées se recouvrent.",
                ));
            }
        }
    }
    if o.reconstruction != ReconstructionMode::Auto
        && result
            .triangle_faces
            .iter()
            .enumerate()
            .any(|(t, f)| !matches!(f, RoundedFaceRef::Plane { .. }) && !occupied.contains(&t))
    {
        return Err(FabError::new(
            "guidance.family_unproven",
            "Le raccord exige d’autres surfaces hors de la sélection prise en charge.",
        ));
    }
    refresh(a, o, &mut result)?;
    Ok(result)
}
