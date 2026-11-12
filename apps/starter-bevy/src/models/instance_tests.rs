//! Native loader/ECS/raycast tests, without a GPU, GUI or external network.
use super::{
    assets::{ModelAsset, ModelAssets, ModelStatus},
    instances::{self, ModelRoot, ModelScene},
    picking::{self, ModelPick},
};
use crate::{
    map::{real_scene::RealScene, region::ActiveMapRegion},
    project::{Project, ProjectIo},
};
use bevy::{gltf::GltfAssetLabel, prelude::*};
use starter_core::{GeoOrigin, ProjectAsset, ProjectSelection};
use std::sync::Arc;
fn app() -> App {
    let mut app = super::tests::app();
    app.init_resource::<Project>()
        .init_resource::<ProjectIo>()
        .init_resource::<ModelAssets>()
        .init_resource::<ModelScene>()
        .init_resource::<ModelPick>()
        .init_resource::<RealScene>()
        .init_resource::<ActiveMapRegion>();
    app.add_systems(Update, (instances::sync, picking::pick).chain());
    app.finish();
    app.cleanup();
    app
}
fn add(app: &mut App, point: GeoOrigin) -> starter_core::ObjectId {
    let path = "Assets/Models/box.glb";
    if app.world().resource::<Project>().document.assets.is_empty() {
        app.world_mut()
            .resource_mut::<Project>()
            .document
            .assets
            .push(ProjectAsset {
                path: path.into(),
                media_type: "model/gltf-binary".into(),
                size: 1,
            });
        let scene: Handle<WorldAsset> = app
            .world()
            .resource::<AssetServer>()
            .load(GltfAssetLabel::Scene(0).from_asset("box.glb"));
        app.world_mut()
            .resource_mut::<ModelAssets>()
            .entries
            .insert(
                path.into(),
                ModelAsset {
                    status: ModelStatus::Ready,
                    scene: Some(scene),
                    gltf: None,
                    task: None,
                },
            );
    }
    app.world_mut()
        .resource_mut::<Project>()
        .add_model(path.into(), point)
        .unwrap()
}
fn dem(app: &mut App) -> GeoOrigin {
    let dir = crate::map::dem_tests::package(3, 3);
    let data = crate::map::source::load_terrain(dir.path()).unwrap();
    let region = starter_core::MapRegion::new(data.manifest.bbox, data.manifest.origin).unwrap();
    let point = region.bounds.center();
    app.world_mut()
        .resource_mut::<ActiveMapRegion>()
        .confirm(region)
        .unwrap();
    let revision = app.world().resource::<ActiveMapRegion>().get().unwrap().0;
    app.world_mut()
        .resource_mut::<Project>()
        .document
        .map_region = Some(region);
    let mut scene = app.world_mut().resource_mut::<RealScene>();
    scene.data = Some(Arc::new(data));
    scene.revision = Some(revision);
    point
}
fn ready(app: &mut App, count: usize) {
    let start = std::time::Instant::now();
    loop {
        app.update();
        if app
            .world()
            .resource::<ModelScene>()
            .information
            .values()
            .filter(|i| i.center.is_some())
            .count()
            == count
        {
            break;
        }
        assert!(start.elapsed().as_secs() < 10, "Model did not become ready");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    app.update(); // Apply inherited visibility and transforms before raycasting.
}
#[test]
fn no_dem_preserves_coordinates_then_native_instances_anchor_and_update_independently() {
    let mut app = app();
    let region_dir = crate::map::dem_tests::package(3, 3);
    let data = crate::map::source::load_terrain(region_dir.path()).unwrap();
    let point = data.manifest.bbox.center();
    let first = add(&mut app, point);
    let second = add(&mut app, point);
    app.update();
    assert!(app.world().resource::<ModelScene>().entities.is_empty());
    assert_eq!(
        app.world().resource::<Project>().document.models[0].longitude,
        point.longitude
    );
    dem(&mut app);
    ready(&mut app, 2);
    let roots = app.world().resource::<ModelScene>().entities.clone();
    let original = *app.world().get::<Transform>(roots[&second]).unwrap();
    let mut model = app.world().resource::<Project>().document.models[0].clone();
    model.altitude_agl = 7.0;
    model.scale = 2.0;
    model.yaw_degrees = 90.0;
    app.world_mut()
        .resource_mut::<Project>()
        .edit_model(model)
        .unwrap();
    app.update();
    assert_eq!(app.world().resource::<ModelScene>().entities, roots);
    let updated = app.world().get::<Transform>(roots[&first]).unwrap();
    let elevation =
        crate::map::objects::terrain_elevation(&data, point.longitude, point.latitude).unwrap();
    assert!(
        (f64::from(updated.translation.y + 5.0 * 2.0) - (elevation - 100.0 + 7.0)).abs() < 0.001
    );
    assert!(updated.scale.abs_diff_eq(Vec3::splat(2.0), 0.001));
    assert!(
        updated
            .rotation
            .mul_vec3(Vec3::X)
            .abs_diff_eq(Vec3::NEG_Z, 0.001)
    );
    assert_eq!(
        *app.world().get::<Transform>(roots[&second]).unwrap(),
        original
    );
    let handles: Vec<_> = app
        .world_mut()
        .query::<(&ModelRoot, &WorldAssetRoot)>()
        .iter(app.world())
        .map(|(_, h)| h.0.id())
        .collect();
    assert_eq!(handles.len(), 2);
    assert_eq!(handles[0], handles[1]);
    app.world_mut().resource_mut::<Project>().selection = ProjectSelection::Model(first);
    app.world_mut().resource_mut::<Project>().delete_selected();
    app.update();
    assert!(app.world().get_entity(roots[&first]).is_err());
    assert!(app.world().get_entity(roots[&second]).is_ok());
    app.world_mut().resource_mut::<ActiveMapRegion>().clear();
    app.update();
    assert!(app.world().resource::<ModelScene>().entities.is_empty());
    assert_eq!(
        app.world().resource::<Project>().document.models[0].id,
        second
    );
    dem(&mut app);
    ready(&mut app, 1);
    assert_eq!(
        app.world_mut()
            .query::<&ModelRoot>()
            .iter(app.world())
            .count(),
        1
    );
    app.world_mut().resource_mut::<ProjectIo>().benchmark = true;
    app.update();
    assert!(app.world().resource::<ModelScene>().entities.is_empty());
    assert_eq!(app.world().resource::<Project>().document.models.len(), 1);
    app.world_mut().resource_mut::<ProjectIo>().benchmark = false;
    for generation in 1..=3 {
        let previous = app.world().resource::<ModelScene>().entities.clone();
        app.world_mut().resource_mut::<ProjectIo>().generation = generation;
        ready(&mut app, 1);
        assert!(
            previous
                .values()
                .all(|e| app.world().get_entity(*e).is_err())
        );
        assert_eq!(
            app.world_mut()
                .query::<&ModelRoot>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
            1
        );
    }
    app.world_mut()
        .resource_mut::<Project>()
        .document
        .models
        .clear();
    app.world_mut()
        .resource_mut::<ModelAssets>()
        .entries
        .clear();
    let started = std::time::Instant::now();
    while !app.world().resource::<Assets<Mesh>>().is_empty() {
        app.update();
        assert!(
            started.elapsed().as_secs() < 10,
            "Released model mesh leaked"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(
        app.world_mut()
            .query::<&ModelRoot>()
            .iter(app.world())
            .count(),
        0
    );
}
#[test]
fn native_child_mesh_picking_selects_the_stable_object_and_rejects_stale_clicks() {
    let mut app = app();
    let point = dem(&mut app);
    let id = add(&mut app, point);
    ready(&mut app, 1);
    let center = app.world().resource::<ModelScene>().information[&id]
        .center
        .unwrap();
    let fingerprint = app.world().resource::<RealScene>().fingerprint();
    app.world_mut().resource_mut::<Project>().selection = ProjectSelection::Project;
    app.world_mut().resource_mut::<ModelPick>().pending = Some((
        crate::viewport::picking::Ray {
            origin: center + Vec3::Z * 30.0,
            direction: Vec3::NEG_Z,
        },
        fingerprint,
        0,
    ));
    app.update();
    assert_eq!(
        app.world().resource::<Project>().selection,
        ProjectSelection::Model(id)
    );
    app.world_mut().resource_mut::<Project>().selection = ProjectSelection::Project;
    app.world_mut().resource_mut::<ModelPick>().pending = Some((
        crate::viewport::picking::Ray {
            origin: center + Vec3::Z * 30.0,
            direction: Vec3::NEG_Z,
        },
        fingerprint,
        999,
    ));
    app.update();
    assert_eq!(
        app.world().resource::<Project>().selection,
        ProjectSelection::Project
    );
}
