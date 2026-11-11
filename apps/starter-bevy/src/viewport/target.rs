use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy_egui::{EguiContexts, EguiTextureHandle};

use super::{ViewportCamera, ViewportState};

pub fn create_image(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    let size = Extent3d {
        width: size.x.max(1),
        height: size.y.max(1),
        ..default()
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("starter-bevy-viewport"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    images.add(image)
}

pub fn apply_pending_resize(
    contexts: &mut EguiContexts,
    images: &mut Assets<Image>,
    viewport: &mut ViewportState,
    cameras: &mut Query<&mut RenderTarget, With<ViewportCamera>>,
) {
    let Some(requested) = viewport.requested_size.take() else {
        return;
    };
    let requested = requested.clamp(UVec2::splat(1), UVec2::splat(8192));
    if requested == viewport.size {
        return;
    }
    let handle = create_image(images, requested);
    contexts.remove_image(viewport.image.id());
    viewport.texture_id = Some(contexts.add_image(EguiTextureHandle::Strong(handle.clone())));
    viewport.image = handle.clone();
    viewport.size = requested;
    for mut target in cameras {
        *target = RenderTarget::Image(handle.clone().into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_matches_the_requested_size() {
        let mut images = Assets::<Image>::default();
        let handle = create_image(&mut images, UVec2::new(320, 200));
        let image = images.get(&handle).unwrap();
        assert_eq!(image.texture_descriptor.size.width, 320);
        assert_eq!(image.texture_descriptor.size.height, 200);
    }
}
