use super::ProjectIo;
use bevy_egui::egui;
use starter_core::ProjectState;

pub fn unsaved(ctx: &egui::Context, io: &mut ProjectIo, project: &mut ProjectState) {
    if io.prompt.is_some() {
        egui::Modal::new("unsaved_project".into()).show(ctx, |ui| {
            ui.heading("Unsaved project");
            ui.label("Save changes before continuing?");
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    io.save_before(project);
                }
                if ui.button("Discard").clicked() {
                    io.discard(project);
                }
                if ui.button("Cancel").clicked() {
                    io.prompt = None;
                }
            });
        });
    }
    if let Some(error) = &io.error {
        let error = error.clone();
        egui::Modal::new("project_error".into()).show(ctx, |ui| {
            ui.heading("Project error");
            ui.label(error);
            if ui.button("Close").clicked() {
                io.error = None;
            }
        });
    }
}
