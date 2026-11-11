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
    pub picker: &'a mut crate::map::picker::MapPicker,
    pub active: &'a mut crate::map::region::ActiveMapRegion,
    pub runtime: &'a mut crate::map::RegionRuntime,
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
        picker,
        active,
        runtime,
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
    let mut open = false;
    let mut map_visible = false;
    let mut viewport_visible = false;
    workspace.show(ui, |panel_ui, pane| match pane {
        PanelId::Map => {
            map_visible = true;
            open = crate::map::ui::show(panel_ui, picker, active, runtime);
        }
        PanelId::Explorer => {
            commands.extend(show_explorer(panel_ui, document, explorer));
        }
        PanelId::Viewport => {
            viewport_visible = true;
            if runtime.showing_demo {
                panel_ui.horizontal(|ui| {
                    ui.label("Synthetic terrain · east X / up Y / south Z");
                    if ui.button("Editor scene").clicked() {
                        runtime.showing_demo = false;
                        viewport
                            .camera
                            .reset(crate::scene::objects::bounds(document));
                        viewport.drag = None;
                    }
                });
            }
            viewport::input::show(panel_ui, viewport, document, notice, runtime.showing_demo);
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
    if !viewport_visible {
        viewport.drag = None;
    }
    if !map_visible {
        picker.selection.drag_start = None;
    }
    if open {
        workspace.activate(PanelId::Viewport);
    }
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
