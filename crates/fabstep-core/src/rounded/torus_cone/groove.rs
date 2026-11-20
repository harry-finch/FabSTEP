//! A circular groove between two coaxial lands of the same radius. Its
//! shoulders are witnessed changes of support and need not be tangent.
use super::*;

pub(super) fn search(
    nodes: &[Node],
    options: &Options,
    work: &mut Work,
) -> Result<Option<Layout>, FabError> {
    if !(7..=128).contains(&nodes.len()) {
        return Ok(None);
    }
    work.spend(5 * nodes.len())?;
    let last = nodes.len() - 1;
    let fit = options.cylinder_fit_tolerance_mm;
    let epsilon = fit
        .max(options.weld_tolerance_mm)
        .max(options.plane_tolerance_mm);
    let land = (nodes[0].radius + nodes[last].radius) * 0.5;
    let off_land = |n: &Node| (n.radius - land).abs() > fit;
    let Some(first_off) = nodes.iter().position(off_land) else {
        return Ok(None);
    };
    let last_off = nodes.iter().rposition(off_land).unwrap();
    if first_off < 2 || last_off + 2 > last || last_off < first_off + 2 {
        return Ok(None);
    }
    let a = first_off - 1;
    let b = last_off + 1;
    let z = (nodes[a].z + nodes[b].z) * 0.5;
    let half_width = (nodes[b].z - nodes[a].z) * 0.5;
    let deepest = nodes[a..=b]
        .iter()
        .min_by(|l, r| l.radius.total_cmp(&r.radius))
        .unwrap();
    let depth = land - deepest.radius;
    if depth <= 16. * epsilon
        || half_width <= 16. * epsilon
        || nodes[first_off..=last_off]
            .iter()
            .any(|n| n.radius >= land - fit)
    {
        return Ok(None);
    }
    // The center is on the axial bisector of the two equally sized rings.
    // One further measured ring fixes its radial coordinate. All remaining
    // interior rings independently witness the same circle and its branch.
    let offset = (half_width.powi(2) - (deepest.z - z).powi(2) - depth.powi(2)) / (2. * depth);
    let major = land + offset;
    let minor = half_width.hypot(offset);
    if !minor.is_finite() || major - minor <= 8. * epsilon || offset < -fit {
        return Ok(None);
    }
    let low = half_width.atan2(-offset);
    let high = TAU - low;
    if !(nodes[a..=b].iter().all(|n| {
        let radial = n.radius - major;
        radial <= fit && ((n.z - z).hypot(radial) - minor).abs() <= fit
    })) {
        return Ok(None);
    }
    Ok(Some(Layout {
        sections: vec![
            ([0, a], Shape::Cylinder),
            (
                [a, b],
                Shape::Torus {
                    center_z: z,
                    major,
                    minor,
                    v: [low, high],
                },
            ),
            ([b, last], Shape::Cylinder),
        ],
        boundaries: [0, a, b, last].map(|i| (i, nodes[i].z, land)).to_vec(),
    }))
}
