use super::*;
use bevy::{
    asset::AssetPlugin,
    gltf::{GltfAssetLabel, GltfPlugin},
    image::ImagePlugin,
    mesh::MeshPlugin,
    transform::TransformPlugin,
    world_serialization::WorldSerializationPlugin,
};
const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/box.glb");

#[test]
fn glb_boundary_rejects_damage_and_cache_verifies_content() {
    assert_eq!(
        files::validate(FIXTURE, files::DEFAULT_MAX_BYTES).unwrap(),
        0
    );
    for offset in [0, 4, 8] {
        let mut bytes = FIXTURE.to_vec();
        bytes[offset] ^= 0xFF;
        assert!(files::validate(&bytes, files::DEFAULT_MAX_BYTES).is_err());
    }
    assert!(files::validate(FIXTURE, 12).is_err());
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("box.glb"), FIXTURE).unwrap();
    let first = files::prepare(
        project.path(),
        "box.glb",
        cache.path(),
        files::DEFAULT_MAX_BYTES,
    )
    .unwrap();
    assert!(
        files::prepare(
            project.path(),
            "missing.glb",
            cache.path(),
            files::DEFAULT_MAX_BYTES
        )
        .is_err()
    );
    assert!(
        files::prepare(
            project.path(),
            "../box.glb",
            cache.path(),
            files::DEFAULT_MAX_BYTES
        )
        .is_err()
    );
    std::fs::write(cache.path().join(&first.file), b"corrupt cache").unwrap();
    let second = files::prepare(
        project.path(),
        "box.glb",
        cache.path(),
        files::DEFAULT_MAX_BYTES,
    )
    .unwrap();
    assert_eq!(first.file, second.file);
    assert_eq!(
        std::fs::read(cache.path().join(first.file)).unwrap(),
        FIXTURE
    );
}

#[test]
fn native_loader_shares_assets_and_despawns_only_the_deleted_hierarchy() {
    let mut app = app();
    app.finish();
    app.cleanup();
    let server = app.world().resource::<AssetServer>();
    let scene: Handle<WorldAsset> = server.load(GltfAssetLabel::Scene(0).from_asset("box.glb"));
    let second: Handle<WorldAsset> = server.load(GltfAssetLabel::Scene(0).from_asset("box.glb"));
    assert_eq!(scene.id(), second.id());
    let first = app
        .world_mut()
        .spawn((WorldAssetRoot(scene), geometry::ModelGeometry::default()))
        .id();
    let other = app
        .world_mut()
        .spawn((WorldAssetRoot(second), geometry::ModelGeometry::default()))
        .id();
    let start = std::time::Instant::now();
    while app
        .world()
        .get::<geometry::ModelGeometry>(other)
        .unwrap()
        .bounds
        .is_none()
    {
        app.update();
        assert!(
            start.elapsed().as_secs() < 10,
            "Native glTF load did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let bounds = app
        .world()
        .get::<geometry::ModelGeometry>(other)
        .unwrap()
        .bounds
        .unwrap();
    assert!(bounds.0.abs_diff_eq(Vec3::new(2.0, 5.0, -1.0), 0.001));
    assert!(bounds.1.abs_diff_eq(Vec3::new(6.0, 11.0, 1.0), 0.001));
    let meshes = app.world_mut().query::<&Mesh3d>().iter(app.world()).count();
    assert_eq!(meshes, 2);
    app.world_mut().entity_mut(first).despawn();
    app.update();
    assert_eq!(
        app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
        1
    );
    assert!(app.world().get_entity(other).is_ok());
    app.world_mut().entity_mut(other).despawn();
    app.update();
    assert_eq!(
        app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
        0
    );
    for _ in 0..50 {
        app.update();
        if app.world().resource::<Assets<Mesh>>().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(
        app.world().resource::<Assets<Mesh>>().is_empty(),
        "Mesh assets retained after last root was removed"
    );
}

#[test]
fn import_glb_publishes_validated_bytes_and_save_as_preserves_the_resource() {
    let source = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let file = source.path().join("box.glb");
    std::fs::write(&file, FIXTURE).unwrap();
    let asset = crate::project::storage::import_asset(first.path(), &file).unwrap();
    assert_eq!(asset.path, "Assets/Models/box.glb");
    let mut doc = starter_core::ProjectDocument::default();
    doc.assets.push(asset.clone());
    let a = first.path().join("project.json");
    let b = second.path().join("project.json");
    crate::project::storage::save(&a, &doc).unwrap();
    crate::project::storage::save_as(&b, Some(&a), &doc).unwrap();
    assert_eq!(
        std::fs::read(second.path().join(&asset.path)).unwrap(),
        FIXTURE
    );
    assert_eq!(crate::project::storage::load(&b).unwrap(), doc);
    std::fs::write(&file, b"bad").unwrap();
    assert!(crate::project::storage::import_asset(second.path(), &file).is_err());
}

pub(super) fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        MeshPlugin,
        ImagePlugin::default(),
        WorldSerializationPlugin,
        GltfPlugin::default(),
    ));
    app.add_observer(geometry::ready);
    app
}

#[test]
fn unsafe_hierarchies_empty_scenes_and_external_dependencies_are_rejected() {
    let edits: [fn(&mut serde_json::Value); 5] = [
        |j| j["nodes"][1]["children"] = serde_json::json!([0]),
        |j| j["scenes"][0]["nodes"] = serde_json::json!([]),
        |j| j["buffers"][0]["uri"] = "https://example.invalid/mesh.bin".into(),
        |j| {
            j["extensionsUsed"] = serde_json::json!(["EXT_unsupported"]);
            j["extensionsRequired"] = serde_json::json!(["EXT_unsupported"]);
        },
        |j| {
            j["nodes"] = (0..300)
                .map(|i| {
                    if i == 299 {
                        serde_json::json!({"mesh":0})
                    } else {
                        serde_json::json!({"children":[i+1]})
                    }
                })
                .collect::<Vec<_>>()
                .into();
        },
    ];
    for edit in edits {
        let mut container = gltf::binary::Glb::from_slice(FIXTURE).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&container.json).unwrap();
        edit(&mut json);
        container.json = std::borrow::Cow::Owned(serde_json::to_vec(&json).unwrap());
        let bytes = container.to_vec().unwrap();
        assert!(files::validate(&bytes, files::DEFAULT_MAX_BYTES).is_err());
    }
}
