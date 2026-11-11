use starter_core::{Command, DocumentError, DocumentState, ObjectId};

pub const PREFERENCES_KEY: &str = "starter_preferences";

pub fn dispatch(
    state: &mut DocumentState,
    notice: &mut Option<String>,
    command: Command,
) -> Option<ObjectId> {
    match state.dispatch(command) {
        Ok(result) => {
            *notice = None;
            result
        }
        Err(error) => {
            *notice = Some(error.to_string());
            None
        }
    }
}

pub fn load_document(
    state: &mut DocumentState,
    document: starter_core::Document,
) -> Result<(), DocumentError> {
    state
        .dispatch(Command::ReplaceDocument(document))
        .map(|_| ())
}

pub fn is_text_edit_active(ctx: &egui::Context) -> bool {
    ctx.egui_wants_keyboard_input()
}
