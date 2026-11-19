use fabstep_cad::build_rounded_step;
use fabstep_core::{analyze_stl, recognize_automatic, MixedLoop, Options, ReconstructionMode};

#[test]
fn stale_radius_corner_or_shared_vertex_id_cannot_rebuild_the_stock() {
    let stl = include_bytes!("../../../fixtures/composed_cross_drill/disjoint-coarse.stl");
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let a = analyze_stl(stl, &options).unwrap();
    let base = recognize_automatic(&a, &options).unwrap().model;
    for case in 0..6 {
        let mut model = base.clone();
        let drill = model.cross_drill.as_mut().unwrap();
        let fillet = drill.stock_fillet.as_mut().unwrap();
        match case {
            0 => fillet.y_sign *= -1,
            1 => fillet.z_sign = 0,
            2 => fillet.radius_mm *= 1.01,
            3 => fillet.radius_mm = drill.half_lengths[1] - drill.radii[1],
            4 => {
                // Two nominally coincident endpoints no longer share an ID.
                let m = &mut model.revolved.mixed;
                let MixedLoop::Polygon { vertices } = &mut m.planes[1].loops[0] else {
                    panic!()
                };
                let old = vertices[0];
                vertices[0] = m.vertices.len();
                m.vertices.push(m.vertices[old]);
            }
            _ => {
                let m = &mut model.revolved.mixed;
                for i in [0, 3] {
                    let MixedLoop::Polygon { vertices } = &mut m.planes[i].loops[0] else {
                        panic!()
                    };
                    vertices.retain(|&id| {
                        let [_, y, z] = m.vertices[id];
                        // Leave just the ends of the quarter circle, like a bevel.
                        !(y < -8.50001 && z > 7.50001)
                    });
                }
            }
        }
        assert_eq!(
            build_rounded_step(&model).unwrap_err().code,
            "cad.invalid_model"
        );
    }
}
