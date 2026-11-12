//! Bounds include every child-local transform; imported vertices stay untouched.
use bevy::world_serialization::WorldInstanceReady;
use bevy::{
    camera::{primitives::MeshAabb, visibility::RenderLayers},
    math::Affine3A,
    prelude::*,
};

#[derive(Component, Default, Debug)]
pub struct ModelGeometry {
    pub bounds: Option<(Vec3, Vec3)>,
    pub error: Option<String>,
}

pub fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut roots: Query<&mut ModelGeometry>,
    children: Query<&Children>,
    nodes: Query<(&Transform, Option<&Mesh3d>)>,
    meshes: Res<Assets<Mesh>>,
) {
    let Ok(mut root) = roots.get_mut(event.entity) else {
        return;
    };
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut stack = vec![(event.entity, Affine3A::IDENTITY)];
    while let Some((entity, parent)) = stack.pop() {
        let mut matrix = parent;
        if entity != event.entity
            && let Ok((transform, mesh)) = nodes.get(entity)
        {
            matrix *= transform.compute_affine();
            commands.entity(entity).insert(RenderLayers::layer(3));
            if let Some(aabb) = mesh
                .and_then(|m| meshes.get(&m.0))
                .and_then(MeshAabb::get_aabb)
            {
                let lo = Vec3::from(aabb.min());
                let hi = Vec3::from(aabb.max());
                for x in [lo.x, hi.x] {
                    for y in [lo.y, hi.y] {
                        for z in [lo.z, hi.z] {
                            let p = matrix.transform_point3(Vec3::new(x, y, z));
                            min = min.min(p);
                            max = max.max(p);
                        }
                    }
                }
            }
        }
        if let Ok(children) = children.get(entity) {
            stack.extend(children.iter().map(|e| (e, matrix)));
        }
    }
    if min.is_finite() && max.is_finite() && (max - min).is_finite() && min.cmple(max).all() {
        root.bounds = Some((min, max));
    } else {
        root.error = Some("Unsupported Model: selected scene has no finite mesh bounds".into());
    }
}
