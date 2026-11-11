use super::{
    RegionRuntime, SceneMode, loading::TerrainImport, real_scene::RealScene,
    region::ActiveMapRegion,
};
use crate::viewport::{ViewportState, focus_geo};
use bevy_egui::egui::{self, Ui};

pub fn status(ui: &mut Ui, active: &ActiveMapRegion, import: &TerrainImport, scene: &RealScene) {
    if import.loading() {
        ui.spinner();
        ui.label("Loading DEM package… UI remains available.");
    } else if active.get().is_none() {
        ui.label("No region · select and confirm an area, or import a package area.");
    } else if scene.revision == active.get().map(|(r, _)| r) {
        ui.label("Ready · Real DEM");
    } else if import.prepared.is_some() {
        ui.label("Package ready · Open in 3D after confirming its region.");
    } else {
        ui.label("No terrain package for this region · Import DEM.");
    }
    if let Some(error) = &import.error {
        ui.colored_label(ui.visuals().error_fg_color, format!("Error · {error}"));
    }
}

pub fn viewport_bar(
    ui: &mut Ui,
    viewport: &mut ViewportState,
    runtime: &mut RegionRuntime,
    active: &ActiveMapRegion,
    import: &mut TerrainImport,
    scene: &mut RealScene,
) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(match runtime.mode {
            SceneMode::Demo => "Demo · synthetic",
            SceneMode::Real => "Real DEM · Terrain",
            SceneMode::Editor => "Editor",
        });
        if runtime.mode == SceneMode::Real {
            if let Some(data) = &scene.data {
                ui.label(&data.manifest.source);
            }
            ui.checkbox(&mut scene.grid_enabled, "Grid");
        }
        if ui
            .add_enabled(
                runtime.mode != SceneMode::Editor,
                egui::Button::new("Reset Camera"),
            )
            .clicked()
        {
            focus_geo(viewport);
        }
        if ui
            .add_enabled(
                runtime.mode != SceneMode::Editor,
                egui::Button::new("Fit Terrain"),
            )
            .clicked()
        {
            let orbit = (viewport.camera.yaw, viewport.camera.pitch);
            focus_geo(viewport);
            (viewport.camera.yaw, viewport.camera.pitch) = orbit;
        }
        if ui.small_button("Import DEM…").clicked() {
            import.start(active, None, scene.fingerprint());
        }
        if ui.small_button("Editor scene").clicked() {
            runtime.mode = SceneMode::Editor;
            viewport.drag = None;
        }
    });
    if runtime.mode == SceneMode::Real
        && let Some(data) = &scene.data
    {
        let m = &data.manifest;
        ui.small(format!(
            "{} × {} · {:.1}…{:.1} m MSL · {}",
            m.width, m.height, m.min_elevation_msl, m.max_elevation_msl, m.terrain_id
        ));
        if scene.revision != active.get().map(|(r, _)| r) {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Previous region remains visible. Import and open the new region to replace it.",
            );
        }
    }
    if import.loading() || import.error.is_some() || runtime.mode == SceneMode::Editor {
        status(ui, active, import, scene);
    }
}
