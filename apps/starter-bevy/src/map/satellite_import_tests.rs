use super::{
    dem_tests,
    real_scene::RealScene,
    region::ActiveMapRegion,
    satellite_scene::{self, SatelliteImport},
    satellite_tests, source,
};
use bevy::{
    ecs::schedule::Schedule,
    prelude::*,
    tasks::{AsyncComputeTaskPool, TaskPool},
};
use std::time::{Duration, Instant};
fn poll_until(world: &mut World, check: impl Fn(&World) -> bool) {
    let mut schedule = Schedule::default();
    schedule.add_systems(satellite_scene::poll_satellite);
    let start = Instant::now();
    while !check(world) {
        schedule.run(world);
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "Satellite task did not complete"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn start(world: &mut World, directory: &std::path::Path) {
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let mut import = SatelliteImport::default();
    import.start(
        world.resource::<ActiveMapRegion>(),
        world.resource::<RealScene>(),
        Some(directory.to_owned()),
    );
    world.insert_resource(import);
}
#[test]
fn package_waits_without_dem_then_installs_on_matching_surface_and_keeps_document() {
    let dem = dem_tests::package(3, 3);
    let data = source::load_terrain(dem.path()).unwrap();
    let sat = satellite_tests::package(&data);
    let mut world = dem_tests::world();
    let mut project = crate::project::Project::default();
    let point = data.manifest.bbox.center();
    project.add_marker(point).unwrap();
    project.document.assets.push(starter_core::ProjectAsset {
        path: "Assets/Models/box.glb".into(),
        media_type: "model/gltf-binary".into(),
        size: 932,
    });
    project
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    let document = project.document.clone();
    world.insert_resource(project);
    start(&mut world, sat.path());
    assert!(world.resource::<SatelliteImport>().loading());
    poll_until(&mut world, |w| {
        w.resource::<SatelliteImport>().pending.is_some()
    });
    assert!(world.resource::<RealScene>().satellite.is_none());
    assert_eq!(world.resource::<Assets<Image>>().len(), 1);
    dem_tests::install(&mut world, data);
    let meshes = world.resource::<RealScene>().meshes.clone();
    poll_until(&mut world, |w| {
        w.resource::<RealScene>().satellite.is_some()
    });
    assert!(world.resource::<SatelliteImport>().pending.is_none());
    assert_eq!(world.resource::<RealScene>().meshes, meshes);
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
    assert_eq!(
        world.resource::<crate::project::Project>().document,
        document
    );
}
#[test]
fn same_bbox_different_origin_keeps_dem_and_pending_package_until_cancelled() {
    let dem = dem_tests::package(3, 3);
    let data = source::load_terrain(dem.path()).unwrap();
    let sat = satellite_tests::package(&data);
    let file = sat.path().join("manifest.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    json["origin"]["longitude"] = serde_json::json!(data.manifest.origin.longitude + 0.001);
    std::fs::write(file, serde_json::to_vec(&json).unwrap()).unwrap();
    let mut world = dem_tests::world();
    dem_tests::install(&mut world, data);
    let meshes = world.resource::<RealScene>().meshes.clone();
    let region = world.resource::<ActiveMapRegion>().get().unwrap().1;
    world
        .resource_mut::<ActiveMapRegion>()
        .confirm(region)
        .unwrap();
    start(&mut world, sat.path());
    assert!(world.resource::<SatelliteImport>().loading());
    poll_until(&mut world, |w| {
        w.resource::<SatelliteImport>().pending.is_some()
    });
    assert!(
        world
            .resource::<SatelliteImport>()
            .error
            .as_ref()
            .unwrap()
            .contains("does not match")
    );
    assert!(world.resource::<RealScene>().satellite.is_none());
    assert_eq!(world.resource::<RealScene>().meshes, meshes);
    world.resource_mut::<SatelliteImport>().cancel();
    assert!(world.resource::<SatelliteImport>().pending.is_none());
}
