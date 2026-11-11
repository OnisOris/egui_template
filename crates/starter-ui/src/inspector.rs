use egui::{self, Color32, Ui};
use starter_core::{Command, DocumentState, MAX_NAME_LENGTH, ObjectId};

#[derive(Default)]
pub struct InspectorState {
    name_draft: Option<(ObjectId, String)>,
}

impl InspectorState {
    pub fn draft_id(&self) -> Option<ObjectId> {
        self.name_draft.as_ref().map(|(id, _)| *id)
    }

    pub fn draft_name(&self) -> Option<&str> {
        self.name_draft.as_ref().map(|(_, name)| name.as_str())
    }
}

pub fn show(ui: &mut Ui, state: &DocumentState, inspector: &mut InspectorState) -> Vec<Command> {
    ui.heading("Inspector");
    let Some(object) = state.selected_object().cloned() else {
        inspector.name_draft = None;
        ui.add_space(16.0);
        ui.label("Select an object to inspect its properties.");
        return Vec::new();
    };
    let mut commands = Vec::new();
    let id = object.id();
    ui.small(format!("ID {}", id.0));
    if inspector
        .name_draft
        .as_ref()
        .is_none_or(|(draft_id, _)| *draft_id != id)
    {
        inspector.name_draft = Some((id, object.name().to_owned()));
    }
    let (_, name) = inspector
        .name_draft
        .as_mut()
        .expect("draft initialized above");
    let response = ui.text_edit_singleline(name);
    if response.lost_focus() {
        commands.push(Command::SetName(id, name.clone()));
    } else if !response.has_focus() && valid_name(name) && name != object.name() {
        // Follow external renames, but keep an invalid draft so it stays editable.
        *name = object.name().to_owned();
        // The widget already drew the old value this frame.
        ui.ctx().request_repaint();
    }
    let mut x = object.position().x;
    let mut y = object.position().y;
    let mut position_changed = false;
    ui.horizontal(|ui| {
        ui.label("Position");
        position_changed |= ui.add(egui::DragValue::new(&mut x).prefix("X ")).changed();
        position_changed |= ui.add(egui::DragValue::new(&mut y).prefix("Y ")).changed();
    });
    if position_changed {
        commands.push(Command::SetPosition(id, starter_core::Position { x, y }));
    }
    let mut size = object.size();
    if ui
        .add(egui::Slider::new(&mut size, 1.0..=200.0).text("Size"))
        .changed()
    {
        commands.push(Command::SetSize(id, size));
    }
    let [r, g, b, a] = object.color();
    let mut color = Color32::from_rgba_unmultiplied(r, g, b, a);
    if ui.color_edit_button_srgba(&mut color).changed() {
        commands.push(Command::SetColor(
            id,
            [color.r(), color.g(), color.b(), color.a()],
        ));
    }
    let mut visible = object.visible();
    if ui.checkbox(&mut visible, "Visible").changed() {
        commands.push(Command::SetVisible(id, visible));
    }
    let mut enabled = object.enabled();
    if ui.checkbox(&mut enabled, "Enabled").changed() {
        commands.push(Command::SetEnabled(id, enabled));
    }
    commands
}

fn valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.len() <= MAX_NAME_LENGTH
}
