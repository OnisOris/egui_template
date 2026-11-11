use std::ops::{Deref, DerefMut};

use bevy::prelude::*;
use starter_core::DocumentState;
use starter_ui::{ExplorerNodeId, ExplorerTree, InspectorState, TableState, Theme, Workspace};

use crate::ui::gallery::GalleryState;

#[derive(Resource, Default)]
pub struct DocumentResource(pub DocumentState);

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
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum EditorAction {
    New,
    AddObject,
    DeleteSelection,
    FocusSelection,
    ResetView,
    ResetLayout,
    ToggleProjection,
    ToggleTheme,
    SetLoad(usize),
}
