use super::{clip, data::*, mvt, package};
use crate::map::builder::{Job, cache, coverage, download::Download};
use serde::Deserialize;
use starter_core::MapRegion;
use std::path::Path;
pub const TILEJSON: &str = "https://tiles.openfreemap.org/planet";
#[derive(Deserialize)]
pub struct TileJson {
    pub tiles: Vec<String>,
    pub minzoom: Option<u8>,
    pub maxzoom: Option<u8>,
    #[serde(default)]
    pub attribution: String,
}
pub fn plan(meta: &TileJson, region: MapRegion) -> Result<Vec<coverage::Tile>, String> {
    let min = meta.minzoom.unwrap_or(0).max(12);
    let max = meta.maxzoom.unwrap_or(14).min(14);
    if min > max {
        return Err("Vector source has no supported zoom (12–14)".into());
    }
    for z in (min..=max).rev() {
        if let Ok(t) = coverage::tiles(&coverage::bounds(region, z, 256), z, 256, false, 64) {
            return Ok(t);
        }
    }
    Err("Vector tile limit exceeded; choose a smaller region".into())
}
pub fn load(
    region: MapRegion,
    root: &Path,
    tilejson: &str,
    job: &Job,
) -> Result<VectorData, String> {
    let started = std::time::Instant::now();
    job.check()?;
    let id = cache::id(&("vectors-v1", tilejson, region))?;
    let directory = root.join("vectors").join(&id);
    if let Ok(mut data) = read_cache(&directory, region, &id) {
        data.metrics.processed_cache_hit = true;
        data.metrics.load_ms = started.elapsed().as_secs_f64() * 1000.;
        job.status("Vectors: cache reused".into());
        return Ok(data);
    }
    let http = Download::new(4)?;
    job.status("Vectors: Reading TileJSON…".into());
    let metadata = http.get(tilejson, None, job)?;
    let meta: TileJson =
        serde_json::from_slice(&metadata).map_err(|_| "Invalid vector TileJSON")?;
    let tiles = plan(&meta, region)?;
    let template = meta
        .tiles
        .first()
        .ok_or("Vector TileJSON has no tile templates")?;
    let template = reqwest::Url::parse(tilejson)
        .and_then(|u| u.join(template))
        .map_err(|_| "Invalid vector template URL")?
        .to_string()
        .replace("%7B", "{")
        .replace("%7D", "}");
    if !["{z}", "{x}", "{y}"].iter().all(|s| template.contains(s)) {
        return Err("Unsupported vector tile template".into());
    }
    let tilecache = root.join("vector-tiles").join(cache::id(&template)?);
    let hits = tiles
        .iter()
        .filter(|t| {
            tilecache
                .join(format!("{}/{}/{}.tile", t.z, t.x, t.y))
                .is_file()
        })
        .count();
    let bytes = http.tiles(&template, None, &tiles, &tilecache, job, "Loading vectors")?;
    let decoding = std::time::Instant::now();
    let mut data = mvt::empty(region, id);
    if !meta.attribution.is_empty() && !meta.attribution.contains('<') {
        data.attribution = meta.attribution;
    }
    let mut hasher = blake3::Hasher::new();
    for (tile, bytes) in bytes {
        job.check()?;
        hasher.update(&bytes);
        if let Err(e) = mvt::extract(bytes, tile, &mut data) {
            let _ = std::fs::remove_file(
                tilecache.join(format!("{}/{}/{}.tile", tile.z, tile.x, tile.y)),
            );
            return Err(e);
        }
    }
    mvt::deduplicate(&mut data);
    clip::validate(&data)?;
    data.fingerprint = hasher.finalize();
    data.metrics = VectorMetrics {
        tiles: tiles.len(),
        http_requests: http.requests.load(std::sync::atomic::Ordering::Relaxed),
        tile_cache_hits: hits,
        processed_cache_hit: false,
        load_ms: started.elapsed().as_secs_f64() * 1000.,
        decode_ms: decoding.elapsed().as_secs_f64() * 1000.,
    };
    job.check()?;
    store_cache(&data, &directory)?;
    Ok(data)
}
pub fn store_cache(data: &VectorData, directory: &Path) -> Result<(), String> {
    for (name, bytes) in [
        (
            "roads.bin",
            bincode::serde::encode_to_vec(&data.roads, bincode::config::standard()),
        ),
        (
            "buildings.bin",
            bincode::serde::encode_to_vec(&data.buildings, bincode::config::standard()),
        ),
    ] {
        cache::write_atomic(&directory.join(name), &bytes.map_err(|e| e.to_string())?)?;
    }
    let manifest = serde_json::json!({"region":data.region,"source":data.source,"attribution":data.attribution,
        "tiles":data.metrics.tiles,"skipped_buildings":data.skipped_buildings,"fingerprint":data.fingerprint.to_hex().to_string(),"roads":cache::id(&data.roads)?,"buildings":cache::id(&data.buildings)?});
    cache::write_atomic(
        &directory.join("vectors.json"),
        &serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}
pub fn read_cache(path: &Path, region: MapRegion, id: &str) -> Result<VectorData, String> {
    #[derive(Deserialize)]
    struct Manifest {
        region: MapRegion,
        source: String,
        attribution: String,
        fingerprint: String,
        #[serde(default)]
        skipped_buildings: usize,
        #[serde(default)]
        tiles: usize,
        roads: String,
        buildings: String,
    }
    let m: Manifest = serde_json::from_slice(&crate::map::source::read_limited(
        &path.join("vectors.json"),
        1_048_576,
    )?)
    .map_err(|e| e.to_string())?;
    if !m.region.matches(region) {
        return Err("Vector cache georeference differs".into());
    }
    let mut data = mvt::empty(region, id.into());
    data.roads = package::decode(&crate::map::source::read_limited(
        &path.join("roads.bin"),
        67_108_864,
    )?)?;
    data.buildings = package::decode(&crate::map::source::read_limited(
        &path.join("buildings.bin"),
        67_108_864,
    )?)?;
    if cache::id(&data.roads)? != m.roads || cache::id(&data.buildings)? != m.buildings {
        return Err("Corrupt vector cache".into());
    }
    data.skipped_buildings = m.skipped_buildings;
    data.metrics.tiles = m.tiles;
    data.metrics.processed_cache_hit = true;
    data.source = m.source;
    data.attribution = m.attribution;
    data.fingerprint = blake3::Hash::from_hex(m.fingerprint).map_err(|e| e.to_string())?;
    clip::validate(&data)?;
    Ok(data)
}
