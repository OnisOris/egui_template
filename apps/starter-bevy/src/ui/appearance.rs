use crate::state::UiState;
use bevy_egui::egui;
use starter_ui::{Theme, style::Density};
pub fn show(ctx: &egui::Context, state: &mut UiState) {
    egui::Window::new("Appearance")
        .open(&mut state.appearance_open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Theme");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.theme, Theme::Dark, "Dark");
                ui.selectable_value(&mut state.theme, Theme::Light, "Light");
            });
            ui.separator();
            ui.add(
                egui::Slider::new(&mut state.appearance.scale, 0.75..=2.)
                    .text("UI Scale")
                    .step_by(0.05),
            );
            ui.weak("1.0 follows your monitor DPI");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.appearance.density, Density::Compact, "Compact");
                ui.selectable_value(
                    &mut state.appearance.density,
                    Density::Comfortable,
                    "Comfortable",
                );
            });
            if ui.button("Reset Appearance").clicked() {
                state.theme = Theme::Dark;
                state.appearance = Default::default();
            }
        });
}
