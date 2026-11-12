use crate::{
    map::{loading::TerrainImport, real_scene::RealScene},
    project::{Project, ProjectIo},
};
use bevy_egui::egui::{self, Ui};
use starter_core::{LayerKind, ProjectSelection};
pub fn show(
    ui: &mut Ui,
    project: &mut Project,
    io: &ProjectIo,
    real: &mut RealScene,
    import: &TerrainImport,
) {
    ui.heading("Inspector");
    match project.selection.clone() {
        ProjectSelection::Project => {
            ui.strong("Project");
            let mut name = project.document.name.clone();
            if ui.text_edit_singleline(&mut name).changed()
                && !name.trim().is_empty()
                && name.len() <= 256
            {
                project.document.name = name;
                project.changed();
            }
            ui.label(
                io.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or("Not saved to disk".into()),
            );
            ui.label(if project.dirty {
                "Unsaved changes"
            } else {
                "Saved"
            });
        }
        ProjectSelection::Manifest => {
            ui.strong("Project JSON");
            ui.label(
                io.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|p| p.to_str())
                    .unwrap_or("project.json"),
            );
            ui.label("application/json");
            ui.label(
                io.file_size
                    .map(|size| format!("{size} bytes"))
                    .unwrap_or("Not saved".into()),
            );
        }
        ProjectSelection::Layer(kind) => {
            ui.strong(format!("{kind:?}"));
            let mut visible = project.document.layer(kind).visible;
            if ui.checkbox(&mut visible, "Visible").changed() {
                project.document.layer_mut(kind).visible = visible;
                project.changed();
                if kind == LayerKind::Satellite {
                    real.satellite_enabled = visible && real.satellite.is_some();
                }
            }
            match kind {
                LayerKind::Terrain => {
                    if let Some(data) = &real.data {
                        let m = &data.manifest;
                        ui.label(&m.source);
                        ui.label(format!("{} × {} samples", m.width, m.height));
                        ui.label(format!(
                            "{:.1}…{:.1} m MSL",
                            m.min_elevation_msl, m.max_elevation_msl
                        ));
                        ui.label(format!(
                            "Step {:.2} × {:.2} m",
                            m.spacing_x_m, m.spacing_y_m
                        ));
                        ui.label(&m.reference_elevation_source);
                        ui.label(&m.attribution);
                    } else {
                        ui.label("No DEM loaded · select region and Load in 3D");
                    }
                }
                LayerKind::Satellite => {
                    if let Some(s) = &real.satellite {
                        let m = &s.manifest.satellite;
                        ui.label(&m.provider);
                        ui.label(format!(
                            "Zoom {} · {} × {} pixels",
                            m.zoom, m.width, m.height
                        ));
                        ui.label(&m.attribution);
                    } else {
                        ui.label("Satellite not loaded");
                        ui.label(import.builder.status());
                    }
                }
            }
        }
        ProjectSelection::Marker(id) => {
            if let Some(marker) = project.document.objects.iter().find(|m| m.id == id) {
                let mut marker = marker.clone();
                let before = marker.clone();
                ui.strong(format!("Map Marker · {}", id.0));
                ui.text_edit_singleline(&mut marker.name);
                for (label, value) in [
                    ("Latitude", &mut marker.latitude),
                    ("Longitude", &mut marker.longitude),
                    ("Altitude AGL (m)", &mut marker.altitude_agl),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value).speed(0.0001).max_decimals(7));
                    });
                }
                ui.checkbox(&mut marker.visible, "Visible");
                ui.color_edit_button_srgba_unmultiplied(&mut marker.color);
                if before != marker
                    && let Err(e) = project.edit_marker(marker.clone())
                {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                if let Some(data) = real.data.as_ref().filter(|d| {
                    project
                        .document
                        .map_region
                        .is_some_and(|r| d.manifest.matches(r))
                }) && let Some(h) =
                    crate::map::objects::terrain_elevation(data, marker.longitude, marker.latitude)
                {
                    ui.label(format!("Terrain {:.2} m MSL", h));
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "No DEM elevation data for this point",
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Focus Selected").clicked() {
                        project.focus_selected = true;
                    }
                    if ui.button("Delete").clicked() {
                        project.delete_selected();
                    }
                });
            }
        }
        ProjectSelection::Asset(i) => {
            if let Some(asset) = project.document.assets.get(i) {
                ui.strong("Project resource");
                ui.label(&asset.path);
                ui.label(&asset.media_type);
                ui.label(format!("{} bytes", asset.size));
                ui.label(if io.missing.get(i) == Some(&true) {
                    "Missing resource"
                } else {
                    "Available at last refresh"
                });
            }
        }
    }
}
