use super::*;
use crate::project::Project;
use bevy::{ecs::schedule::Schedule, mesh::VertexAttributeValues};
use starter_core::GeoViewMode;
use std::sync::Arc;
#[test]
fn city_and_terrain_reuse_assets_and_preserve_dem_geography_and_shared_uvs() {
    let dir = dem_tests::package(131, 3);
    let data = Arc::new(source::load_terrain(dir.path()).unwrap());
    let sat = satellite_tests::package(&data);
    let payload = satellite::load_satellite(sat.path(), &data).unwrap();
    let mut world = World::new();
    world.init_resource::<Assets<Mesh>>();
    world.init_resource::<Assets<Image>>();
    world.init_resource::<Assets<StandardMaterial>>();
    world.init_resource::<bevy_egui::EguiUserTextures>();
    world.init_resource::<crate::viewport::ViewportState>();
    world.init_resource::<city_view::CityGround>();
    world.init_resource::<Project>();
    let mut scene = real_scene::RealScene {
        data: Some(data.clone()),
        ..default()
    };
    for mesh in dem_mesh::chunks(&data) {
        let h = world.resource_mut::<Assets<Mesh>>().add(mesh);
        scene.entities.push(world.spawn(Mesh3d(h.clone())).id());
        scene.meshes.push(h);
    }
    let original = scene.meshes.clone();
    let image = world.resource_mut::<Assets<Image>>().add(Image::default());
    scene.satellite = Some(satellite_scene::InstalledSatellite {
        image,
        source: satellite_scene::ImagerySource::Package("fixture".into()),
        metadata: payload.manifest.satellite,
        fingerprint: payload.fingerprint,
    });
    world.insert_resource(scene);
    let doc = world.resource::<Project>().document.clone();
    let mut schedule = Schedule::default();
    schedule.add_systems(city_view::update);
    for mode in [
        GeoViewMode::City,
        GeoViewMode::Terrain,
        GeoViewMode::Combined,
        GeoViewMode::City,
    ] {
        world.resource_mut::<Project>().document.geo_view_mode = mode;
        schedule.run(&mut world);
        assert_eq!(world.resource::<Assets<Mesh>>().len(), 4);
        let scene = world.resource::<real_scene::RealScene>();
        for (i, e) in scene.entities.iter().enumerate() {
            let handle = &world.get::<Mesh3d>(*e).unwrap().0;
            assert_eq!(handle == &original[i], mode != GeoViewMode::City);
            if mode == GeoViewMode::City {
                let Some(VertexAttributeValues::Float32x3(p)) = world
                    .resource::<Assets<Mesh>>()
                    .get(handle)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!()
                };
                assert!(p.iter().all(|p| p[1] == 0.));
            }
        }
    }
    let scene = world.resource::<real_scene::RealScene>();
    let uv = |i| {
        let h = &world.get::<Mesh3d>(scene.entities[i]).unwrap().0;
        let Some(VertexAttributeValues::Float32x2(uv)) = world
            .resource::<Assets<Mesh>>()
            .get(h)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!()
        };
        uv.clone()
    };
    assert_eq!(uv(0)[1], uv(1)[0]);
    assert_eq!(uv(0)[3], uv(1)[2]);
    assert_eq!(data.position(130, 2)[2], 134.);
    assert_eq!(world.resource::<Project>().document.objects, doc.objects);
    world.resource_mut::<real_scene::RealScene>().data = None;
    schedule.run(&mut world);
    assert_eq!(world.resource::<Assets<Mesh>>().len(), 2);
}
#[test]
fn all_modes_project_objects_without_mutating_saved_coordinates() {
    let dir = dem_tests::package(3, 3);
    let terrain = source::load_terrain(dir.path()).unwrap();
    let point = terrain.manifest.bbox.center();
    let model = starter_core::ModelInstance {
        id: starter_core::ObjectId(1),
        name: "fixture".into(),
        asset_path: "Assets/Models/test.glb".into(),
        longitude: point.longitude,
        latitude: point.latitude,
        altitude_agl: 7.,
        yaw_degrees: 30.,
        scale: 2.,
        visible: true,
    };
    let original = model.clone();
    let dem = objects::geo_position(point.longitude, point.latitude, 7., &terrain).unwrap();
    for mode in [
        GeoViewMode::City,
        GeoViewMode::Terrain,
        GeoViewMode::Combined,
    ] {
        let marker =
            objects::view_position(point.longitude, point.latitude, 7., &terrain, mode).unwrap();
        let glb = crate::models::instances::transform_for_view(&model, &terrain, 1., mode).unwrap();
        assert_eq!(marker.x, dem.x);
        assert_eq!(marker.z, dem.z);
        assert_eq!(marker.y, if mode == GeoViewMode::City { 7. } else { dem.y });
        assert_eq!(glb.translation, marker - Vec3::Y * 2.);
    }
    assert_eq!(model, original);
}
