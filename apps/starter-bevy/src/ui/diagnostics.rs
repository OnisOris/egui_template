use crate::{
    map::{picker::MapPicker, real_scene::RealScene},
    metrics::EditorMetrics,
};
use bevy_egui::egui;
pub fn show(
    ctx: &egui::Context,
    open: &mut bool,
    m: &EditorMetrics,
    real: &RealScene,
    picker: &MapPicker,
) {
    egui::Window::new("Diagnostics").open(open).default_width(460.).show(ctx, |ui| {
        egui::Grid::new("diagnostic_metrics").num_columns(2).spacing([16.,8.]).show(ui, |ui| {
            for (name,value) in [("FPS",m.fps),("Frame interval (ms)",m.frame_ms),("Main schedule CPU (ms)",m.cpu_frame_ms),("UI CPU (ms)",m.ui_ms),("Scene sync (ms)",m.sync_ms),("Mesh building CPU (ms)",m.scene_build_ms)] {
                ui.label(name); ui.monospace(format!("{value:.2}")); ui.end_row();
            }
            ui.label("Projected marker entities"); ui.label(m.projected_entities.to_string()); ui.end_row();
        });
        ui.weak("CPU wall times. GPU frame and upload timings are not exposed.");
        if let Some(service) = &picker.imagery.service {
            let (requests,hits,bytes,decode) = service.stats();
            ui.label(format!("Tiles: {requests} HTTP · {hits} cache hits · {:.1} MiB · decode {decode:.1} ms",bytes as f64 / 1048576.));
        }
        ui.separator();
        ui.label(&real.vector_status);
        ui.label(&picker.imagery.live_status);
        if let Some(data) = &real.data {
            ui.label(format!("Terrain package: {}",data.manifest.terrain_id));
        }
    });
}
