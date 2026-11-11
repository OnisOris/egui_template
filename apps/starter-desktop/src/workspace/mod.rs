mod menu;
mod status;
mod toolbar;

#[derive(Clone, Copy, Debug)]
pub enum AppAction {
    New,
    Open,
    Save,
    AddObject,
    DeleteSelection,
    ResetView,
    ResetLayout,
    ToggleTheme,
    ResetSettings,
    ToggleRepaint,
    SetLoad(usize),
    FocusSelection,
}

pub fn menu(ui: &mut eframe::egui::Ui, workspace: &mut starter_ui::Workspace) -> Vec<AppAction> {
    menu::show(ui, workspace)
}

pub fn toolbar(ui: &mut eframe::egui::Ui, load_size: usize, continuous: bool) -> Vec<AppAction> {
    toolbar::show(ui, load_size, continuous)
}

pub fn status(
    ui: &mut eframe::egui::Ui,
    document: &starter_core::DocumentState,
    metrics: &crate::metrics::MetricsState,
    zoom: f32,
    continuous: bool,
    notice: Option<&str>,
) {
    status::show(ui, document, metrics, zoom, continuous, notice);
}
