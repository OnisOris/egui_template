use super::{
    dem_tests, imagery_3d, imagery_detail::ImageryDetail, imagery_fixture::Fixture,
    imagery_service::TileService, imagery_tiles::ImageryTiles, real_scene::RealScene, source,
};
use bevy::{ecs::schedule::Schedule, prelude::*};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
fn run_until(world: &mut World, schedule: &mut Schedule, ready: impl Fn(&World) -> bool) {
    let start = Instant::now();
    loop {
        schedule.run(world);
        if ready(world) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn closer_camera_fetches_new_zoom_without_mesh_changes_and_failure_keeps_texture() {
    let fixture = Fixture::new();
    let service = TileService::new(fixture.provider.clone(), None).unwrap();
    let dir = dem_tests::package(3, 3);
    let mut world = dem_tests::world();
    dem_tests::install(&mut world, source::load_terrain(dir.path()).unwrap());
    let mut picker = super::picker::MapPicker::default();
    picker.imagery.tiles = Some(ImageryTiles::from_service(
        service,
        bevy_egui::egui::Context::default(),
    ));
    world.insert_non_send(picker);
    world.init_resource::<imagery_3d::LiveImagery>();
    world.init_resource::<ImageryDetail>();
    world.init_resource::<super::satellite_scene::SatelliteImport>();
    world
        .resource_mut::<crate::viewport::ViewportState>()
        .camera
        .distance = 100_000.0;
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            imagery_3d::update,
            super::satellite_scene::update_material,
            super::imagery_detail::update,
        )
            .chain(),
    );
    run_until(&mut world, &mut schedule, |w| {
        w.resource::<RealScene>().satellite.is_some()
    });
    let base_zoom = world
        .resource::<RealScene>()
        .satellite
        .as_ref()
        .unwrap()
        .metadata
        .zoom;
    let handles = world.resource::<RealScene>().meshes.clone();
    let attributes = format!(
        "{:?}",
        world
            .resource::<Assets<Mesh>>()
            .get(handles[0].id())
            .unwrap()
    );
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
    world
        .resource_mut::<crate::viewport::ViewportState>()
        .camera
        .distance = 2000.0;
    run_until(&mut world, &mut schedule, |w| {
        w.resource::<ImageryDetail>().material_updates > 0
    });
    let first_updates = world.resource::<ImageryDetail>().material_updates;
    let material = world
        .get::<MeshMaterial3d<StandardMaterial>>(world.resource::<RealScene>().entities[0])
        .unwrap()
        .0
        .clone();
    let old_image = world
        .resource::<Assets<StandardMaterial>>()
        .get(material.id())
        .unwrap()
        .base_color_texture
        .clone();
    let max_zoom = fixture
        .requests
        .lock()
        .unwrap()
        .iter()
        .map(|p| p.split('/').nth(1).unwrap().parse::<u32>().unwrap())
        .max()
        .unwrap();
    assert!(max_zoom > base_zoom);
    assert_eq!(world.resource::<RealScene>().meshes, handles);
    assert_eq!(
        format!(
            "{:?}",
            world
                .resource::<Assets<Mesh>>()
                .get(handles[0].id())
                .unwrap()
        ),
        attributes
    );
    fixture.fail.store(true, Ordering::Relaxed);
    world
        .resource_mut::<crate::viewport::ViewportState>()
        .camera
        .distance = 20.0;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(800) {
        schedule.run(&mut world);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        world.resource::<ImageryDetail>().material_updates,
        first_updates
    );
    assert_eq!(
        world
            .resource::<Assets<StandardMaterial>>()
            .get(material.id())
            .unwrap()
            .base_color_texture,
        old_image
    );
    assert_eq!(world.resource::<Assets<Image>>().len(), 3);
    world
        .resource_mut::<crate::viewport::ViewportState>()
        .camera
        .distance = 100_000.0;
    schedule.run(&mut world);
    assert_eq!(world.resource::<Assets<Image>>().len(), 2);
    assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 2);
    let region = world
        .resource::<super::region::ActiveMapRegion>()
        .get()
        .unwrap()
        .1;
    world
        .resource_mut::<super::region::ActiveMapRegion>()
        .confirm(region)
        .unwrap();
    schedule.run(&mut world);
    assert_eq!(world.resource::<Assets<Image>>().len(), 1);
}
#[test]
fn material_transform_matches_geographic_uv_at_chunk_and_tile_edges() {
    use super::satellite::{Georeference, PixelBounds};
    let base = Georeference {
        zoom: 2,
        tile_size: 256,
        crop_pixel_bounds: PixelBounds {
            left: 200.0,
            top: 200.0,
            right: 800.0,
            bottom: 800.0,
        },
    };
    for z in [3, 5, 9] {
        let patch = Georeference {
            zoom: z,
            tile_size: 256,
            crop_pixel_bounds: PixelBounds {
                left: 500.0,
                top: 600.0,
                right: 1800.0,
                bottom: 2000.0,
            },
        };
        let transform = super::imagery_lod::transform(&base, &patch);
        for (lon, lat) in [(0.0, 0.0), (-90.0, 0.0), (0.0, 42.0), (-0.000001, 0.000001)] {
            let actual = transform.transform_point2(Vec2::from_array(base.uv(lon, lat)));
            let expected = Vec2::from_array(patch.uv(lon, lat));
            assert!((actual - expected).length() < 0.0001);
        }
    }
}
