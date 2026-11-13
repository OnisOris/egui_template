//! ObjectId → native world root. All editable parameters remain in ProjectDocument.
use super::{
    assets::{ModelAssets, ModelStatus},
    geometry::ModelGeometry,
};
use crate::{
    map::{real_scene::RealScene, region::ActiveMapRegion},
    project::{Project, ProjectIo},
};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use starter_core::{ModelInstance, ObjectId};
use std::{collections::HashMap, path::PathBuf};

#[derive(Component)]
pub struct ModelRoot(pub ObjectId);
#[derive(Resource, Default)]
pub struct ModelScene {
    pub entities: HashMap<ObjectId, Entity>,
    pub information: HashMap<ObjectId, ModelInformation>,
    project_key: (Option<PathBuf>, u64),
}
#[derive(Clone)]
pub struct ModelInformation {
    pub status: String,
    pub dimensions: Option<Vec3>,
    pub center: Option<Vec3>,
}

pub fn transform(
    model: &ModelInstance,
    data: &crate::map::source::TerrainPayload,
    min_y: f32,
) -> Option<Transform> {
    transform_for_view(model, data, min_y, starter_core::GeoViewMode::Terrain)
}
pub fn transform_for_view(
    model: &ModelInstance,
    data: &crate::map::source::TerrainPayload,
    min_y: f32,
    mode: starter_core::GeoViewMode,
) -> Option<Transform> {
    let p = crate::map::objects::view_position(
        model.longitude,
        model.latitude,
        model.altitude_agl,
        data,
        mode,
    )?;
    Some(
        Transform::from_translation(p - Vec3::Y * min_y * model.scale)
            .with_rotation(Quat::from_rotation_y(
                model.yaw_degrees.rem_euclid(360.0).to_radians(),
            ))
            .with_scale(Vec3::splat(model.scale)),
    )
}

pub fn sync(
    mut commands: Commands,
    project: Res<Project>,
    io: Res<ProjectIo>,
    terrain: (Res<ActiveMapRegion>, Res<RealScene>),
    assets: Res<ModelAssets>,
    mut scene: ResMut<ModelScene>,
    mut roots: Query<
        (
            &WorldAssetRoot,
            &ModelGeometry,
            &mut Transform,
            &mut Visibility,
        ),
        With<ModelRoot>,
    >,
) {
    let (active, terrain) = terrain;
    let key = (io.path.clone(), io.generation);
    let changed_project = scene.project_key != key;
    scene.project_key = key;
    let data = terrain
        .data
        .as_ref()
        .filter(|_| !io.benchmark && terrain.revision == active.get().map(|(r, _)| r));
    let obsolete: Vec<_> = scene
        .entities
        .iter()
        .filter(|(id, _)| {
            changed_project
                || data.is_none()
                || !project.document.models.iter().any(|m| m.id == **id)
        })
        .map(|(id, e)| (*id, *e))
        .collect();
    for (id, e) in obsolete {
        commands.entity(e).despawn();
        scene.entities.remove(&id);
    }
    scene.information.clear();
    for model in &project.document.models {
        let asset = assets.entries.get(&model.asset_path);
        let mut info = ModelInformation {
            status: asset
                .map(|a| a.status.to_string())
                .unwrap_or_else(|| "Loading".into()),
            dimensions: None,
            center: None,
        };
        let Some(position) =
            data.and_then(|d| transform_for_view(model, d, 0., project.document.geo_view_mode))
        else {
            if let Some(entity) = scene.entities.remove(&model.id) {
                commands.entity(entity).despawn();
            }
            scene.information.insert(model.id, info);
            continue;
        };
        if let Some(asset) = asset.filter(|a| a.status == ModelStatus::Ready)
            && let Some(handle) = &asset.scene
        {
            if let Some(entity) = scene.entities.get(&model.id).copied()
                && let Ok((root, geometry, mut current, mut visibility)) = roots.get_mut(entity)
            {
                if root.0.id() != handle.id() {
                    commands.entity(entity).despawn();
                    scene.entities.remove(&model.id);
                } else if let Some(error) = &geometry.error {
                    info.status = error.clone();
                    *visibility = Visibility::Hidden;
                } else if let Some((min, max)) = geometry.bounds {
                    let next = Transform {
                        translation: position.translation - Vec3::Y * min.y * model.scale,
                        ..position
                    };
                    if *current != next {
                        *current = next;
                    }
                    let next_visibility = if model.visible {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    if *visibility != next_visibility {
                        *visibility = next_visibility;
                    }
                    info.dimensions = Some((max - min) * model.scale);
                    info.center = Some(next.transform_point((min + max) * 0.5));
                } else {
                    info.status = "Loading scene geometry".into();
                }
            }
            if let std::collections::hash_map::Entry::Vacant(entry) = scene.entities.entry(model.id)
            {
                let entity = commands
                    .spawn((
                        ModelRoot(model.id),
                        WorldAssetRoot(handle.clone()),
                        ModelGeometry::default(),
                        position,
                        Visibility::Hidden,
                        RenderLayers::layer(3),
                    ))
                    .id();
                entry.insert(entity);
                info.status = "Loading scene geometry".into();
            }
        }
        scene.information.insert(model.id, info);
    }
}
