use super::{
    RegionRuntime,
    coordinates::{local_to_wgs84, wgs84_to_local},
    dem_mesh,
    loading::{PreparedTerrain, TerrainImport},
    real_scene::{self, RealScene},
    region::{ActiveMapRegion, GeoOrigin},
    source,
};
use crate::viewport::ViewportState;
use bevy::{ecs::schedule::Schedule, mesh::VertexAttributeValues, prelude::*};
use std::{fs, sync::Arc};

pub fn package(width: u32, height: u32) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let origin = GeoOrigin {
        longitude: 42.0,
        latitude: 42.0,
    };
    let (east, south) = local_to_wgs84(
        origin,
        f64::from(width - 1) * 10.0,
        f64::from(height - 1) * 10.0,
    );
    let samples: Vec<f32> = (0..height)
        .flat_map(|y| (0..width).map(move |x| 100.0 + x as f32 + y as f32 * 2.0))
        .collect();
    let manifest = serde_json::json!({
        "formatVersion":1, "terrainId":"test-grid", "source":"test-fixture", "attribution":"test",
        "origin":origin, "bbox":{"north":42.0,"south":south,"west":42.0,"east":east},
        "width":width,"height":height,"minXM":0.0,"minYM":0.0,"spacingXM":10.0,"spacingYM":10.0,
        "referenceElevationMsl":100.0,"referenceElevationSource":"origin-sample",
        "minElevationMsl":100.0,"maxElevationMsl":samples.last().unwrap(),"encoding":"little-endian-f32-row-major"
    });
    fs::write(
        directory.path().join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.path().join("heights.f32"),
        samples
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    directory
}

#[test]
fn package_import_checks_version_sizes_numeric_values_and_territory() {
    let dir = package(3, 3);
    let data = source::load_terrain(dir.path()).unwrap();
    assert_eq!(data.position(2, 2), [20.0, 20.0, 6.0]);
    let json_path = dir.path().join("manifest.json");
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&json_path).unwrap()).unwrap();
    for (key, value) in [
        ("formatVersion", serde_json::json!(2)),
        ("width", serde_json::json!(0)),
        ("spacingXM", serde_json::json!(0)),
        ("minXM", serde_json::json!(100)),
        ("encoding", serde_json::json!("bad")),
        ("maxElevationMsl", serde_json::json!(10000)),
    ] {
        let mut broken = json.clone();
        broken[key] = value;
        fs::write(&json_path, serde_json::to_vec(&broken).unwrap()).unwrap();
        assert!(
            source::load_terrain(dir.path()).is_err(),
            "accepted invalid {key}"
        );
    }
    fs::write(&json_path, serde_json::to_vec(&json).unwrap()).unwrap();
    fs::write(dir.path().join("heights.f32"), [0u8; 4]).unwrap();
    assert!(source::load_terrain(dir.path()).is_err());
    let mut bytes = vec![0u8; 36];
    bytes[..4].copy_from_slice(&f32::NAN.to_le_bytes());
    fs::write(dir.path().join("heights.f32"), bytes).unwrap();
    assert!(source::load_terrain(dir.path()).is_err());
    fs::remove_file(&json_path).unwrap();
    assert!(source::load_terrain(dir.path()).is_err());
}

#[test]
fn known_uav_geo_points_match_both_directions() {
    let origin = GeoOrigin {
        longitude: 42.45,
        latitude: 43.35,
    };
    // Same public contract points as docs/fixtures/geo-coordinate-parity.json in UAV.
    for (east, south, lon, lat) in [
        (1000.0, 0.0, 42.4623534557872, 43.35),
        (-1750.0, 2250.0, 42.4283814523724, 43.3297879985627),
        (-10000.0, 10000.0, 42.326465442127976, 43.2601688825009),
    ] {
        let (x, y) = local_to_wgs84(origin, east, south);
        assert!((x - lon).abs() < 1e-11 && (y - lat).abs() < 1e-11);
        let p = wgs84_to_local(origin, lon, lat);
        assert!((p[0] - east).abs() < 1e-6 && (p[1] - south).abs() < 1e-6);
    }
}

fn attribute(mesh: &Mesh, id: bevy::mesh::MeshVertexAttribute) -> &Vec<[f32; 3]> {
    if let Some(VertexAttributeValues::Float32x3(v)) = mesh.attribute(id) {
        v
    } else {
        panic!("missing attribute");
    }
}

#[test]
fn chunk_seams_have_identical_vertices_normals_and_msl_reference() {
    let dir = package(259, 131);
    let data = source::load_terrain(dir.path()).unwrap();
    let chunks = dem_mesh::chunks(&data);
    assert_eq!(chunks.len(), 6);
    let a = &chunks[0];
    let b = &chunks[1];
    for y in 0..=128 {
        assert_eq!(
            attribute(a, Mesh::ATTRIBUTE_POSITION)[y * 129 + 128],
            attribute(b, Mesh::ATTRIBUTE_POSITION)[y * 129]
        );
        assert_eq!(
            attribute(a, Mesh::ATTRIBUTE_NORMAL)[y * 129 + 128],
            attribute(b, Mesh::ATTRIBUTE_NORMAL)[y * 129]
        );
    }
    assert_eq!(attribute(a, Mesh::ATTRIBUTE_POSITION)[0], [0.0, 0.0, 0.0]);
    assert_eq!(
        attribute(a, Mesh::ATTRIBUTE_POSITION)[130],
        [10.0, 3.0, 10.0]
    );
    assert!(
        attribute(a, Mesh::ATTRIBUTE_NORMAL)
            .iter()
            .all(|n| n[1] > 0.0)
    );
}

pub fn world() -> World {
    let mut world = World::new();
    world.insert_resource(Assets::<Mesh>::default());
    world.insert_resource(Assets::<StandardMaterial>::default());
    world.insert_resource(Assets::<Image>::default());
    world.init_resource::<bevy_egui::EguiUserTextures>();
    world.init_resource::<ViewportState>();
    world.init_resource::<ActiveMapRegion>();
    world.init_resource::<TerrainImport>();
    world.init_resource::<RealScene>();
    world.init_resource::<RegionRuntime>();
    world
}

pub fn install(world: &mut World, data: source::TerrainPayload) {
    world
        .resource_mut::<ActiveMapRegion>()
        .confirm(data.manifest.region())
        .unwrap();
    let prepared = PreparedTerrain {
        meshes: dem_mesh::chunks(&data),
        grid: Some(dem_mesh::grid(&data)),
        data: Arc::new(data),
    };
    world.resource_mut::<TerrainImport>().prepared = Some(prepared);
    world.resource_mut::<RegionRuntime>().real_requested = true;
    let mut schedule = Schedule::default();
    schedule.add_systems(real_scene::open_real);
    schedule.run(world);
}

#[test]
fn three_packages_replace_assets_and_reopen_keeps_geometry() {
    let dirs = [package(3, 3), package(130, 4), package(131, 131)];
    let mut world = world();
    for _ in 0..3 {
        for dir in &dirs {
            let data = source::load_terrain(dir.path()).unwrap();
            let expected = dem_mesh::chunks(&data).len() + 1;
            install(&mut world, data);
            assert_eq!(world.resource::<Assets<Mesh>>().len(), expected);
            assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 2);
            assert_eq!(world.query::<&Mesh3d>().iter(&world).count(), expected);
            let before = world.resource::<RealScene>().entities.clone();
            world.resource_mut::<RegionRuntime>().real_requested = true;
            let mut schedule = Schedule::default();
            schedule.add_systems(real_scene::open_real);
            schedule.run(&mut world);
            assert_eq!(world.resource::<RealScene>().entities, before);
        }
    }
}

#[test]
fn mismatched_region_does_not_replace_correct_scene() {
    let dir = package(3, 3);
    let mut world = world();
    install(&mut world, source::load_terrain(dir.path()).unwrap());
    let before = world.resource::<RealScene>().entities.clone();
    let (_, mut region) = world.resource::<ActiveMapRegion>().get().unwrap();
    region.origin.longitude += 0.001;
    world
        .resource_mut::<ActiveMapRegion>()
        .confirm(region)
        .unwrap();
    world.resource_mut::<RegionRuntime>().real_requested = true;
    let mut schedule = Schedule::default();
    schedule.add_systems(real_scene::open_real);
    schedule.run(&mut world);
    assert_eq!(world.resource::<RealScene>().entities, before);
    assert!(world.resource::<TerrainImport>().error.is_some());
}

#[test]
fn changed_revision_invalidates_inflight_request() {
    let mut active = ActiveMapRegion::default();
    let import = TerrainImport::default();
    assert!(import.is_current(&active));
    let dir = package(3, 3);
    let data = source::load_terrain(dir.path()).unwrap();
    active.confirm(data.manifest.region()).unwrap();
    assert!(!import.is_current(&active));
}

#[test]
fn installed_uav_cache_packages_smoke() {
    let Some(root) = std::env::var_os("STARTER_TEST_TERRAIN_ROOT") else {
        return;
    };
    let mut paths: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    let mut world = world();
    let mut checked = 0;
    for path in paths {
        if !path.join("manifest.json").is_file() {
            continue;
        }
        let data = source::load_terrain(&path).unwrap();
        assert_ne!(data.manifest.source, "test-fixture");
        install(&mut world, data);
        assert_eq!(
            world.query::<&Mesh3d>().iter(&world).count(),
            world.resource::<RealScene>().entities.len()
        );
        checked += 1;
        if checked == 3 {
            break;
        }
    }
    assert_eq!(checked, 3, "Need at least three real packages");
}
