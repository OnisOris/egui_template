use super::{
    RegionRuntime,
    picker::{MapPicker, search_bar},
    region::{ActiveMapRegion, MapRegion},
    tiles::MapTiles,
};
use bevy_egui::egui::{self, Color32, Ui};

/// Returns true when the dock should activate Viewport (after this layout pass).
pub fn show(
    ui: &mut Ui,
    picker: &mut MapPicker,
    active: &mut ActiveMapRegion,
    runtime: &mut RegionRuntime,
) -> bool {
    let mut open = false;
    egui::Panel::top("map_tools").show(ui, |ui| {
        ui.heading("Map · Region picker");
        search_bar(ui, picker);
    });
    let details_height = (ui.available_height() * 0.25).clamp(48.0, 112.0);
    egui::Panel::bottom("map_region")
        .exact_size(details_height)
        .show(ui, |ui| {
            // Keep details usable even in a short docking pane.
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    open = region_details(ui, picker, active, runtime);
                });
        });
    egui::Panel::bottom("map_attribution")
        .min_size(38.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to(
                    "© OpenStreetMap contributors",
                    "https://www.openstreetmap.org/copyright",
                );
                if let Some(cursor) = picker.selection.cursor {
                    ui.monospace(format!("{:.6}°, {:.6}°", cursor.latitude, cursor.longitude));
                }
                if ui.small_button("Retry tiles").clicked() {
                    picker.tiles = None;
                }
            });
            if let Some(tiles) = picker.tiles.as_mut()
                && let Some(status) = tiles.status()
            {
                ui.colored_label(Color32::from_rgb(234, 180, 80), status);
            }
        });
    let rect = ui.available_rect_before_wrap();
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return open;
    }
    let bounds = picker
        .selection
        .preview
        .or_else(|| active.get().map(|(_, r)| r.bounds));
    if picker.view.fit_requested {
        if let Some(bounds) = bounds {
            picker.view.fit(bounds, rect);
        }
        picker.view.fit_requested = false;
    }
    picker
        .tiles
        .get_or_insert_with(|| MapTiles::new(ui.ctx().clone()));
    picker.show_map(ui, active.get().map(|(_, r)| r.bounds));
    open
}

fn region_details(
    ui: &mut Ui,
    picker: &mut MapPicker,
    active: &mut ActiveMapRegion,
    runtime: &mut RegionRuntime,
) -> bool {
    let preview = picker.selection.preview;
    if let Some(b) = preview.or_else(|| active.get().map(|(_, r)| r.bounds)) {
        let (width, height) = b.dimensions();
        let center = b.center();
        ui.horizontal_wrapped(|ui| {
            ui.strong(if preview.is_some() {
                "Draft region"
            } else {
                "Confirmed region"
            });
            ui.monospace(format!(
                "N {:.6}°  S {:.6}°  W {:.6}°  E {:.6}°",
                b.north, b.south, b.west, b.east
            ));
        });
        ui.label(format!(
            "{:.3} × {:.3} km · ≈ {:.3} km² · Center {:.6}°, {:.6}°",
            width / 1000.0,
            height / 1000.0,
            b.area() / 1e6,
            center.latitude,
            center.longitude
        ));
    } else {
        ui.label("Pan and zoom to a place, then use Select Region to draw a rectangle.");
    }
    let mut open = false;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                preview.is_some() || picker.selection.error.is_some(),
                egui::Button::new("Clear Selection"),
            )
            .clicked()
        {
            picker.selection.clear();
        }
        if ui
            .add_enabled(
                preview.is_some() || active.get().is_some(),
                egui::Button::new("Fit to Region"),
            )
            .clicked()
        {
            picker.view.fit_requested = true;
        }
        if ui
            .add_enabled(
                preview.is_some() && picker.selection.drag_start.is_none(),
                egui::Button::new("Confirm Region"),
            )
            .clicked()
            && let Some(bounds) = preview
        {
            match MapRegion::new(bounds, bounds.center()).and_then(|r| active.confirm(r)) {
                Ok(()) => picker.selection.clear(),
                Err(error) => picker.selection.error = Some(error.into()),
            }
        }
        if ui
            .add_enabled(active.get().is_some(), egui::Button::new("Open in 3D"))
            .clicked()
            && let Some((_, region)) = active.get()
        {
            match super::demo::validate_demo(region) {
                Ok(()) => {
                    runtime.open_requested = true;
                    open = true;
                }
                Err(error) => picker.selection.error = Some(error.into()),
            }
        }
    });
    if let Some((revision, _)) = active.get() {
        ui.small(format!("Confirmed revision {revision} · Open in 3D uses this revision. Terrain is synthetic, not real elevations."));
    }
    if let Some(error) = &picker.selection.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    open
}
