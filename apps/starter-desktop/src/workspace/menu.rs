use eframe::egui::{self, Ui};
use starter_ui::{PanelId, Workspace};

use super::AppAction;

pub fn show(ui: &mut Ui, workspace: &mut Workspace) -> Vec<AppAction> {
    let mut actions = Vec::new();
    egui::containers::panel::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                for (label, action) in [
                    ("New document", AppAction::New),
                    ("Open…", AppAction::Open),
                    ("Save as…", AppAction::Save),
                ] {
                    if ui.button(label).clicked() {
                        actions.push(action);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
                for (label, action) in [
                    ("Add object", AppAction::AddObject),
                    ("Delete selection", AppAction::DeleteSelection),
                ] {
                    if ui.button(label).clicked() {
                        actions.push(action);
                        ui.close();
                    }
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Reset layout").clicked() {
                    actions.push(AppAction::ResetLayout);
                    ui.close();
                }
                ui.separator();
                for panel in PanelId::ALL
                    .into_iter()
                    .filter(|panel| *panel != PanelId::Map)
                {
                    let mut visible = workspace.is_visible(panel);
                    if ui.checkbox(&mut visible, panel.title()).changed() {
                        workspace.set_visible(panel, visible);
                    }
                }
            });
            ui.menu_button("Settings", |ui| {
                if ui.button("Toggle theme").clicked() {
                    actions.push(AppAction::ToggleTheme);
                    ui.close();
                }
                if ui.button("Reset settings").clicked() {
                    actions.push(AppAction::ResetSettings);
                    ui.close();
                }
            });
        });
    });
    actions
}
