//! Read-only UAV terrain package v1 adapter. No network or UAV workspace dependencies.
use super::{
    coordinates::wgs84_to_local,
    region::{GeoBounds, GeoOrigin, MapRegion},
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};

pub const MAX_SAMPLES: usize = 2_097_152;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainManifest {
    pub format_version: u32,
    pub terrain_id: String,
    pub bbox: GeoBounds,
    pub origin: GeoOrigin,
    pub width: u32,
    pub height: u32,
    pub min_x_m: f64,
    pub min_y_m: f64,
    pub spacing_x_m: f64,
    pub spacing_y_m: f64,
    pub reference_elevation_msl: f32,
    pub reference_elevation_source: String,
    pub min_elevation_msl: f32,
    pub max_elevation_msl: f32,
    pub encoding: String,
    pub source: String,
    pub attribution: String,
}

#[derive(Debug)]
pub struct TerrainPayload {
    pub manifest: TerrainManifest,
    pub samples_msl: Vec<f32>,
    pub fingerprint: blake3::Hash,
}

impl TerrainManifest {
    pub fn region(&self) -> MapRegion {
        MapRegion {
            bounds: self.bbox,
            origin: self.origin,
        }
    }

    pub fn matches(&self, region: MapRegion) -> bool {
        let a = self.region();
        let b = region;
        [
            a.bounds.west - b.bounds.west,
            a.bounds.east - b.bounds.east,
            a.bounds.south - b.bounds.south,
            a.bounds.north - b.bounds.north,
            a.origin.longitude - b.origin.longitude,
            a.origin.latitude - b.origin.latitude,
        ]
        .into_iter()
        .all(|d| d.abs() <= 1e-9)
    }

    fn validate(&self) -> Result<usize, String> {
        let fail = |s: &str| Err(s.to_owned());
        if self.format_version != 1 || self.encoding != "little-endian-f32-row-major" {
            return fail("Unsupported terrain format: expected v1 / little-endian-f32-row-major.");
        }
        MapRegion::new(self.bbox, self.origin).map_err(String::from)?;
        let count = (self.width as usize)
            .checked_mul(self.height as usize)
            .filter(|n| *n <= MAX_SAMPLES && self.width >= 2 && self.height >= 2)
            .ok_or("DEM must be at least 2×2 and at most 2,097,152 samples.")?;
        if self.terrain_id.trim().is_empty()
            || self.source.trim().is_empty()
            || !["origin-sample", "fallback-mean"]
                .contains(&self.reference_elevation_source.as_str())
        {
            return fail("Missing package ID/source or unsupported reference elevation source.");
        }
        if [
            self.min_x_m,
            self.min_y_m,
            self.spacing_x_m,
            self.spacing_y_m,
            f64::from(self.reference_elevation_msl),
            f64::from(self.min_elevation_msl),
            f64::from(self.max_elevation_msl),
        ]
        .into_iter()
        .any(|v| !v.is_finite())
            || self.spacing_x_m < 0.01
            || self.spacing_y_m < 0.01
            || self.min_elevation_msl > self.max_elevation_msl
        {
            return fail("Nonfinite or invalid terrain grid/elevation metadata.");
        }
        if [
            self.reference_elevation_msl,
            self.min_elevation_msl,
            self.max_elevation_msl,
        ]
        .into_iter()
        .any(|v| v.abs() > 100_000.0)
        {
            return fail("MSL elevations outside the supported ±100 km range.");
        }
        let nw = wgs84_to_local(self.origin, self.bbox.west, self.bbox.north);
        let se = wgs84_to_local(self.origin, self.bbox.east, self.bbox.south);
        let max_x = self.min_x_m + f64::from(self.width - 1) * self.spacing_x_m;
        let max_y = self.min_y_m + f64::from(self.height - 1) * self.spacing_y_m;
        if [
            self.min_x_m - nw[0],
            self.min_y_m - nw[1],
            max_x - se[0],
            max_y - se[1],
        ]
        .into_iter()
        .any(|d| d.abs() > 0.05)
        {
            return fail("Grid extent does not agree with bbox/origin (UAV v1 coordinates).");
        }
        if [nw[0], nw[1], se[0], se[1]]
            .into_iter()
            .any(|v| v.abs() > 1_000_000.0)
        {
            return fail("Local terrain coordinates exceed the supported ±1000 km float range.");
        }
        let precision = [nw[0], nw[1], se[0], se[1]]
            .into_iter()
            .map(f64::abs)
            .fold(1.0, f64::max)
            * f64::from(f32::EPSILON)
            * 2.0;
        if self.spacing_x_m.min(self.spacing_y_m) < precision {
            return fail("Grid spacing is too small for local f32 coordinate precision.");
        }
        Ok(count)
    }
}

impl TerrainPayload {
    pub fn elevation(&self, x: u32, y: u32) -> f32 {
        self.samples_msl[(y * self.manifest.width + x) as usize]
    }
    pub fn position(&self, x: u32, y: u32) -> [f64; 3] {
        let m = &self.manifest;
        [
            m.min_x_m + f64::from(x) * m.spacing_x_m,
            m.min_y_m + f64::from(y) * m.spacing_y_m,
            f64::from(self.elevation(x, y)) - f64::from(m.reference_elevation_msl),
        ]
    }
}

pub fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Package entry must be a regular file.".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Package file exceeds the supported size limit.".into());
    }
    Ok(bytes)
}

pub fn load_terrain(directory: &Path) -> Result<TerrainPayload, String> {
    let json = read_limited(&directory.join("manifest.json"), 1_048_576)?;
    let manifest: TerrainManifest =
        serde_json::from_slice(&json).map_err(|e| format!("Invalid terrain manifest: {e}"))?;
    let count = manifest.validate()?;
    let bytes = read_limited(&directory.join("heights.f32"), count * 4)?;
    if bytes.len() != count * 4 {
        return Err("heights.f32 size does not match width × height × 4.".into());
    }
    let samples_msl: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    if samples_msl.iter().any(|v| !v.is_finite()) {
        return Err("DEM samples must be finite.".into());
    }
    let min = samples_msl.iter().copied().fold(f32::INFINITY, f32::min);
    let max = samples_msl
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    if (min - manifest.min_elevation_msl).abs() > 0.001
        || (max - manifest.max_elevation_msl).abs() > 0.001
    {
        return Err("Actual elevation range disagrees with manifest.".into());
    }
    let mut hash = blake3::Hasher::new();
    hash.update(&json);
    hash.update(&bytes);
    Ok(TerrainPayload {
        manifest,
        samples_msl,
        fingerprint: hash.finalize(),
    })
}
