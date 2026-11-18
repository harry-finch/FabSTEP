use fabstep_core::{RoundedFaceRef, RoundedModel};
use serde_json::json;

#[test]
fn legacy_rounded_models_and_the_new_spherical_face_keep_their_serialized_identity() {
    let legacy = json!({
        "revolved": {
            "mixed": {"vertices": [], "rings": [], "planes": [], "cylinders": []},
            "cones": []
        },
        "spheres": [], "tori": []
    });
    let model: RoundedModel = serde_json::from_value(legacy).unwrap();
    assert!(model.octants.is_empty());
    assert!(serde_json::to_value(model)
        .unwrap()
        .get("octants")
        .is_none());
    let face = RoundedFaceRef::SphericalOctant { index: 3 };
    let value = serde_json::to_value(face).unwrap();
    assert_eq!(value, json!({"kind": "spherical_octant", "index": 3}));
    assert_eq!(
        serde_json::from_value::<RoundedFaceRef>(value).unwrap(),
        face
    );
}
