use crate::*;
fn state() -> ProjectState {
    let mut state = ProjectState::default();
    state.document.assets.push(ProjectAsset {
        path: "Assets/Models/box.glb".into(),
        media_type: "model/gltf-binary".into(),
        size: 1,
    });
    state
}
#[test]
fn v1_migration_preserves_existing_project_data() {
    let mut document: ProjectDocument =
        serde_json::from_str(include_str!("../../../docs/fixtures/m4-project.json")).unwrap();
    let original = document.clone();
    assert_eq!(document.schema_version, 1);
    document.migrate().unwrap();
    assert_eq!(document.schema_version, 3);
    assert_eq!(document.objects, original.objects);
    assert_eq!(document.layers[..2], original.layers);
    assert_eq!(document.geo_view_mode, GeoViewMode::Terrain);
    assert_eq!(document.assets, original.assets);
    assert_eq!(document.map_region, original.map_region);
    assert_eq!(document.name, original.name);
    assert!(document.models.is_empty());
}
#[test]
fn model_roundtrip_shares_asset_but_preserves_independent_ids_and_transforms() {
    let mut state = state();
    let point = GeoOrigin {
        longitude: 8.0,
        latitude: 46.0,
    };
    let marker = state.add_marker(point).unwrap();
    let first = state
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    let second = state
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    assert_ne!(first, marker);
    assert_ne!(second, first);
    let mut model = state.document.models[0].clone();
    model.scale = 2.5;
    model.yaw_degrees = -73.0;
    model.altitude_agl = 12.0;
    state.edit_model(model).unwrap();
    let restored: ProjectDocument =
        serde_json::from_slice(&serde_json::to_vec(&state.document).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, state.document);
    assert_eq!(restored.models[1].scale, 1.0);
    assert_eq!(restored.object_count(), 3);
    state.selection = ProjectSelection::Model(first);
    state.delete_selected();
    assert_eq!(state.document.models[0].id, second);
    assert_eq!(state.document.objects[0].id, marker);
}
#[test]
fn model_validation_rejects_paths_bad_numbers_and_duplicate_ids() {
    let mut state = state();
    let point = GeoOrigin {
        longitude: 8.0,
        latitude: 46.0,
    };
    let marker = state.add_marker(point).unwrap();
    state
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    let original = state.document.models[0].clone();
    for path in [
        "../box.glb",
        "/tmp/box.glb",
        "C:/box.glb",
        "Assets\\box.glb",
        "model.gltf",
    ] {
        let mut m = original.clone();
        m.asset_path = path.into();
        assert!(m.validate().is_err());
    }
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY, 1001.0] {
        let mut m = original.clone();
        m.scale = scale;
        assert!(state.edit_model(m).is_err());
    }
    let mut m = original.clone();
    m.latitude = f64::NAN;
    assert!(state.edit_model(m).is_err());
    state.document.models[0].id = marker;
    assert!(state.document.validate().is_err());
    state.document.models[0] = original;
    state.document.assets.clear();
    assert!(state.document.validate().is_err());
}

#[test]
fn v2_migration_preserves_models_and_layer_ids_and_v3_roundtrips_view_settings() {
    let mut state = state();
    state.document.schema_version = 2;
    state
        .document
        .layers
        .retain(|l| matches!(l.kind, LayerKind::Terrain | LayerKind::Satellite));
    let point = GeoOrigin {
        longitude: 8.,
        latitude: 46.,
    };
    state
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    state.add_marker(point).unwrap();
    state.document.layer_mut(LayerKind::Satellite).package_id = Some("satellite-cache".into());
    let before = state.document.clone();
    state.document.migrate().unwrap();
    assert_eq!(state.document.schema_version, 3);
    assert_eq!(state.document.geo_view_mode, GeoViewMode::Terrain);
    assert_eq!(state.document.layers[..2], before.layers);
    assert_eq!(state.document.models, before.models);
    assert_eq!(state.document.objects, before.objects);
    state.document.geo_view_mode = GeoViewMode::Combined;
    state.document.layer_mut(LayerKind::Roads).visible = false;
    state.document.layer_mut(LayerKind::Buildings).package_id = Some("vectors-cache".into());
    let copy: ProjectDocument =
        serde_json::from_slice(&serde_json::to_vec(&state.document).unwrap()).unwrap();
    copy.validate().unwrap();
    assert_eq!(copy, state.document);
}
