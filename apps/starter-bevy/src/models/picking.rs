//! Native mesh raycast over the existing texture Viewport, including child meshes.
use super::instances::ModelRoot;
use crate::{
    map::{objects::ProjectedMarker, real_scene::RealScene},
    project::{Project, ProjectIo},
};
use bevy::{
    camera::visibility::RenderLayers,
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility},
    prelude::*,
};
use starter_core::ProjectSelection;
#[derive(Resource, Default)]
pub struct ModelPick {
    pub pending: Option<(crate::viewport::picking::Ray, Option<blake3::Hash>, u64)>,
}
pub fn root_id(
    entity: Entity,
    roots: &Query<&ModelRoot>,
    parents: &Query<&ChildOf>,
) -> Option<starter_core::ObjectId> {
    let mut current = entity;
    loop {
        if let Ok(root) = roots.get(current) {
            return Some(root.0);
        }
        current = parents.get(current).ok()?.parent();
    }
}
pub fn pick(
    mut pending: ResMut<ModelPick>,
    mut raycast: MeshRayCast,
    mut project: ResMut<Project>,
    io: Res<ProjectIo>,
    terrain: (Res<RealScene>, Res<crate::map::region::ActiveMapRegion>),
    geometry: (
        Query<&ModelRoot>,
        Query<&ChildOf>,
        Query<&ProjectedMarker>,
        Query<&RenderLayers>,
    ),
) {
    let (roots, parents, markers, layers) = geometry;
    let (scene, active) = terrain;
    let Some((ray, fingerprint, generation)) = pending.pending.take() else {
        return;
    };
    if scene.revision != active.get().map(|(r, _)| r)
        || io.benchmark
        || generation != io.generation
        || fingerprint != scene.fingerprint()
    {
        return;
    }
    let Ok(direction) = Dir3::new(ray.direction) else {
        return;
    };
    let filter = |entity| {
        layers
            .get(entity)
            .is_ok_and(|l| l.intersects(&RenderLayers::layer(3)))
    };
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Visible)
        .with_filter(&filter);
    if let Some((entity, _)) = raycast
        .cast_ray(Ray3d::new(ray.origin, direction), &settings)
        .first()
    {
        // Terrain participates in the raycast and occludes objects behind it.
        if let Some(id) = root_id(*entity, &roots, &parents) {
            project.selection = ProjectSelection::Model(id);
        } else if let Ok(marker) = markers.get(*entity) {
            project.selection = ProjectSelection::Marker(marker.0);
        }
    }
}
