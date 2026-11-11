use egui::Ui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

pub fn line_chart(ui: &mut Ui, id: &'static str, label: &'static str, points: PlotPoints<'_>) {
    Plot::new(id)
        .legend(Legend::default())
        .show(ui, |plot_ui| plot_ui.line(Line::new(label, points)));
}
