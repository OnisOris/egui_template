use crate::project::{Project, ProjectAction, ProjectIo};
use bevy_egui::egui::{self, Ui};
use starter_core::{LayerKind, ObjectId, ProjectSelection};
use starter_ui::{Action, ExplorerTree};
mod files;
mod scene;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Project,
    Map,
    Objects,
    Layer(LayerKind),
    Marker(ObjectId),
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
}
impl Node {
    fn selected(s: &ProjectSelection) -> Self {
        match s {
            ProjectSelection::Project => Self::Project,
            ProjectSelection::Manifest => Self::Manifest,
            ProjectSelection::Layer(k) => Self::Layer(*k),
            ProjectSelection::Marker(id) => Self::Marker(*id),
            ProjectSelection::Asset(i) => Self::File(*i),
        }
    }
    fn selection(&self) -> ProjectSelection {
        match self {
            Self::Layer(k) => ProjectSelection::Layer(*k),
            Self::Marker(id) => ProjectSelection::Marker(*id),
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
                rows += 2;
            }
            if state.tree.is_open(&Node::Objects) {
                rows += project.document.objects.len();
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
    if let ProjectSelection::Marker(id) = project.selection {
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Focus").clicked() {
                project.focus_selected = true;
            }
            if ui.small_button("Show / Hide").clicked()
                && let Some(m) = project.document.objects.iter().find(|m| m.id == id)
            {
                let mut m = m.clone();
                m.visible = !m.visible;
                let _ = project.edit_marker(m);
            }
            if ui.small_button("Delete").clicked() {
                project.delete_selected();
            }
        });
        ui.small("Rename and coordinates in Inspector");
    }
    if state.files && ui.button("Show in File Manager").clicked() {
        files::show_manager(project, io);
    }
}
