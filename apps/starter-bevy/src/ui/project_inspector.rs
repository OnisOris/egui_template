use crate::{
    map::real_scene::RealScene,
    project::{Project, ProjectIo},
};
use bevy_egui::egui::{self, Ui};
use starter_core::{LayerKind, ProjectSelection};
use starter_ui::style::section;

pub use super::project_names::InspectorState;

pub fn show(
    ui: &mut Ui,
    project: &mut Project,
    io: &ProjectIo,
    real: &mut RealScene,
    inspector: &mut InspectorState,
    models: (
        &crate::models::assets::ModelAssets,
        &crate::models::instances::ModelScene,
    ),
) {
    inspector.sync(project, io.generation);
    ui.heading("Inspector");
    match project.selection.clone() {
        ProjectSelection::Project => {
            ui.strong("Project");
            section(ui, "General", |ui| {
                if let Some(name) = inspector.name(ui, &project.selection, &project.document.name)
                    && name != project.document.name
                {
                    project.document.name = name;
                    project.changed();
                }
            });
            section(ui, "Location", |ui| {
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
            });
        }
        ProjectSelection::Model(id) => {
            super::model_inspector::show(ui, project, inspector, id, real, models)
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
            if real.data.as_ref().is_some_and(|d| {
                project
                    .document
                    .map_region
                    .is_none_or(|r| !d.manifest.matches(r))
            }) {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Previous region data · load the current region",
                );
            }
            let mut visible = project.document.layer(kind).visible;
            if ui.checkbox(&mut visible, "Visible").changed() {
                project.document.layer_mut(kind).visible = visible;
                project.changed();
                if kind == LayerKind::Satellite {
                    real.satellite_enabled = visible && real.satellite.is_some();
                }
            }
            section(ui, "Source & Status", |ui| match kind {
                LayerKind::Roads | LayerKind::Buildings => {
                    ui.label(&real.vector_status);
                    if ui.button("Import Vectors…").clicked() {
                        real.vector_import_requested = true;
                    }
                    if project.document.geo_view_mode == starter_core::GeoViewMode::Terrain {
                        ui.small("Hidden in Terrain mode · use City or Combined");
                    }
                    let mut source = project.document.vector_source;
                    ui.radio_value(
                        &mut source,
                        starter_core::VectorSource::OpenFreeMap,
                        "OpenFreeMap (automatic)",
                    );
                    ui.radio_value(
                        &mut source,
                        starter_core::VectorSource::LocalPackage,
                        "Local Package",
                    );
                    if source != project.document.vector_source {
                        project.document.vector_source = source;
                        project.changed();
                    }
                }
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
                        ui.label(format!(
                            "Reference {:.2} m MSL · {}",
                            m.reference_elevation_msl, m.reference_elevation_source
                        ));
                        ui.collapsing("Package details", |ui| {
                            ui.label(&m.terrain_id);
                        });
                        ui.label(&m.attribution);
                    } else {
                        ui.label("No DEM loaded · select region and Load in 3D");
                    }
                }
                LayerKind::Satellite => {
                    if let Some(s) = &real.satellite {
                        let m = &s.metadata;
                        ui.label(&m.provider);
                        ui.label(format!(
                            "Zoom {} · {} × {} pixels",
                            m.zoom, m.width, m.height
                        ));
                        ui.label(&m.attribution);
                    } else {
                        ui.label("Satellite not loaded");
                    }
                }
            });
        }
        ProjectSelection::Marker(id) => {
            if let Some(marker) = project.document.objects.iter().find(|m| m.id == id) {
                let mut marker = marker.clone();
                let before = marker.clone();
                ui.strong(&marker.name);
                section(ui, "General", |ui| {
                    if let Some(name) = inspector.name(ui, &project.selection, &marker.name) {
                        marker.name = name;
                    }
                    ui.checkbox(&mut marker.visible, "Visible");
                });
                section(ui, "Geographic Position", |ui| {
                    egui::Grid::new("marker_position")
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("Latitude", &mut marker.latitude),
                                ("Longitude", &mut marker.longitude),
                                ("Altitude AGL (m)", &mut marker.altitude_agl),
                            ] {
                                ui.label(label);
                                ui.add(
                                    egui::DragValue::new(value)
                                        .speed(if label == "Altitude AGL (m)" {
                                            0.1
                                        } else {
                                            0.0001
                                        })
                                        .max_decimals(7)
                                        .suffix(if label == "Altitude AGL (m)" {
                                            " m"
                                        } else {
                                            "°"
                                        }),
                                );
                                ui.end_row();
                            }
                        });
                });
                section(ui, "Appearance", |ui| {
                    ui.color_edit_button_srgba_unmultiplied(&mut marker.color);
                });
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
                if let Some(model) = models.0.entries.get(&asset.path) {
                    ui.label(model.status.to_string());
                }
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
