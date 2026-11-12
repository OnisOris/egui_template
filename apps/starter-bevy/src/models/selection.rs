use super::{geometry::ModelGeometry, instances::ModelRoot};
use bevy::{
    camera::visibility::RenderLayers,
    gizmos::config::{GizmoConfigGroup, GizmoConfigStore},
    prelude::*,
};
use starter_core::ProjectSelection;
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ModelGizmos;
pub fn configure(mut config: ResMut<GizmoConfigStore>) {
    config.config_mut::<ModelGizmos>().0.render_layers = RenderLayers::layer(3);
}
pub fn show(
    project: Res<crate::project::Project>,
    roots: Query<(
        &ModelRoot,
        &ModelGeometry,
        &GlobalTransform,
        &InheritedVisibility,
    )>,
    mut gizmos: Gizmos<ModelGizmos>,
) {
    for (root, geometry, transform, visible) in &roots {
        if project.selection != ProjectSelection::Model(root.0) || !visible.get() {
            continue;
        }
        if let Some((min, max)) = geometry.bounds {
            let transform = transform.compute_transform();
            gizmos.cube(
                Transform {
                    translation: transform.transform_point((min + max) * 0.5),
                    rotation: transform.rotation,
                    scale: transform.scale * (max - min),
                },
                Color::srgb(1.0, 0.85, 0.15),
            );
        }
    }
}
