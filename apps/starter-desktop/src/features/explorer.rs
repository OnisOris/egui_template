use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use starter_core::{Command, DocumentState, GroupId, ObjectId};
use starter_ui::{Action, ExplorerTree};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ExplorerNodeId {
    Scene,
    Group(GroupId),
    Object(ObjectId),
    Data,
    Resources,
}

pub fn show(ui: &mut Ui, state: &mut DocumentState, tree: &mut ExplorerTree<ExplorerNodeId>) {
    ui.heading("Explorer");
    let selected = state.selection();
    let view_selection = tree.selected();
    if view_selection.len() != selected.len()
        || view_selection
            .iter()
            .zip(selected)
            .any(|(node, id)| *node != ExplorerNodeId::Object(*id))
    {
        tree.set_selected(
            selected
                .iter()
                .copied()
                .map(ExplorerNodeId::Object)
                .collect(),
        );
    }
    let mut context_command = None;
    let actions = tree.show_with_context(
        ui,
        ui.scope_id().with("scene_explorer"),
        |builder| {
            if builder.dir(ExplorerNodeId::Scene, "Scene") {
                for (group, name) in [
                    (GroupId::A, "Group A"),
                    (GroupId::B, "Group B"),
                    (GroupId::C, "Group C"),
                ] {
                    if builder.dir(ExplorerNodeId::Group(group), name) {
                        for object in state
                            .document()
                            .objects()
                            .iter()
                            .filter(|o| o.group() == group)
                        {
                            builder.leaf(ExplorerNodeId::Object(object.id()), object.name());
                        }
                    }
                    builder.close_dir();
                }
            }
            builder.close_dir();
            builder.leaf(ExplorerNodeId::Data, "Data");
            builder.leaf(ExplorerNodeId::Resources, "Resources");
        },
        |ui, selected| {
            if ui.button("Add object to Group A").clicked() {
                context_command = Some(Command::AddObject(GroupId::A));
                ui.close();
            }
            if ui
                .button(format!("Delete {} selected", selected.len()))
                .clicked()
            {
                context_command = Some(Command::DeleteSelection);
                ui.close();
            }
        },
    );
    for action in actions {
        if let Action::SetSelected(nodes) = action {
            let mut ids = BTreeSet::new();
            for node in nodes {
                match node {
                    ExplorerNodeId::Object(id) => {
                        ids.insert(id);
                    }
                    ExplorerNodeId::Group(group) => {
                        ids.extend(
                            state
                                .document()
                                .objects()
                                .iter()
                                .filter(|object| object.group() == group)
                                .map(|object| object.id()),
                        );
                    }
                    _ => {}
                }
            }
            state.dispatch(Command::Select(ids)).ok();
        }
    }
    if let Some(command) = context_command {
        state.dispatch(command).ok();
    }
}
use std::collections::BTreeSet;
