use eframe::egui::{Pos2, Rect};
use starter_core::{Document, ObjectId};

use super::Camera;

pub fn object_at(
    pointer: Pos2,
    rect: Rect,
    document: &Document,
    camera: &Camera,
) -> Option<ObjectId> {
    document.objects().iter().rev().find_map(|object| {
        if !object.visible() {
            return None;
        }
        let center = camera.world_to_screen(object.position(), rect.center());
        let radius = (object.size() * camera.zoom()).max(3.0) + 5.0;
        (pointer.distance_sq(center) <= radius * radius).then_some(object.id())
    })
}
