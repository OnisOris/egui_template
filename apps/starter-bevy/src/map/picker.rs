use super::{region::GeoBounds, selection::RegionSelectionState, tiles::MapTiles};
use bevy_egui::egui::{Color32, DragPanButtons, Rect, Ui, Vec2};
use walkers::{Map, MapMemory, Position, Projector, Tiles, lon_lat};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Tool {
    #[default]
    Pan,
    Select,
}

pub struct MapViewState {
    pub memory: MapMemory,
    pub tool: Tool,
    pub search: String,
    pub fit_requested: bool,
}

impl Default for MapViewState {
    fn default() -> Self {
        let mut memory = MapMemory::default();
        let _ = memory.set_zoom(11.0);
        Self {
            memory,
            tool: Tool::Pan,
            search: "55.751244, 37.618423".into(),
            fit_requested: false,
        }
    }
}

#[derive(Default)]
pub struct MapPicker {
    pub view: MapViewState,
    pub selection: RegionSelectionState,
    pub tiles: Option<MapTiles>,
}

pub fn initial_center() -> Position {
    lon_lat(37.618423, 55.751244)
}

impl MapPicker {
    /// `tiles = None` is also useful for offline input tests without HTTP traffic.
    pub fn show_map(&mut self, ui: &mut Ui, confirmed: Option<GeoBounds>) {
        self.show_overlay(ui, confirmed, None);
    }
    pub fn show_project_map(
        &mut self,
        ui: &mut Ui,
        confirmed: Option<GeoBounds>,
        project: &mut starter_core::ProjectState,
    ) {
        self.show_overlay(ui, confirmed, Some(project));
    }
    fn show_overlay(
        &mut self,
        ui: &mut Ui,
        confirmed: Option<GeoBounds>,
        project: Option<&mut starter_core::ProjectState>,
    ) {
        let tiles = self.tiles.as_mut().map(|tiles| {
            tiles.begin_frame();
            tiles as &mut dyn Tiles
        });
        let selecting =
            self.view.tool == Tool::Select && project.as_ref().is_none_or(|p| !p.placing_marker);
        let dragging = self.selection.drag_start.is_some();
        Map::new(tiles, &mut self.view.memory, initial_center())
            .zoom_with_ctrl(false)
            .zoom_gesture(!dragging)
            .panning(!selecting)
            .drag_pan_buttons(if selecting {
                DragPanButtons::empty()
            } else {
                DragPanButtons::PRIMARY
            })
            .show(ui, |ui, response, projector, _| {
                if let Some(bounds) = confirmed {
                    super::selection::paint_bounds(
                        ui,
                        projector,
                        bounds,
                        Color32::from_rgb(85, 220, 150),
                    );
                }
                self.selection.overlay(ui, response, projector, selecting);
                if let Some(project) = project {
                    super::marker_overlay::show(ui, response, projector, project, selecting);
                }
            });
    }
}

impl MapViewState {
    pub fn search(&mut self) -> Result<(), &'static str> {
        let (latitude, longitude) = self
            .search
            .split_once(',')
            .ok_or("Enter latitude, longitude, for example 55.751244, 37.618423.")?;
        let origin = super::region::GeoOrigin {
            latitude: latitude.trim().parse().map_err(|_| "Invalid latitude.")?,
            longitude: longitude.trim().parse().map_err(|_| "Invalid longitude.")?,
        };
        origin.validate()?;
        self.memory
            .center_at(lon_lat(origin.longitude, origin.latitude));
        Ok(())
    }

    pub fn fit(&mut self, bounds: GeoBounds, rect: Rect) {
        let center = bounds.center();
        let center = lon_lat(center.longitude, center.latitude);
        self.memory.center_at(center);
        // Ask Walkers to project at a known zoom; only fit pixel extents, no projection math.
        let _ = self.memory.set_zoom(10.0);
        let projector = Projector::new(rect, &self.memory, center);
        let a = projector.project(lon_lat(bounds.west, bounds.north));
        let b = projector.project(lon_lat(bounds.east, bounds.south));
        let size = (b - a).abs().max(Vec2::splat(0.001));
        let scale =
            ((rect.width() - 40.0).max(1.0) / size.x).min((rect.height() - 40.0).max(1.0) / size.y);
        let _ = self
            .memory
            .set_zoom((10.0 + f64::from(scale).log2()).clamp(0.0, 19.0));
        self.fit_requested = false;
    }
}

pub fn search_bar(ui: &mut Ui, picker: &mut MapPicker) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Latitude, longitude");
        let response = ui.add(
            bevy_egui::egui::TextEdit::singleline(&mut picker.view.search).desired_width(210.0),
        );
        if ui.button("Go").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(bevy_egui::egui::Key::Enter)))
        {
            picker.selection.drag_start = None;
            picker.selection.error = picker.view.search().err().map(String::from);
        }
        for (tool, title) in [(Tool::Pan, "Pan"), (Tool::Select, "Select Region")] {
            if ui
                .selectable_value(&mut picker.view.tool, tool, title)
                .changed()
            {
                picker.selection.drag_start = None;
            }
        }
    });
}
