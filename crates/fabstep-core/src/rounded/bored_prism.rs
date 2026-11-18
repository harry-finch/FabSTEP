//! Recover a convex quadrilateral prism with one through bore after ordinary
//! recognition retained only planes. The two end polygons supply the four
//! generators. All six planar charts and the periodic cylinder are checked
//! against their original triangles before publishing the replacement.
use super::*;
use crate::planar::Segmentation;

const MAX_TRIANGLES: usize = 32_000;

pub(super) fn split_caps(s: &Segmentation) -> bool {
    // Only a short list of opposite large outlines can be evidence of two
    // fragmented ends. Keep the speculative contour search bounded.
    if s.triangle_regions.len() > MAX_TRIANGLES {
        return false;
    }
    let normals: Vec<_> = s
        .model
        .faces
        .iter()
        .filter(|f| f.loops.len() == 1 && f.loops[0].len() >= 12)
        .map(|f| f.normal)
        .take(17)
        .collect();
    normals.len() <= 16
        && normals
            .iter()
            .enumerate()
            .any(|(i, &n)| normals[..i].iter().any(|&m| dot(m, n) < -1. + 1e-8))
}

fn decline() -> FabError {
    unproven("prisme à perçage traversant non démontré")
}

/// Use the exterior diagonals for the axis and all samples on both openings
/// for the common radius and center. A tiny STL seed facet is a poor normal
/// estimator on a long end face.
fn cylinder_frame(
    mesh: &crate::Mesh,
    s: &Segmentation,
    ends: &mut [&Rim; 2],
    o: &Options,
) -> Result<(Frame, [f64; 2], f64), FabError> {
    let diagonal_normal = |r: &Rim| {
        let p = &s.model.faces[r.region].loops[0];
        cross(
            sub(mesh.vertices[p[2]], mesh.vertices[p[0]]),
            sub(mesh.vertices[p[3]], mesh.vertices[p[1]]),
        )
    };
    let normal = sub(diagonal_normal(ends[0]), diagonal_normal(ends[1]));
    if norm(normal) <= 0. {
        return Err(decline());
    }
    let mut frame = Frame::new(ends[0].center, normal);

    ends.sort_by(|a, b| frame.project(a.center)[2].total_cmp(&frame.project(b.center)[2]));
    let fit = o.cylinder_fit_tolerance_mm;

    let mut heights = [0.; 2];
    let mut centers = [[0.; 2]; 2];
    let mut radius = 0.;
    for (i, r) in ends.iter().enumerate() {
        let projected: Vec<_> = r
            .vertices
            .iter()
            .map(|&v| frame.project(mesh.vertices[v]))
            .collect();
        heights[i] = projected.iter().map(|p| p[2]).sum::<f64>() / projected.len() as f64;
        let circle = fit_circle(
            &projected.iter().map(|p| [p[0], p[1]]).collect::<Vec<_>>(),
            fit,
        )
        .ok_or_else(decline)?;
        centers[i] = circle.center;
        radius += circle.radius * 0.5;
    }
    if heights[1] - heights[0] <= 2. * o.plane_tolerance_mm {
        return Err(decline());
    }
    frame.origin = frame.lift([
        (centers[0][0] + centers[1][0]) * 0.5,
        (centers[0][1] + centers[1][1]) * 0.5,
        0.,
    ]);
    Ok((frame, heights, radius))
}

pub(super) fn recover(
    a: &Analysis,
    o: &Options,
    s: &Segmentation,
    rims: &[Rim],
    edges: &BTreeMap<Edge, Vec<usize>>,
    work: &mut Work,
    result: &mut RoundedRecognition,
) -> Result<(), FabError> {
    if rims.len() != 2
        || a.mesh.triangles.len() > MAX_TRIANGLES
        || rims.iter().any(|r| {
            let loops = &s.model.faces[r.region].loops;
            loops.len() != 2 || loops[0].len() != 4 || loops[1] != r.vertices
        })
        || rims[0].region == rims[1].region
    {
        return Ok(());
    }
    let mesh = &a.mesh;
    work.spend(12 * mesh.triangles.len() + 8 * mesh.vertices.len())?;
    let outer = |r: &Rim| &s.model.faces[r.region].loops[0];
    let mut ends = [&rims[0], &rims[1]];
    let (frame, heights, radius) = cylinder_frame(mesh, s, &mut ends, o)?;
    let tol = o.plane_tolerance_mm;
    let mut vertices = mesh.vertices.clone();
    let mut displacement = 0_f64;
    let mut sides = Vec::new();
    let mut pairs = Vec::new();
    for &v in outer(ends[0]) {
        let q = frame.project(mesh.vertices[v]);
        let mut matching = outer(ends[1]).iter().copied().filter(|&w| {
            let p = frame.project(mesh.vertices[w]);
            (p[0] - q[0]).hypot(p[1] - q[1]) <= 2. * tol
        });
        let w = matching.next().ok_or_else(decline)?;
        if matching.next().is_some() || pairs.iter().any(|&(_, other)| other == w) {
            return Err(decline());
        }
        let p = frame.project(mesh.vertices[w]);
        for (i, id) in [v, w].into_iter().enumerate() {
            vertices[id] = frame.lift([(q[0] + p[0]) * 0.5, (q[1] + p[1]) * 0.5, heights[i]]);
            displacement = displacement.max(norm(sub(vertices[id], mesh.vertices[id])));
        }
        pairs.push((v, w));
    }
    let mut planes: Vec<_> = ends
        .iter()
        .enumerate()
        .map(|(i, r)| MixedPlane {
            normal: scale(frame.axis, if i == 0 { -1. } else { 1. }),
            loops: vec![
                MixedLoop::Polygon {
                    vertices: outer(r).clone(),
                },
                MixedLoop::Circle { ring: i },
            ],
        })
        .collect();
    for i in 0..4 {
        let (a, aa) = pairs[i];
        let (b, bb) = pairs[(i + 1) % 4];
        let ids = [b, a, aa, bb];
        let n = cross(sub(vertices[a], vertices[b]), frame.axis);
        if norm(n) <= 0. {
            return Err(decline());
        }
        let n = scale(n, 1. / norm(n));
        // Every corner and the full analytic circle lie in the interior
        // half-spaces of this convex outer profile.
        if pairs
            .iter()
            .any(|&(id, _)| id != a && id != b && dot(sub(vertices[id], vertices[a]), n) >= -tol)
            || dot(sub(frame.origin, vertices[a]), n) + radius >= -tol
        {
            return Err(decline());
        }
        sides.push(ids);
        planes.push(MixedPlane {
            normal: n,
            loops: vec![MixedLoop::Polygon {
                vertices: ids.to_vec(),
            }],
        });
    }
    // Test the actual floating-point geometry carried by the native model.
    // Every polygon must be constructible even at translated coordinates.
    for plane in &planes {
        let MixedLoop::Polygon { vertices: ids } = &plane.loops[0] else {
            unreachable!()
        };
        let origin = vertices[ids[0]];
        if ids
            .iter()
            .any(|&id| dot(sub(vertices[id], origin), plane.normal).abs() > 5e-8)
        {
            return Err(decline());
        }
        if let Some(MixedLoop::Circle { ring }) = plane.loops.get(1) {
            let center = frame.lift([0., 0., heights[*ring]]);
            if dot(sub(center, origin), plane.normal).abs() > 5e-8 {
                return Err(decline());
            }
        }
    }
    let mut owners = Vec::with_capacity(mesh.triangles.len());
    let mut wall = Vec::new();
    let mut side_counts = [0usize; 4];
    // Map end triangles to the same circle/corners that define the solid.
    // Remaining (interior) cap vertices project normally onto their end plane.
    let corners: BTreeSet<_> = pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mouths: BTreeSet<_> = ends
        .iter()
        .flat_map(|r| r.vertices.iter().copied())
        .collect();
    for (t, &tri) in mesh.triangles.iter().enumerate() {
        let owner = if let Some(i) = ends.iter().position(|r| r.region == s.triangle_regions[t]) {
            for &id in &tri {
                if corners.contains(&id) {
                    continue;
                }
                let [mut x, mut y, _] = frame.project(mesh.vertices[id]);
                if mouths.contains(&id) {
                    let mag = x.hypot(y);
                    if mag <= 0. {
                        return Err(decline());
                    }
                    x *= radius / mag;
                    y *= radius / mag;
                }
                vertices[id] = frame.lift([x, y, heights[i]]);
                displacement = displacement.max(norm(sub(vertices[id], mesh.vertices[id])));
            }
            RoundedFaceRef::Plane { index: i }
        } else if let Some(i) = sides
            .iter()
            .position(|side| tri.iter().all(|id| side.contains(id)))
        {
            side_counts[i] += 1;
            RoundedFaceRef::Plane { index: i + 2 }
        } else {
            wall.push(t);
            RoundedFaceRef::Cylinder { index: 0 }
        };
        if let RoundedFaceRef::Plane { index } = owner {
            let plane = &planes[index];
            let MixedLoop::Polygon { vertices: ids } = &plane.loops[0] else {
                unreachable!()
            };
            for id in tri {
                displacement = displacement
                    .max(dot(sub(mesh.vertices[id], vertices[ids[0]]), plane.normal).abs());
            }
            let [p, q, r] = tri.map(|id| vertices[id]);
            if dot(cross(sub(q, p), sub(r, p)), planes[index].normal) <= 0. {
                return Err(decline());
            }
        }
        owners.push(owner);
    }
    if side_counts != [2; 4] || displacement > tol || wall.is_empty() {
        return Err(decline());
    }
    let section = Section {
        ends: [0, 1],
        shape: Shape::Cylinder,
        triangles: wall,
    };
    let patch = Patch {
        frame,
        nodes: ends
            .iter()
            .enumerate()
            .map(|(i, r)| Node {
                vertices: r.vertices.clone(),
                radius,
                z: heights[i],
            })
            .collect(),
        sections: vec![section],
        sphere: None,
        triangles: vec![],
    };
    let (errors, inside) = prove_section(a, &patch, &patch.sections[0], edges, o, work)?;
    let bound = o.weld_tolerance_mm + errors.radial + errors.chord + displacement;
    if !inside || !bound.is_finite() || bound > o.max_deviation_mm {
        return Err(decline());
    }
    let profile: Vec<_> = outer(ends[0])
        .iter()
        .map(|&id| {
            let [x, y, _] = patch.frame.project(vertices[id]);
            [x, y]
        })
        .collect();
    let volume = (signed_area(&profile).abs() - PI * radius.powi(2)) * (heights[1] - heights[0]);
    if !volume.is_finite() || volume <= 0. {
        return Err(decline());
    }

    let mixed = &mut result.model.revolved.mixed;
    mixed.vertices = vertices;
    mixed.planes = planes;
    mixed.rings = heights
        .map(|z| CircularRing {
            center: patch.frame.lift([0., 0., z]),
            axis: patch.frame.axis,
            radius_mm: radius,
        })
        .into();
    mixed.cylinders = vec![MixedCylinder {
        rings: [0, 1],
        inner: true,
    }];
    result.triangle_faces = owners;
    result.report.cylinders = 1;
    result.report.expected_volume_mm3 = volume;
    result.report.max_wall_residual_mm = displacement;
    result.report.max_radial_residual_mm = errors.radial;
    result.report.max_chord_error_mm = errors.chord;
    result.report.max_deviation_bound_mm = bound;
    result.report.diagnostics.retain(|d| {
        d.code != "auto.regions_faceted" && d.code != "recognition.conic_intent_assumed"
    });
    result.report.diagnostics.push(Diagnostic {
            code: "recognition.circular_intent_assumed".into(),
            severity: Severity::Warning,
            count: 1,
            message: "Les deux contours polygonaux sont interprétés comme les ouvertures d’un perçage circulaire.".into(),
        });
    let moved_vertices = mixed
        .vertices
        .iter()
        .zip(&mesh.vertices)
        .filter(|(adjusted, source)| adjusted != source)
        .count();
    if moved_vertices > 0 {
        result.report.diagnostics.push(Diagnostic {
            code: "recognition.bored_prism_projection".into(),
            severity: Severity::Info,
            count: moved_vertices,
            message: "Les sommets de la plaque ont été ajustés sur les plans reconstruits. Le déplacement est inclus dans la borne d’écart.".into(),
        });
    }
    Ok(())
}
