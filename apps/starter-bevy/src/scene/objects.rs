use std::collections::HashMap;

use bevy::prelude::*;
use starter_core::{DocumentState, GroupId, ObjectId, SceneObject};

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectedObject(pub ObjectId);

#[derive(Clone, Copy, Debug, Default)]
pub struct SceneBounds {
    pub center: Vec3,
    pub radius: f32,
}

pub fn bounds(document: &DocumentState) -> SceneBounds {
    let objects = document.document().objects();
    if objects.is_empty() {
        return SceneBounds {
            center: Vec3::ZERO,
            radius: 120.0,
        };
    }
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut max_size = 0.0_f32;
    for object in objects {
        let position = object.position();
        let point = Vec2::new(position.x as f32, position.y as f32);
        min = min.min(point);
        max = max.max(point);
        max_size = max_size.max(object.size());
    }
    let size = max - min;
    SceneBounds {
        center: Vec3::new((min.x + max.x) * 0.5, 0.0, -(min.y + max.y) * 0.5),
        radius: (size.length() * 0.5 + max_size).max(120.0),
    }
}

pub fn object_transform(object: &SceneObject) -> Transform {
    Transform::from_translation(object_center(object)).with_scale(Vec3::splat(object.size()))
}

pub fn object_center(object: &SceneObject) -> Vec3 {
    let position = object.position();
    Vec3::new(position.x as f32, object.size() * 0.5, -(position.y as f32))
}

pub fn mesh_index(group: GroupId) -> usize {
    match group {
        GroupId::A => 0,
        GroupId::B => 1,
        GroupId::C => 2,
    }
}

#[derive(Resource)]
pub struct SceneAssets {
    pub meshes: [Handle<Mesh>; 3],
    pub highlight: Handle<StandardMaterial>,
    materials: HashMap<[u8; 4], Handle<StandardMaterial>>,
}

impl FromWorld for SceneAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let sphere = meshes.add(Sphere::new(0.5));
        let cylinder = meshes.add(Cylinder::new(0.5, 1.0));
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let highlight = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.82, 0.2),
            unlit: true,
            ..default()
        });
        Self {
            meshes: [cube, sphere, cylinder],
            highlight,
            materials: HashMap::new(),
        }
    }
}

impl SceneAssets {
    pub fn mesh(&self, object: &SceneObject) -> Handle<Mesh> {
        self.meshes[mesh_index(object.group())].clone()
    }

    pub fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        color: [u8; 4],
    ) -> Handle<StandardMaterial> {
        self.materials
            .entry(color)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgba_u8(color[0], color[1], color[2], color[3]),
                    perceptual_roughness: 0.65,
                    metallic: 0.05,
                    ..default()
                })
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_cover_the_document() {
        let document = DocumentState::default();
        let bounds = bounds(&document);
        assert!(bounds.radius > 0.0);
        for object in document.document().objects() {
            let position = object.position();
            let point = Vec3::new(position.x as f32, 0.0, -(position.y as f32));
            assert!(point.distance(bounds.center) <= bounds.radius);
        }
    }

    #[test]
    fn each_group_uses_a_shared_mesh() {
        assert_ne!(mesh_index(GroupId::A), mesh_index(GroupId::B));
        assert_ne!(mesh_index(GroupId::B), mesh_index(GroupId::C));
    }
}
