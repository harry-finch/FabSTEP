//! One spherical octant cut by three observed planar sectors. The normal-ray
//! nomination is a hypothesis; all four source disks and their six joins
//! must cover the exact solid independently of that fit.
use super::*;
use crate::{CircularArc, MixedSegment, MixedSphericalOctant};
use std::f64::consts::FRAC_PI_2;

const WALL: usize = 3;
type Cuts = BTreeMap<(usize, usize), Vec<Edge>>;

struct Chart {
    center_id: usize,
    ends: [usize; 3],
    axes: [V; 3],
    radius: f64,
    // Plane opposite axis i.
    regions: [usize; 3],
}

fn unit(v: V) -> Option<V> {
    let n = norm(v);
    (n.is_finite() && n > 0.).then(|| scale(v, 1. / n))
}

fn frame(
    mesh: &crate::Mesh,
    masks: &[u8],
    center: V,
    radius: f64,
    mut regions: [usize; 3],
    o: &Options,
) -> Option<Chart> {
    let sole = |bits| {
        let mut ids = masks
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| (m == bits).then_some(i));
        let id = ids.next()?;
        ids.next().is_none().then_some(id)
    };
    // The low three bits tag the cutting planes, the fourth tags the
    // sphere. Exactly four source vertices join three of these disks.
    let center_id = sole(0b0111)?;
    let origin = mesh.vertices[center_id];
    if norm(sub(origin, center)) > o.cylinder_fit_tolerance_mm {
        return None;
    }
    let mut ends = [sole(0b1110)?, sole(0b1101)?, sole(0b1011)?];
    let mut rays = ends.map(|v| sub(mesh.vertices[v], origin));
    if dot(cross(rays[0], rays[1]), rays[2]) < 0. {
        ends.swap(0, 1);
        regions.swap(0, 1);
        rays.swap(0, 1);
    }
    let measured_radius = norm(rays[0]);
    if measured_radius <= 4e-7 || (radius - measured_radius).abs() > o.cylinder_fit_tolerance_mm {
        return None;
    }
    let radius = measured_radius;
    let x = unit(rays[0])?;
    let z = unit(cross(x, rays[1]))?;
    let axes = [x, cross(z, x), z];
    let max_error = (0..3)
        .map(|i| norm(sub(rays[i], scale(axes[i], radius))))
        .fold(0., f64::max);
    (max_error <= 3e-8_f64.min(o.cylinder_fit_tolerance_mm)).then_some(Chart {
        center_id,
        ends,
        axes,
        radius,
        regions,
    })
}

pub(super) fn candidate(
    a: &Analysis,
    s: &crate::planar::Segmentation,
    adj: &[Vec<usize>],
    edges: &BTreeMap<Edge, Vec<usize>>,
    o: &Options,
    work: &mut Work,
) -> Result<Option<RoundedRecognition>, FabError> {
    let n = a.mesh.triangles.len();
    if !(32..=4096).contains(&n) || !work.reserve_optional(s.model.faces.len()) {
        return Ok(None);
    }
    let mut supports = s
        .model
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, p)| matches!(p.loops.as_slice(), [l] if l.len() >= 8).then_some(i));
    let (Some(p), Some(q), Some(r), None) = (
        supports.next(),
        supports.next(),
        supports.next(),
        supports.next(),
    ) else {
        return Ok(None);
    };
    let regions = [p, q, r];
    if (0..3).any(|i| {
        dot(
            s.model.faces[regions[i]].normal,
            s.model.faces[regions[(i + 1) % 3]].normal,
        )
        .abs()
            > 1e-6
    }) || !work.reserve_optional(32 * n)
    {
        return Ok(None);
    }
    let owners: Vec<_> = s
        .triangle_regions
        .iter()
        .map(|r| regions.iter().position(|p| p == r).unwrap_or(WALL))
        .collect();
    let wall = owners
        .iter()
        .enumerate()
        .filter_map(|(i, &owner)| (owner == WALL).then_some(i))
        .collect();
    let Some((center, radius)) = crate::residual_supports::spherical_wall(&a.mesh, &wall, o, work)
    else {
        return Ok(None);
    };
    let mut masks = vec![0; a.mesh.vertices.len()];
    for (&owner, tri) in owners.iter().zip(&a.mesh.triangles) {
        for &v in tri {
            masks[v] |= 1 << owner;
        }
    }
    let Some(chart) = frame(&a.mesh, &masks, center, radius, regions, o) else {
        return Ok(None);
    };
    let owners: Vec<_> = s
        .triangle_regions
        .iter()
        .map(|r| chart.regions.iter().position(|p| p == r).unwrap_or(WALL))
        .collect();
    let mut cuts = Cuts::new();
    for (&key, pair) in edges {
        let [t, u] = pair.as_slice() else {
            return Ok(None);
        };
        let (t, u) = (owners[*t], owners[*u]);
        if t != u {
            cuts.entry(edge(t, u)).or_default().push(key);
        }
    }
    if cuts.len() != 6 {
        return Ok(None);
    }
    // This is a terminal optional interpretation. Invalid contours simply leave
    // the previous recognizers' outcome available.
    if let Err(e) = block::disks::<4>(a, &owners, adj, &cuts, work) {
        return if e.code == "recognition.too_complex" {
            Err(e)
        } else {
            Ok(None)
        };
    }
    let Some((residual, chord)) = chart.prove(a, s, &owners, &cuts, o) else {
        return Ok(None);
    };
    let model = chart.model(a);
    let volume = volume(&model)?;
    let report = RecognitionReport {
        cylinders:0, cones:0, spheres:1, tori:0, toroidal_fillets:0, revolutions:0,
        max_radial_residual_mm:residual, max_wall_residual_mm:residual, max_chord_error_mm:chord,
        max_deviation_bound_mm:o.weld_tolerance_mm + residual + chord, expected_volume_mm3:volume,
        diagnostics:vec![Diagnostic { code:"recognition.rounded_intent_assumed".into(), severity:Severity::Warning, count:1,
            message:"La zone délimitée par les trois plans de coupe est interprétée comme un octant sphérique.".into() }],
    };
    Ok(Some(RoundedRecognition {
        model,
        report,
        triangle_faces: owners
            .into_iter()
            .map(|owner| {
                if owner == WALL {
                    RoundedFaceRef::SphericalOctant { index: 0 }
                } else {
                    RoundedFaceRef::Plane {
                        index: (owner + 1) % 3,
                    }
                }
            })
            .collect(),
    }))
}

impl Chart {
    fn prove(
        &self,
        a: &Analysis,
        s: &crate::planar::Segmentation,
        owners: &[usize],
        cuts: &Cuts,
        o: &Options,
    ) -> Option<(f64, f64)> {
        let center = a.mesh.vertices[self.center_id];
        let mut masks = vec![0; a.mesh.vertices.len()];
        for (&owner, tri) in owners.iter().zip(&a.mesh.triangles) {
            for &v in tri {
                masks[v] |= 1 << owner;
            }
        }
        let mut projected = Vec::with_capacity(masks.len());
        let mut residual = 0_f64;
        for (i, &mask) in masks.iter().enumerate() {
            let actual = self.axes.map(|d| dot(sub(a.mesh.vertices[i], center), d));
            let mut ideal = std::array::from_fn(|k| {
                if mask & (1 << k) != 0 {
                    0.
                } else {
                    actual[k].max(0.)
                }
            });
            if mask & 8 != 0 {
                ideal = scale(unit(ideal)?, self.radius);
            }
            let distance = norm(sub(actual, ideal));
            if distance > o.cylinder_fit_tolerance_mm
                || (mask & 7 != 0 && distance > o.plane_tolerance_mm)
            {
                return None;
            }
            residual = residual.max(distance);
            projected.push(ideal);
        }
        for axis in 0..3 {
            if dot(s.model.faces[self.regions[axis]].normal, self.axes[axis]) > -1. + 1e-8 {
                return None;
            }
            let i = (axis + 1) % 3;
            let j = (axis + 2) % 3;
            for (key, start, end, arc) in [
                (edge(axis, WALL), self.ends[i], self.ends[j], true),
                (edge(i, j), self.center_id, self.ends[axis], false),
            ] {
                let mut chain = block::ordered_chain(cuts.get(&key)?).ok()?;
                if chain.first() != Some(&start) {
                    chain.reverse();
                }
                if chain.first() != Some(&start)
                    || chain.last() != Some(&end)
                    || (arc && chain.len() < 6)
                {
                    return None;
                }
                for pair in chain.windows(2) {
                    let lo = projected[pair[0]];
                    let hi = projected[pair[1]];
                    if arc {
                        let du = hi[j].atan2(hi[i]) - lo[j].atan2(lo[i]);
                        if !(0.0..=PI / 4.).contains(&du) || du <= 1e-10 {
                            return None;
                        }
                    } else if hi[axis] <= lo[axis] {
                        return None;
                    }
                }
            }
        }
        let mut area = Sum::default();
        let mut chord = 0_f64;
        for (tri, &owner) in a.mesh.triangles.iter().zip(owners) {
            let [a, b, c] = tri.map(|i| projected[i]);
            if owner != WALL {
                if cross(sub(b, a), sub(c, a))[owner] >= 0. {
                    return None;
                }
                continue;
            }
            let [u, v, w] = [a, b, c].map(|p| scale(p, 1. / self.radius));
            let orient = dot(u, cross(v, w));
            if orient <= 1e-14 {
                return None;
            }
            area.add(2. * orient.atan2(1. + dot(u, v) + dot(v, w) + dot(w, u)));
            chord = chord.max(self.radius - min_triangle_norm(a, b, c));
        }
        // Source disks, monotone quarter-circle trims, positive orientation
        // and degree-one area prove coverage. The radial sagitta also bounds
        // each planar sector's curved rim in both directions.
        ((area.get() - FRAC_PI_2).abs() < 1e-8
            && o.weld_tolerance_mm + residual + chord <= o.max_deviation_mm)
            .then_some((residual, chord))
    }

    fn model(&self, a: &Analysis) -> RoundedModel {
        let center = a.mesh.vertices[self.center_id];
        let arcs = (0..3)
            .map(|i| CircularArc {
                center,
                axis: self.axes[(i + 2) % 3],
                radius_mm: self.radius,
                sweep_rad: FRAC_PI_2,
                vertices: [self.ends[i], self.ends[(i + 1) % 3]],
            })
            .collect();
        let planes = (0..3)
            .map(|i| {
                let [a, b] = [self.ends[i], self.ends[(i + 1) % 3]];
                MixedPlane {
                    normal: scale(self.axes[(i + 2) % 3], -1.),
                    loops: vec![MixedLoop::Segments {
                        segments: vec![
                            MixedSegment::Line {
                                vertices: [self.center_id, b],
                            },
                            MixedSegment::Arc {
                                arc: i,
                                reversed: true,
                            },
                            MixedSegment::Line {
                                vertices: [a, self.center_id],
                            },
                        ],
                    }],
                }
            })
            .collect();
        RoundedModel {
            drilled_shaft: None,
            revolved: RevolvedModel {
                mixed: MixedModel {
                    vertices: a.mesh.vertices.clone(),
                    planes,
                    arcs,
                    rings: vec![],
                    cylinders: vec![],
                    partial_cylinders: vec![],
                    elliptic_arcs: vec![],
                },
                cones: vec![],
            },
            octants: vec![MixedSphericalOctant {
                center,
                radius_mm: self.radius,
                directions: self.axes,
                arc_uses: [(0, false), (1, false), (2, false)],
                inner: false,
            }],
            spheres: vec![],
            tori: vec![],
            partial_cones: vec![],
            torus_sectors: vec![],
            smooth_revolution: None,
            unequal_pocket: None,
            cross_drill: None,
            linear_blend: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const INPUT: &[u8] = include_bytes!("../../../../fixtures/r117-cut-sphere.stl");

    fn change(f: impl Fn(V) -> V) -> Vec<u8> {
        let mut source = INPUT.to_vec();
        for record in source[84..].chunks_exact_mut(50) {
            for p in record[12..48].chunks_exact_mut(12) {
                let before = std::array::from_fn(|i| {
                    f32::from_le_bytes(p[4 * i..4 * i + 4].try_into().unwrap()) as f64
                });
                for (encoded, c) in p.chunks_exact_mut(4).zip(f(before)) {
                    encoded.copy_from_slice(&(c as f32).to_le_bytes());
                }
            }
        }
        source
    }

    #[test]
    fn three_cutting_planes_are_required_alongside_the_normal_ray_nomination() {
        let opts = Options {
            reconstruction: ReconstructionMode::Surfaces,
            ..Options::default()
        };
        let recognize = |data: &[u8]| {
            let a = crate::analyze_stl(data, &opts).unwrap();
            crate::recognize_rounded(&a, &opts)
        };
        let valid = recognize(INPUT).unwrap();
        assert_eq!(valid.model.octants.len(), 1);
        assert!((valid.report.expected_volume_mm3 - PI * 125. / 6.).abs() < 1e-8);
        for invalid in [
            change(|[x, y, z]| [x, y, z + (z - 3.) * 0.02]), // elliptical instead of spherical
            change(|[x, y, z]| {
                if [x, y, z] == [7., 4., 3.] {
                    [x + 0.15, y + 0.12, z + 0.07]
                } else {
                    [x, y, z]
                }
            }), // unsupported sector disks
        ] {
            assert!(!recognize(&invalid).is_ok_and(|r| !r.model.octants.is_empty()));
        }
    }
}
