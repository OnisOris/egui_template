use eframe::egui::{self, Ui};
use starter_core::TEST_LOADS;

use super::AppAction;

pub fn show(ui: &mut Ui, load_size: usize, continuous: bool) -> Vec<AppAction> {
    let mut actions = Vec::new();
    egui::containers::panel::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                actions.push(AppAction::New);
            }
            if ui.button("Open").clicked() {
                actions.push(AppAction::Open);
            }
            if ui.button("Save").clicked() {
                actions.push(AppAction::Save);
            }
            ui.separator();
            if ui.button("+ Object").clicked() {
                actions.push(AppAction::AddObject);
            }
            if ui.button("Delete").clicked() {
                actions.push(AppAction::DeleteSelection);
            }
            ui.separator();
            if ui.button("Reset view").clicked() {
                actions.push(AppAction::ResetView);
            }
            if ui.button("Reset layout").clicked() {
                actions.push(AppAction::ResetLayout);
            }
            ui.separator();
            egui::ComboBox::from_id_salt("scene_load")
                .selected_text(format!("{load_size} objects"))
                .show_ui(ui, |ui| {
                    for size in TEST_LOADS {
                        if ui
                            .selectable_label(size == load_size, size.to_string())
                            .clicked()
                        {
                            actions.push(AppAction::SetLoad(size));
                        }
                    }
                });
            let mut repaint = continuous;
            if ui.checkbox(&mut repaint, "Continuous repaint").changed() {
                actions.push(AppAction::ToggleRepaint);
            }
        });
    });
    actions
}
