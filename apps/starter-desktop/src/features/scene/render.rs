use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke};
use starter_core::{DocumentState, Position};

use super::ViewportState;

pub fn show(
    painter: &Painter,
    rect: Rect,
    document: &DocumentState,
    viewport: &ViewportState,
    dark: bool,
) {
    let grid = if dark {
        Color32::from_gray(47)
    } else {
        Color32::from_gray(215)
    };
    let axis = if dark {
        Color32::from_gray(100)
    } else {
        Color32::from_gray(130)
    };
    painter.rect_filled(
        rect,
        0.0,
        if dark {
            Color32::from_rgb(24, 28, 35)
        } else {
            Color32::from_gray(248)
        },
    );
    draw_grid(painter, rect, viewport, grid, axis);
    for object in document.document().objects() {
        if !object.visible() {
            continue;
        }
        let center = viewport
            .camera
            .world_to_screen(object.position(), rect.center());
        let radius = (object.size() * viewport.camera.zoom()).max(3.0);
        if !rect.expand(radius).contains(center) {
            continue;
        }
        let [r, g, b, a] = object.color();
        let color =
            Color32::from_rgba_unmultiplied(r, g, b, if object.enabled() { a } else { a / 3 });
        painter.circle_filled(center, radius, color);
        if document.selection().contains(&object.id()) {
            painter.circle_stroke(center, radius + 3.0, Stroke::new(2.0, Color32::YELLOW));
        }
    }
}

fn draw_grid(
    painter: &Painter,
    rect: Rect,
    viewport: &ViewportState,
    grid: Color32,
    axis: Color32,
) {
    let camera = &viewport.camera;
    let step = (40.0 * (1.0 / camera.zoom()).log2().round().exp2()).clamp(10.0, 2_000.0);
    let top_left = camera.screen_to_world(rect.left_top(), rect.center());
    let bottom_right = camera.screen_to_world(rect.right_bottom(), rect.center());
    let x_start = (top_left.x / step as f64).floor() as i32;
    let x_end = (bottom_right.x / step as f64).ceil() as i32;
    let y_start = (bottom_right.y / step as f64).floor() as i32;
    let y_end = (top_left.y / step as f64).ceil() as i32;
    for index in x_start..=x_end {
        let x = camera
            .world_to_screen(
                Position {
                    x: index as f64 * step as f64,
                    y: 0.0,
                },
                rect.center(),
            )
            .x;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, if index == 0 { axis } else { grid }),
        );
    }
    for index in y_start..=y_end {
        let y = camera
            .world_to_screen(
                Position {
                    x: 0.0,
                    y: index as f64 * step as f64,
                },
                rect.center(),
            )
            .y;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, if index == 0 { axis } else { grid }),
        );
    }
}
