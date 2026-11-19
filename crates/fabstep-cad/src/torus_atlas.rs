use super::*;
use fabstep_core::torus_atlas::TorusAtlas;

#[repr(C)]
struct Corner {
    vertex: usize,
    uv: [f64; 2],
    constant_u: i32,
}
#[repr(C)]
struct Boundary {
    corners: *const Corner,
    count: usize,
}
#[repr(C)]
struct Chart {
    center: [f64; 3],
    major: f64,
    minor: f64,
    bound: f64,
    orientation: i32,
    owners: *const usize,
    owner_count: usize,
    loops: *const Boundary,
    loop_count: usize,
}
unsafe extern "C" {
    fn fabstep_build_torus_atlas(
        vertices: *const f64,
        vertex_count: usize,
        triangles: *const usize,
        triangle_count: usize,
        axis: *const f64,
        radial: *const f64,
        charts: *const Chart,
        chart_count: usize,
        tokens: *const usize,
        token_count: usize,
        deviation: f64,
        bound: *mut f64,
        out: *mut NativeStep,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
}

/// Tokens correspond to the charts in order and then the unclaimed triangles
/// in source order. The returned continuous source bound includes the trims
/// of residual faces and the allowance for native sewing and STEP transfer.
pub fn build_torus_atlas_step_with_provenance(
    mesh: &Mesh,
    atlas: &TorusAtlas,
    tokens: &[usize],
    max_deviation_mm: f64,
) -> Result<(ValidatedStep, f64), FabError> {
    let invalid = || {
        FabError::new(
            "cad.invalid_model",
            "Les domaines toriques, leurs contours ou les identités source sont incohérents.",
        )
    };
    if mesh.vertices.len() > MAX_SOURCE_VERTICES
        || mesh.triangles.is_empty()
        || mesh.triangles.len() > MAX_SOURCE_TRIANGLES
        || atlas.charts.is_empty()
        || atlas.charts.len() > 1024
        || !unit_direction(&atlas.axis)
        || !unit_direction(&atlas.radial)
        || !max_deviation_mm.is_finite()
        || max_deviation_mm <= 0.
        || tokens.is_empty()
        || tokens.len() > 100_000
    {
        return Err(invalid());
    }
    let mut claimed = vec![false; mesh.triangles.len()];
    let mut uses = 0usize;
    for chart in &atlas.charts {
        if chart.loops.is_empty()
            || chart.loops.len() > chart.triangles.len()
            || chart.triangles.is_empty()
            || chart.triangles.len() > claimed.len()
        {
            return Err(invalid());
        }
        for &i in &chart.triangles {
            let flag = claimed.get_mut(i).ok_or_else(invalid)?;
            if std::mem::replace(flag, true) {
                return Err(invalid());
            }
        }
        for ring in &chart.loops {
            if ring.len() < 3 || ring.len() > mesh.vertices.len() {
                return Err(invalid());
            }
            uses += ring.len();
            if uses > 3 * mesh.triangles.len() {
                return Err(invalid());
            }
        }
    }
    if atlas.charts.len() + claimed.iter().filter(|&&v| !v).count() != tokens.len() {
        return Err(invalid());
    }
    let mut seen = vec![false; 100_001];
    for &token in tokens {
        if token == 0 || token >= seen.len() || std::mem::replace(&mut seen[token], true) {
            return Err(invalid());
        }
    }
    let corners: Vec<Vec<Vec<Corner>>> = atlas
        .charts
        .iter()
        .map(|c| {
            c.loops
                .iter()
                .map(|l| {
                    l.iter()
                        .map(|v| Corner {
                            vertex: v.vertex,
                            uv: v.uv,
                            constant_u: i32::from(v.constant_u),
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let loops: Vec<Vec<Boundary>> = corners
        .iter()
        .map(|c| {
            c.iter()
                .map(|l| Boundary {
                    corners: l.as_ptr(),
                    count: l.len(),
                })
                .collect()
        })
        .collect();
    let charts: Vec<_> = atlas
        .charts
        .iter()
        .zip(&loops)
        .map(|(c, r)| Chart {
            center: c.center_mm,
            major: c.major_radius_mm,
            minor: c.minor_radius_mm,
            bound: c.bound_mm,
            orientation: i32::from(c.orientation),
            owners: c.triangles.as_ptr(),
            owner_count: c.triangles.len(),
            loops: r.as_ptr(),
            loop_count: r.len(),
        })
        .collect();
    let mut bound = 0.;
    let step = native_step(|out, error, capacity| unsafe {
        fabstep_build_torus_atlas(
            mesh.vertices.as_ptr().cast(),
            mesh.vertices.len(),
            mesh.triangles.as_ptr().cast(),
            mesh.triangles.len(),
            atlas.axis.as_ptr(),
            atlas.radial.as_ptr(),
            charts.as_ptr(),
            charts.len(),
            tokens.as_ptr(),
            tokens.len(),
            max_deviation_mm,
            &mut bound,
            out,
            error,
            capacity,
        )
    })?;
    if !bound.is_finite() || !(0.0..=max_deviation_mm).contains(&bound) {
        return Err(invalid());
    }
    Ok((step, bound))
}
