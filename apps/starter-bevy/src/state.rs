use std::ops::{Deref, DerefMut};

use bevy::prelude::*;
use starter_core::DocumentState;
use starter_ui::{ExplorerNodeId, ExplorerTree, InspectorState, TableState, Theme, Workspace};

use crate::ui::gallery::GalleryState;

#[derive(Resource, Default)]
pub struct DocumentResource(pub DocumentState);

impl DocumentResource {
    pub fn empty() -> Self {
        Self(DocumentState::empty())
    }
}

impl Deref for DocumentResource {
    type Target = DocumentState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DocumentResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource)]
pub struct UiState {
    pub workspace: Workspace,
    pub theme: Theme,
    pub explorer: ExplorerTree<ExplorerNodeId>,
    pub inspector: InspectorState,
    pub table: TableState,
    pub gallery: GalleryState,
    pub notice: Option<String>,
    pub project_explorer: crate::ui::project_explorer::ExplorerState,
    pub marker_table: crate::ui::project_table::TableState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            workspace: {
                let mut workspace = crate::layout::load().unwrap_or_else(|| {
                    let mut workspace = Workspace::default();
                    workspace.ensure_panel(starter_ui::PanelId::Map);
                    workspace.activate(starter_ui::PanelId::Map);
                    workspace
                });
                workspace.ensure_panel(starter_ui::PanelId::Map);
                workspace
            },
            theme: Theme::Dark,
            explorer: ExplorerTree::default(),
            inspector: InspectorState::default(),
            table: TableState::default(),
            gallery: GalleryState::default(),
            notice: None,
            project_explorer: Default::default(),
            marker_table: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum EditorAction {
    New,
    Open,
    Save,
    SaveAs,
    ImportAsset,
    Exit,
    ExitDemo,
    AddObject,
    DeleteSelection,
    FocusSelection,
    ResetView,
    ResetLayout,
    ToggleProjection,
    ToggleTheme,
    SetLoad(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_workspace_is_empty_and_legacy_desktop_default_is_unchanged() {
        assert!(DocumentResource::empty().document().objects().is_empty());
        assert_eq!(DocumentState::default().document().objects().len(), 100);
        let mut benchmark = DocumentResource::empty();
        let project = starter_core::ProjectState::default();
        benchmark
            .dispatch(starter_core::Command::LoadTestScene(10_000))
            .unwrap();
        assert_eq!(benchmark.document().objects().len(), 10_000);
        assert!(project.document.objects.is_empty());
    }
}
