use eframe::egui::{Pos2, Vec2};
use starter_core::Position;

#[derive(Default)]
pub struct ViewportState {
    pub camera: Camera,
    pub dragging: DragTarget,
}

#[derive(Default)]
pub struct Camera {
    zoom: f32,
    pan: Vec2,
    viewport_center: Option<Pos2>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum DragTarget {
    #[default]
    Pan,
    Object(starter_core::ObjectId),
}

impl Camera {
    pub fn zoom(&self) -> f32 {
        if self.zoom == 0.0 { 1.0 } else { self.zoom }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn preserve_center(&mut self, center: Pos2) {
        if let Some(previous) = self.viewport_center.replace(center) {
            self.pan += previous - center;
        }
    }

    pub fn world_to_screen(&self, world: Position, center: Pos2) -> Pos2 {
        center + self.pan + Vec2::new(world.x as f32, -world.y as f32) * self.zoom()
    }

    pub fn screen_to_world(&self, screen: Pos2, center: Pos2) -> Position {
        let p = (screen - center - self.pan) / self.zoom();
        Position {
            x: p.x as f64,
            y: -(p.y as f64),
        }
    }

    pub fn zoom_at(&mut self, pointer: Pos2, center: Pos2, scroll: f32) {
        let anchor = self.screen_to_world(pointer, center);
        // Matches egui's ctrl+scroll zoom speed (1 / 200 per scroll point).
        self.zoom = (self.zoom() * (scroll / 200.0).exp()).clamp(0.08, 8.0);
        self.pan = pointer - center - Vec2::new(anchor.x as f32, -anchor.y as f32) * self.zoom;
    }

    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
    }

    pub fn focus(&mut self, position: Position) {
        self.pan = Vec2::new(-position.x as f32, position.y as f32) * self.zoom();
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::Pos2;

    use super::*;

    #[test]
    fn zoom_keeps_the_world_point_under_the_cursor_fixed() {
        let mut camera = Camera::default();
        let center = Pos2::new(500.0, 300.0);
        let pointer = Pos2::new(720.0, 180.0);
        let world = camera.screen_to_world(pointer, center);
        camera.zoom_at(pointer, center, 350.0);
        assert!(camera.world_to_screen(world, center).distance(pointer) < 0.001);
        assert!(camera.zoom() > 1.0);
    }

    #[test]
    fn viewport_resize_keeps_scene_fixed_on_screen() {
        let mut camera = Camera::default();
        let before = Pos2::new(640.0, 400.0);
        let after = Pos2::new(570.0, 400.0);
        let object = Position { x: 80.0, y: 25.0 };
        let position_before = camera.world_to_screen(object, before);
        camera.preserve_center(before);
        camera.preserve_center(after);
        assert!(
            camera
                .world_to_screen(object, after)
                .distance(position_before)
                < 0.001
        );
    }
}
