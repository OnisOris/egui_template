//! Four-vertex flat ground per existing DEM chunk; switching never edits DEM assets.
use super::{
    dem_mesh::CHUNK_CELLS,
    real_scene::{RealScene, SceneAssets},
    satellite::Georeference,
};
use crate::project::Project;
use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::Aabb,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use starter_core::GeoViewMode;

#[derive(Resource, Default)]
pub struct CityGround {
    terrain: Option<blake3::Hash>,
    imagery: Option<blake3::Hash>,
    mode: Option<GeoViewMode>,
    meshes: Vec<Handle<Mesh>>,
}
fn quad(data: &super::source::TerrainPayload, x: u32, y: u32, g: Option<&Georeference>) -> Mesh {
    let m = &data.manifest;
    let x1 = (x + CHUNK_CELLS).min(m.width - 1);
    let y1 = (y + CHUNK_CELLS).min(m.height - 1);
    let local = [(x, y), (x1, y), (x, y1), (x1, y1)].map(|(x, y)| data.position(x, y));
    let points = local.map(|p| [p[0] as f32, 0., p[1] as f32]);
    let uvs = local.map(|p| {
        let (lon, lat) = super::coordinates::local_to_wgs84(m.origin, p[0], p[1]);
        g.map(|g| g.uv(lon, lat)).unwrap_or([0.; 2])
    });
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec())
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 1, 2, 3]))
}
pub fn update(
    mut commands: Commands,
    project: Res<Project>,
    mut scene: ResMut<RealScene>,
    mut ground: ResMut<CityGround>,
    mut viewport: ResMut<crate::viewport::ViewportState>,
    mut assets: SceneAssets,
) {
    let fingerprint = scene.fingerprint();
    let imagery = scene.satellite.as_ref().map(|s| s.fingerprint);
    if ground.terrain != fingerprint {
        for h in ground.meshes.drain(..) {
            assets.meshes.remove(h.id());
        }
        ground.terrain = fingerprint;
        ground.mode = None;
        ground.imagery = imagery;
        if let Some(data) = &scene.data {
            let g = scene.satellite.as_ref().map(|s| &s.metadata.georeference);
            for y in (0..data.manifest.height - 1).step_by(CHUNK_CELLS as usize) {
                for x in (0..data.manifest.width - 1).step_by(CHUNK_CELLS as usize) {
                    ground.meshes.push(assets.meshes.add(quad(data, x, y, g)));
                }
            }
        }
    }
    if ground.imagery != imagery {
        ground.imagery = imagery;
        if let (Some(data), Some(s)) = (&scene.data, &scene.satellite) {
            let mut i = 0;
            for y in (0..data.manifest.height - 1).step_by(CHUNK_CELLS as usize) {
                for x in (0..data.manifest.width - 1).step_by(CHUNK_CELLS as usize) {
                    let mesh = quad(data, x, y, Some(&s.metadata.georeference));
                    if let Some(mut target) = assets.meshes.get_mut(&ground.meshes[i]) {
                        *target = mesh;
                    }
                    i += 1;
                }
            }
        }
    }
    let mode = project.document.geo_view_mode;
    scene.view_mode = mode;
    if ground.mode != Some(mode) {
        ground.mode = Some(mode);
        if let Some(data) = &scene.data {
            viewport.camera.target.y = if mode == GeoViewMode::City {
                0.
            } else {
                super::vectors::mesh::ground(
                    data,
                    super::vectors::data::LocalPoint2 {
                        x: f64::from(viewport.camera.target.x),
                        y: f64::from(viewport.camera.target.z),
                    },
                )
                .unwrap_or(0.)
            };
            if let Some(bounds) = &mut viewport.scene_bounds {
                bounds.center.y = if mode == GeoViewMode::City {
                    0.
                } else {
                    (data.manifest.min_elevation_msl + data.manifest.max_elevation_msl) * 0.5
                        - data.manifest.reference_elevation_msl
                };
            }
        }
        for (i, flat) in ground.meshes.iter().enumerate() {
            if let (Some(e), Some(dem)) = (scene.entities.get(i), scene.meshes.get(i)) {
                commands
                    .entity(*e)
                    .insert(Mesh3d(if mode == GeoViewMode::City {
                        flat.clone()
                    } else {
                        dem.clone()
                    }))
                    .remove::<Aabb>();
            }
        }
        if mode == GeoViewMode::City {
            scene.grid_enabled = false;
        }
    }
    if let Some(h) = scene.materials.first()
        && let Some(mut m) = assets.materials.get_mut(h)
        && m.unlit != (mode == GeoViewMode::City)
    {
        m.unlit = mode == GeoViewMode::City;
    }
}
