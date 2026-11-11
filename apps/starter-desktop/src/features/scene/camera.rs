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
    fn new_camera_centers_the_scene_in_the_viewport() {
        let camera = Camera::default();
        let center = Pos2::new(800.0, 500.0);
        assert_eq!(camera.world_to_screen(Position::default(), center), center);
    }

    #[test]
    fn viewport_resize_keeps_the_view_center_on_the_same_world_point() {
        let mut camera = Camera::default();
        camera.pan_by(Vec2::new(40.0, -20.0));
        let before = Pos2::new(640.0, 400.0);
        let after = Pos2::new(570.0, 400.0);
        let world_at_center = camera.screen_to_world(before, before);

        // The camera is relative to the viewport center, so resizing recenters
        // the same world point instead of pinning it to an old screen position.
        assert_eq!(camera.world_to_screen(world_at_center, after), after);
    }

    #[test]
    fn reset_centers_the_scene_again() {
        let mut camera = Camera::default();
        let center = Pos2::new(640.0, 400.0);
        camera.focus(Position { x: 120.0, y: -30.0 });
        camera.reset();
        assert_eq!(camera.world_to_screen(Position::default(), center), center);
    }
}
