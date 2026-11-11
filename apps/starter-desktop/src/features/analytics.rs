use eframe::egui::{self, Ui};
use starter_ui::{PlotPoint, line_chart};

use crate::metrics::MetricsState;

pub struct AnalyticsState {
    signal: Vec<PlotPoint>,
}

impl Default for AnalyticsState {
    fn default() -> Self {
        Self {
            signal: (0..240)
                .map(|i| {
                    let x = i as f64 / 24.0;
                    PlotPoint::new(x, (x * 1.3).sin() * 0.7 + (x * 0.31).cos() * 0.2)
                })
                .collect(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &AnalyticsState, metrics: &MetricsState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Analytics");
        ui.label("Synthetic signal · drag to pan, scroll to zoom");
        line_chart(
            ui,
            "synthetic_signal",
            "Signal",
            state.signal.as_slice().into(),
        );
        ui.separator();
        ui.label("Measured UI callback CPU time (ms), not GPU frame time");
        line_chart(
            ui,
            "callback_cpu_time",
            "UI callback ms",
            metrics.samples.as_slice().into(),
        );
    });
}
