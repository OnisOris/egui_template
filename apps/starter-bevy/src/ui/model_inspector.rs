use super::project_inspector::InspectorState;
use crate::{
    map::real_scene::RealScene,
    models::{assets::ModelAssets, instances::ModelScene},
    project::Project,
};
use bevy_egui::egui::{self, Ui};
use starter_core::ObjectId;
use starter_ui::style::section;
pub fn show(
    ui: &mut Ui,
    project: &mut Project,
    inspector: &mut InspectorState,
    id: ObjectId,
    real: &RealScene,
    models: (&ModelAssets, &ModelScene),
) {
    let Some(mut model) = project.document.models.iter().find(|m| m.id == id).cloned() else {
        return;
    };
    let before = model.clone();
    ui.strong(&model.name);
    section(ui, "General", |ui| {
        if let Some(name) = inspector.name(ui, &project.selection, &model.name) {
            model.name = name;
        }

        ui.checkbox(&mut model.visible, "Visible");
    });
    section(ui, "Asset", |ui| {
        ui.label(&model.asset_path);
    });
    section(ui, "Geographic Position", |ui| {
        egui::Grid::new("model_position")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, value, range, speed) in [
                    (
                        "Latitude",
                        &mut model.latitude,
                        -starter_core::MERCATOR_LATITUDE..=starter_core::MERCATOR_LATITUDE,
                        0.0001,
                    ),
                    ("Longitude", &mut model.longitude, -180.0..=180.0, 0.0001),
                    (
                        "Altitude AGL (m)",
                        &mut model.altitude_agl,
                        -100_000.0..=100_000.0,
                        0.1,
                    ),
                ] {
                    ui.label(label);
                    ui.add(
                        egui::DragValue::new(value)
                            .range(range)
                            .speed(speed)
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
    section(ui, "Transform", |ui| {
        egui::Grid::new("model_transform")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Rotation Yaw (°)");
                ui.add(
                    egui::DragValue::new(&mut model.yaw_degrees)
                        .range(-360_000.0..=360_000.0)
                        .speed(1.0),
                );
                ui.end_row();
                ui.label("Uniform Scale");
                ui.add(
                    egui::DragValue::new(&mut model.scale)
                        .range(0.001..=1000.0)
                        .speed(0.01),
                );
            });
    });
    if model != before
        && let Err(e) = project.edit_model(model.clone())
    {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    section(ui, "Loading Status", |ui| {
        if let Some(info) = models.1.information.get(&id) {
            ui.label(&info.status);
            if let Some(d) = info.dimensions {
                ui.label(format!("{:.3} × {:.3} × {:.3} m", d.x, d.y, d.z));
            }
        } else if let Some(asset) = models.0.entries.get(&model.asset_path) {
            ui.label(asset.status.to_string());
        } else {
            ui.label("Loading");
        }
        let elevation = real
            .data
            .as_ref()
            .filter(|d| {
                project
                    .document
                    .map_region
                    .is_some_and(|r| d.manifest.matches(r))
            })
            .and_then(|d| {
                crate::map::objects::terrain_elevation(d, model.longitude, model.latitude)
            });
        if let Some(h) = elevation {
            ui.label(format!("Terrain {h:.3} m MSL"));
        } else {
            ui.colored_label(
                egui::Color32::YELLOW,
                "No DEM elevation data for this point",
            );
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Focus Selected").clicked() {
            project.focus_selected = true;
        }
        if ui.button("Delete").clicked() {
            project.delete_selected();
        }
    });
}
