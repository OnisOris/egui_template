use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

pub mod grid;
pub mod objects;
pub mod sync;

pub use objects::SceneBounds;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<objects::SceneAssets>()
            .init_resource::<sync::SceneSync>()
            .add_systems(Startup, setup_scene)
            .add_systems(
                Update,
                (sync::sync_scene, grid::draw_grid, grid::draw_selection),
            );
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_config: ResMut<GizmoConfigStore>,
) {
    let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
    config.render_layers = RenderLayers::layer(1);

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(300.0, 600.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::from_layers(&[1, 2, 3]),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20_000.0, 20_000.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.055, 0.065, 0.08),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.05, 0.0),
        RenderLayers::layer(1),
    ));
}
