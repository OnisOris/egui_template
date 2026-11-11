use bevy::prelude::*;
use starter_core::{DocumentState, ObjectId, SceneObject};

use super::camera::{FOV, OrbitCamera, ProjectionMode};

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub fn screen_ray(camera: &OrbitCamera, pointer: Vec2, viewport: Vec2, aspect: f32) -> Ray {
    let ndc = Vec3::new(
        2.0 * pointer.x / viewport.x.max(1.0) - 1.0,
        1.0 - 2.0 * pointer.y / viewport.y.max(1.0),
        0.0,
    );
    let (right, up, forward) = camera.basis();
    let half_height = camera.distance * (FOV * 0.5).tan();
    match camera.projection {
        ProjectionMode::Perspective => Ray {
            origin: camera.eye(),
            direction: (forward + right * ndc.x * half_height * aspect + up * ndc.y * half_height)
                .normalize(),
        },
        ProjectionMode::Orthographic => Ray {
            origin: camera.eye() + right * ndc.x * half_height * aspect + up * ndc.y * half_height,
            direction: forward,
        },
    }
}

pub fn ray_plane_y(ray: &Ray, y: f32) -> Option<Vec3> {
    if ray.direction.y.abs() < 1e-5 {
        return None;
    }
    let t = (y - ray.origin.y) / ray.direction.y;
    (t > 0.0).then(|| ray.origin + ray.direction * t)
}

pub fn object_center(object: &SceneObject) -> Vec3 {
    let position = object.position();
    Vec3::new(position.x as f32, object.size() * 0.5, -(position.y as f32))
}

pub fn ray_sphere(ray: &Ray, center: Vec3, radius: f32) -> Option<f32> {
    let offset = center - ray.origin;
    let along = offset.dot(ray.direction);
    if along < 0.0 {
        return None;
    }
    let perpendicular2 = offset.length_squared() - along * along;
    let radius2 = radius * radius;
    if perpendicular2 > radius2 {
        return None;
    }
    Some((along - (radius2 - perpendicular2).sqrt()).max(0.0))
}

pub fn pick_object(document: &DocumentState, ray: &Ray) -> Option<ObjectId> {
    document
        .document()
        .objects()
        .iter()
        .filter_map(|object| {
            let distance = ray_sphere(ray, object_center(object), object.size() * 0.8)?;
            Some((distance, object.id()))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_core::{Command, Position};

    fn camera_looking_at_origin() -> OrbitCamera {
        OrbitCamera {
            target: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 100.0,
            ..default()
        }
    }

    #[test]
    fn center_screen_ray_points_at_the_target() {
        let camera = camera_looking_at_origin();
        let ray = screen_ray(
            &camera,
            Vec2::new(400.0, 300.0),
            Vec2::new(800.0, 600.0),
            4.0 / 3.0,
        );
        assert!(ray.direction.dot(Vec3::NEG_Z) > 0.99);
    }

    #[test]
    fn picking_hits_the_object_at_the_center() {
        let mut document = DocumentState::default();
        document.dispatch(Command::NewDocument).unwrap();
        document
            .dispatch(Command::AddObject(starter_core::GroupId::A))
            .unwrap();
        let id = *document.selection().first().unwrap();
        document
            .dispatch(Command::SetPosition(id, Position { x: 0.0, y: 0.0 }))
            .unwrap();
        document.dispatch(Command::SetSize(id, 20.0)).unwrap();
        let camera = camera_looking_at_origin();
        let ray = screen_ray(
            &camera,
            Vec2::new(400.0, 300.0),
            Vec2::new(800.0, 600.0),
            4.0 / 3.0,
        );
        assert_eq!(pick_object(&document, &ray), Some(id));
    }

    #[test]
    fn ray_plane_intersection_returns_a_point() {
        let ray = Ray {
            origin: Vec3::new(0.0, 10.0, 0.0),
            direction: Vec3::NEG_Y,
        };
        assert_eq!(ray_plane_y(&ray, 0.0), Some(Vec3::ZERO));
    }
}
