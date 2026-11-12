use super::{
    Config, Job, cache,
    coverage::{self, Tile},
    dem::decode,
    download::Download,
    provider::ImageryProvider,
};
use crate::map::{
    satellite::{
        self, Georeference, MapManifest, PixelBounds, SatelliteMetadata, SatellitePayload,
    },
    source::TerrainPayload,
};
use std::{collections::BTreeMap, path::Path};

pub fn crop(region: crate::map::region::MapRegion, z: u8, size: u32) -> PixelBounds {
    let b = coverage::bounds(region, z, size);
    PixelBounds {
        left: b.left.floor(),
        top: b.top.floor(),
        right: b.right.ceil(),
        bottom: b.bottom.ceil(),
    }
}
pub fn assemble(
    tiles: &BTreeMap<Tile, image::RgbImage>,
    z: u8,
    size: u32,
    b: &PixelBounds,
    job: &Job,
) -> Result<image::RgbImage, String> {
    let width = (b.right - b.left) as u32;
    let height = (b.bottom - b.top) as u32;
    let mut image = image::RgbImage::new(width, height);
    for y in 0..height {
        job.check()?;
        for x in 0..width {
            let gx = b.left as u32 + x;
            let gy = b.top as u32 + y;
            let tile = tiles
                .get(&Tile {
                    z,
                    x: gx / size,
                    y: gy / size,
                })
                .ok_or("Missing imagery tile")?;
            image.put_pixel(x, y, *tile.get_pixel(gx % size, gy % size));
        }
    }
    Ok(image)
}
pub fn build(
    terrain: &TerrainPayload,
    provider: &ImageryProvider,
    config: &Config,
    root: &Path,
    http: &Download,
    job: &Job,
) -> Result<SatellitePayload, String> {
    provider.validate()?;
    let token = provider
        .bearer_env
        .as_ref()
        .map(|name| {
            std::env::var(name)
                .map_err(|_| "Satellite authorization environment variable is missing")
        })
        .transpose()?;
    let region = terrain.manifest.region();
    let desired = coverage::zoom(
        region,
        provider.min_zoom,
        provider.max_zoom,
        provider.tile_size,
        config.imagery_step_m,
    );
    let (z, b, required) = (provider.min_zoom..=desired)
        .rev()
        .find_map(|z| {
            let b = crop(region, z, provider.tile_size);
            let w = b.right - b.left;
            let h = b.bottom - b.top;
            if w > f64::from(config.imagery_max_edge)
                || h > f64::from(config.imagery_max_edge)
                || w * h > config.imagery_max_pixels as f64
            {
                return None;
            }
            coverage::tiles(&b, z, provider.tile_size, false, config.max_tiles)
                .ok()
                .map(|t| (z, b, t))
        })
        .ok_or("Satellite area exceeds image/tile limits; reduce region")?;
    let id = cache::id(&(
        2,
        region,
        provider,
        z,
        config.imagery_step_m,
        config.imagery_max_edge,
        config.imagery_max_pixels,
        config.jpeg_quality,
        config.max_tiles,
    ))?;
    let package = root.join("packages").join(&id);
    let _lock = cache::Lock::acquire(&root.join("locks").join(&id), job)?;
    if let Ok(payload) = satellite::load_satellite(&package.join("map-data"), terrain)
        && payload.manifest.map_data_id == id
    {
        return Ok(payload);
    }
    let tile_cache = root.join("tiles").join(cache::id(provider)?);
    let raw = http.tiles(
        &provider.url,
        token.as_deref(),
        &required,
        &tile_cache,
        job,
        "Downloading imagery",
    )?;
    let mut tiles = BTreeMap::new();
    for (t, bytes) in raw {
        match decode(&bytes, provider.tile_size, false) {
            Ok(image) => {
                tiles.insert(t, image);
            }
            Err(e) => {
                let _ =
                    std::fs::remove_file(tile_cache.join(format!("{}/{}/{}.tile", t.z, t.x, t.y)));
                return Err(e);
            }
        }
    }
    job.status("Preparing satellite".into());
    let image = assemble(&tiles, z, provider.tile_size, &b, job)?;
    let manifest = MapManifest {
        format_version: 2,
        map_data_id: id,
        bbox: region.bounds,
        origin: region.origin,
        satellite: SatelliteMetadata {
            provider: provider.name.clone(),
            zoom: u32::from(z),
            width: image.width(),
            height: image.height(),
            georeference: Georeference {
                zoom: u32::from(z),
                tile_size: provider.tile_size,
                crop_pixel_bounds: b,
            },
            file: "satellite.jpg".into(),
            media_type: "image/jpeg".into(),
            attribution: provider.attribution.clone(),
        },
    };
    let stage = cache::Staging::new(&root.join("packages"))?;
    let path = stage.path.join("map-data");
    std::fs::create_dir(&path).map_err(|e| e.to_string())?;
    let mut file = std::fs::File::create(path.join("satellite.jpg")).map_err(|e| e.to_string())?;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, config.jpeg_quality)
        .encode_image(&image)
        .map_err(|_| "Cannot encode satellite JPEG")?;
    std::fs::write(
        path.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let payload = satellite::load_satellite(&path, terrain)?;
    job.check()?;
    stage.publish(&package)?;
    Ok(payload)
}
pub fn configured(path: Option<std::ffi::OsString>) -> Result<Option<ImageryProvider>, String> {
    let provider = if let Some(path) = path {
        let bytes = crate::map::source::read_limited(Path::new(&path), 1_048_576)?;
        Some(
            serde_json::from_slice::<ImageryProvider>(&bytes)
                .map_err(|_| "Invalid satellite configuration")?,
        )
    } else {
        let settings = crate::map::sources::load_saved()?;
        settings.provider.filter(|_| settings.online_authorized)
    };
    if let Some(provider) = &provider {
        provider.validate_preview()?;
    }
    let provider = provider.filter(|p| p.allow_permanent_packages);
    if let Some(provider) = &provider {
        provider.validate()?;
    }
    Ok(provider)
}
