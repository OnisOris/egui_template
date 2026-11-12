//! Synthetic widget checks, not a manual UX acceptance test.
use bevy_egui::egui::{Key, Modifiers, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use starter_core::{GeoOrigin, ProjectAsset, ProjectSelection, ProjectState};
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
fn model_name_commits_on_selection_change_and_invalid_draft_survives_returning() {
    let mut project = project();
    let point = GeoOrigin {
        longitude: 8.0,
        latitude: 46.0,
    };
    let marker = project.add_marker(point).unwrap();
    let model = project
        .add_model("Assets/Models/box.glb".into(), point)
        .unwrap();
    let mut harness =
        Harness::new_ui_state(
            |ui,
             (project, inspector): &mut (
                ProjectState,
                super::project_inspector::InspectorState,
            )| {
                inspector.sync(project, 0);
                let current = project
                    .document
                    .object_rows()
                    .find(|m| m.selection == project.selection)
                    .map(|m| m.name.to_owned())
                    .unwrap();
                if let Some(name) = inspector.name(ui, &project.selection, &current) {
                    match project.selection {
                        ProjectSelection::Model(_) => {
                            let mut m = project.document.models[0].clone();
                            m.name = name;
                            project.edit_model(m).unwrap();
                        }
                        ProjectSelection::Marker(_) => {
                            let mut m = project.document.objects[0].clone();
                            m.name = name;
                            project.edit_marker(m).unwrap();
                        }
                        _ => {}
                    }
                }
            },
            (project, super::project_inspector::InspectorState::default()),
        );
    harness.run();
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role(Role::TextInput).type_text("Survey GLB");
    harness.run();
    harness.state_mut().0.selection = ProjectSelection::Marker(marker);
    harness.run();
    assert_eq!(harness.state().0.document.models[0].name, "Survey GLB");
    harness.state_mut().0.selection = ProjectSelection::Model(model);
    harness.run();
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Enter);
    harness.run();
    harness.state_mut().0.selection = ProjectSelection::Marker(marker);
    harness.run();
    harness.state_mut().0.selection = ProjectSelection::Model(model);
    harness.run();
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("")
    );
    assert_eq!(harness.state().0.document.models[0].name, "Survey GLB");
}

#[test]
fn place_on_map_remains_clickable_below_a_scrolling_explorer() {
    let mut project = crate::project::Project(project());
    project.selection = ProjectSelection::Asset(0);
    let mut explorer = super::project_explorer::ExplorerState::default();
    explorer.files = true;
    let mut harness = Harness::builder().with_size((280.0, 360.0)).build_ui_state(
        |ui,
         (project, explorer, io): &mut (
            crate::project::Project,
            super::project_explorer::ExplorerState,
            crate::project::ProjectIo,
        )| {
            super::project_explorer::show(
                ui,
                explorer,
                project,
                io,
                &crate::map::real_scene::RealScene::default(),
                &crate::map::loading::TerrainImport::default(),
            );
        },
        (project, explorer, crate::project::ProjectIo::default()),
    );
    harness.run();
    harness.get_by_label("Place on Map").click();
    harness.run();
    assert_eq!(
        harness.state().0.placing_model.as_deref(),
        Some("Assets/Models/box.glb")
    );
    assert!(harness.state().0.document.models.is_empty());
}
