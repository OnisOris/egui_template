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
    Map,
}

impl PanelId {
    pub const ALL: [Self; 7] = [
        Self::Explorer,
        Self::Viewport,
        Self::Inspector,
        Self::DataTable,
        Self::Charts,
        Self::Components,
        Self::Map,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Viewport => "Viewport",
            Self::Inspector => "Inspector",
            Self::DataTable => "Data Table",
            Self::Charts => "Charts",
            Self::Components => "Components",
            Self::Map => "Map",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspacePreset {
    MapEditing,
    Scene,
    Analysis,
}
impl WorkspacePreset {
    pub const ALL: [Self; 3] = [Self::MapEditing, Self::Scene, Self::Analysis];
    pub fn title(self) -> &'static str {
        match self {
            Self::MapEditing => "Map Editing",
            Self::Scene => "3D Scene",
            Self::Analysis => "Analysis",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    tree: Tree<PanelId>,
    #[serde(default)]
    preset: Option<WorkspacePreset>,
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
            preset: None,
        }
    }

    pub fn preset(preset: WorkspacePreset) -> Self {
        let mut tiles = Tiles::default();
        let explorer = tiles.insert_pane(PanelId::Explorer);
        let inspector = tiles.insert_pane(PanelId::Inspector);
        let viewport = tiles.insert_pane(PanelId::Viewport);
        let map = tiles.insert_pane(PanelId::Map);
        let center = if preset == WorkspacePreset::Analysis {
            let charts = tiles.insert_pane(PanelId::Charts);
            let table = tiles.insert_pane(PanelId::DataTable);
            let analysis = tiles.insert_tab_tile(vec![charts, table]);
            let view = tiles.insert_tab_tile(vec![viewport, map]);
            tiles.insert_vertical_tile(vec![view, analysis])
        } else {
            tiles.insert_tab_tile(vec![viewport, map])
        };
        let root = tiles.insert_horizontal_tile(vec![explorer, center, inspector]);
        if let Some(Tile::Container(Container::Linear(Linear { shares, .. }))) = tiles.get_mut(root)
        {
            shares.set_share(explorer, 0.17);
            shares.set_share(center, 0.61);
            shares.set_share(inspector, 0.22);
        }
        let mut workspace = Self {
            tree: Tree::new(Id::new("starter_workspace"), root, tiles),
            preset: Some(preset),
        };
        workspace.activate(if preset == WorkspacePreset::MapEditing {
            PanelId::Map
        } else {
            PanelId::Viewport
        });
        workspace
    }
    pub fn selected_preset(&self) -> Option<WorkspacePreset> {
        self.preset
    }

    /// Add an optional application panel without changing the desktop default layout.
    pub fn ensure_panel(&mut self, pane: PanelId) {
        if self.tile_id(pane).is_some() {
            return;
        }
        let id = self.tree.tiles.insert_pane(pane);
        let parent = self.tile_id(PanelId::Viewport).and_then(|viewport| {
            self.tree.tiles.iter().find_map(|(id, tile)| match tile {
                Tile::Container(Container::Tabs(tabs)) if tabs.children.contains(&viewport) => {
                    Some(*id)
                }
                _ => None,
            })
        });
        if let Some(Tile::Container(Container::Tabs(tabs))) =
            parent.and_then(|id| self.tree.tiles.get_mut(id))
        {
            tabs.children.push(id);
        } else if let Some(root) = self.tree.root {
            self.tree.root = Some(self.tree.tiles.insert_tab_tile(vec![root, id]));
        } else {
            self.tree.root = Some(id);
        }
    }

    pub fn activate(&mut self, pane: PanelId) {
        self.set_visible(pane, true);
        self.tree
            .make_active(|_, tile| matches!(tile, Tile::Pane(candidate) if *candidate == pane));
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
    fn gap_width(&self, _style: &egui::Style) -> f32 {
        // Leave a usable pointer target between interactive map/viewport surfaces.
        6.0
    }

    fn tab_bar_height(&self, style: &egui::Style) -> f32 {
        style.spacing.interact_size.y + 4.0
    }

    fn tab_title_for_pane(&mut self, pane: &PanelId) -> WidgetText {
        pane.title().into()
    }

    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut PanelId) -> UiResponse {
        // Panes are surfaces: without this the window clear color shows through.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        let rect = ui.max_rect().shrink(10.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            (self.draw)(ui, *pane);
        });
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
