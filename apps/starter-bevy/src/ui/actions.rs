use super::dispatch;
use crate::{
    scene::objects::{bounds, object_center},
    state::{DocumentResource, EditorAction, UiState},
    viewport::ViewportState,
};
use starter_core::Command;
use starter_ui::Theme;
pub(super) fn apply(
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
        EditorAction::MapSources | EditorAction::LoadRegion => {}
        EditorAction::SelectRegion => {
            ui_state.workspace.ensure_panel(starter_ui::PanelId::Map);
            ui_state.workspace.activate(starter_ui::PanelId::Map);
        }
        EditorAction::Diagnostics => ui_state.diagnostics = !ui_state.diagnostics,
        EditorAction::Appearance => ui_state.appearance_open = true,
        EditorAction::Select | EditorAction::CancelTool => {
            project.placing_marker = false;
            project.placing_model = None;
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
            project.placing_model = None;
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
            ui_state.workspace = starter_ui::Workspace::preset(starter_ui::WorkspacePreset::Scene);
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
