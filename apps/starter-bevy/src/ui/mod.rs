use std::time::Instant;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use starter_core::Command;

use crate::state::DocumentResource;
use starter_ui::Theme;

use crate::{
    metrics::EditorMetrics,
    scene::objects::{bounds, object_center},
    state::{EditorAction, UiState},
    viewport::{self, ViewportCamera, ViewportState},
};

pub mod chrome;
pub mod gallery;
pub mod panels;
pub mod project_explorer;
pub mod project_inspector;
pub mod project_table;

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MapUi<'w> {
    project: ResMut<'w, crate::project::Project>,
    project_io: ResMut<'w, crate::project::ProjectIo>,
    picker: NonSendMut<'w, crate::map::picker::MapPicker>,
    active: ResMut<'w, crate::map::region::ActiveMapRegion>,
    runtime: ResMut<'w, crate::map::RegionRuntime>,
    import: ResMut<'w, crate::map::loading::TerrainImport>,
    real: ResMut<'w, crate::map::real_scene::RealScene>,
    satellite: ResMut<'w, crate::map::satellite_scene::SatelliteImport>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RenderUi<'w, 's> {
    images: ResMut<'w, Assets<Image>>,
    cameras: Query<'w, 's, &'static mut RenderTarget, With<ViewportCamera>>,
}

pub(crate) fn editor_ui(
    mut contexts: EguiContexts,
    map: MapUi,
    mut ui_state: ResMut<UiState>,
    mut document: ResMut<DocumentResource>,
    mut viewport: ResMut<ViewportState>,
    mut metrics: ResMut<EditorMetrics>,
    targets: RenderUi,
) -> Result {
    let MapUi {
        mut project,
        mut project_io,
        mut picker,
        mut active,
        mut runtime,
        mut import,
        mut real,
        mut satellite,
    } = map;
    let RenderUi {
        mut images,
        mut cameras,
    } = targets;
    let started = Instant::now();
    crate::project::ui::unsaved(contexts.ctx_mut()?, &mut project_io, &mut project);
    let projection = viewport.camera.projection;
    let mut actions;
    let commands;
    {
        let ctx = contexts.ctx_mut()?;
        let mut root = egui::Ui::new(
            ctx.clone(),
            "starter_bevy_root".into(),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(ctx.viewport_rect()),
        );
        if project_io.busy() || project_io.prompt.is_some() {
            root.disable();
        }
        let UiState {
            workspace,
            theme,
            explorer,
            inspector,
            table,
            gallery,
            notice,
            project_explorer,
            marker_table,
            project_inspector,
        } = &mut *ui_state;
        if ctx.global_style().visuals.dark_mode != matches!(theme, Theme::Dark) {
            theme.apply(ctx);
        }
        actions = chrome::show(
            &mut root,
            workspace,
            (&document, &project, project_io.benchmark),
            &metrics,
            projection,
            &mut viewport.camera.sensitivity,
            notice.as_deref(),
        );
        if !project_io.busy()
            && project_io.prompt.is_none()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S))
        {
            actions.push(EditorAction::Save);
        }
        commands = panels::show(
            &mut root,
            panels::Panels {
                project: &mut project,
                project_io: &mut project_io,
                project_explorer,
                marker_table,
                project_inspector,
                picker: &mut picker,
                active: &mut active,
                runtime: &mut runtime,
                import: &mut import,
                real: &mut real,
                satellite: &mut satellite,
                workspace,
                explorer,
                inspector,
                table,
                gallery,
                viewport: &mut viewport,
                document: &mut document,
                metrics: &metrics,
                notice,
            },
        );
    }

    let viewport_drawn = viewport.requested_size.is_some();
    viewport::target::apply_pending_resize(&mut contexts, &mut images, &mut viewport, &mut cameras);
    if viewport_drawn && viewport.fit_geo_pending {
        crate::viewport::focus_geo(&mut viewport);
        viewport.fit_geo_pending = false;
    }
    for command in commands {
        dispatch(&mut document, &mut ui_state.notice, command);
    }
    for action in actions {
        apply(
            action,
            &mut ui_state,
            &mut document,
            &mut viewport,
            &mut runtime,
            &mut project,
            &mut project_io,
        );
    }
    crate::map::objects::focus(&mut project, &mut picker, &real, &mut viewport);
    metrics.ui_ms = started.elapsed().as_secs_f32() * 1000.0;
    Ok(())
}

fn apply(
    action: EditorAction,
    ui_state: &mut UiState,
    document: &mut DocumentResource,
    viewport: &mut ViewportState,
    runtime: &mut crate::map::RegionRuntime,
    project: &mut crate::project::Project,
    io: &mut crate::project::ProjectIo,
) {
    if runtime.mode != crate::map::SceneMode::Editor
        && (io.benchmark || matches!(action, EditorAction::ResetView))
        && matches!(
            action,
            EditorAction::ResetView | EditorAction::FocusSelection
        )
    {
        crate::viewport::focus_geo(viewport);
        return;
    }
    if matches!(action, EditorAction::SetLoad(_)) {
        if !io.benchmark {
            io.return_mode = runtime.mode;
        }
        runtime.mode = crate::map::SceneMode::Editor;
    }
    match action {
        EditorAction::New
        | EditorAction::Open
        | EditorAction::Save
        | EditorAction::SaveAs
        | EditorAction::ImportAsset
        | EditorAction::Exit => {
            use crate::project::ProjectAction as A;
            let action = match action {
                EditorAction::New => A::New,
                EditorAction::Open => A::Open,
                EditorAction::Save => A::Save,
                EditorAction::SaveAs => A::SaveAs,
                EditorAction::ImportAsset => A::Import,
                _ => A::Exit,
            };
            io.request(action, project);
        }
        EditorAction::ExitDemo => {
            io.benchmark = false;
            dispatch(document, &mut ui_state.notice, Command::NewDocument);
            runtime.mode = io.return_mode;
            crate::viewport::focus_geo(viewport);
        }
        EditorAction::AddObject => {
            if io.benchmark {
                return;
            }
            project.placing_marker = true;
            ui_state.workspace.ensure_panel(starter_ui::PanelId::Map);
            ui_state.workspace.activate(starter_ui::PanelId::Map);
        }
        EditorAction::DeleteSelection => {
            if io.benchmark {
                dispatch(document, &mut ui_state.notice, Command::DeleteSelection);
            } else {
                project.delete_selected();
            }
        }
        EditorAction::FocusSelection if !io.benchmark => {
            project.focus_selected = true;
        }
        EditorAction::FocusSelection => {
            if let Some(object) = document.selected_object() {
                viewport.camera.focus(object_center(object));
            }
        }
        EditorAction::ResetView => viewport.camera.reset(bounds(document)),
        EditorAction::ResetLayout => {
            ui_state.workspace.reset();
            ui_state.workspace.ensure_panel(starter_ui::PanelId::Map);
        }
        EditorAction::ToggleProjection => viewport.camera.projection.toggle(),
        EditorAction::ToggleTheme => {
            ui_state.theme = match ui_state.theme {
                Theme::Dark => Theme::Light,
                Theme::Light => Theme::Dark,
            };
        }
        EditorAction::SetLoad(size) => {
            io.benchmark = true;
            project.placing_marker = false;
            ui_state
                .workspace
                .ensure_panel(starter_ui::PanelId::Viewport);
            ui_state.workspace.activate(starter_ui::PanelId::Viewport);
            dispatch(document, &mut ui_state.notice, Command::LoadTestScene(size));
            viewport.camera.reset(bounds(document));
        }
    }
}

fn dispatch(document: &mut DocumentResource, notice: &mut Option<String>, command: Command) {
    match document.dispatch(command) {
        Ok(_) => *notice = None,
        Err(error) => *notice = Some(error.to_string()),
    }
}

#[cfg(test)]
mod action_tests;

#[cfg(test)]
mod project_ui_tests;

#[cfg(test)]
mod project_inspector_tests;
