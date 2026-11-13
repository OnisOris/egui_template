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
    imports: (
        &mut super::loading::TerrainImport,
        &mut super::satellite_scene::SatelliteImport,
    ),
    scene: &super::real_scene::RealScene,
    project: &mut starter_core::ProjectState,
) -> bool {
    let (import, satellite) = imports;
    let mut open = false;
    egui::Panel::top("map_tools").show(ui, |ui| {
        search_bar(ui, picker);
        picker.imagery.controls(ui);
    });
    egui::ScrollArea::vertical()
        .id_salt("map_region_controls")
        .max_height(220.0)
        .show(ui, |ui| {
            open = region_details(ui, picker, active, runtime, import, satellite, scene);
        });
    egui::Panel::bottom("map_attribution")
        .min_size(28.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                picker.imagery.attribution(ui);
                if ui.small_button("Retry tiles").clicked() {
                    picker.tiles = None;
                    if picker.imagery.enabled {
                        picker.imagery.retry();
                    }
                }
            });
            if !picker.imagery.enabled
                && let Some(tiles) = picker.tiles.as_mut()
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
    if !picker.imagery.enabled {
        picker
            .tiles
            .get_or_insert_with(|| MapTiles::new(ui.ctx().clone()));
    }
    if project.placing_model.is_some() {
        ui.label("Click the map to place Model · Escape to cancel");
    } else if project.placing_marker {
        ui.label("Click the map to place Marker · Escape to cancel");
    }
    picker.show_project_map(ui, active.get().map(|(_, r)| r.bounds), project);
    open
}

fn region_details(
    ui: &mut Ui,
    picker: &mut MapPicker,
    active: &mut ActiveMapRegion,
    runtime: &mut RegionRuntime,
    import: &mut super::loading::TerrainImport,
    satellite: &mut super::satellite_scene::SatelliteImport,
    scene: &super::real_scene::RealScene,
) -> bool {
    let preview = picker.selection.preview;
    if let Some(b) = preview.or_else(|| active.get().map(|(_, r)| r.bounds)) {
        let (width, height) = b.dimensions();
        ui.horizontal_wrapped(|ui| {
            ui.strong(if preview.is_some() {
                "Selected Region · Draft"
            } else {
                "Selected Region · Confirmed"
            });
            ui.weak(format!("{:.2} × {:.2} km", width / 1000., height / 1000.));
        });
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
                preview.is_some() || active.get().is_some(),
                egui::Button::new("Fit Region"),
            )
            .clicked()
        {
            picker.view.fit_requested = true;
        }
        if ui
            .add_enabled(
                active.get().is_some() && preview.is_none() && !import.loading(),
                egui::Button::new("Load in 3D")
                    .min_size(egui::vec2(140.0, 30.0))
                    .fill(ui.visuals().selection.bg_fill),
            )
            .clicked()
        {
            load_in_3d(active, runtime, import);
            return_open = true;
        }
        ui.menu_button("More…", |ui| {
            if ui.button("Clear Selection").clicked() {
                picker.selection.clear();
                ui.close();
            }
            if ui.button("Import Existing Package…").clicked() {
                if scene.data.is_some()
                    && scene.revision == active.get().map(|(revision, _)| revision)
                {
                    import.builder.cancel();
                }
                satellite.start(active, scene, None);
                ui.close();
            }
            if ui.button("Browse Existing Packages…").clicked() {
                satellite.packages.open = true;
                ui.close();
            }
            if let Some(b) = preview.or_else(|| active.get().map(|(_, r)| r.bounds)) {
                ui.collapsing("Region details", |ui| {
                    let center = b.center();
                    ui.label(format!(
                        "Center {:.6}°, {:.6}°",
                        center.latitude, center.longitude
                    ));
                    ui.label(format!(
                        "N {:.6}° · S {:.6}° · W {:.6}° · E {:.6}°",
                        b.north, b.south, b.west, b.east
                    ));
                });
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
                        import.builder.cancel();
                        runtime.open_requested = true;
                        runtime.vector_requested = true;
                        return_open = true;
                    }
                    Err(error) => picker.selection.error = Some(error.into()),
                }
            }
        });
    });
    if import.builder.loading && ui.button("Cancel load").clicked() {
        import.builder.cancel();
    }
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
    if let Some(error) = &picker.selection.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    return_open
}

pub(crate) fn load_in_3d(
    active: &ActiveMapRegion,
    runtime: &mut RegionRuntime,
    import: &mut super::loading::TerrainImport,
) {
    if active.get().is_none() || import.loading() {
        return;
    }
    if import.prepared.as_ref().is_some_and(|p| {
        active
            .get()
            .is_some_and(|(_, r)| p.data.manifest.matches(r))
    }) {
        runtime.real_requested = true;
    } else {
        import.start_automatic(active);
    }
    runtime.vector_requested = true;
}
