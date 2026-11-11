mod camera;
mod input;
mod picking;
mod render;

use eframe::egui::{Sense, Ui};
use starter_core::{Command, DocumentState};

pub use camera::{Camera, DragTarget, ViewportState};

pub fn show(ui: &mut Ui, document: &DocumentState, viewport: &mut ViewportState) -> Vec<Command> {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let commands = input::handle(ui, &response, rect, document, viewport);
    render::show(
        &painter.with_clip_rect(rect),
        rect,
        document,
        viewport,
        ui.visuals().dark_mode,
    );
    commands
}
