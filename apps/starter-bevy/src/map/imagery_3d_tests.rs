use super::{
    dem_tests, imagery_3d,
    imagery_fixture::{Fixture, job},
    imagery_service::TileService,
    imagery_tiles::ImageryTiles,
    satellite_scene::ImagerySource,
    source,
};
use bevy::{ecs::schedule::Schedule, prelude::*};
use std::time::{Duration, Instant};
fn tick_until(world: &mut World, schedule: &mut Schedule, ready: impl Fn(&World) -> bool) {
    let start = Instant::now();
    loop {
        schedule.run(world);
        if ready(world) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn automatic_live_imagery_is_georeferenced_memory_only_and_discards_stale_regions() {
    let fixture = Fixture::new();
    let service = TileService::new(fixture.provider.clone(), None).unwrap();
    let dem = dem_tests::package(130, 4);
    let data = source::load_terrain(dem.path()).unwrap();
    let mut world = dem_tests::world();
    dem_tests::install(&mut world, data);
    let mut picker = super::picker::MapPicker::default();
    picker.imagery.tiles = Some(ImageryTiles::from_service(
        service.clone(),
        bevy_egui::egui::Context::default(),
    ));
    world.insert_non_send(picker);
    world.init_resource::<imagery_3d::LiveImagery>();
    world.init_resource::<super::satellite_scene::SatelliteImport>();
    let meshes = world
        .resource::<super::real_scene::RealScene>()
        .meshes
        .clone();
    let mut schedule = Schedule::default();
    schedule.add_systems((imagery_3d::update, super::satellite_scene::update_material).chain());
    tick_until(&mut world, &mut schedule, |w| {
        w.resource::<super::real_scene::RealScene>()
            .satellite
            .is_some()
    });
    let scene = world.resource::<super::real_scene::RealScene>();
    let satellite = scene.satellite.as_ref().unwrap();
    assert!(matches!(satellite.source, ImagerySource::Live { .. }));
    assert!(satellite.metadata.file.is_empty());
    assert_eq!(scene.meshes, meshes);
    let geo = &satellite.metadata.georeference;
    let data = scene.data.as_ref().unwrap();
    let uv = super::satellite::terrain_uvs(data, geo);
    assert_eq!(uv[0][128], uv[1][0], "shared DEM edge has identical UV");
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
    let mut active = world.resource_mut::<super::region::ActiveMapRegion>();
    let region = active.get().unwrap().1;
    active.confirm(region).unwrap();
    schedule.run(&mut world);
    assert!(
        world
            .resource::<super::real_scene::RealScene>()
            .satellite
            .is_none()
    );
    assert_eq!(world.resource::<Assets<Image>>().len(), 1);
    assert_eq!(
        fixture.requests.lock().unwrap().len() as u64,
        service.stats().0
    );
}
#[test]
fn preview_only_cannot_prepare_3d_or_export_and_cancelled_work_stops() {
    let fixture = Fixture::new();
    let mut p = fixture.provider.clone();
    p.allow_online_3d = false;
    let service = TileService::new(p, None).unwrap();
    let dem = dem_tests::package(3, 3);
    let data = source::load_terrain(dem.path()).unwrap();
    assert!(
        imagery_3d::prepare(&service, &data, data.manifest.region(), 15, &job(), true).is_err()
    );
    assert!(service.provider().validate().is_err());
    assert!(fixture.requests.lock().unwrap().is_empty());
    let service = TileService::new(fixture.provider.clone(), None).unwrap();
    let job = job();
    job.cancelled
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(imagery_3d::prepare(&service, &data, data.manifest.region(), 15, &job, true).is_err());
    assert!(fixture.requests.lock().unwrap().is_empty());
}
