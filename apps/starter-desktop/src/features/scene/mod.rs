mod camera;
mod input;
mod picking;
mod render;

use eframe::egui::{Sense, Ui};
use starter_core::DocumentState;

pub use camera::{Camera, DragTarget, ViewportState};

pub fn show(ui: &mut Ui, document: &mut DocumentState, viewport: &mut ViewportState) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    viewport.camera.preserve_center(rect.center());
    input::handle(ui, &response, rect, document, viewport);
    render::show(
        &painter.with_clip_rect(rect),
        rect,
        document,
        viewport,
        ui.visuals().dark_mode,
    );
}
