use super::*;
use crate::{
    map::{RegionRuntime, SceneMode},
    project::{Project, ProjectIo},
};
#[test]
fn benchmark_actions_preserve_project_and_return_to_its_previous_mode() {
    let mut world = World::new();
    world.init_resource::<Assets<Image>>();
    world.init_resource::<bevy_egui::EguiUserTextures>();
    world.init_resource::<ViewportState>();
    let mut viewport = world.remove_resource::<ViewportState>().unwrap();
    let mut project = Project::default();
    let point = starter_core::GeoOrigin {
        latitude: 46.6,
        longitude: 7.95,
    };
    project.add_marker(point).unwrap();
    let saved = project.document.clone();
    let mut document = DocumentResource::default();
    let mut ui = UiState::default();
    let mut io = ProjectIo::default();
    let mut runtime = RegionRuntime {
        mode: SceneMode::Real,
        ..Default::default()
    };
    apply(
        EditorAction::SetLoad(10_000),
        &mut ui,
        &mut document,
        &mut viewport,
        &mut runtime,
        &mut project,
        &mut io,
    );
    assert!(io.benchmark);
    assert_eq!(document.document().objects().len(), 10_000);
    assert_eq!(project.document, saved);
    apply(
        EditorAction::ExitDemo,
        &mut ui,
        &mut document,
        &mut viewport,
        &mut runtime,
        &mut project,
        &mut io,
    );
    assert!(!io.benchmark);
    assert!(document.document().objects().is_empty());
    assert_eq!(project.document, saved);
    assert_eq!(runtime.mode, SceneMode::Real);
}

#[test]
fn workspace_navigation_keeps_project_coordinates_selection_and_dirty_state() {
    use egui_kittest::{Harness, kittest::Queryable};
    let mut project = starter_core::ProjectState::default();
    project
        .add_marker(starter_core::GeoOrigin {
            latitude: 46.,
            longitude: 8.,
        })
        .unwrap();
    project.dirty = false;
    let before = project.document.clone();
    let selected = project.selection.clone();
    let mut h = Harness::new_ui_state(
        |ui, state: &mut (starter_ui::Workspace, starter_core::ProjectState)| {
            let mut sensitivity = 1.;
            chrome::show(
                ui,
                &mut state.0,
                (&starter_core::DocumentState::empty(), &state.1, false),
                false,
                crate::viewport::ProjectionMode::Perspective,
                &mut sensitivity,
                None,
            );
        },
        (
            starter_ui::Workspace::preset(starter_ui::WorkspacePreset::Scene),
            project,
        ),
    );
    h.run();
    h.get_by_label("Analysis").click();
    h.run();
    assert_eq!(
        h.state().0.selected_preset(),
        Some(starter_ui::WorkspacePreset::Analysis)
    );
    assert_eq!(h.state().1.document, before);
    assert_eq!(h.state().1.selection, selected);
    assert!(!h.state().1.dirty);
    h.get_by_label("3D Scene").click();
    h.run();
    assert_eq!(h.state().1.document, before);
    assert_eq!(h.state().1.selection, selected);
}
