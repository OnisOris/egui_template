use super::{
    RegionRuntime, SceneMode, loading::TerrainImport, region::ActiveMapRegion,
    source::TerrainPayload,
};
use crate::{
    scene::SceneBounds,
    viewport::{ViewportState, focus_geo},
};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use std::sync::Arc;

#[derive(Resource)]
pub struct RealScene {
    pub data: Option<Arc<TerrainPayload>>,
    pub revision: Option<u64>,
    pub grid_enabled: bool,
    pub entities: Vec<Entity>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}
impl Default for RealScene {
    fn default() -> Self {
        Self {
            data: None,
            revision: None,
            grid_enabled: true,
            entities: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
        }
    }
}
#[derive(Component)]
pub struct TerrainGrid;

impl RealScene {
    pub fn fingerprint(&self) -> Option<blake3::Hash> {
        self.data.as_ref().map(|d| d.fingerprint)
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct SceneAssets<'w> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
}

pub fn open_real(
    mut commands: Commands,
    active: Res<ActiveMapRegion>,
    mut runtime: ResMut<RegionRuntime>,
    mut import: ResMut<TerrainImport>,
    mut scene: ResMut<RealScene>,
    mut viewport: ResMut<ViewportState>,
    mut assets: SceneAssets,
) {
    if !std::mem::take(&mut runtime.real_requested) {
        return;
    }
    let Some((revision, region)) = active.get() else {
        import.error = Some("Confirm a region first.".into());
        return;
    };
    let candidate = import
        .prepared
        .as_ref()
        .map(|p| &p.data)
        .or(scene.data.as_ref());
    let Some(data) = candidate else {
        import.error = Some("No terrain package. Import a DEM folder.".into());
        return;
    };
    if !data.manifest.matches(region) {
        import.error = Some("Package bbox/origin differs from the confirmed region. Use package area, then Confirm Region.".into());
        return;
    }
    let data = data.clone();
    if scene.fingerprint() != Some(data.fingerprint) {
        let Some(prepared) = import.prepared.take() else {
            return;
        };
        if prepared.meshes.is_empty() {
            import.error = Some("Cached geometry changed. Import the package again.".into());
            return;
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
        let material = assets.materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.57, 0.49),
            perceptual_roughness: 0.95,
            ..default()
        });
        let line = assets.materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.32, 0.35),
            unlit: true,
            ..default()
        });
        for mesh in prepared.meshes {
            let handle = assets.meshes.add(mesh);
            scene.entities.push(
                commands
                    .spawn((
                        Mesh3d(handle.clone()),
                        MeshMaterial3d(material.clone()),
                        RenderLayers::layer(3),
                    ))
                    .id(),
            );
            scene.meshes.push(handle);
        }
        if let Some(grid) = prepared.grid {
            let handle = assets.meshes.add(grid);
            scene.entities.push(
                commands
                    .spawn((
                        Mesh3d(handle.clone()),
                        MeshMaterial3d(line.clone()),
                        RenderLayers::layer(3),
                        TerrainGrid,
                    ))
                    .id(),
            );
            scene.meshes.push(handle);
        }
        scene.materials = vec![material, line];
    } else {
        import.prepared = None;
    }
    scene.data = Some(data.clone());
    scene.revision = Some(revision);
    let m = &data.manifest;
    let min = Vec3::new(
        m.min_x_m as f32,
        m.min_elevation_msl - m.reference_elevation_msl,
        m.min_y_m as f32,
    );
    let max = Vec3::new(
        (m.min_x_m + f64::from(m.width - 1) * m.spacing_x_m) as f32,
        m.max_elevation_msl - m.reference_elevation_msl,
        (m.min_y_m + f64::from(m.height - 1) * m.spacing_y_m) as f32,
    );
    viewport.scene_bounds = Some(SceneBounds {
        center: (min + max) * 0.5,
        radius: (max - min).length() * 0.5,
    });
    viewport.drag = None;
    viewport.fit_geo_pending = true;
    focus_geo(&mut viewport);
    runtime.mode = SceneMode::Real;
    import.error = None;
}

pub fn grid_visibility(
    scene: Res<RealScene>,
    mut grids: Query<&mut Visibility, With<TerrainGrid>>,
) {
    let visibility = if scene.grid_enabled {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut current in &mut grids {
        if *current != visibility {
            *current = visibility;
        }
    }
}
