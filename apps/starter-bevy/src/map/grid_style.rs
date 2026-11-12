//! Standard Bevy gizmos provide pixel line width; reuse the existing grid vertices.
use super::{RegionRuntime, SceneMode, real_scene::RealScene};
use bevy::{camera::visibility::RenderLayers, mesh::VertexAttributeValues, prelude::*};
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct TerrainGridStyle;
pub fn setup(mut configs: ResMut<GizmoConfigStore>) {
    let (config, _) = configs.config_mut::<TerrainGridStyle>();
    config.render_layers = RenderLayers::layer(3);
    config.line.width = 1.5;
}
pub fn draw(
    mut gizmos: Gizmos<TerrainGridStyle>,
    scene: Res<RealScene>,
    runtime: Res<RegionRuntime>,
    project: Res<crate::project::Project>,
    meshes: Res<Assets<Mesh>>,
) {
    if runtime.mode != SceneMode::Real
        || !scene.grid_enabled
        || !project
            .document
            .layer(starter_core::LayerKind::Terrain)
            .visible
    {
        return;
    }
    if let Some(mesh) = scene.meshes.last().and_then(|h| meshes.get(h))
        && let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        for pair in positions.chunks_exact(2) {
            gizmos.line(
                Vec3::from_array(pair[0]),
                Vec3::from_array(pair[1]),
                Color::srgb(0.12, 0.23, 0.29),
            );
        }
    }
}
