use egui::{Id, Ui};
use egui_ltreeview::{Action, NodeId, TreeView, TreeViewBuilder, TreeViewState};

pub struct ExplorerTree<IdType: NodeId> {
    state: TreeViewState<IdType>,
}

impl<IdType: NodeId> Default for ExplorerTree<IdType> {
    fn default() -> Self {
        Self {
            state: TreeViewState::default(),
        }
    }
}

impl<IdType: NodeId + Send + Sync + 'static> ExplorerTree<IdType> {
    pub fn selected(&self) -> &[IdType] {
        self.state.selected()
    }

    pub fn set_selected(&mut self, selected: Vec<IdType>) {
        self.state.set_selected(selected);
    }

    /// Default-open directories report `true` until the user toggles them.
    pub fn is_open(&self, id: &IdType) -> bool {
        self.state.is_open(id).unwrap_or(true)
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        id: Id,
        build: impl FnOnce(&mut TreeViewBuilder<'_, IdType>),
    ) -> Vec<Action<IdType>> {
        self.show_with_context(ui, id, build, |_, _| {})
    }

    pub fn show_with_context(
        &mut self,
        ui: &mut Ui,
        id: Id,
        build: impl FnOnce(&mut TreeViewBuilder<'_, IdType>),
        context_menu: impl FnOnce(&mut Ui, &Vec<IdType>),
    ) -> Vec<Action<IdType>> {
        TreeView::new(id)
            .allow_multi_selection(true)
            .fallback_context_menu(context_menu)
            .show_state(ui, &mut self.state, build)
            .1
    }
}
