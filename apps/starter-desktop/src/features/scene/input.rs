use eframe::egui::{self, Rect, Response, Ui};
use starter_core::{Command, DocumentState, ObjectId};

use super::{DragTarget, ViewportState, picking};

pub fn handle(
    ui: &mut Ui,
    response: &Response,
    rect: Rect,
    state: &mut DocumentState,
    viewport: &mut ViewportState,
) {
    let pointer = response.interact_pointer_pos();
    if response.drag_started() {
        viewport.dragging = pointer
            .and_then(|pos| picking::object_at(pos, rect, state.document(), &viewport.camera))
            .map_or(DragTarget::Pan, DragTarget::Object);
        if let DragTarget::Object(id) = viewport.dragging {
            state.dispatch(Command::Select([id].into())).ok();
        }
    }
    if response.dragged() {
        let delta = ui.input(|input| input.pointer.delta());
        match viewport.dragging {
            DragTarget::Pan => viewport.camera.pan_by(delta),
            DragTarget::Object(id) => move_object(state, id, delta, viewport.camera.zoom()),
        }
    } else if response.clicked() {
        let selected = pointer
            .and_then(|pos| picking::object_at(pos, rect, state.document(), &viewport.camera))
            .into_iter()
            .collect();
        state.dispatch(Command::Select(selected)).ok();
    }
    if response.contains_pointer()
        && let Some(pointer) = pointer
    {
        let scroll = ui.input(|input| {
            input
                .raw
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::MouseWheel { unit, delta, .. } => Some(match unit {
                        egui::MouseWheelUnit::Point => delta.y,
                        egui::MouseWheelUnit::Line => delta.y * 40.0,
                        egui::MouseWheelUnit::Page => delta.y * rect.height(),
                    }),
                    _ => None,
                })
                .sum::<f32>()
        });
        if scroll != 0.0 {
            viewport.camera.zoom_at(pointer, rect.center(), -scroll);
        }
    }
}

fn move_object(state: &mut DocumentState, id: ObjectId, delta: egui::Vec2, zoom: f32) {
    if let Some(object) = state.object(id) {
        let current = object.position();
        let next = starter_core::Position {
            x: current.x + delta.x as f64 / zoom as f64,
            y: current.y - delta.y as f64 / zoom as f64,
        };
        state.dispatch(Command::SetPosition(id, next)).ok();
    }
}

#[cfg(test)]
mod tests {
    use starter_core::{Command, DocumentState, GroupId};

    use super::*;

    #[test]
    fn dragging_up_moves_the_object_up_in_world_coordinates() {
        let mut state = DocumentState::default();
        let id = state
            .dispatch(Command::AddObject(GroupId::A))
            .unwrap()
            .unwrap();
        move_object(&mut state, id, egui::vec2(0.0, -20.0), 2.0);
        assert_eq!(state.object(id).unwrap().position().y, 10.0);
    }
}
