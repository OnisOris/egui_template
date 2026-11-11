use egui::{Id, Ui, WidgetText};
use egui_tiles::{Behavior, Container, Linear, Tile, TileId, Tiles, Tree, UiResponse};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum PanelId {
    Explorer,
    Viewport,
    Inspector,
    DataTable,
    Charts,
    Components,
}

impl PanelId {
    pub const ALL: [Self; 6] = [
        Self::Explorer,
        Self::Viewport,
        Self::Inspector,
        Self::DataTable,
        Self::Charts,
        Self::Components,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Viewport => "Viewport",
            Self::Inspector => "Inspector",
            Self::DataTable => "Data Table",
            Self::Charts => "Charts",
            Self::Components => "Components",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    tree: Tree<PanelId>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::standard()
    }
}

impl Workspace {
    pub fn standard() -> Self {
        let mut tiles = Tiles::default();
        let explorer = tiles.insert_pane(PanelId::Explorer);
        let viewport = tiles.insert_pane(PanelId::Viewport);
        let table = tiles.insert_pane(PanelId::DataTable);
        let charts = tiles.insert_pane(PanelId::Charts);
        let components = tiles.insert_pane(PanelId::Components);
        let inspector = tiles.insert_pane(PanelId::Inspector);
        let center = tiles.insert_tab_tile(vec![viewport, table, charts, components]);
        let root = tiles.insert_horizontal_tile(vec![explorer, center, inspector]);
        if let Some(Tile::Container(Container::Linear(Linear { shares, .. }))) = tiles.get_mut(root)
        {
            shares.set_share(explorer, 0.22);
            shares.set_share(center, 0.58);
            shares.set_share(inspector, 0.20);
        }
        Self {
            tree: Tree::new(Id::unique("starter_workspace"), root, tiles),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::standard();
    }

    pub fn is_visible(&self, pane: PanelId) -> bool {
        self.tile_id(pane)
            .is_some_and(|id| self.tree.is_visible(id))
    }

    pub fn set_visible(&mut self, pane: PanelId, visible: bool) {
        if let Some(id) = self.tile_id(pane) {
            self.tree.set_visible(id, visible);
        }
    }

    pub fn show(&mut self, ui: &mut Ui, mut draw: impl FnMut(&mut Ui, PanelId)) {
        self.tree.ui(&mut WorkspaceBehavior { draw: &mut draw }, ui);
    }

    fn tile_id(&self, pane: PanelId) -> Option<TileId> {
        self.tree.tiles.iter().find_map(|(id, tile)| match tile {
            Tile::Pane(candidate) if *candidate == pane => Some(*id),
            _ => None,
        })
    }
}

struct WorkspaceBehavior<'a, Draw> {
    draw: &'a mut Draw,
}

impl<Draw> Behavior<PanelId> for WorkspaceBehavior<'_, Draw>
where
    Draw: FnMut(&mut Ui, PanelId),
{
    fn tab_title_for_pane(&mut self, pane: &PanelId) -> WidgetText {
        pane.title().into()
    }

    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut PanelId) -> UiResponse {
        (self.draw)(ui, *pane);
        UiResponse::None
    }

    fn is_tab_closable(&self, _tiles: &Tiles<PanelId>, _tile_id: TileId) -> bool {
        true
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<PanelId>, tile_id: TileId) -> bool {
        tiles.set_visible(tile_id, false);
        false
    }
}
