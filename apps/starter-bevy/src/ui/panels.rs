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
    pub models: (
        &'a crate::models::assets::ModelAssets,
        &'a crate::models::instances::ModelScene,
    ),
    pub model_pick: &'a mut crate::models::picking::ModelPick,
    pub project: &'a mut crate::project::Project,
    pub project_explorer: &'a mut super::project_explorer::ExplorerState,
    pub project_inspector: &'a mut super::project_inspector::InspectorState,
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

pub fn show(ui: &mut Ui, panels: Panels<'_>) -> (Vec<Command>, Vec<crate::state::EditorAction>) {
    let Panels {
        project,
        models,
        model_pick,
        project_io,
        project_explorer,
        marker_table,
        project_inspector,
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
    let mut actions = Vec::new();
    let mut open = false;
    let mut map_visible = false;
    let mut viewport_visible = false;
    picker.imagery.poll(ui.ctx());
    let placing_before = project.placing_model.clone();
    workspace.show(ui, |panel_ui, pane| match pane {
        PanelId::Map => {
            map_visible = true;
            open = crate::map::ui::show(
                panel_ui,
                picker,
                active,
                runtime,
                (import, satellite),
                real,
                project,
            );
            if open && project_io.benchmark {
                project_io.benchmark = false;
                commands.push(Command::NewDocument);
            }
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
                    (project, project_io.benchmark),
                );
                if mode != runtime.mode && runtime.mode == crate::map::SceneMode::Editor {
                    viewport
                        .camera
                        .reset(crate::scene::objects::bounds(document));
                }
            }
            if !project_io.benchmark
                && (real.data.is_none() || project.document.object_count() == 0)
                && let Some(action) =
                    super::empty_states::show(panel_ui, project, real.data.is_some())
            {
                actions.push(action);
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
                && real.data.is_some()
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
                model_pick.pending = Some((ray, real.fingerprint(), project_io.generation));
            }
        }
        PanelId::Inspector => {
            if project_io.benchmark {
                commands.extend(show_inspector(panel_ui, document, inspector));
            } else {
                bevy_egui::egui::ScrollArea::vertical()
                    .id_salt("inspector_scroll")
                    .auto_shrink([false, false])
                    .show(panel_ui, |panel_ui| {
                        super::project_inspector::show(
                            panel_ui,
                            project,
                            project_io,
                            real,
                            project_inspector,
                            models,
                        );
                    });
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
    crate::map::source_settings::show(ui, &mut picker.imagery);
    if let Some(directory) = picker.imagery.package_directory_update.take() {
        satellite.packages.directory = directory;
    }
    crate::map::package_browser::show(ui, satellite, active, real);
    if satellite.loading()
        && real.data.is_some()
        && real.revision == active.get().map(|(revision, _)| revision)
    {
        import.builder.cancel();
    }
    if !map_visible {
        picker.selection.drag_start = None;
    }
    if project.placing_model != placing_before && project.placing_model.is_some() {
        workspace.activate(PanelId::Map);
    }
    if open {
        workspace.activate(PanelId::Viewport);
    }
    (commands, actions)
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
