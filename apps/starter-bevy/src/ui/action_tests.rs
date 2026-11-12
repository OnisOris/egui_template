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
