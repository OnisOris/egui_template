//! Derived marker projection. Coordinates and style live only in ProjectDocument.
use super::{
    coordinates::{to_bevy, wgs84_to_local},
    real_scene::RealScene,
    region::ActiveMapRegion,
    source::TerrainPayload,
};
use crate::project::{Project, ProjectIo};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use starter_core::{MapMarker, ObjectId, ProjectSelection};
use std::collections::HashMap;

pub fn terrain_elevation(data: &TerrainPayload, lon: f64, lat: f64) -> Option<f64> {
    let m = &data.manifest;
    let b = m.bbox;
    if !lon.is_finite()
        || !lat.is_finite()
        || lon < b.west
        || lon > b.east
        || lat < b.south
        || lat > b.north
    {
        return None;
    }
    let p = wgs84_to_local(m.origin, lon, lat);
    let x = ((p[0] - m.min_x_m) / m.spacing_x_m).clamp(0.0, f64::from(m.width - 1));
    let y = ((p[1] - m.min_y_m) / m.spacing_y_m).clamp(0.0, f64::from(m.height - 1));
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(m.width - 1);
    let y1 = (y0 + 1).min(m.height - 1);
    let dx = x - f64::from(x0);
    let dy = y - f64::from(y0);
    Some(
        (f64::from(data.elevation(x0, y0)) * (1.0 - dx) + f64::from(data.elevation(x1, y0)) * dx)
            * (1.0 - dy)
            + (f64::from(data.elevation(x0, y1)) * (1.0 - dx)
                + f64::from(data.elevation(x1, y1)) * dx)
                * dy,
    )
}
pub fn position(marker: &MapMarker, data: &TerrainPayload) -> Option<Vec3> {
    let elevation = terrain_elevation(data, marker.longitude, marker.latitude)?;
    let mut p = wgs84_to_local(data.manifest.origin, marker.longitude, marker.latitude);
    p[2] = elevation - f64::from(data.manifest.reference_elevation_msl) + marker.altitude_agl;
    Some(to_bevy(p))
}
#[derive(Component)]
pub struct ProjectedMarker(pub ObjectId);
#[derive(PartialEq)]
struct ProjectionRevision {
    document: u64,
    terrain: Option<blake3::Hash>,
    active: Option<u64>,
    installed: Option<u64>,
    selection: ProjectSelection,
    benchmark: bool,
}
#[derive(Resource, Default)]
pub struct MarkerScene {
    pub entities: HashMap<ObjectId, Entity>,
    materials: HashMap<ObjectId, Handle<StandardMaterial>>,
    mesh: Option<Handle<Mesh>>,
    revision: Option<ProjectionRevision>,
}
pub fn sync(
    mut commands: Commands,
    project: Res<Project>,
    io: Res<ProjectIo>,
    active: Res<ActiveMapRegion>,
    scene: Res<RealScene>,
    mut projection: ResMut<MarkerScene>,
    (mut meshes, mut materials): (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
) {
    let revision = ProjectionRevision {
        document: project.revision,
        terrain: scene.fingerprint(),
        active: active.get().map(|(r, _)| r),
        installed: scene.revision,
        selection: project.selection.clone(),
        benchmark: io.benchmark,
    };
    if projection.revision.as_ref() == Some(&revision) {
        return;
    }
    projection.revision = Some(revision);
    let data = scene
        .data
        .as_ref()
        .filter(|_| !io.benchmark && scene.revision == active.get().map(|(r, _)| r));
    let current: HashMap<_, _> = data
        .map(|d| {
            project
                .document
                .objects
                .iter()
                .filter(|m| m.visible)
                .filter_map(|m| position(m, d).map(|p| (m.id, (m, p))))
                .collect()
        })
        .unwrap_or_default();
    let obsolete: Vec<_> = projection
        .entities
        .keys()
        .filter(|id| !current.contains_key(id))
        .copied()
        .collect();
    for id in obsolete {
        if let Some(e) = projection.entities.remove(&id) {
            commands.entity(e).despawn();
        }
        if let Some(h) = projection.materials.remove(&id) {
            materials.remove(h.id());
        }
    }
    if current.is_empty() {
        return;
    }
    let mesh = projection
        .mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.0)))
        .clone();
    let size = data
        .map(|d| (d.manifest.spacing_x_m.max(d.manifest.spacing_y_m) as f32 * 0.4).clamp(2.0, 25.0))
        .unwrap_or(2.0);
    for (id, (marker, p)) in current {
        let selected = project.selection == ProjectSelection::Marker(id);
        let color = if selected {
            Color::srgb(1.0, 0.85, 0.15)
        } else {
            Color::srgba_u8(
                marker.color[0],
                marker.color[1],
                marker.color[2],
                marker.color[3],
            )
        };
        let material = if let Some(h) = projection.materials.get(&id) {
            if let Some(mut m) = materials.get_mut(h) {
                m.base_color = color;
            }
            h.clone()
        } else {
            let h = materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            });
            projection.materials.insert(id, h.clone());
            h
        };
        let transform = Transform::from_translation(p)
            .with_scale(Vec3::splat(size * if selected { 1.3 } else { 1.0 }));
        if let Some(entity) = projection.entities.get(&id) {
            commands.entity(*entity).insert(transform);
        } else {
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    transform,
                    ProjectedMarker(id),
                    RenderLayers::layer(3),
                ))
                .id();
            projection.entities.insert(id, entity);
        }
    }
}
pub fn focus(
    project: &mut starter_core::ProjectState,
    picker: &mut super::picker::MapPicker,
    scene: &RealScene,
    viewport: &mut crate::viewport::ViewportState,
) {
    if !std::mem::take(&mut project.focus_selected) {
        return;
    }
    if let ProjectSelection::Marker(id) = project.selection
        && let Some(marker) = project.document.objects.iter().find(|m| m.id == id)
    {
        picker
            .view
            .memory
            .center_at(walkers::lon_lat(marker.longitude, marker.latitude));
        if let Some(data) = &scene.data
            && let Some(p) = position(marker, data)
        {
            viewport.camera.focus(p);
            viewport.camera.distance = viewport.camera.distance.min(300.0);
        }
    }
}

pub fn pick(
    project: &starter_core::ProjectState,
    data: &TerrainPayload,
    ray: &crate::viewport::picking::Ray,
    radius: f32,
) -> Option<ObjectId> {
    project
        .document
        .objects
        .iter()
        .filter(|m| m.visible)
        .filter_map(|m| {
            let p = position(m, data)?;
            crate::viewport::picking::ray_sphere(ray, p, radius).map(|distance| (distance, m.id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_uses_msl_reference_and_agl_without_inventing_missing_heights() {
        let dir = super::super::dem_tests::package(3, 3);
        let data = super::super::source::load_terrain(dir.path()).unwrap();
        let b = data.manifest.bbox;
        let center = b.center();
        let marker = MapMarker {
            id: ObjectId(1),
            name: "Survey".into(),
            latitude: center.latitude,
            longitude: center.longitude,
            altitude_agl: 7.0,
            visible: true,
            color: [255; 4],
        };
        let p = position(&marker, &data).unwrap();
        let h = terrain_elevation(&data, marker.longitude, marker.latitude).unwrap();
        assert!(
            (f64::from(p.y) - (h - f64::from(data.manifest.reference_elevation_msl) + 7.0)).abs()
                < 0.001
        );
        assert!(terrain_elevation(&data, b.east + 1.0, b.north).is_none());
        assert!(terrain_elevation(&data, f64::NAN, 0.0).is_none());
    }
}
