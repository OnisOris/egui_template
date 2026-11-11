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
            tree: Tree::new(Id::new("starter_workspace"), root, tiles),
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
        let mut hidden = Vec::new();
        self.tree.ui(
            &mut WorkspaceBehavior {
                draw: &mut draw,
                hidden: &mut hidden,
            },
            ui,
        );
        // Hide after the layout pass: hiding the last tab of a nested container mid-frame
        // leaves egui_tiles with a stale visible-child list and panics in debug builds.
        for tile_id in hidden {
            self.tree.tiles.set_visible(tile_id, false);
        }
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
    hidden: &'a mut Vec<TileId>,
}

impl<Draw> Behavior<PanelId> for WorkspaceBehavior<'_, Draw>
where
    Draw: FnMut(&mut Ui, PanelId),
{
    fn tab_title_for_pane(&mut self, pane: &PanelId) -> WidgetText {
        pane.title().into()
    }

    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut PanelId) -> UiResponse {
        // Panes are surfaces: without this the window clear color shows through.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        (self.draw)(ui, *pane);
        UiResponse::None
    }

    fn is_tab_closable(&self, _tiles: &Tiles<PanelId>, _tile_id: TileId) -> bool {
        true
    }

    fn on_tab_close(&mut self, _tiles: &mut Tiles<PanelId>, tile_id: TileId) -> bool {
        self.hidden.push(tile_id);
        false
    }
}
