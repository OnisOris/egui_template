//! Contextual actions use the same application commands as menus and shortcuts.
use crate::{project::Project, state::EditorAction};
use bevy_egui::egui::{self, Ui};
pub fn show(ui: &mut Ui, project: &Project, terrain_ready: bool) -> Option<EditorAction> {
    let mut action = None;
    let title = if project.document.map_region.is_none() {
        "Build your geographic scene"
    } else {
        "Your region is ready"
    };
    ui.add_space(16.);
    egui::Frame::group(ui.style())
        .inner_margin(16.)
        .show(ui, |ui| {
            ui.heading(title);
            if project.document.map_region.is_none() {
                ui.label(
                    "Choose an area on the map, then load satellite imagery and 3D geography.",
                );
                if ui.button("Select a Region").clicked() {
                    action = Some(EditorAction::SelectRegion);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("New Project").clicked() {
                        action = Some(EditorAction::New);
                    }
                    if ui.button("Open Project…").clicked() {
                        action = Some(EditorAction::Open);
                    }
                });
            } else if !terrain_ready {
                ui.label("Load the confirmed area to start working in 3D.");
                if ui.button("Load Region in 3D").clicked() {
                    action = Some(EditorAction::LoadRegion);
                }
            } else {
                ui.label("Add survey points or place a model on your map.");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add Marker").clicked() {
                        action = Some(EditorAction::AddObject);
                    }
                    if ui.button("Import Model…").clicked() {
                        action = Some(EditorAction::ImportAsset);
                    }
                });
            }
        });
    action
}
#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[test]
    fn empty_project_routes_select_region_to_the_application() {
        let mut h = Harness::new_ui_state(
            |ui, state: &mut Option<EditorAction>| {
                *state = show(ui, &Project::default(), false);
            },
            None,
        );
        h.run();
        h.get_by_label("Select a Region").click();
        h.run_steps(1);
        assert!(matches!(h.state(), Some(EditorAction::SelectRegion)));
    }
}
