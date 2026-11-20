//! Compose a straight-edged plane, or a planar collar along a measured rim,
//! from the original segmentation. Recheck after projecting the rim vertices.
use super::{collars, Frame, Torus, TorusAtlas};
use crate::math::{cross, dot, sub};
use crate::{Analysis, PlanarFace};
use std::collections::BTreeMap;

pub struct ResidualPlane<'a> {
    pub face: &'a PlanarFace,
    pub triangles: Vec<usize>,
}

pub fn residual_planes<'a>(
    analysis: &'a Analysis,
    triangle_faces: &[usize],
    atlas: &TorusAtlas,
) -> Vec<ResidualPlane<'a>> {
    let mesh = &analysis.mesh;
    let faces = &analysis.model.faces;
    if triangle_faces.len() != mesh.triangles.len() || faces.len() > mesh.triangles.len() {
        return Vec::new();
    }
    let mut usable = vec![true; faces.len()];
    let mut counts = vec![0; faces.len()];
    let frame = Frame([atlas.axis, atlas.radial, cross(atlas.axis, atlas.radial)]);
    let mut adjusted = BTreeMap::new();
    let mut arcs = BTreeMap::new();
    for chart in &atlas.charts {
        for &t in &chart.triangles {
            let Some(flag) = triangle_faces.get(t).and_then(|&i| usable.get_mut(i)) else {
                return Vec::new();
            };
            *flag = false;
        }
        let torus = Torus {
            center: frame.local(chart.center_mm),
            major: chart.major_radius_mm,
            minor: chart.minor_radius_mm,
        };
        for ring in &chart.loops {
            for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                if a.vertex >= mesh.vertices.len() || b.vertex >= mesh.vertices.len() {
                    return Vec::new();
                }
                let arc = collars::arc(torus, *a, *b);
                arcs.insert(
                    (a.vertex.min(b.vertex), a.vertex.max(b.vertex)),
                    (frame.world(arc.pole), arc.weight),
                );
                adjusted.insert(a.vertex, frame.world(torus.point(a.uv)));
            }
        }
    }
    for (&id, t) in triangle_faces.iter().zip(&mesh.triangles) {
        let Some(flag) = usable.get_mut(id) else {
            return Vec::new();
        };
        counts[id] += 1;
        if !*flag {
            continue;
        }
        let face = &faces[id];
        let Some(origin) = face
            .loops
            .first()
            .and_then(|r| r.first())
            .and_then(|&v| mesh.vertices.get(v))
        else {
            *flag = false;
            continue;
        };
        if face.loops.len() != 1
            || !(3..=4096).contains(&face.loops[0].len())
            || t.iter().any(|&v| v >= mesh.vertices.len())
        {
            *flag = false;
            continue;
        }
        let on_plane = |p| dot(sub(p, *origin), face.normal).abs() <= 1e-10;
        let src = t.map(|v| mesh.vertices[v]);
        let dst = t.map(|v| adjusted.get(&v).copied().unwrap_or(mesh.vertices[v]));
        *flag = src.into_iter().all(on_plane)
            && dst.into_iter().all(on_plane)
            && [src, dst]
                .into_iter()
                .all(|[a, b, c]| dot(cross(sub(b, a), sub(c, a)), face.normal) > 1e-14);
        let mut curved = false;
        for k in 0..3 {
            let a = t[k];
            let b = t[(k + 1) % 3];
            if let Some(&(pole, weight)) = arcs.get(&(a.min(b), a.max(b))) {
                *flag &= !curved
                    && on_plane(pole)
                    && collars::positive_fan(
                        pole,
                        weight,
                        dst[k],
                        dst[(k + 1) % 3],
                        dst[(k + 2) % 3],
                        face.normal,
                    );
                curved = true;
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for (t, &id) in triangle_faces.iter().enumerate() {
        if usable[id] && counts[id] > 1 {
            groups.entry(id).or_default().push(t);
        }
    }
    groups
        .into_iter()
        .map(|(id, triangles)| ResidualPlane {
            face: &faces[id],
            triangles,
        })
        .collect()
}
