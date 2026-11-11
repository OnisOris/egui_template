use crate::state::DocumentResource;
use bevy::prelude::*;

use super::objects::{bounds, object_center};

const GRID_STEP: f32 = 32.0;

pub fn draw_grid(mut gizmos: Gizmos, document: Res<DocumentResource>) {
    let extent = (bounds(&document).radius * 1.25)
        .clamp(128.0, 4096.0)
        .round();
    let lines = (extent / GRID_STEP) as i32;
    for index in -lines..=lines {
        let offset = index as f32 * GRID_STEP;
        let color = match index {
            0 => Color::srgba(0.55, 0.32, 0.25, 1.0),
            _ => Color::srgba(0.16, 0.19, 0.24, 1.0),
        };
        gizmos.line(
            Vec3::new(offset, 0.0, -extent),
            Vec3::new(offset, 0.0, extent),
            color,
        );
        let color = match index {
            0 => Color::srgba(0.25, 0.35, 0.55, 1.0),
            _ => Color::srgba(0.16, 0.19, 0.24, 1.0),
        };
        gizmos.line(
            Vec3::new(-extent, 0.0, offset),
            Vec3::new(extent, 0.0, offset),
            color,
        );
    }
}

pub fn draw_selection(mut gizmos: Gizmos, document: Res<DocumentResource>) {
    let color = Color::srgb(1.0, 0.82, 0.2);
    for id in document.selection() {
        let Some(object) = document.object(*id) else {
            continue;
        };
        gizmos.cube(
            Transform::from_translation(object_center(object))
                .with_scale(Vec3::splat(object.size() * 1.12)),
            color,
        );
    }
}
