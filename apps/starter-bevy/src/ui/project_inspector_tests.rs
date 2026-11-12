//! Synthetic widget regression; not a manual GUI smoke test.
use super::project_inspector::InspectorState;
use bevy_egui::egui::{Key, Modifiers, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use starter_core::{ObjectId, ProjectSelection};

#[test]
fn names_keep_invalid_drafts_commit_on_enter_or_blur_and_follow_selection() {
    let mut harness = Harness::new_ui_state(
        |ui, (inspector, selection, name): &mut (InspectorState, ProjectSelection, String)| {
            if let Some(value) = inspector.name(ui, selection, name) {
                *name = value;
            }
            ui.button("Other field").clicked();
        },
        (
            InspectorState::default(),
            ProjectSelection::Project,
            "Project".into(),
        ),
    );
    harness.run();
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    assert_eq!(harness.state().2, "Project");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().2, "Project");
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("")
    );
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness
        .get_by_role(Role::TextInput)
        .type_text("Renamed project");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().2, "Renamed project");

    harness.state_mut().1 = ProjectSelection::Marker(ObjectId(1));
    harness.state_mut().2 = "Marker 1".into();
    harness.run();
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Marker 1")
    );
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by_role(Role::TextInput)
        .type_text("Survey point");
    harness.run();
    assert_eq!(harness.state().2, "Marker 1");
    harness.get_by_label("Other field").click();
    harness.run();
    assert_eq!(harness.state().2, "Survey point");
}
