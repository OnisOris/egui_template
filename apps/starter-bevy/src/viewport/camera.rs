use bevy::prelude::*;

use crate::scene::SceneBounds;

pub const FOV: f32 = std::f32::consts::FRAC_PI_4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}

impl ProjectionMode {
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Perspective => Self::Orthographic,
            Self::Orthographic => Self::Perspective,
        };
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Orthographic => "Orthographic",
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub projection: ProjectionMode,
    pub sensitivity: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: -0.7,
            pitch: 0.55,
            distance: 300.0,
            projection: ProjectionMode::Perspective,
            sensitivity: 1.0,
            min_distance: 1.0,
            max_distance: 20_000.0,
        }
    }
}

impl OrbitCamera {
    pub fn eye(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        self.target
            + Vec3::new(
                self.distance * cos_pitch * sin_yaw,
                self.distance * sin_pitch,
                self.distance * cos_pitch * cos_yaw,
            )
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.target, Vec3::Y)
    }

    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = (self.target - self.eye()).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        (right, up, forward)
    }

    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.008 * self.sensitivity;
        self.pitch = (self.pitch + delta.y * 0.008 * self.sensitivity).clamp(-1.45, 1.45);
    }

    pub fn pan(&mut self, delta: Vec2, viewport_height: f32) {
        let (right, up, _) = self.basis();
        let scale = self.distance * 0.0015 * self.sensitivity * 480.0 / viewport_height.max(1.0);
        self.target += (-right * delta.x + up * delta.y) * scale;
    }

    pub fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance * (-scroll * 0.0015 * self.sensitivity).exp())
            .clamp(self.min_distance, self.max_distance);
    }

    pub fn reset(&mut self, bounds: SceneBounds) {
        let projection = self.projection;
        *self = Self {
            projection,
            ..Self::default()
        };
        self.fit(bounds);
    }

    pub fn fit(&mut self, bounds: SceneBounds) {
        self.target = bounds.center;
        self.distance =
            (bounds.radius / (FOV * 0.5).tan() * 0.9).clamp(self.min_distance, self.max_distance);
    }

    pub fn focus(&mut self, position: Vec3) {
        self.target = position;
    }

    pub fn projection(&self, aspect: f32, viewport_height: f32) -> Projection {
        match self.projection {
            ProjectionMode::Perspective => Projection::Perspective(PerspectiveProjection {
                fov: FOV,
                aspect_ratio: aspect,
                near: 0.05,
                far: 100_000.0,
                ..default()
            }),
            ProjectionMode::Orthographic => {
                let half_height = self.distance * (FOV * 0.5).tan();
                Projection::Orthographic(OrthographicProjection {
                    near: 0.0,
                    far: 100_000.0,
                    scale: 2.0 * half_height / viewport_height.max(1.0),
                    ..OrthographicProjection::default_3d()
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_sits_at_the_orbit_distance() {
        let camera = OrbitCamera::default();
        assert!((camera.eye().distance(camera.target) - camera.distance).abs() < 0.001);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut camera = OrbitCamera::default();
        camera.zoom(100_000.0);
        assert_eq!(camera.distance, camera.min_distance);
        camera.zoom(-100_000.0);
        assert_eq!(camera.distance, camera.max_distance);
    }

    #[test]
    fn projection_can_be_toggled() {
        let mut camera = OrbitCamera::default();
        camera.projection.toggle();
        assert_eq!(camera.projection, ProjectionMode::Orthographic);
        assert!(matches!(
            camera.projection(1.5, 600.0),
            Projection::Orthographic(_)
        ));
        camera.projection.toggle();
        assert_eq!(camera.projection, ProjectionMode::Perspective);
    }

    #[test]
    fn reset_restores_the_default_orbit() {
        let mut camera = OrbitCamera::default();
        camera.orbit(Vec2::new(200.0, -300.0));
        camera.projection.toggle();
        camera.reset(SceneBounds {
            center: Vec3::new(5.0, 0.0, 5.0),
            radius: 50.0,
        });
        assert_eq!(camera.yaw, OrbitCamera::default().yaw);
        assert_eq!(camera.pitch, OrbitCamera::default().pitch);
        assert_eq!(camera.projection, ProjectionMode::Orthographic);
        assert_eq!(camera.target, Vec3::new(5.0, 0.0, 5.0));
    }

    #[test]
    fn fit_centers_the_target() {
        let mut camera = OrbitCamera::default();
        let bounds = SceneBounds {
            center: Vec3::new(12.0, 0.0, -4.0),
            radius: 500.0,
        };
        camera.fit(bounds);
        assert_eq!(camera.target, bounds.center);
        assert!(camera.distance > 0.0);
    }
}
