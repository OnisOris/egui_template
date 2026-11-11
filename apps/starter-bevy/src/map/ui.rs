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
    import: &mut super::loading::TerrainImport,
    scene: &super::real_scene::RealScene,
) -> bool {
    let mut open = false;
    egui::Panel::top("map_tools").show(ui, |ui| {
        ui.heading("Map · Region picker");
        search_bar(ui, picker);
    });
    egui::ScrollArea::vertical()
        .id_salt("map_region_controls")
        .max_height(260.0)
        .show(ui, |ui| {
            open = region_details(ui, picker, active, runtime, import, scene);
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
    import: &mut super::loading::TerrainImport,
    scene: &super::real_scene::RealScene,
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
    let mut return_open = false;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                preview.is_some() && picker.selection.drag_start.is_none(),
                egui::Button::new("Confirm Region")
                    .min_size(egui::vec2(140.0, 30.0))
                    .fill(ui.visuals().selection.bg_fill),
            )
            .clicked()
            && let Some(bounds) = preview
        {
            match MapRegion::new(
                bounds,
                picker.selection.origin.unwrap_or_else(|| bounds.center()),
            )
            .and_then(|r| active.confirm(r))
            {
                Ok(()) => picker.selection.clear(),
                Err(error) => picker.selection.error = Some(error.into()),
            }
        }
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
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                active.get().is_some() && preview.is_none() && !import.loading(),
                egui::Button::new("Open in 3D")
                    .min_size(egui::vec2(140.0, 30.0))
                    .fill(ui.visuals().selection.bg_fill),
            )
            .clicked()
        {
            runtime.real_requested = true;
            return_open = true;
        }
        if ui.button("Import DEM…").clicked() {
            import.start(active, None, scene.fingerprint());
        }
        if ui
            .add_enabled(active.get().is_some(), egui::Button::new("Demo"))
            .clicked()
            && let Some((_, region)) = active.get()
        {
            match super::demo::validate_demo(region) {
                Ok(()) => {
                    runtime.open_requested = true;
                    return_open = true;
                }
                Err(error) => picker.selection.error = Some(error.into()),
            }
        }
    });
    if let Some(package) = &import.prepared {
        let m = &package.data.manifest;
        if active.get().is_none_or(|(_, r)| !m.matches(r)) {
            ui.label("Package has a different bbox/origin. Review its area before opening.");
            if ui.button("Use package area").clicked() {
                picker.selection.preview = Some(m.bbox);
                picker.selection.origin = Some(m.origin);
                picker.selection.error = None;
                picker.view.fit_requested = true;
            }
        }
    }
    super::controls::status(ui, active, import, scene);
    if let Some((revision, _)) = active.get() {
        ui.small(format!("Confirmed revision {revision} · Open in 3D uses this region. Demo is explicitly synthetic."));
    }
    if let Some(error) = &picker.selection.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    return_open
}
