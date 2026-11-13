use std::time::Instant;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use starter_core::Command;

use crate::state::DocumentResource;

use crate::{
    metrics::EditorMetrics,
    state::{EditorAction, UiState},
    viewport::{self, ViewportCamera, ViewportState},
};

mod actions;
mod appearance;
mod shortcuts;
use actions::apply;
pub mod chrome;
mod diagnostics;
mod empty_states;
pub mod gallery;
mod model_inspector;
pub mod panels;
pub mod project_explorer;
pub mod project_inspector;
mod project_names;
pub mod project_table;

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MapUi<'w> {
    project: ResMut<'w, crate::project::Project>,
    project_io: ResMut<'w, crate::project::ProjectIo>,
    models: Res<'w, crate::models::assets::ModelAssets>,
    model_scene: Res<'w, crate::models::instances::ModelScene>,
    model_pick: ResMut<'w, crate::models::picking::ModelPick>,
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
        models,
        model_scene,
        mut model_pick,
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
        if project_io.busy() || project_io.prompt.is_some() || project_io.error.is_some() {
            root.disable();
        }
        let UiState {
            workspace,
            theme,
            appearance,
            explorer,
            inspector,
            table,
            gallery,
            notice,
            project_explorer,
            marker_table,
            project_inspector,
            ..
        } = &mut *ui_state;
        appearance.apply(ctx, *theme);
        actions = chrome::show(
            &mut root,
            workspace,
            (&document, &project, project_io.benchmark),
            import.loading()
                || import.builder.loading
                || satellite.loading()
                || real.vector_status.contains("Loading")
                || real.vector_status.contains("Building geometry"),
            projection,
            &mut viewport.camera.sensitivity,
            notice.as_deref(),
        );
        actions.extend(shortcuts::collect(
            ctx,
            project_io.busy() || project_io.prompt.is_some() || project_io.error.is_some(),
        ));
        let (panel_commands, panel_actions) = panels::show(
            &mut root,
            panels::Panels {
                project: &mut project,
                project_io: &mut project_io,
                models: (&models, &model_scene),
                model_pick: &mut model_pick,
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
        commands = panel_commands;
        actions.extend(panel_actions);
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
        if matches!(action, EditorAction::CancelTool) {
            picker.selection.clear();
            picker.view.tool = crate::map::picker::Tool::Pan;
        }
        if matches!(action, EditorAction::LoadRegion) {
            crate::map::ui::load_in_3d(&active, &mut runtime, &mut import);
            continue;
        }
        if matches!(action, EditorAction::MapSources) {
            picker.imagery.settings_open = true;
            continue;
        }
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
    crate::map::objects::focus(
        &mut project,
        &mut picker,
        &real,
        &mut viewport,
        &model_scene,
    );
    appearance::show(contexts.ctx_mut()?, &mut ui_state);
    diagnostics::show(
        contexts.ctx_mut()?,
        &mut ui_state.diagnostics,
        &metrics,
        &real,
        &picker,
    );
    metrics.ui_ms = started.elapsed().as_secs_f32() * 1000.0;
    Ok(())
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
mod model_ui_tests;
#[cfg(test)]
mod project_inspector_tests;
