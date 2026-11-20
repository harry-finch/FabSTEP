//! Certify the two planar disks and the unwrapped cylindrical domains. The
//! rounded mouth's two boundaries are matched to the native periodic spline
//! by the CAD certificate separately from the rest of the blend.
use super::{BTreeMap, RollingBore};
use crate::{
    drilled_shaft::{wall, DrilledShaft},
    math::{cross, norm, sub, V},
    Mesh, Options,
};
use std::f64::consts::{FRAC_PI_2, TAU};

pub(super) fn prove(
    m: &RollingBore,
    mesh: &Mesh,
    local: &[V],
    loops: &BTreeMap<(usize, usize), Vec<usize>>,
    o: &Options,
    fit: f64,
) -> Option<f64> {
    let mut side = [0_f64; 2];
    let mut cap_drift = [0_f64; 2];
    let bore_frame = |p: V| [p[1] - m.offset, p[2] - m.level, p[0]];
    for (&t, &owner) in mesh.triangles.iter().zip(&m.owners) {
        let p = t.map(|v| local[v]);
        let a = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        match owner {
            0 | 1 => {
                let sense = if owner == 0 { -1. } else { 1. };
                if a[2] * sense <= 1e-12 {
                    return None;
                }
                let z = if owner == 0 { 0. } else { m.height };
                let d = p.map(|v| (v[2] - z).abs()).into_iter().fold(0., f64::max);
                if d > o.plane_tolerance_mm {
                    return None;
                }
                cap_drift[owner] = cap_drift[owner].max(d);
            }
            2 => side[0] = side[0].max(wall(p, m.shaft, 1., fit, norm(a))?),
            3 => side[1] = side[1].max(wall(p.map(bore_frame), m.bore, -1., fit, norm(a))?),
            4 => {}
            _ => return None,
        }
    }
    let drilled = DrilledShaft {
        origin: m.origin,
        shaft_axis: [0., 0., 1.],
        bore_axis: [1., 0., 0.],
        radius_mm: m.shaft,
        bore_radius_mm: m.bore,
        height_mm: m.height,
        bore_offset_mm: m.offset,
        bore_height_mm: m.level,
        cycles: Default::default(),
        witness_tolerance_mm: fit,
    };
    let mut rims = drilled.rim(2, &loops[&(2, 3)], &mesh.vertices)?;
    let mut bound = 0_f64;
    for cap in 0..2 {
        // Existing disk certificate uses the boundary as oriented on the shaft.
        let ring: Vec<_> = loops[&(cap, 2)].iter().rev().copied().collect();
        let [shaft_rim, disk_rim] = drilled.rim(cap, &ring, &mesh.vertices)?;
        rims[0] = rims[0].max(shaft_rim);
        bound = bound.max(disk_rim + cap_drift[cap]);
    }
    let available = o.max_deviation_mm - o.weld_tolerance_mm;
    let native_rim = 0.013_f64.min(available * 0.26);
    for (i, r) in [m.shaft, m.bore].into_iter().enumerate() {
        let ring = &loops[&(i + 2, 4)];
        let winding = if i == 0 { 1. } else { -1. };
        let mut sum = 0.;
        for (&u, &v) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let [p, q] = [u, v].map(|j| local[j]);
            if p[0] >= 0. || q[0] >= 0. {
                return None;
            }
            let phase = |x: V| (x[2] - m.level).atan2(x[1] - m.offset);
            let turn = |a: f64| a - TAU * (a / TAU).round();
            let dt = turn(phase(q) - phase(p));
            if dt * winding <= 1e-12 || dt.abs() >= FRAC_PI_2 {
                return None;
            }
            sum += dt;
            let [p, q] = if i == 1 {
                [p, q].map(bore_frame)
            } else {
                [p, q]
            };
            let da = turn(q[1].atan2(q[0]) - p[1].atan2(p[0]));
            if da.abs() >= FRAC_PI_2 {
                return None;
            }
            // A linear segment in the (R*theta,z) development maps to a
            // cylindrical helix. Its distance to the STL chord is bounded by
            // the radial endpoint residual plus the angular interpolation term.
            let delta = native_rim + fit + r * da * da / 8. + 1e-7;
            let ratio = delta / (2. * r);
            if !(0.0..0.25).contains(&ratio) {
                return None;
            }
            // The inverse development stretches chord distance by at most this
            // factor. Both boundaries stay on this branch; their paired
            // segment homotopy preserves winding outside the resulting tube.
            let chart_dist = delta * ratio.asin() / ratio;
            rims[i] = rims[i].max(chart_dist);
        }
        if (sum - winding * TAU).abs() > 1e-8 {
            return None;
        }
        bound = bound.max(side[i] + rims[i] + 1e-7);
    }
    // The native fitter snaps its two transverse coordinates and four radii /
    // offsets to a fit/10 grid. Each coordinate moves <= fit/20. On the
    // unrounded mouth |y|/R < 2/3, hence the derivative of sqrt(R²-y²)
    // with respect to either input is below two. This allowance also pays
    // for the independent before/after STEP tolerance.
    bound += 4. * fit + 2e-6;
    (bound.is_finite() && bound <= available).then_some(bound)
}
