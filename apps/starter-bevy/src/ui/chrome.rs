use bevy_egui::egui::{self, Ui};
use starter_core::{DocumentState, TEST_LOADS};
use starter_ui::{PanelId, Workspace};

use crate::{metrics::EditorMetrics, state::EditorAction, viewport::ProjectionMode};

pub fn show(
    ui: &mut Ui,
    workspace: &mut Workspace,
    document: &DocumentState,
    metrics: &EditorMetrics,
    projection: ProjectionMode,
    sensitivity: &mut f32,
    notice: Option<&str>,
) -> Vec<EditorAction> {
    let mut actions = menu(ui, workspace, projection, sensitivity);
    actions.extend(toolbar(ui, document.document().objects().len(), projection));
    status(ui, document, metrics, projection, notice);
    actions
}

fn menu(
    ui: &mut Ui,
    workspace: &mut Workspace,
    projection: ProjectionMode,
    sensitivity: &mut f32,
) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    egui::Panel::top("bevy_menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New document").clicked() {
                    actions.push(EditorAction::New);
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                for (label, action) in [
                    ("Add object", EditorAction::AddObject),
                    ("Delete selection", EditorAction::DeleteSelection),
                ] {
                    if ui.button(label).clicked() {
                        actions.push(action);
                        ui.close();
                    }
                }
            });
            ui.menu_button("View", |ui| {
                for (label, action) in [
                    ("Focus selection", EditorAction::FocusSelection),
                    ("Reset view", EditorAction::ResetView),
                    ("Reset layout", EditorAction::ResetLayout),
                ] {
                    if ui.button(label).clicked() {
                        actions.push(action);
                        ui.close();
                    }
                }
                if ui
                    .button(format!("Toggle {} projection", projection.label()))
                    .clicked()
                {
                    actions.push(EditorAction::ToggleProjection);
                    ui.close();
                }
                ui.add(egui::Slider::new(sensitivity, 0.1..=3.0).text("Camera sensitivity"));
                ui.separator();
                for panel in PanelId::ALL {
                    let mut visible = workspace.is_visible(panel);
                    if ui.checkbox(&mut visible, panel.title()).changed() {
                        workspace.set_visible(panel, visible);
                    }
                }
            });
            ui.menu_button("Settings", |ui| {
                if ui.button("Toggle theme").clicked() {
                    actions.push(EditorAction::ToggleTheme);
                    ui.close();
                }
            });
        });
    });
    actions
}

fn toolbar(ui: &mut Ui, objects: usize, projection: ProjectionMode) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    egui::Panel::top("bevy_toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                actions.push(EditorAction::New);
            }
            if ui.button("+ Object").clicked() {
                actions.push(EditorAction::AddObject);
            }
            if ui.button("Delete").clicked() {
                actions.push(EditorAction::DeleteSelection);
            }
            ui.separator();
            if ui.button("Focus").clicked() {
                actions.push(EditorAction::FocusSelection);
            }
            if ui.button("Reset view").clicked() {
                actions.push(EditorAction::ResetView);
            }
            if ui
                .button(format!("Projection: {}", projection.label()))
                .clicked()
            {
                actions.push(EditorAction::ToggleProjection);
            }
            ui.separator();
            egui::ComboBox::from_id_salt("bevy_scene_load")
                .selected_text(format!("{objects} objects"))
                .show_ui(ui, |ui| {
                    for size in TEST_LOADS {
                        if ui
                            .selectable_label(objects == size, size.to_string())
                            .clicked()
                        {
                            actions.push(EditorAction::SetLoad(size));
                        }
                    }
                });
        });
    });
    actions
}

fn status(
    ui: &mut Ui,
    document: &DocumentState,
    metrics: &EditorMetrics,
    projection: ProjectionMode,
    notice: Option<&str>,
) {
    egui::Panel::bottom("bevy_status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!(
                "FPS {:.1} · {:.2} ms",
                metrics.fps, metrics.frame_ms
            ));
            ui.separator();
            ui.label(format!(
                "UI {:.2} ms · sync {:.2} ms · build {:.2} ms",
                metrics.ui_ms, metrics.sync_ms, metrics.scene_build_ms
            ));
            ui.separator();
            ui.label(format!(
                "Objects {} · entities {}",
                document.document().objects().len(),
                metrics.projected_entities
            ));
            ui.separator();
            ui.label(format!("Selected {}", document.selection().len()));
            ui.separator();
            ui.label(projection.label());
            ui.weak("GPU frame time not exposed");
            if let Some(notice) = notice {
                ui.separator();
                ui.label(notice);
            }
        });
    });
}
