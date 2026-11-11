use bevy::math::{UVec2, Vec2 as BevyVec2};
use bevy_egui::egui::{Align2, Color32, FontId, PointerButton, Rect, Sense, Ui, pos2, vec2};
use starter_core::{Command, DocumentState, Position};

use super::{
    DragTarget, ViewportState,
    picking::{self, screen_ray},
};

pub fn show(
    ui: &mut Ui,
    viewport: &mut ViewportState,
    document: &mut DocumentState,
    notice: &mut Option<String>,
) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let pixels_per_point = ui.ctx().pixels_per_point();
    let size = rect.size() * pixels_per_point;
    viewport.requested_size = Some(UVec2::new(
        size.x.round().max(1.0) as u32,
        size.y.round().max(1.0) as u32,
    ));

    if let Some(texture_id) = viewport.texture_id {
        ui.painter().image(
            texture_id,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
    }
    ui.painter().text(
        rect.left_top() + vec2(8.0, 8.0),
        Align2::LEFT_TOP,
        format!(
            "{} · LMB orbit/drag · RMB pan · wheel zoom",
            viewport.camera.projection.label()
        ),
        FontId::proportional(12.0),
        Color32::from_gray(200),
    );

    let local = ui
        .input(|input| input.pointer.hover_pos())
        .map(|pointer| pointer - rect.min);
    let aspect = rect.width() / rect.height().max(1.0);
    let scroll = ui.input(|input| input.smooth_scroll_delta.y);
    if response.hovered() && scroll != 0.0 {
        debug_input(format_args!(
            "scroll {scroll} distance {}",
            viewport.camera.distance
        ));
        viewport.camera.zoom(scroll);
    }

    if response.drag_started_by(PointerButton::Primary) {
        viewport.drag = Some(match local {
            Some(pointer) => {
                let ray = screen_ray(
                    &viewport.camera,
                    to_vec2(pointer),
                    to_vec2(rect.size()),
                    aspect,
                );
                match picking::pick_object(document, &ray) {
                    Some(id) => {
                        let (center, start_position) = {
                            let object = document.object(id).expect("picked object exists");
                            (picking::object_center(object), object.position())
                        };
                        apply(document, notice, Command::Select([id].into()));
                        DragTarget::Object {
                            id,
                            start_position,
                            start_hit: picking::ray_plane_y(&ray, center.y).unwrap_or(center),
                        }
                    }
                    None => DragTarget::Orbit,
                }
            }
            None => DragTarget::Orbit,
        });
    } else if response.drag_started_by(PointerButton::Secondary) {
        viewport.drag = Some(DragTarget::Pan);
    }

    if response.dragged() {
        let delta = ui.input(|input| input.pointer.delta());
        match viewport.drag {
            Some(DragTarget::Orbit) => viewport.camera.orbit(to_vec2(delta)),
            Some(DragTarget::Pan) => viewport.camera.pan(to_vec2(delta), rect.height()),
            Some(DragTarget::Object {
                id,
                start_position,
                start_hit,
            }) => {
                if let Some(pointer) = local {
                    let ray = screen_ray(
                        &viewport.camera,
                        to_vec2(pointer),
                        to_vec2(rect.size()),
                        aspect,
                    );
                    if let Some(hit) = picking::ray_plane_y(&ray, start_hit.y) {
                        let offset = hit - start_hit;
                        apply(
                            document,
                            notice,
                            Command::SetPosition(
                                id,
                                Position {
                                    x: start_position.x + offset.x as f64,
                                    y: start_position.y - offset.z as f64,
                                },
                            ),
                        );
                    }
                }
            }
            None => {}
        }
    }
    if response.drag_stopped() {
        viewport.drag = None;
    }

    if response.clicked()
        && let Some(pointer) = local
    {
        let ray = screen_ray(
            &viewport.camera,
            to_vec2(pointer),
            to_vec2(rect.size()),
            aspect,
        );
        let selection = picking::pick_object(document, &ray);
        let command = match selection {
            Some(id) => Command::Select([id].into()),
            None => Command::Select(Default::default()),
        };
        apply(document, notice, command);
    }

    if response.double_clicked()
        && let Some(pointer) = local
    {
        let ray = screen_ray(
            &viewport.camera,
            to_vec2(pointer),
            to_vec2(rect.size()),
            aspect,
        );
        if let Some(id) = picking::pick_object(document, &ray)
            && let Some(object) = document.object(id)
        {
            viewport.camera.focus(picking::object_center(object));
        }
    }
}

fn apply(document: &mut DocumentState, notice: &mut Option<String>, command: Command) {
    match document.dispatch(command) {
        Ok(_) => *notice = None,
        Err(error) => *notice = Some(error.to_string()),
    }
}

fn to_vec2(value: bevy_egui::egui::Vec2) -> BevyVec2 {
    BevyVec2::new(value.x, value.y)
}

fn debug_input(args: std::fmt::Arguments<'_>) {
    if std::env::var_os("STARTER_DEBUG_INPUT").is_some() {
        eprintln!("INPUT {args}");
    }
}
