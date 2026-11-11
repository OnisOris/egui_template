use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::prelude::*;
use bevy_egui::{EguiTextureHandle, EguiUserTextures, PrimaryEguiContext};
use starter_core::{ObjectId, Position};

use crate::state::DocumentResource;

use crate::scene::objects::bounds;

pub mod camera;
pub mod input;
pub mod picking;
pub mod target;

pub use camera::{OrbitCamera, ProjectionMode};

#[derive(Clone, Copy, Debug)]
pub enum DragTarget {
    Orbit,
    Pan,
    Object {
        id: ObjectId,
        start_position: Position,
        start_hit: Vec3,
    },
}

#[derive(Resource)]
pub struct ViewportState {
    pub camera: OrbitCamera,
    pub image: Handle<Image>,
    pub size: UVec2,
    pub requested_size: Option<UVec2>,
    pub texture_id: Option<bevy_egui::egui::TextureId>,
    pub drag: Option<DragTarget>,
}

impl FromWorld for ViewportState {
    fn from_world(world: &mut World) -> Self {
        let image = {
            let mut images = world.resource_mut::<Assets<Image>>();
            target::create_image(&mut images, UVec2::splat(512))
        };
        let texture_id = Some(
            world
                .resource_mut::<EguiUserTextures>()
                .add_image(EguiTextureHandle::Strong(image.clone())),
        );
        Self {
            camera: OrbitCamera::default(),
            image,
            size: UVec2::splat(512),
            requested_size: None,
            texture_id,
            drag: None,
        }
    }
}

#[derive(Component)]
pub struct ViewportCamera;

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewportState>()
            .add_systems(Startup, setup_viewport)
            .add_systems(Update, apply_camera);
    }
}

fn setup_viewport(
    mut commands: Commands,
    mut viewport: ResMut<ViewportState>,
    document: Res<DocumentResource>,
) {
    viewport.camera.reset(bounds(&document));
    commands.spawn((
        AmbientLight {
            color: Color::WHITE,
            brightness: 250.0,
            ..default()
        },
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.025, 0.035)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            far: 100_000.0,
            ..default()
        }),
        RenderTarget::Image(viewport.image.clone().into()),
        RenderLayers::layer(1),
        ViewportCamera,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.03, 0.035, 0.045)),
            ..default()
        },
        PrimaryEguiContext,
    ));
}

fn apply_camera(
    viewport: Res<ViewportState>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<ViewportCamera>>,
) {
    let aspect = viewport.size.x as f32 / viewport.size.y.max(1) as f32;
    for (mut transform, mut projection) in &mut cameras {
        *transform = viewport.camera.transform();
        *projection = viewport.camera.projection(aspect, viewport.size.y as f32);
    }
}
