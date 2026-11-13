use crate::project::{Project, ProjectAction, ProjectIo};
use bevy_egui::egui::{self, Ui};
use starter_core::{LayerKind, ObjectId, ProjectSelection};
use starter_ui::{Action, ExplorerTree};
mod actions;
mod files;
mod scene;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Project,
    Map,
    Objects,
    Layer(LayerKind),
    Marker(ObjectId),
    Model(ObjectId),
    Assets,
    Models,
    Textures,
    Geodata,
    Manifest,
    File(usize),
}
#[derive(Default)]
pub struct ExplorerState {
    pub files: bool,
    tree: ExplorerTree<Node>,
    remove_asset: Option<usize>,
    last_selection: Option<ProjectSelection>,
    generation: u64,
}
impl Node {
    fn selected(s: &ProjectSelection) -> Self {
        match s {
            ProjectSelection::Project => Self::Project,
            ProjectSelection::Manifest => Self::Manifest,
            ProjectSelection::Layer(k) => Self::Layer(*k),
            ProjectSelection::Marker(id) => Self::Marker(*id),
            ProjectSelection::Model(id) => Self::Model(*id),
            ProjectSelection::Asset(i) => Self::File(*i),
        }
    }
    fn selection(&self) -> ProjectSelection {
        match self {
            Self::Layer(k) => ProjectSelection::Layer(*k),
            Self::Marker(id) => ProjectSelection::Marker(*id),
            Self::Model(id) => ProjectSelection::Model(*id),
            Self::File(i) => ProjectSelection::Asset(*i),
            Self::Manifest => ProjectSelection::Manifest,
            _ => ProjectSelection::Project,
        }
    }
}
pub fn show(
    ui: &mut Ui,
    state: &mut ExplorerState,
    project: &mut Project,
    io: &mut ProjectIo,
    real: &crate::map::real_scene::RealScene,
    import: &crate::map::loading::TerrainImport,
) {
    if state.generation != io.generation {
        state.generation = io.generation;
        state.remove_asset = None;
        state.last_selection = None;
    }
    if state.last_selection.as_ref() != Some(&project.selection)
        && matches!(
            project.selection,
            ProjectSelection::Marker(_) | ProjectSelection::Model(_)
        )
    {
        state.files = false;
    }
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.files, true, "Files");
        ui.selectable_value(&mut state.files, false, "Scene");
    });
    if state.files {
        ui.horizontal_wrapped(|ui| {
            for (label, action) in [
                ("New", ProjectAction::New),
                ("Open…", ProjectAction::Open),
                ("Save", ProjectAction::Save),
                ("Save As…", ProjectAction::SaveAs),
                ("Import…", ProjectAction::Import),
                ("Refresh", ProjectAction::Refresh),
            ] {
                if ui.small_button(label).clicked() {
                    io.request(action, project);
                }
            }
        });
    }
    egui::Panel::bottom("project_explorer_actions").show(ui, |ui| {
        actions::show(ui, state, project, io);
    });
    state
        .tree
        .set_selected(vec![Node::selected(&project.selection)]);
    let count = if state.files {
        project.document.assets.len() + 12
    } else {
        let mut rows = 1;
        if state.tree.is_open(&Node::Project) {
            rows += 2;
            if state.tree.is_open(&Node::Map) {
                rows += 4;
            }
            if state.tree.is_open(&Node::Objects) {
                rows += project.document.object_count();
            }
        }
        rows
    };
    let height = (count as f32) * (ui.spacing().interact_size.y + ui.spacing().item_spacing.y);
    let actions = egui::ScrollArea::vertical()
        .id_salt("project_explorer_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(ui.available_width(), height),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                state
                    .tree
                    .show(ui, ui.id().with("project_tree"), |builder| {
                        if state.files {
                            files::build(builder, project, io);
                        } else {
                            scene::build(builder, project, real, import);
                        }
                    })
            })
            .inner
        })
        .inner;
    for action in actions {
        if let Action::SetSelected(nodes) = action
            && let Some(node) = nodes.last()
        {
            project.selection = node.selection();
        }
    }
    state.last_selection = Some(project.selection.clone());
}
