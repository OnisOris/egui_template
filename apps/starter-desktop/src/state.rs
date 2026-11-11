use serde::{Deserialize, Serialize};
use starter_core::{ObjectId, TEST_LOADS};
use starter_ui::{Theme, Workspace};

use crate::features::{
    analytics::AnalyticsState, explorer::ExplorerNodeId, gallery::GalleryState,
    inspector::InspectorState, scene::ViewportState, table::TableState,
};

pub struct UiState {
    pub workspace: Workspace,
    pub theme: Theme,
    pub continuous_repaint: bool,
    pub load_size: usize,
    pub viewport: ViewportState,
    pub explorer: starter_ui::ExplorerTree<ExplorerNodeId>,
    pub inspector: InspectorState,
    pub table: TableState,
    pub analytics: AnalyticsState,
    pub gallery: GalleryState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            theme: Theme::Dark,
            continuous_repaint: false,
            load_size: 100,
            viewport: ViewportState::default(),
            explorer: starter_ui::ExplorerTree::default(),
            inspector: InspectorState::default(),
            table: TableState::default(),
            analytics: AnalyticsState::default(),
            gallery: GalleryState::default(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Preferences {
    pub workspace: Workspace,
    pub theme: Theme,
    pub continuous_repaint: bool,
    pub load_size: usize,
    pub table_filter: String,
    pub table_sort: TableSort,
    pub table_ascending: bool,
    pub selected_objects: Vec<ObjectId>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum TableSort {
    #[default]
    Id,
    Name,
    X,
    Y,
    Size,
}

impl UiState {
    pub fn preferences(&self, selection: &[ObjectId]) -> Preferences {
        Preferences {
            workspace: self.workspace.clone(),
            theme: self.theme,
            continuous_repaint: self.continuous_repaint,
            load_size: self.load_size,
            table_filter: self.table.filter.clone(),
            table_sort: self.table.sort,
            table_ascending: self.table.ascending,
            selected_objects: selection.to_vec(),
        }
    }

    pub fn restore(&mut self, preferences: Preferences) -> Vec<ObjectId> {
        self.workspace = preferences.workspace;
        self.theme = preferences.theme;
        self.continuous_repaint = preferences.continuous_repaint;
        self.load_size = if TEST_LOADS.contains(&preferences.load_size) {
            preferences.load_size
        } else {
            TEST_LOADS[0]
        };
        self.table.filter = preferences.table_filter;
        self.table.sort = preferences.table_sort;
        self.table.ascending = preferences.table_ascending;
        preferences.selected_objects
    }
}

#[cfg(test)]
mod tests {
    use starter_core::ObjectId;
    use starter_ui::PanelId;

    use super::*;

    #[test]
    fn preferences_restore_layout_theme_and_table_controls() {
        let mut workspace = Workspace::default();
        workspace.set_visible(PanelId::Inspector, false);
        let mut table = TableState::default();
        table.filter = "route".to_owned();
        table.sort = TableSort::Name;
        let state = UiState {
            workspace,
            theme: Theme::Light,
            continuous_repaint: true,
            table,
            ..Default::default()
        };
        let preferences = state.preferences(&[ObjectId(7)]);
        let encoded = serde_json::to_string(&preferences).unwrap();
        let restored_preferences: Preferences = serde_json::from_str(&encoded).unwrap();

        let mut restored = UiState::default();
        let selection = restored.restore(restored_preferences);
        assert_eq!(restored.theme, Theme::Light);
        assert!(restored.continuous_repaint);
        assert_eq!(restored.table.filter, "route");
        assert_eq!(restored.table.sort, TableSort::Name);
        assert!(!restored.workspace.is_visible(PanelId::Inspector));
        assert_eq!(selection, [ObjectId(7)]);
    }
}
