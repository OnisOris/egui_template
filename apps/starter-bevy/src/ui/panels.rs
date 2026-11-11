use bevy_egui::egui::Ui;
use starter_core::{Command, DocumentState};
use starter_ui::{
    ExplorerNodeId, ExplorerTree, InspectorState, PanelId, TableState, Workspace, line_chart,
    show_data_table, show_explorer, show_inspector,
};

use crate::{
    metrics::EditorMetrics,
    ui::gallery::{self, GalleryState},
    viewport::{self, ViewportState},
};

pub struct Panels<'a> {
    pub workspace: &'a mut Workspace,
    pub explorer: &'a mut ExplorerTree<ExplorerNodeId>,
    pub inspector: &'a mut InspectorState,
    pub table: &'a mut TableState,
    pub gallery: &'a mut GalleryState,
    pub viewport: &'a mut ViewportState,
    pub document: &'a mut DocumentState,
    pub metrics: &'a EditorMetrics,
    pub notice: &'a mut Option<String>,
}

pub fn show(ui: &mut Ui, panels: Panels<'_>) -> Vec<Command> {
    let Panels {
        workspace,
        explorer,
        inspector,
        table,
        gallery,
        viewport,
        document,
        metrics,
        notice,
    } = panels;
    let mut commands = Vec::new();
    workspace.show(ui, |panel_ui, pane| match pane {
        PanelId::Explorer => {
            commands.extend(show_explorer(panel_ui, document, explorer));
        }
        PanelId::Viewport => {
            viewport::input::show(panel_ui, viewport, document, notice);
        }
        PanelId::Inspector => {
            commands.extend(show_inspector(panel_ui, document, inspector));
        }
        PanelId::DataTable => {
            commands.extend(show_data_table(panel_ui, document, table));
        }
        PanelId::Charts => charts(panel_ui, metrics),
        PanelId::Components => gallery::show(panel_ui, gallery),
    });
    commands
}

fn charts(ui: &mut Ui, metrics: &EditorMetrics) {
    ui.heading("Charts");
    ui.label("Measured Bevy frame time (CPU wall clock), not GPU time");
    line_chart(
        ui,
        "bevy_frame_chart",
        "Frame ms",
        metrics.signal.as_slice().into(),
    );
}
