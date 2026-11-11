use std::time::Instant;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use starter_core::{Command, GroupId};

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

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MapUi<'w> {
    picker: NonSendMut<'w, crate::map::picker::MapPicker>,
    active: ResMut<'w, crate::map::region::ActiveMapRegion>,
    runtime: ResMut<'w, crate::map::RegionRuntime>,
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
        mut picker,
        mut active,
        mut runtime,
    } = map;
    let RenderUi {
        mut images,
        mut cameras,
    } = targets;
    let started = Instant::now();
    let projection = viewport.camera.projection;
    let actions;
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
        let UiState {
            workspace,
            theme,
            explorer,
            inspector,
            table,
            gallery,
            notice,
        } = &mut *ui_state;
        if ctx.global_style().visuals.dark_mode != matches!(theme, Theme::Dark) {
            theme.apply(ctx);
        }
        actions = chrome::show(
            &mut root,
            workspace,
            &document,
            &metrics,
            projection,
            &mut viewport.camera.sensitivity,
            notice.as_deref(),
        );
        commands = panels::show(
            &mut root,
            panels::Panels {
                picker: &mut picker,
                active: &mut active,
                runtime: &mut runtime,
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
    if viewport_drawn && viewport.fit_demo_pending {
        crate::map::demo::focus_demo(&mut viewport);
        viewport.fit_demo_pending = false;
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
        );
    }
    metrics.ui_ms = started.elapsed().as_secs_f32() * 1000.0;
    Ok(())
}

fn apply(
    action: EditorAction,
    ui_state: &mut UiState,
    document: &mut DocumentResource,
    viewport: &mut ViewportState,
    runtime: &mut crate::map::RegionRuntime,
) {
    if runtime.showing_demo
        && matches!(
            action,
            EditorAction::ResetView | EditorAction::FocusSelection
        )
    {
        crate::map::demo::focus_demo(viewport);
        return;
    }
    if matches!(
        action,
        EditorAction::New | EditorAction::SetLoad(_) | EditorAction::AddObject
    ) {
        runtime.showing_demo = false;
    }
    match action {
        EditorAction::New => {
            dispatch(document, &mut ui_state.notice, Command::NewDocument);
            viewport.camera.reset(bounds(document));
        }
        EditorAction::AddObject => {
            dispatch(
                document,
                &mut ui_state.notice,
                Command::AddObject(GroupId::A),
            );
        }
        EditorAction::DeleteSelection => {
            dispatch(document, &mut ui_state.notice, Command::DeleteSelection);
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
