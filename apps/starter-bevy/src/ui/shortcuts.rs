use crate::state::EditorAction;
use bevy_egui::egui::{Context, Key, Modifiers};
pub fn collect(ctx: &Context, blocked: bool) -> Vec<EditorAction> {
    if blocked || ctx.memory(|m| m.top_modal_layer().is_some()) {
        return Vec::new();
    }
    let typing = ctx.egui_wants_keyboard_input();
    ctx.input_mut(|i| {
        let mut actions = Vec::new();
        for (key, action) in [
            (Key::S, EditorAction::Save),
            (Key::O, EditorAction::Open),
            (Key::N, EditorAction::New),
        ] {
            if i.consume_key(Modifiers::COMMAND, key) {
                actions.push(action);
            }
        }
        if !typing {
            if i.consume_key(Modifiers::NONE, Key::Delete) {
                actions.push(EditorAction::DeleteSelection);
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                actions.push(EditorAction::CancelTool);
            }
        }
        actions
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy_egui::egui::{Event, RawInput};
    #[test]
    fn commands_route_and_blocked_dialogs_do_not_dispatch() {
        let ctx = Context::default();
        for blocked in [false, true] {
            let mut actions = Vec::new();
            let mut output = ctx.run_ui(
                RawInput {
                    events: vec![Event::Key {
                        key: Key::S,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::COMMAND,
                    }],
                    ..Default::default()
                },
                |ui| {
                    actions = collect(ui.ctx(), blocked);
                },
            );
            output.textures_delta.clear();
            assert_eq!(actions.len(), usize::from(!blocked));
            if !blocked {
                assert!(matches!(actions[0], EditorAction::Save));
            }
        }
    }
    #[test]
    fn delete_in_text_input_does_not_delete_the_selected_object() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut h = Harness::new_ui_state(
            |ui, state: &mut (String, Vec<EditorAction>)| {
                ui.text_edit_singleline(&mut state.0);
                state.1 = collect(ui.ctx(), false);
            },
            ("Name".into(), Vec::new()),
        );
        h.run();
        h.get_by_role(bevy_egui::egui::accesskit::Role::TextInput)
            .click();
        h.run();
        h.key_press(Key::Delete);
        h.run();
        assert!(h.state().1.is_empty());
    }
}
