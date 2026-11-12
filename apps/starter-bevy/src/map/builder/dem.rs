use super::{
    Config, Job, cache,
    coverage::{self, Tile},
    download::Download,
    provider::TerrainProvider,
};
use crate::map::{
    coordinates::wgs84_to_local,
    region::MapRegion,
    source::{self, TerrainManifest, TerrainPayload},
};
use std::{collections::BTreeMap, io::Cursor, path::Path, sync::Arc};

pub fn decode_terrarium(r: u8, g: u8, b: u8) -> f32 {
    f32::from(r) * 256.0 + f32::from(g) + f32::from(b) / 256.0 - 32768.0
}
pub fn decode(bytes: &[u8], size: u32, terrain: bool) -> Result<image::RgbImage, String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Corrupt tile header")?;
    if terrain && reader.format() != Some(image::ImageFormat::WebP) {
        return Err("DEM tile must be WebP".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(size);
    limits.max_image_height = Some(size);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| "Corrupt geographic tile")?;
    if decoded.width() != size
        || decoded.height() != size
        || (terrain
            && !matches!(
                decoded.color(),
                image::ColorType::Rgb8 | image::ColorType::Rgba8
            ))
    {
        return Err("Invalid geographic tile dimensions/channels".into());
    }
    if decoded.color().has_alpha() && decoded.to_rgba8().pixels().any(|p| p[3] != 255) {
        return Err("Geographic tile contains missing/transparent samples".into());
    }
    Ok(decoded.into_rgb8())
}
pub fn sample(
    tiles: &BTreeMap<Tile, image::RgbImage>,
    z: u8,
    size: u32,
    p: [f64; 2],
) -> Result<f32, String> {
    let world = f64::from(size) * 2f64.powi(i32::from(z));
    let x = (p[0] - 0.5).clamp(0.0, world - 1.0);
    let y = (p[1] - 0.5).clamp(0.0, world - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(world as u32 - 1);
    let y1 = (y0 + 1).min(world as u32 - 1);
    let read = |x: u32, y: u32| -> Result<f64, String> {
        let image = tiles
            .get(&Tile {
                z,
                x: x / size,
                y: y / size,
            })
            .ok_or("Missing DEM tile; cannot interpolate")?;
        let p = image.get_pixel(x % size, y % size);
        Ok(f64::from(decode_terrarium(p[0], p[1], p[2])))
    };
    let dx = x - f64::from(x0);
    let dy = y - f64::from(y0);
    let h = (read(x0, y0)? * (1.0 - dx) + read(x1, y0)? * dx) * (1.0 - dy)
        + (read(x0, y1)? * (1.0 - dx) + read(x1, y1)? * dx) * dy;
    if !h.is_finite() || h.abs() > 100_000.0 {
        return Err("Invalid DEM elevation".into());
    }
    Ok(h as f32)
}
pub fn build(
    region: MapRegion,
    config: &Config,
    root: &Path,
    http: &Download,
    job: &Job,
) -> Result<Arc<TerrainPayload>, String> {
    config.validate()?;
    MapRegion::new(region.bounds, region.origin).map_err(String::from)?;
    let metadata = root
        .join("metadata")
        .join(cache::id(&config.terrain_tilejson)?);
    let parse = |bytes: &[u8]| -> Result<TerrainProvider, String> {
        let provider: TerrainProvider =
            serde_json::from_slice(bytes).map_err(|_| "Invalid terrain TileJSON")?;
        provider.validate()?;
        Ok(provider)
    };
    let cached = source::read_limited(&metadata, 1_048_576).and_then(|b| parse(&b));
    let provider = match cached {
        Ok(provider) => provider,
        Err(_) => {
            let bytes = http.get(&config.terrain_tilejson, None, job)?;
            let provider = parse(&bytes)?;
            cache::write_atomic(&metadata, &bytes)?;
            provider
        }
    };
    if let Some([w, s, e, n]) = provider.bounds {
        let b = region.bounds;
        if b.west < w || b.south < s || b.east > e || b.north > n {
            return Err("Region outside DEM source coverage".into());
        }
    }
    let (width, height) = coverage::grid(region, config)?;
    let b = region.bounds;
    let nw = wgs84_to_local(region.origin, b.west, b.north);
    let se = wgs84_to_local(region.origin, b.east, b.south);
    let sx = (se[0] - nw[0]) / f64::from(width - 1);
    let sy = (se[1] - nw[1]) / f64::from(height - 1);
    let desired = coverage::zoom(
        region,
        provider.minzoom,
        provider.max_zoom(),
        512,
        sx.max(sy),
    );
    let namespace = cache::id(&(
        &config.terrain_tilejson,
        &provider.tiles,
        &provider.attribution,
    ))?;
    let tile_cache = root.join("tiles").join(namespace);
    let candidates = (provider.minzoom..=desired)
        .rev()
        .map(|z| {
            let id = cache::id(&(
                1,
                region,
                config.step_m,
                config.max_edge,
                config.max_samples,
                config.max_tiles,
                &config.terrain_tilejson,
                z,
                &provider.tiles,
                &provider.attribution,
            ))?;
            Ok((z, id))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Search every viable cached zoom before attempting any network requests.
    for (_, id) in &candidates {
        let _lock = cache::Lock::acquire(&root.join("locks").join(id), job)?;
        if let Ok(data) = source::load_terrain(&root.join("packages").join(id).join("terrain"))
            && data.manifest.matches(region)
            && data.manifest.terrain_id == *id
        {
            return Ok(Arc::new(data));
        }
    }
    let mut last = "No DEM coverage; reduce region".to_owned();
    for (z, id) in candidates {
        job.check()?;
        let package = root.join("packages").join(&id);
        let _lock = cache::Lock::acquire(&root.join("locks").join(&id), job)?;
        job.status("Checking cache".into());
        if let Ok(data) = source::load_terrain(&package.join("terrain"))
            && data.manifest.matches(region)
            && data.manifest.terrain_id == id
        {
            return Ok(Arc::new(data));
        }
        let required = match coverage::tiles(
            &coverage::bounds(region, z, 512),
            z,
            512,
            true,
            config.max_tiles,
        ) {
            Ok(t) => t,
            Err(e) => {
                last = e;
                continue;
            }
        };
        let coarse = Config {
            step_m: config.step_m.max(coverage::resolution(region, z, 512)),
            ..config.clone()
        };
        let (width, height) = coverage::grid(region, &coarse)?;
        let sx = (se[0] - nw[0]) / f64::from(width - 1);
        let sy = (se[1] - nw[1]) / f64::from(height - 1);
        let raw = match http.tiles(
            &provider.tiles[0],
            None,
            &required,
            &tile_cache,
            job,
            "Downloading terrain tiles",
        ) {
            Ok(t) => t,
            Err(e) if e.contains("coverage") => {
                last = e;
                continue;
            }
            Err(e) => return Err(e),
        };
        let mut tiles = BTreeMap::new();
        for (t, bytes) in raw {
            match decode(&bytes, 512, true) {
                Ok(image) => {
                    tiles.insert(t, image);
                }
                Err(e) => {
                    let _ = std::fs::remove_file(
                        tile_cache.join(format!("{}/{}/{}.tile", t.z, t.x, t.y)),
                    );
                    return Err(e);
                }
            }
        }
        job.status("Building DEM".into());
        let mut heights = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            job.check()?;
            let lat = b.north - (b.north - b.south) * f64::from(y) / f64::from(height - 1);
            for x in 0..width {
                let lon = b.west + (b.east - b.west) * f64::from(x) / f64::from(width - 1);
                heights.push(sample(&tiles, z, 512, coverage::pixel(lon, lat, z, 512))?);
            }
        }
        let origin = coverage::pixel(region.origin.longitude, region.origin.latitude, z, 512);
        let reference = sample(&tiles, z, 512, origin).or_else(|_| {
            let ob = crate::map::satellite::PixelBounds {
                left: origin[0],
                right: origin[0],
                top: origin[1],
                bottom: origin[1],
            };
            let needed = coverage::tiles(&ob, z, 512, true, 4)?;
            let raw = http.tiles(
                &provider.tiles[0],
                None,
                &needed,
                &tile_cache,
                job,
                "Downloading origin elevation",
            )?;
            let extra = raw
                .into_iter()
                .map(|(t, b)| Ok((t, decode(&b, 512, true)?)))
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            sample(&extra, z, 512, origin)
        });
        job.check()?;
        let (reference, source) = match reference {
            Ok(h) => (h, "origin-sample"),
            Err(_) => (
                (heights.iter().map(|h| f64::from(*h)).sum::<f64>() / heights.len() as f64) as f32,
                "fallback-mean",
            ),
        };
        let manifest = TerrainManifest {
            format_version: 1,
            terrain_id: id,
            bbox: b,
            origin: region.origin,
            width,
            height,
            min_x_m: nw[0],
            min_y_m: nw[1],
            spacing_x_m: sx,
            spacing_y_m: sy,
            reference_elevation_msl: reference,
            reference_elevation_source: source.into(),
            min_elevation_msl: heights.iter().copied().fold(f32::INFINITY, f32::min),
            max_elevation_msl: heights.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            encoding: "little-endian-f32-row-major".into(),
            source: "Mapterhorn".into(),
            attribution: provider.attribution.clone(),
        };
        let stage = cache::Staging::new(&root.join("packages"))?;
        let path = stage.path.join("terrain");
        std::fs::create_dir(&path).map_err(|e| e.to_string())?;
        std::fs::write(
            path.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let bytes: Vec<u8> = heights.iter().flat_map(|h| h.to_le_bytes()).collect();
        std::fs::write(path.join("heights.f32"), bytes).map_err(|e| e.to_string())?;
        let data = source::load_terrain(&path)?;
        job.check()?;
        stage.publish(&package)?;
        return Ok(Arc::new(data));
    }
    Err(format!("{last}. Reduce the region."))
}
