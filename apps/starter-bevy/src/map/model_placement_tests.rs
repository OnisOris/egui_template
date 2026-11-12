//! Synthetic widget checks, not a manual UX acceptance test.
use bevy_egui::egui::{self, Key};
use egui_kittest::Harness;
use starter_core::{ProjectAsset, ProjectSelection, ProjectState};
fn project() -> ProjectState {
    let mut project = ProjectState::default();
    project.document.assets.push(ProjectAsset {
        path: "Assets/Models/box.glb".into(),
        media_type: "model/gltf-binary".into(),
        size: 1,
    });
    project
}
#[test]
fn walkers_places_selects_and_cancels_models_without_a_dem_or_network() {
    let mut project = project();
    project.placing_model = Some(project.document.assets[0].path.clone());
    let mut harness = Harness::new_ui_state(
        |ui, (project, memory): &mut (ProjectState, walkers::MapMemory)| {
            walkers::Map::new(None, memory, walkers::lon_lat(8.0, 46.0)).show(
                ui,
                |ui, response, projector, _| {
                    super::marker_overlay::show(ui, response, projector, project, false);
                },
            );
        },
        (project, walkers::MapMemory::default()),
    );
    harness.run();
    let point = egui::pos2(400.0, 300.0);
    harness.hover_at(point);
    harness.run();
    harness.drag_at(point);
    harness.run_steps(1);
    harness.drop_at(point);
    harness.run();
    assert_eq!(harness.state().0.document.models.len(), 1);
    let model = &harness.state().0.document.models[0];
    let id = model.id;
    assert!((model.longitude - 8.0).abs() < 0.001);
    assert!((model.latitude - 46.0).abs() < 0.001);
    assert_eq!(harness.state().0.selection, ProjectSelection::Model(id));
    assert!(harness.state().0.placing_model.is_none());
    harness.state_mut().0.placing_model = Some("Assets/Models/box.glb".into());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().0.placing_model.is_none());
    assert_eq!(harness.state().0.document.models.len(), 1);
    harness.state_mut().0.selection = ProjectSelection::Project;
    harness.hover_at(point);
    harness.run();
    harness.drag_at(point);
    harness.run_steps(1);
    harness.drop_at(point);
    harness.run();
    assert_eq!(harness.state().0.selection, ProjectSelection::Model(id));
}
