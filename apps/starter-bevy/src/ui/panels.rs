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
    pub project: &'a mut crate::project::Project,
    pub project_explorer: &'a mut super::project_explorer::ExplorerState,
    pub marker_table: &'a mut super::project_table::TableState,
    pub project_io: &'a mut crate::project::ProjectIo,
    pub picker: &'a mut crate::map::picker::MapPicker,
    pub active: &'a mut crate::map::region::ActiveMapRegion,
    pub runtime: &'a mut crate::map::RegionRuntime,
    pub import: &'a mut crate::map::loading::TerrainImport,
    pub real: &'a mut crate::map::real_scene::RealScene,
    pub satellite: &'a mut crate::map::satellite_scene::SatelliteImport,
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
        project,
        project_io,
        project_explorer,
        marker_table,
        picker,
        active,
        runtime,
        import,
        real,
        satellite,
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
            open = crate::map::ui::show(panel_ui, picker, active, runtime, import, real, project);
        }
        PanelId::Explorer => {
            if project_io.benchmark {
                commands.extend(show_explorer(panel_ui, document, explorer));
            } else {
                super::project_explorer::show(
                    panel_ui,
                    project_explorer,
                    project,
                    project_io,
                    real,
                    import,
                );
            }
        }
        PanelId::Viewport => {
            viewport_visible = true;
            {
                let mode = runtime.mode;
                crate::map::controls::viewport_bar(
                    panel_ui,
                    viewport,
                    runtime,
                    active,
                    (import, satellite),
                    real,
                );
                if mode != runtime.mode && runtime.mode == crate::map::SceneMode::Editor {
                    viewport
                        .camera
                        .reset(crate::scene::objects::bounds(document));
                }
            }
            let response = viewport::input::show(
                panel_ui,
                viewport,
                document,
                notice,
                !project_io.benchmark || runtime.mode != crate::map::SceneMode::Editor,
            );
            if !project_io.benchmark
                && response.clicked()
                && real.revision == active.get().map(|(r, _)| r)
                && let Some(data) = &real.data
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let p = pointer - response.rect.min;
                let size = response.rect.size();
                let ray = viewport::picking::screen_ray(
                    &viewport.camera,
                    bevy::math::Vec2::new(p.x, p.y),
                    bevy::math::Vec2::new(size.x, size.y),
                    size.x / size.y.max(1.0),
                );
                if let Some(id) = crate::map::objects::pick(
                    project,
                    data,
                    &ray,
                    (viewport.camera.distance * 0.005).max(4.0),
                ) {
                    project.selection = starter_core::ProjectSelection::Marker(id);
                }
            }
        }
        PanelId::Inspector => {
            if project_io.benchmark {
                commands.extend(show_inspector(panel_ui, document, inspector));
            } else {
                super::project_inspector::show(panel_ui, project, project_io, real, import);
            }
        }
        PanelId::DataTable => {
            if project_io.benchmark {
                commands.extend(show_data_table(panel_ui, document, table));
            } else {
                super::project_table::show(panel_ui, project, marker_table);
            }
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
