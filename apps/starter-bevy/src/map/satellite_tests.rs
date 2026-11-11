use super::{
    dem_tests,
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
    satellite::{self, SatellitePayload},
    satellite_scene::{self, SatelliteImport},
    source,
};
use bevy::{ecs::schedule::Schedule, mesh::VertexAttributeValues, prelude::*};
use std::{fs, path::Path};

fn package(terrain: &source::TerrainPayload) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let m = &terrain.manifest;
    let zoom = 14;
    let nw = walkers::mercator::project(
        walkers::Position::new(m.bbox.west, m.bbox.north),
        f64::from(zoom),
    );
    let se = walkers::mercator::project(
        walkers::Position::new(m.bbox.east, m.bbox.south),
        f64::from(zoom),
    );
    let left = nw.x().floor() - 17.0;
    let top = nw.y().floor() - 23.0;
    let width = (se.x().ceil() - left + 11.0) as u32;
    let height = (se.y().ceil() - top + 13.0) as u32;
    let json = serde_json::json!({"formatVersion":2,"mapDataId":"fixture-satellite","bbox":m.bbox,"origin":m.origin,"satellite":{
        "provider":"fixture","zoom":zoom,"width":width,"height":height,"file":"satellite.jpg","mediaType":"image/jpeg","attribution":"fixture attribution",
        "georeference":{"zoom":zoom,"tileSize":256,"cropPixelBounds":{"left":left,"top":top,"right":left+f64::from(width),"bottom":top+f64::from(height)}}}});
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 255) as u8, (y % 255) as u8, 150])
    })
    .save(dir.path().join("satellite.jpg"))
    .unwrap();
    dir
}

#[test]
fn mercator_anchor_has_known_global_pixel_coordinates() {
    let g = satellite::Georeference {
        zoom: 1,
        tile_size: 256,
        crop_pixel_bounds: satellite::PixelBounds {
            left: 128.0,
            top: 128.0,
            right: 384.0,
            bottom: 384.0,
        },
    };
    assert_eq!(g.uv(0.0, 0.0), [0.5, 0.5]);
    assert_eq!(g.uv(-90.0, 0.0), [0.0, 0.5]);
    assert_eq!(g.uv(90.0, 0.0), [1.0, 0.5]);
    assert!((g.uv(0.0, 66.51326044311186)[1]).abs() < 1e-7);
}

#[test]
fn jpeg_georeference_respects_crop_offset_v_direction_and_shared_edges() {
    let dem = dem_tests::package(259, 131);
    let data = source::load_terrain(dem.path()).unwrap();
    let dir = package(&data);
    let payload = satellite::load_satellite(dir.path(), &data).unwrap();
    let s = &payload.manifest.satellite;
    assert_eq!(payload.rgba.len(), (s.width * s.height * 4) as usize);
    let first = payload.uvs[0][0];
    assert!(first[0] > 0.0 && first[1] > 0.0); // Crop includes pixels outside bbox.
    assert!(payload.uvs.last().unwrap().last().unwrap()[1] > first[1]);
    for row in 0..=128 {
        assert_eq!(payload.uvs[0][row * 129 + 128], payload.uvs[1][row * 129]);
    }
    let p = walkers::mercator::project(
        walkers::Position::new(data.manifest.bbox.west, data.manifest.bbox.north),
        14.0,
    );
    let bounds = &s.georeference.crop_pixel_bounds;
    assert!((first[0] - ((p.x() - bounds.left) / f64::from(s.width)) as f32).abs() < 1e-7);
    assert!((first[1] - ((p.y() - bounds.top) / f64::from(s.height)) as f32).abs() < 1e-7);
    // Equivalent 512px tiles use twice the global pixel coordinates.
    let mut doubled = s.georeference.clone();
    doubled.tile_size = 512;
    let b = &mut doubled.crop_pixel_bounds;
    b.left *= 2.0;
    b.top *= 2.0;
    b.right *= 2.0;
    b.bottom *= 2.0;
    assert_eq!(
        doubled.uv(data.manifest.bbox.west, data.manifest.bbox.north),
        first
    );
}

#[test]
fn rejects_wrong_version_territory_crop_dimensions_and_corrupt_jpeg() {
    let dem = dem_tests::package(3, 3);
    let data = source::load_terrain(dem.path()).unwrap();
    let dir = package(&data);
    let path = dir.path().join("manifest.json");
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (pointer, value) in [
        ("/formatVersion", serde_json::json!(1)),
        ("/origin/latitude", serde_json::json!(42.01)),
        ("/satellite/width", serde_json::json!(8193)),
        ("/satellite/height", serde_json::json!(1)),
        ("/satellite/georeference/zoom", serde_json::json!(31)),
        (
            "/satellite/georeference/cropPixelBounds/left",
            serde_json::json!(0.5),
        ),
        ("/satellite/file", serde_json::json!("../other.jpg")),
    ] {
        let mut json = original.clone();
        *json.pointer_mut(pointer).unwrap() = value;
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(
            satellite::load_satellite(dir.path(), &data).is_err(),
            "accepted {pointer}"
        );
    }
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    image::RgbImage::new(1, 1)
        .save(dir.path().join("satellite.jpg"))
        .unwrap();
    assert!(satellite::load_satellite(dir.path(), &data).is_err());
    fs::write(dir.path().join("satellite.jpg"), b"corrupt").unwrap();
    assert!(satellite::load_satellite(dir.path(), &data).is_err());
}

#[derive(Resource)]
struct Incoming(Option<SatellitePayload>);
fn install(world: &mut World, payload: SatellitePayload) {
    world.insert_resource(Incoming(Some(payload)));
    let mut schedule = Schedule::default();
    schedule.add_systems(
        |mut scene: ResMut<RealScene>, mut assets: SceneAssets, mut input: ResMut<Incoming>| {
            satellite_scene::install(&mut scene, &mut assets, input.0.take().unwrap()).unwrap();
        },
    );
    schedule.run(world);
}
fn check_lifecycle(world: &mut World, dem: &Path, sat: &Path) {
    dem_tests::install(world, source::load_terrain(dem).unwrap());
    assert_eq!(world.resource::<Assets<Image>>().len(), 1); // Persistent viewport target only.
    assert!(world.resource::<RealScene>().satellite.is_none());
    let handles = world.resource::<RealScene>().meshes.clone();
    let entities = world.resource::<RealScene>().entities.clone();
    let payload =
        satellite::load_satellite(sat, world.resource::<RealScene>().data.as_ref().unwrap())
            .unwrap();
    install(world, payload);
    let image = world
        .resource::<RealScene>()
        .satellite
        .as_ref()
        .unwrap()
        .image
        .clone();
    for _ in 0..4 {
        world.resource_mut::<RealScene>().satellite_enabled = false;
        let mut schedule = Schedule::default();
        schedule.add_systems(satellite_scene::update_material);
        schedule.run(world);
        assert!(
            world
                .resource::<Assets<StandardMaterial>>()
                .iter()
                .all(|(_, m)| m.base_color_texture.is_none())
        );
        world.resource_mut::<RealScene>().satellite_enabled = true;
        schedule.run(world);
        assert_eq!(world.resource::<Assets<Image>>().len(), 2);
        assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 2);
        assert_eq!(world.resource::<RealScene>().meshes, handles);
        assert_eq!(world.resource::<RealScene>().entities, entities);
    }
    assert!(matches!(
        world
            .resource::<Assets<Mesh>>()
            .get(handles[0].id())
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_UV_0),
        Some(VertexAttributeValues::Float32x2(_))
    ));
    let payload =
        satellite::load_satellite(sat, world.resource::<RealScene>().data.as_ref().unwrap())
            .unwrap();
    install(world, payload);
    assert_eq!(
        world
            .resource::<RealScene>()
            .satellite
            .as_ref()
            .unwrap()
            .image,
        image
    );
}

#[test]
fn three_packages_repeatedly_release_images_and_toggle_without_geometry_replacement() {
    let dirs = [
        dem_tests::package(3, 3),
        dem_tests::package(130, 4),
        dem_tests::package(131, 131),
    ];
    let sats: Vec<_> = dirs
        .iter()
        .map(|d| package(&source::load_terrain(d.path()).unwrap()))
        .collect();
    let mut world = dem_tests::world();
    for _ in 0..3 {
        for (dem, sat) in dirs.iter().zip(&sats) {
            check_lifecycle(&mut world, dem.path(), sat.path());
        }
    }
}

#[test]
fn satellite_request_becomes_obsolete_on_new_revision_or_dem() {
    let mut world = dem_tests::world();
    let dir = dem_tests::package(3, 3);
    dem_tests::install(&mut world, source::load_terrain(dir.path()).unwrap());
    let mut active = world.remove_resource::<ActiveMapRegion>().unwrap();
    let mut request = SatelliteImport::default();
    bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
    request.start(
        &active,
        world.resource::<RealScene>(),
        Some(dir.path().into()),
    );
    assert!(request.is_current(&active, world.resource::<RealScene>()));
    active.confirm(active.get().unwrap().1).unwrap();
    assert!(!request.is_current(&active, world.resource::<RealScene>()));
    world.insert_resource(active);
    world.insert_resource(request);
    let mut schedule = Schedule::default();
    schedule.add_systems(satellite_scene::poll_satellite);
    schedule.run(&mut world);
    assert!(!world.resource::<SatelliteImport>().loading());
    assert!(world.resource::<RealScene>().satellite.is_none());
}

#[test]
fn installed_real_dem_and_satellite_packages_match_and_release_resources() {
    let (Some(terrain_root), Some(map_root)) = (
        std::env::var_os("STARTER_TEST_TERRAIN_ROOT"),
        std::env::var_os("STARTER_TEST_MAP_ROOT"),
    ) else {
        return;
    };
    let mut terrain_paths: Vec<_> = fs::read_dir(terrain_root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    terrain_paths.sort();
    let maps: Vec<_> = fs::read_dir(map_root)
        .unwrap()
        .filter_map(|e| {
            let path = e.ok()?.path();
            let json = fs::read(path.join("manifest.json")).ok()?;
            let m: satellite::MapManifest = serde_json::from_slice(&json).ok()?;
            Some((path, m))
        })
        .collect();
    let pairs: Vec<_> = terrain_paths
        .into_iter()
        .filter_map(|path| {
            let data = source::load_terrain(&path).ok()?;
            maps.iter()
                .find(|(_, m)| {
                    data.manifest.matches(super::region::MapRegion {
                        bounds: m.bbox,
                        origin: m.origin,
                    })
                })
                .map(|(sat, _)| (path, sat.clone()))
        })
        .take(3)
        .collect();
    assert_eq!(pairs.len(), 3);
    let mut world = dem_tests::world();
    for _ in 0..3 {
        for (dem, sat) in &pairs {
            check_lifecycle(&mut world, dem, sat);
        }
    }
}
