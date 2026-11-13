use super::{Project, ProjectIo};
use crate::map::{
    RegionRuntime, SceneMode,
    loading::TerrainImport,
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
    satellite_scene::SatelliteImport,
};
use bevy::prelude::*;
use starter_core::{Command, LayerKind};

#[derive(bevy::ecs::system::SystemParam)]
pub struct MapProject<'w> {
    active: ResMut<'w, ActiveMapRegion>,
    runtime: ResMut<'w, RegionRuntime>,
    import: ResMut<'w, TerrainImport>,
    scene: ResMut<'w, RealScene>,
    satellite: ResMut<'w, SatelliteImport>,
    assets: SceneAssets<'w>,
    picker: NonSendMut<'w, crate::map::picker::MapPicker>,
    demo: ResMut<'w, crate::map::demo::DemoScene>,
}
pub fn sync_project(
    mut commands: Commands,
    mut project: ResMut<Project>,
    mut io: ResMut<ProjectIo>,
    map: MapProject,
    mut document: ResMut<crate::state::DocumentResource>,
) {
    let MapProject {
        mut active,
        mut runtime,
        mut import,
        mut scene,
        mut satellite,
        mut assets,
        mut picker,
        mut demo,
    } = map;
    if std::mem::take(&mut io.reset_scene) {
        demo.clear(&mut commands, &mut assets.meshes, &mut assets.materials);
        import.cancel();
        satellite.cancel();
        active.clear();
        picker.selection.clear();
        if let Some(s) = scene.satellite.take() {
            assets.images.remove(s.image.id());
        }
        for entity in scene.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        for mesh in scene.meshes.drain(..) {
            assets.meshes.remove(mesh.id());
        }
        for material in scene.materials.drain(..) {
            assets.materials.remove(material.id());
        }
        scene.data = None;
        scene.revision = None;
        scene.satellite_enabled = false;
        runtime.mode = SceneMode::Editor;
        runtime.real_requested = false;
        runtime.open_requested = false;
        runtime.vector_requested = false;
        let _ = document.dispatch(Command::NewDocument);
        if let Some(region) = project.document.map_region {
            let _ = active.confirm(region);
            picker.view.memory.center_at(walkers::lon_lat(
                region.bounds.center().longitude,
                region.bounds.center().latitude,
            ));
            picker.view.fit_requested = true;
            if io.restore
                && let Some(id) = &project.document.layer(LayerKind::Terrain).package_id
                && let Ok(root) = crate::map::builder::cache::root()
            {
                import.start(
                    &active,
                    Some(root.join("packages").join(id).join("terrain")),
                    None,
                );
            }
        }
    }
    if io.restore
        && import.prepared.as_ref().is_some_and(|p| {
            active
                .get()
                .is_some_and(|(_, r)| p.data.manifest.matches(r))
        })
    {
        runtime.real_requested = true;
    }
    if io.restore && scene.revision.is_some() && scene.revision == active.get().map(|(r, _)| r) {
        io.restore = false;
        if let Some(id) = &project.document.layer(LayerKind::Satellite).package_id
            && let Ok(root) = crate::map::builder::cache::root()
        {
            satellite.start(
                &active,
                &scene,
                Some(root.join("packages").join(id).join("map-data")),
            );
        }
    }
    let region = active.get().map(|(_, r)| r);
    if project.document.map_region != region {
        project.document.map_region = region;
        for layer in &mut project.document.layers {
            layer.package_id = None;
        }
        project.changed();
    }
    if scene.revision == active.get().map(|(r, _)| r)
        && let Some(data) = &scene.data
    {
        for (kind, id) in [
            (LayerKind::Terrain, Some(data.manifest.terrain_id.clone())),
            (
                LayerKind::Satellite,
                scene.satellite.as_ref().and_then(|s| match &s.source {
                    crate::map::satellite_scene::ImagerySource::Package(id) => Some(id.clone()),
                    crate::map::satellite_scene::ImagerySource::Live { .. } => None,
                }),
            ),
        ] {
            let layer = project.document.layer_mut(kind);
            if let Some(id) = id
                && layer.package_id.as_ref() != Some(&id)
            {
                layer.package_id = Some(id);
                project.changed();
            }
        }
    }
    scene.satellite_enabled &= project.document.layer(LayerKind::Satellite).visible;
}

pub fn layer_visibility(
    project: Res<Project>,
    scene: Res<RealScene>,
    mut objects: Query<(
        &mut Visibility,
        Option<&crate::map::real_scene::TerrainGrid>,
    )>,
) {
    for entity in &scene.entities {
        if let Ok((mut visibility, grid)) = objects.get_mut(*entity) {
            let visible = project.document.layer(LayerKind::Terrain).visible && grid.is_none();
            *visibility = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
