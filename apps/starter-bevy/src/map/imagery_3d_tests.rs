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
    picker.imagery.service = Some(service.clone());
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
    let region = world
        .resource::<super::region::ActiveMapRegion>()
        .get()
        .unwrap()
        .1;
    let mut project = crate::project::Project::default();
    project.document.map_region = Some(region);
    world.insert_resource(project);
    world.init_resource::<crate::project::ProjectIo>();
    world.insert_resource(crate::state::DocumentResource::empty());
    world.init_resource::<super::demo::DemoScene>();
    let mut sync = Schedule::default();
    sync.add_systems(crate::project::scene::sync_project);
    sync.run(&mut world);
    let project = world.resource::<crate::project::Project>();
    assert!(
        project
            .document
            .layer(starter_core::LayerKind::Satellite)
            .package_id
            .is_none()
    );
    let saved = tempfile::tempdir().unwrap();
    let path = saved.path().join("live-project.json");
    crate::project::storage::save_as(&path, None, &project.document).unwrap();
    let reopened = crate::project::storage::load(&path).unwrap();
    assert!(
        reopened
            .layer(starter_core::LayerKind::Satellite)
            .package_id
            .is_none()
    );
    assert!(
        reopened
            .layer(starter_core::LayerKind::Terrain)
            .package_id
            .is_some()
    );
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
    // Start a new request, change revision before it can be installed, then poll its late result.
    world
        .resource_mut::<super::real_scene::RealScene>()
        .revision = world
        .resource::<super::region::ActiveMapRegion>()
        .get()
        .map(|(r, _)| r);
    schedule.run(&mut world);
    world
        .resource_mut::<super::region::ActiveMapRegion>()
        .confirm(region)
        .unwrap();
    std::thread::sleep(Duration::from_millis(30));
    schedule.run(&mut world);
    assert!(
        world
            .resource::<super::real_scene::RealScene>()
            .satellite
            .is_none()
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

#[test]
fn online_2d_and_3d_permissions_are_independent_and_configuration_releases_adapters() {
    let fixture = Fixture::new();
    let mut view = super::imagery_view::ImageryView::default();
    let mut settings = super::sources::MapSources {
        provider: Some(fixture.provider.clone()),
        ..Default::default()
    };
    view.configure(settings.clone(), &bevy_egui::egui::Context::default())
        .unwrap();
    assert!(view.tiles.is_none());
    let first = view.service.as_ref().unwrap().generation();
    settings.online_authorized = true;
    view.configure(settings.clone(), &bevy_egui::egui::Context::default())
        .unwrap();
    assert!(view.tiles.is_some());
    assert_ne!(view.service.as_ref().unwrap().generation(), first);
    settings.provider.as_mut().unwrap().allow_online_3d = false;
    view.configure(settings, &bevy_egui::egui::Context::default())
        .unwrap();
    assert!(view.tiles.is_some());
    assert!(!view.service.as_ref().unwrap().provider().allow_online_3d);
    view.retry();
    assert!(view.tiles.is_none());
    assert!(view.service.is_none());
    assert!(fixture.requests.lock().unwrap().is_empty());
}
