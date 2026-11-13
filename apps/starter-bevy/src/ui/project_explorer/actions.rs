use super::{ExplorerState, files};
use crate::project::{Project, ProjectAction, ProjectIo};
use bevy_egui::egui::{self, Ui};
use starter_core::ProjectSelection;
pub fn show(ui: &mut Ui, state: &mut ExplorerState, project: &mut Project, io: &mut ProjectIo) {
    if let ProjectSelection::Layer(kind) = project.selection {
        let mut visible = project.document.layer(kind).visible;
        if ui.checkbox(&mut visible, "Visible").changed() {
            project.document.layer_mut(kind).visible = visible;
            project.changed();
        }
        ui.small("Source and properties in Inspector");
    }
    if let ProjectSelection::Marker(id) | ProjectSelection::Model(id) = project.selection {
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Focus").clicked() {
                project.focus_selected = true;
            }
            if ui.small_button("Show / Hide").clicked() {
                match project.selection {
                    ProjectSelection::Marker(_) => {
                        if let Some(mut m) = project
                            .document
                            .objects
                            .iter()
                            .find(|m| m.id == id)
                            .cloned()
                        {
                            m.visible = !m.visible;
                            let _ = project.edit_marker(m);
                        }
                    }
                    ProjectSelection::Model(_) => {
                        if let Some(mut m) =
                            project.document.models.iter().find(|m| m.id == id).cloned()
                        {
                            m.visible = !m.visible;
                            let _ = project.edit_model(m);
                        }
                    }
                    _ => {}
                }
            }
            if ui.small_button("Delete").clicked() {
                project.delete_selected();
            }
        });
        ui.small("Rename and coordinates in Inspector");
    }
    if state.files
        && let ProjectSelection::Asset(i) = project.selection
        && let Some(asset) = project.document.assets.get(i)
        && asset.path.to_lowercase().ends_with(".glb")
        && ui
            .add_enabled(
                !io.busy() && io.missing.get(i) != Some(&true),
                egui::Button::new("Place on Map"),
            )
            .clicked()
    {
        project.placing_model = Some(asset.path.clone());
        project.placing_marker = false;
    }
    if state.files
        && let ProjectSelection::Asset(index) = project.selection
    {
        let used = project.document.assets.get(index).is_some_and(|a| {
            project
                .document
                .models
                .iter()
                .any(|m| m.asset_path == a.path)
        });
        if ui
            .add_enabled(!used, egui::Button::new("Remove Asset…"))
            .on_hover_text("Delete the unused resource file and remove its project reference")
            .clicked()
        {
            state.remove_asset = Some(index);
        }
        if used {
            ui.small("Resource is used by model instances");
        }
    }
    if let Some(index) = state.remove_asset {
        let response =
            egui::Modal::new(ui.id().with("remove_asset_confirmation")).show(ui.ctx(), |ui| {
                ui.label("Permanently delete this unused asset file from the project?");
                ui.horizontal(|ui| {
                    if ui.button("Delete asset").clicked() {
                        io.request(ProjectAction::RemoveAsset(index), project);
                        state.remove_asset = None;
                    }
                    if ui.button("Cancel").clicked() {
                        state.remove_asset = None;
                    }
                });
            });
        if response.should_close() {
            state.remove_asset = None;
        }
    }
    if state.files && ui.button("Show in File Manager").clicked() {
        files::show_manager(project, io);
    }
}
