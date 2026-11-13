//! Read-only map-data v2 satellite adapter; projection is delegated to Walkers.
use super::{
    coordinates::local_to_wgs84,
    dem_mesh::CHUNK_CELLS,
    source::{TerrainPayload, read_limited},
};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelBounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Georeference {
    pub zoom: u32,
    pub tile_size: u32,
    pub crop_pixel_bounds: PixelBounds,
}
impl Georeference {
    pub fn uv(&self, lon: f64, lat: f64) -> [f32; 2] {
        let p = walkers::mercator::project(walkers::Position::new(lon, lat), f64::from(self.zoom));
        let scale = f64::from(self.tile_size) / 256.0;
        let b = &self.crop_pixel_bounds;
        [
            ((p.x() * scale - b.left) / (b.right - b.left)) as f32,
            ((p.y() * scale - b.top) / (b.bottom - b.top)) as f32,
        ]
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SatelliteMetadata {
    pub provider: String,
    pub zoom: u32,
    pub width: u32,
    pub height: u32,
    pub georeference: Georeference,
    pub file: String,
    pub media_type: String,
    pub attribution: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapManifest {
    pub format_version: u32,
    pub map_data_id: String,
    pub bbox: super::region::GeoBounds,
    pub origin: super::region::GeoOrigin,
    pub satellite: SatelliteMetadata,
}
impl MapManifest {
    pub fn validate(&self, terrain: Option<&TerrainPayload>) -> Result<(), String> {
        let s = &self.satellite;
        let g = &s.georeference;
        let b = &g.crop_pixel_bounds;
        if self.format_version != 2
            || self.map_data_id.is_empty()
            || self.map_data_id.len() > 256
            || !self
                .map_data_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || s.file != "satellite.jpg"
            || s.media_type != "image/jpeg"
            || s.provider.trim().is_empty()
            || s.attribution.trim().is_empty()
        {
            return Err(
                "Expected map-data v2 with satellite.jpg, JPEG and source attribution.".into(),
            );
        }
        let region = super::region::MapRegion::new(self.bbox, self.origin).map_err(String::from)?;
        if terrain.is_some_and(|t| !t.manifest.matches(region)) {
            return Err(
                "Satellite package does not match the current terrain region (bbox/origin).".into(),
            );
        }
        if s.width == 0
            || s.height == 0
            || s.width > 8192
            || s.height > 8192
            || u64::from(s.width) * u64::from(s.height) > 16_777_216
            || g.zoom > 30
            || g.zoom != s.zoom
            || !(256..=4096).contains(&g.tile_size)
            || !g.tile_size.is_power_of_two()
        {
            return Err("Unsupported satellite dimensions, zoom or tile size (maximum 16 Mpixels / 8192 per edge).".into());
        }
        let world = f64::from(g.tile_size) * 2f64.powi(g.zoom as i32);
        if [b.left, b.top, b.right, b.bottom]
            .into_iter()
            .any(|v| !v.is_finite() || v.fract() != 0.0 || v < 0.0 || v > world)
            || b.right - b.left != f64::from(s.width)
            || b.bottom - b.top != f64::from(s.height)
        {
            return Err("Invalid WebMercator crop pixel bounds or image extent.".into());
        }
        for (lon, lat) in [
            (self.bbox.west, self.bbox.north),
            (self.bbox.east, self.bbox.south),
        ] {
            if g.uv(lon, lat)
                .into_iter()
                .any(|v| !(-1e-6..=1.000001).contains(&v))
            {
                return Err("Satellite crop does not cover the DEM bbox.".into());
            }
        }
        Ok(())
    }
}

pub struct SatellitePayload {
    pub manifest: MapManifest,
    pub rgba: Vec<u8>,
    pub fingerprint: blake3::Hash,
    pub uvs: Vec<Vec<[f32; 2]>>,
}

pub fn load_satellite(
    directory: &Path,
    terrain: &TerrainPayload,
) -> Result<SatellitePayload, String> {
    read_package(directory, Some(terrain))
}

pub fn read_package(
    directory: &Path,
    terrain: Option<&TerrainPayload>,
) -> Result<SatellitePayload, String> {
    let json = read_limited(&directory.join("manifest.json"), 1_048_576)?;
    let manifest: MapManifest =
        serde_json::from_slice(&json).map_err(|e| format!("Invalid map-data manifest: {e}"))?;
    manifest.validate(terrain)?;
    let jpeg = read_limited(&directory.join("satellite.jpg"), 64 * 1024 * 1024)?;
    let mut reader = image::ImageReader::with_format(Cursor::new(&jpeg), image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(manifest.satellite.width);
    limits.max_image_height = Some(manifest.satellite.height);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| format!("Cannot decode satellite JPEG: {e}"))?;
    if decoded.width() != manifest.satellite.width || decoded.height() != manifest.satellite.height
    {
        return Err("JPEG dimensions disagree with satellite manifest.".into());
    }
    let mut payload = SatellitePayload {
        manifest,
        rgba: decoded.into_rgba8().into_raw(),
        fingerprint: {
            let mut hash = blake3::Hasher::new();
            hash.update(&json);
            hash.update(&jpeg);
            hash.finalize()
        },
        uvs: Vec::new(),
    };
    if let Some(terrain) = terrain {
        prepare_uvs(&mut payload, terrain)?;
    }
    Ok(payload)
}

pub fn prepare_uvs(payload: &mut SatellitePayload, terrain: &TerrainPayload) -> Result<(), String> {
    payload.manifest.validate(Some(terrain))?;
    payload.uvs = terrain_uvs(terrain, &payload.manifest.satellite.georeference);
    Ok(())
}

pub fn terrain_uvs(terrain: &TerrainPayload, georeference: &Georeference) -> Vec<Vec<[f32; 2]>> {
    let m = &terrain.manifest;
    let mut uvs = Vec::new();
    for y in (0..m.height - 1).step_by(CHUNK_CELLS as usize) {
        for x in (0..m.width - 1).step_by(CHUNK_CELLS as usize) {
            let mut uv = Vec::new();
            for row in y..=y + CHUNK_CELLS.min(m.height - 1 - y) {
                for col in x..=x + CHUNK_CELLS.min(m.width - 1 - x) {
                    let p = terrain.position(col, row);
                    let (lon, lat) = local_to_wgs84(m.origin, p[0], p[1]);
                    uv.push(georeference.uv(lon, lat));
                }
            }
            uvs.push(uv);
        }
    }
    uvs
}
