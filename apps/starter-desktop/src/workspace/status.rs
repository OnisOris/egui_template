use eframe::egui::{self, Ui};
use starter_core::DocumentState;

use crate::metrics::MetricsState;

pub fn show(
    ui: &mut Ui,
    document: &DocumentState,
    metrics: &MetricsState,
    zoom: f32,
    continuous: bool,
    notice: Option<&str>,
) {
    egui::containers::panel::Panel::bottom("status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("UI callbacks: {:.1}/s", metrics.callback_hz));
            ui.separator();
            ui.label(format!("Callback CPU: {:.2} ms", metrics.callback_cpu_ms));
            ui.separator();
            ui.label(format!("Objects: {}", document.document().objects().len()));
            ui.separator();
            ui.label(format!("Zoom: {zoom:.2}×"));
            ui.separator();
            ui.label(if continuous {
                "Continuous repaint"
            } else {
                "Event-driven"
            });
            ui.weak("Callback timing only · not GPU FPS");
            if let Some(notice) = notice {
                ui.separator();
                ui.label(notice);
            }
        });
    });
}
