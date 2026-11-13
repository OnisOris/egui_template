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
    session: (&mut crate::project::Project, bool),
) {
    let (project, benchmark) = session;
    let (import, satellite) = imports;
    ui.horizontal_wrapped(|ui| {
        if runtime.mode == SceneMode::Real {
            let before = project.document.geo_view_mode;
            ui.spacing_mut().item_spacing.x = 2.;
            for (mode, label) in [
                (starter_core::GeoViewMode::City, "City 3D"),
                (starter_core::GeoViewMode::Terrain, "Terrain 3D"),
                (starter_core::GeoViewMode::Combined, "Combined"),
            ] {
                ui.selectable_value(&mut project.document.geo_view_mode, mode, label);
            }
            if before != project.document.geo_view_mode {
                project.changed();
            }
            ui.separator();
            for (kind, label) in [
                (starter_core::LayerKind::Buildings, "Buildings"),
                (starter_core::LayerKind::Roads, "Roads"),
                (starter_core::LayerKind::Satellite, "Satellite"),
            ] {
                let layer = project.document.layer_mut(kind);
                if ui.toggle_value(&mut layer.visible, label).changed() {
                    if kind == starter_core::LayerKind::Satellite {
                        scene.satellite_enabled = layer.visible && scene.satellite.is_some();
                    }
                    project.changed();
                }
            }
            ui.toggle_value(&mut scene.grid_enabled, "Grid");
        } else {
            ui.strong(if benchmark {
                "Demo / Benchmark"
            } else {
                "3D Scene"
            });
        }
        ui.separator();
        ui.menu_button("Camera", |ui| {
            if ui
                .button(viewport.camera.projection.label())
                .on_hover_text("Switch perspective / orthographic projection")
                .clicked()
            {
                viewport.camera.projection.toggle();
            }
            if ui.button("Reset Camera").clicked() {
                focus_geo(viewport);
                ui.close();
            }
            ui.label("LMB orbit · RMB pan · wheel zoom");
        });
        if ui
            .add_enabled(runtime.mode != SceneMode::Editor, egui::Button::new("Fit"))
            .on_hover_text("Fit the region while retaining camera orientation")
            .clicked()
        {
            let orbit = (viewport.camera.yaw, viewport.camera.pitch);
            focus_geo(viewport);
            (viewport.camera.yaw, viewport.camera.pitch) = orbit;
        }
        ui.menu_button("More", |ui| {
            if ui.button("Import DEM…").clicked() {
                import.start(active, None, scene.fingerprint());
                ui.close();
            }
            if ui.button("Import Existing Package…").clicked() {
                satellite.packages.open = true;
                ui.close();
            }
            if ui.button("Import Vectors…").clicked() {
                scene.vector_import_requested = true;
                ui.close();
            }
            if ui.button("Reload Vectors").clicked() {
                runtime.vector_requested = true;
                ui.close();
            }
        })
        .response
        .on_hover_text("Import and reload geographic data");
    });
    if runtime.mode == SceneMode::Real {
        ui.horizontal_wrapped(|ui| {
            ui.small("DEM: Ready");
            ui.small(if scene.satellite.is_some() {
                "Satellite: Ready"
            } else {
                "Satellite: Unavailable"
            });
            if !scene.vector_status.is_empty() {
                let compact = scene
                    .vector_status
                    .split('·')
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" · ");
                ui.small(compact)
                    .on_hover_text("Select Buildings or Roads in Explorer for source details");
            }
        });
        if let Some(s) = &scene.satellite {
            ui.small(&s.metadata.attribution);
        }
    }
    if import.loading() || import.builder.loading || import.error.is_some() {
        status(ui, active, import, scene);
    }
    if satellite.loading() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading satellite…");
        });
    }
    if let Some(error) = &satellite.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}
