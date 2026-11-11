//! Replace this loader with a runtime SetMapArea adapter; picker and contract stay unchanged.
use super::{
    RegionRuntime,
    coordinates::{to_bevy, wgs84_to_local},
    region::{ActiveMapRegion, MapRegion},
    terrain,
};
use crate::{scene::SceneBounds, viewport::ViewportState};
use bevy::{camera::visibility::RenderLayers, prelude::*};

#[derive(Resource, Default)]
pub struct DemoScene {
    revision: Option<u64>,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
}

pub fn validate_demo(region: MapRegion) -> Result<(), &'static str> {
    MapRegion::new(region.bounds, region.origin)?;
    let (width, height) = region.bounds.dimensions();
    if width < 1.0 || height < 1.0 || width > 100_000.0 || height > 100_000.0 {
        return Err("Demo 3D supports sides from 1 m to 100 km. Select a smaller/larger region.");
    }
    let offset = wgs84_to_local(
        region.origin,
        region.bounds.center().longitude,
        region.bounds.center().latitude,
    );
    if offset[0].abs().max(offset[1].abs()) > 1_000_000.0 {
        return Err("Demo origin must be within 1000 km of the region for local float precision.");
    }
    Ok(())
}

pub fn open_region(
    mut commands: Commands,
    active: Res<ActiveMapRegion>,
    mut runtime: ResMut<RegionRuntime>,
    mut scene: ResMut<DemoScene>,
    mut viewport: ResMut<ViewportState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !std::mem::take(&mut runtime.open_requested) {
        return;
    }
    let Some((revision, region)) = active.get() else {
        return;
    };
    if validate_demo(region).is_err() {
        return;
    }
    let b = region.bounds;
    let min = to_bevy(wgs84_to_local(region.origin, b.west, b.north));
    let max = to_bevy(wgs84_to_local(region.origin, b.east, b.south));
    if scene.revision != Some(revision) {
        for entity in scene.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        for handle in scene.meshes.drain(..) {
            meshes.remove(handle.id());
        }
        for handle in scene.materials.drain(..) {
            materials.remove(handle.id());
        }
        let terrain = meshes.add(terrain::surface(min, max));
        let grid = meshes.add(terrain::grid(min, max));
        let object = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let ground_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            ..default()
        });
        let line_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        });
        let object_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.55, 0.2),
            ..default()
        });
        for (mesh, material) in [
            (terrain.clone(), ground_material.clone()),
            (grid.clone(), line_material.clone()),
        ] {
            scene.entities.push(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        RenderLayers::layer(2),
                    ))
                    .id(),
            );
        }
        let size = (max.x - min.x).min(max.z - min.z) * 0.025;
        for (u, v) in [(0.25, 0.25), (0.65, 0.4), (0.45, 0.75)] {
            let center = terrain::landmark_position(min, max, u, v) + Vec3::Y * size * 0.5;
            scene.entities.push(
                commands
                    .spawn((
                        Mesh3d(object.clone()),
                        MeshMaterial3d(object_material.clone()),
                        Transform::from_translation(center).with_scale(Vec3::splat(size)),
                        RenderLayers::layer(2),
                    ))
                    .id(),
            );
        }
        scene.meshes = vec![terrain, grid, object];
        scene.materials = vec![ground_material, line_material, object_material];
        scene.revision = Some(revision);
    }
    runtime.showing_demo = true;
    viewport.drag = None;
    viewport.demo_bounds = Some(SceneBounds {
        center: (min + max) * 0.5,
        radius: (max - min).length() * 0.5,
    });
    focus_demo(&mut viewport);
    viewport.fit_demo_pending = true;
}

pub fn focus_demo(viewport: &mut ViewportState) {
    let Some(bounds) = viewport.demo_bounds else {
        return;
    };
    let aspect = viewport.size.x as f32 / viewport.size.y.max(1) as f32;
    let camera = &mut viewport.camera;
    camera.reset(bounds);
    camera.max_distance = bounds.radius * 20.0;
    camera.distance = (bounds.radius * 3.0 / aspect.clamp(0.1, 1.0)).min(camera.max_distance);
}

#[cfg(test)]
mod tests {
    use super::super::region::{GeoBounds, GeoOrigin};
    use super::*;
    use bevy::ecs::schedule::Schedule;

    #[test]
    fn reopening_replaces_entities_and_assets_only_on_request() {
        let mut world = World::new();
        world.insert_resource(Assets::<Mesh>::default());
        world.insert_resource(Assets::<StandardMaterial>::default());
        world.init_resource::<bevy_egui::EguiUserTextures>();
        world.insert_resource(Assets::<Image>::default());
        world.init_resource::<ViewportState>();
        world.init_resource::<ActiveMapRegion>();
        world.init_resource::<RegionRuntime>();
        world.init_resource::<DemoScene>();
        let region = MapRegion::new(
            GeoBounds {
                west: 30.0,
                east: 30.01,
                south: 55.0,
                north: 55.01,
            },
            GeoOrigin {
                longitude: 30.005,
                latitude: 55.005,
            },
        )
        .unwrap();
        let camera = world.spawn(crate::viewport::ViewportCamera).id();
        let mut schedule = Schedule::default();
        schedule.add_systems(open_region);
        world
            .resource_mut::<ActiveMapRegion>()
            .confirm(region)
            .unwrap();
        schedule.run(&mut world);
        assert_eq!(world.query::<&Mesh3d>().iter(&world).count(), 0);
        for iteration in 0..5 {
            let mut next = region;
            next.bounds.east += f64::from(iteration) * 0.001;
            world
                .resource_mut::<ActiveMapRegion>()
                .confirm(next)
                .unwrap();
            world.resource_mut::<RegionRuntime>().open_requested = true;
            schedule.run(&mut world);
            assert_eq!(world.query::<&Mesh3d>().iter(&world).count(), 5);
            assert_eq!(
                world
                    .query::<&crate::viewport::ViewportCamera>()
                    .iter(&world)
                    .count(),
                1
            );
            assert!(world.get_entity(camera).is_ok());
            assert_eq!(world.resource::<Assets<Mesh>>().len(), 3);
            assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 3);
            let before = world.resource::<DemoScene>().entities.clone();
            world.resource_mut::<RegionRuntime>().open_requested = true;
            schedule.run(&mut world);
            assert_eq!(world.resource::<DemoScene>().entities, before);
            assert!(
                world
                    .resource::<ViewportState>()
                    .camera
                    .distance
                    .is_finite()
            );
        }
    }
}
