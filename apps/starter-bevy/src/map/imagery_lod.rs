//! DEM chunks are the only LOD units. No terrain mesh or global quadtree is created.
use super::{
    builder::coverage,
    coordinates::local_to_wgs84,
    dem_mesh::CHUNK_CELLS,
    imagery_service::TileService,
    region::{GeoBounds, MapRegion},
    satellite::Georeference,
    source::TerrainPayload,
};
use crate::viewport::ViewportState;
use bevy::{math::Affine2, prelude::*};
#[derive(Clone)]
pub struct Plan {
    pub index: usize,
    pub region: MapRegion,
    pub zoom: u8,
}
pub fn plans(
    terrain: &TerrainPayload,
    service: &TileService,
    viewport: &ViewportState,
    base: u8,
    installed: &std::collections::BTreeMap<usize, u8>,
) -> Vec<Plan> {
    let m = &terrain.manifest;
    let p = service.provider();
    let camera = &viewport.camera;
    let eye = camera.eye();
    let (right, up, forward) = camera.basis();
    let tangent = (crate::viewport::camera::FOV * 0.5).tan();
    let aspect = viewport.size.x as f32 / viewport.size.y.max(1) as f32;
    let mut candidates = Vec::new();
    let mut index = 0;
    for y in (0..m.height - 1).step_by(CHUNK_CELLS as usize) {
        for x in (0..m.width - 1).step_by(CHUNK_CELLS as usize) {
            let a = terrain.position(x, y);
            let b = terrain.position(
                (x + CHUNK_CELLS).min(m.width - 1),
                (y + CHUNK_CELLS).min(m.height - 1),
            );
            let center = Vec3::new(
                ((a[0] + b[0]) * 0.5) as f32,
                ((a[2] + b[2]) * 0.5) as f32,
                ((a[1] + b[1]) * 0.5) as f32,
            );
            let radius = (Vec3::new(
                (b[0] - a[0]) as f32,
                (m.max_elevation_msl - m.min_elevation_msl).abs(),
                (b[1] - a[1]) as f32,
            )
            .length()
                * 0.5)
                .max(1.0);
            let delta = center - eye;
            let depth = delta.dot(forward);
            let span = match camera.projection {
                crate::viewport::ProjectionMode::Perspective => depth.max(0.0),
                crate::viewport::ProjectionMode::Orthographic => camera.distance,
            };
            let visible = depth + radius > 0.0
                && delta.dot(right).abs() < span * tangent * aspect + radius
                && delta.dot(up).abs() < span * tangent + radius;
            if visible {
                let (west, north) = local_to_wgs84(m.origin, a[0], a[1]);
                let (east, south) = local_to_wgs84(m.origin, b[0], b[1]);
                let region = MapRegion {
                    bounds: GeoBounds {
                        west,
                        east,
                        north,
                        south,
                    },
                    origin: m.origin,
                };
                let distance = (delta.length() - radius).max(1.0);
                let distance = match camera.projection {
                    crate::viewport::ProjectionMode::Perspective => distance,
                    crate::viewport::ProjectionMode::Orthographic => camera.distance,
                };
                let step = f64::from(2.0 * distance * tangent / viewport.size.y.max(1) as f32);
                let mut zoom = coverage::zoom(region, p.min_zoom, p.max_zoom, p.tile_size, step);
                if let Some(&old) = installed.get(&index) {
                    let ratio = coverage::resolution(region, old, p.tile_size) / step;
                    if (0.65..=1.6).contains(&ratio) {
                        zoom = old;
                    }
                }
                while zoom > base {
                    let crop = super::builder::imagery::crop(region, zoom, p.tile_size);
                    if crop.right - crop.left <= 2048.0
                        && crop.bottom - crop.top <= 2048.0
                        && coverage::tiles(&crop, zoom, p.tile_size, false, 64).is_ok()
                    {
                        break;
                    }
                    zoom -= 1;
                }
                if zoom > base {
                    candidates.push((
                        distance,
                        Plan {
                            index,
                            region,
                            zoom,
                        },
                    ));
                }
            }
            index += 1;
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    candidates.truncate(4);
    candidates.into_iter().map(|(_, p)| p).collect()
}
/// Map the already installed base UVs to a higher-zoom crop through global pixels.
pub fn transform(base: &Georeference, patch: &Georeference) -> Affine2 {
    let a = &base.crop_pixel_bounds;
    let b = &patch.crop_pixel_bounds;
    let scale = f64::from(patch.tile_size) / f64::from(base.tile_size)
        * 2f64.powi(patch.zoom as i32 - base.zoom as i32);
    Affine2::from_scale_angle_translation(
        Vec2::new(
            ((a.right - a.left) * scale / (b.right - b.left)) as f32,
            ((a.bottom - a.top) * scale / (b.bottom - b.top)) as f32,
        ),
        0.0,
        Vec2::new(
            ((a.left * scale - b.left) / (b.right - b.left)) as f32,
            ((a.top * scale - b.top) / (b.bottom - b.top)) as f32,
        ),
    )
}
