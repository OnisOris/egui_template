use eframe::egui::{self, Color32, Ui};
use starter_core::{Command, DocumentState};

pub fn show(ui: &mut Ui, state: &mut DocumentState) {
    ui.heading("Inspector");
    let Some(object) = state.selected_object().cloned() else {
        ui.add_space(16.0);
        ui.label("Select an object to inspect its properties.");
        return;
    };
    ui.small(format!("ID {}", object.id().0));
    let id = object.id();
    let mut name = object.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        state.dispatch(Command::SetName(id, name)).ok();
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
        state
            .dispatch(Command::SetPosition(id, starter_core::Position { x, y }))
            .ok();
    }
    let mut size = object.size();
    if ui
        .add(egui::Slider::new(&mut size, 1.0..=200.0).text("Size"))
        .changed()
    {
        state.dispatch(Command::SetSize(id, size)).ok();
    }
    let [r, g, b, a] = object.color();
    let mut color = Color32::from_rgba_unmultiplied(r, g, b, a);
    if ui.color_edit_button_srgba(&mut color).changed() {
        state
            .dispatch(Command::SetColor(
                id,
                [color.r(), color.g(), color.b(), color.a()],
            ))
            .ok();
    }
    let mut visible = object.visible();
    if ui.checkbox(&mut visible, "Visible").changed() {
        state.dispatch(Command::SetVisible(id, visible)).ok();
    }
    let mut enabled = object.enabled();
    if ui.checkbox(&mut enabled, "Enabled").changed() {
        state.dispatch(Command::SetEnabled(id, enabled)).ok();
    }
}
