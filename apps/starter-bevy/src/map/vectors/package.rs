use super::{clip, data::*};
use crate::map::{satellite::MapManifest, source::read_limited};
use serde::{Deserialize, Serialize};
use starter_core::MapRegion;
use std::path::Path;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorManifest {
    pub provider: String,
    pub zoom: u8,
    pub tile_count: u32,
    pub road_count: u32,
    pub building_count: u32,
    pub roads_file: String,
    pub buildings_file: String,
    pub binary_format: String,
    pub attribution: String,
}
#[derive(Deserialize)]
struct Manifest {
    #[serde(flatten)]
    map: MapManifest,
    vectors: VectorManifest,
}
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let (value, consumed) = bincode::serde::decode_from_slice(
        bytes,
        bincode::config::standard().with_limit::<67_108_864>(),
    )
    .map_err(|_| "Invalid or oversized vector bincode")?;
    if consumed != bytes.len() {
        return Err("Trailing vector binary data".into());
    }
    Ok(value)
}
pub fn load(path: &Path, region: MapRegion) -> Result<VectorData, String> {
    let bytes = read_limited(&path.join("manifest.json"), 1_048_576)?;
    let m: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    m.map.validate(None)?;
    if m.map.bbox != region.bounds || m.map.origin != region.origin {
        return Err("Vector package bbox/origin differs from the confirmed region".into());
    }
    let v = m.vectors;
    if v.binary_format != "bincode-v2-serde"
        || v.roads_file != "roads.bin"
        || v.buildings_file != "buildings.bin"
        || v.provider.is_empty()
        || v.attribution.is_empty()
        || v.zoom > 30
        || u64::from(v.road_count) + u64::from(v.building_count) > MAX_FEATURES as u64
    {
        return Err("Unsupported vector manifest or feature count".into());
    }
    let roads = read_limited(&path.join("roads.bin"), 67_108_864)?;
    let buildings = read_limited(&path.join("buildings.bin"), 67_108_864)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes);
    hasher.update(&roads);
    hasher.update(&buildings);
    let data = VectorData {
        region,
        source: v.provider,
        attribution: v.attribution,
        package_id: m.map.map_data_id,
        fingerprint: hasher.finalize(),
        roads: decode(&roads)?,
        buildings: decode(&buildings)?,
    };
    if data.roads.len() != v.road_count as usize
        || data.buildings.len() != v.building_count as usize
    {
        return Err("Vector manifest counts differ from binaries".into());
    }
    clip::validate(&data)?;
    Ok(data)
}
