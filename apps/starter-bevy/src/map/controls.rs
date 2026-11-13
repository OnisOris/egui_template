use super::{
    RegionRuntime, SceneMode, loading::TerrainImport, real_scene::RealScene,
    region::ActiveMapRegion,
};
use crate::viewport::{ViewportState, focus_geo};
use bevy_egui::egui::{self, Ui};

pub fn status(ui: &mut Ui, active: &ActiveMapRegion, import: &TerrainImport, scene: &RealScene) {
    let builder_status = import.builder.status();
    if !builder_status.is_empty() {
        if import.builder.loading {
            ui.spinner();
        }
        ui.label(builder_status);
    } else if import.loading() {
        ui.spinner();
        ui.label("Loading DEM package… UI remains available.");
    } else if active.get().is_none() {
        ui.label("No region · select and confirm an area, or import a package area.");
    } else if scene.revision == active.get().map(|(r, _)| r) {
        ui.label("Ready · Real DEM");
    } else if import.prepared.is_some() {
        ui.label("Package ready · Open in 3D after confirming its region.");
    } else {
        ui.label("No terrain package for this region · Load in 3D.");
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
    imports: (
        &mut TerrainImport,
        &mut super::satellite_scene::SatelliteImport,
    ),
    scene: &mut RealScene,
    benchmark: bool,
) {
    let (import, satellite) = imports;
    if !scene.vector_status.is_empty() {
        ui.small(&scene.vector_status);
    }
    ui.horizontal_wrapped(|ui| {
        ui.strong(match runtime.mode {
            SceneMode::Demo => "Demo · synthetic",
            SceneMode::Real if scene.satellite_enabled => "Real DEM · Satellite",
            SceneMode::Real => "Real DEM · Terrain",
            SceneMode::Editor if benchmark => "Demo / Benchmark",
            SceneMode::Editor => "Project",
        });
        if runtime.mode == SceneMode::Real {
            if let Some(data) = &scene.data {
                ui.label(&data.manifest.source)
                    .on_hover_text(&data.manifest.attribution);
            }
            if ui.small_button("Import Vectors…").clicked() {
                scene.vector_import_requested = true;
            }
            ui.checkbox(&mut scene.grid_enabled, "Grid");
            ui.selectable_value(&mut scene.satellite_enabled, false, "Terrain");
            ui.add_enabled_ui(scene.satellite.is_some(), |ui| {
                ui.selectable_value(&mut scene.satellite_enabled, true, "Satellite");
            });
            if ui.small_button("Import Existing Package…").clicked() {
                satellite.packages.open = true;
            }
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
    });
    if runtime.mode == SceneMode::Real
        && let Some(data) = &scene.data
    {
        let m = &data.manifest;
        ui.small(format!(
            "{} × {} · {:.1}…{:.1} m MSL · {}",
            m.width, m.height, m.min_elevation_msl, m.max_elevation_msl, m.terrain_id
        ));
        ui.small(format!(
            "DEM step {:.1} × {:.1} m · Reference: {}",
            m.spacing_x_m, m.spacing_y_m, m.reference_elevation_source
        ));
        if scene.revision != active.get().map(|(r, _)| r) {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Previous region remains visible. Import and open the new region to replace it.",
            );
        }
    }
    if satellite.loading() {
        ui.label("Loading satellite JPEG… UI remains available.");
    }
    if let Some(error) = &satellite.error
        && !(satellite.packages.open && satellite.pending.is_some())
    {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("Satellite error · {error}"),
        );
    }
    if runtime.mode == SceneMode::Real
        && scene.satellite_enabled
        && let Some(loaded) = &scene.satellite
    {
        if matches!(
            loaded.source,
            super::satellite_scene::ImagerySource::Package(_)
        ) {
            ui.small(format!(
                "Satellite: Local Package · imagery zoom {}",
                loaded.metadata.zoom
            ));
        }
        ui.small(&loaded.metadata.attribution);
    }
    if !import.builder.status().is_empty()
        || import.loading()
        || import.error.is_some()
        || runtime.mode == SceneMode::Editor
    {
        status(ui, active, import, scene);
    }
}
