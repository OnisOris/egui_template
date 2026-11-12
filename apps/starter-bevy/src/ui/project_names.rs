//! Transient name drafts; committed object data lives in ProjectDocument.
use bevy_egui::egui::{self, Ui};
use starter_core::{ProjectSelection, ProjectState};
use std::collections::HashMap;
#[derive(Default)]
pub struct InspectorState {
    selection: Option<ProjectSelection>,
    generation: u64,
    drafts: HashMap<ProjectSelection, Draft>,
}
struct Draft {
    source: String,
    value: String,
    error: bool,
}
fn valid(name: &str) -> bool {
    !name.trim().is_empty() && name.trim().len() <= starter_core::MAX_NAME_LENGTH
}
impl InspectorState {
    pub(super) fn sync(&mut self, project: &mut ProjectState, generation: u64) {
        if self.generation != generation {
            self.drafts.clear();
            self.selection = None;
            self.generation = generation;
        }
        if self
            .selection
            .as_ref()
            .is_some_and(|s| s != &project.selection)
            && let Some(previous) = self.selection.take()
            && let Some(draft) = self.drafts.get(&previous)
            && valid(&draft.value)
        {
            let name = draft.value.trim().to_owned();
            if name != draft.source {
                match previous {
                    ProjectSelection::Project => {
                        project.document.name = name;
                        project.changed();
                    }
                    ProjectSelection::Marker(id) => {
                        if let Some(mut m) = project
                            .document
                            .objects
                            .iter()
                            .find(|m| m.id == id)
                            .cloned()
                        {
                            m.name = name;
                            let _ = project.edit_marker(m);
                        }
                    }
                    ProjectSelection::Model(id) => {
                        if let Some(mut m) =
                            project.document.models.iter().find(|m| m.id == id).cloned()
                        {
                            m.name = name;
                            let _ = project.edit_model(m);
                        }
                    }
                    _ => {}
                }
            }
            self.drafts.remove(&previous);
        }
    }
    pub(super) fn name(
        &mut self,
        ui: &mut Ui,
        selection: &ProjectSelection,
        name: &str,
    ) -> Option<String> {
        self.selection = Some(selection.clone());
        let draft = self
            .drafts
            .entry(selection.clone())
            .or_insert_with(|| Draft {
                source: name.into(),
                value: name.into(),
                error: false,
            });
        if draft.source != name {
            draft.source = name.into();
            draft.value = name.into();
            draft.error = false;
        }
        let response = ui.text_edit_singleline(&mut draft.value);
        if response.lost_focus()
            || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            draft.error = !valid(&draft.value);
            if !draft.error {
                return Some(draft.value.trim().to_owned());
            }
        }
        if draft.error {
            ui.colored_label(
                ui.visuals().error_fg_color,
                "Name must contain 1–256 characters.",
            );
        }
        None
    }
}
