//! Memory-only imagery preparation; no package IDs, files or egui GPU handles.
use super::{
    builder::{Job, coverage, imagery::crop},
    imagery_service::TileService,
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
    satellite::{Georeference, SatelliteMetadata},
    satellite_scene::{self, ImagerySource},
};
use bevy::prelude::*;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestKey {
    pub revision: u64,
    pub terrain: blake3::Hash,
    pub generation: u64,
}
pub struct LiveTexture {
    pub metadata: SatelliteMetadata,
    pub rgba: Vec<u8>,
    pub uvs: Vec<Vec<[f32; 2]>>,
    pub preparation_ms: f64,
}
struct Pending {
    key: RequestKey,
    job: Job,
    reply: Mutex<mpsc::Receiver<Result<LiveTexture, String>>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.job.cancelled.store(true, Ordering::Relaxed);
    }
}
#[derive(Resource, Default)]
pub struct LiveImagery {
    pending: Option<Pending>,
    attempted: Option<RequestKey>,
}

pub fn coverage(
    service: &TileService,
    region: super::region::MapRegion,
    desired: u8,
) -> Result<(u8, super::satellite::PixelBounds, Vec<coverage::Tile>), String> {
    let p = service.provider();
    (p.min_zoom..=desired.min(p.max_zoom))
        .rev()
        .find_map(|z| {
            let b = crop(region, z, p.tile_size);
            let w = b.right - b.left;
            let h = b.bottom - b.top;
            if w > 4096.0 || h > 4096.0 || w * h > 16_777_216.0 {
                return None;
            }
            coverage::tiles(
                &b,
                z,
                p.tile_size,
                false,
                (96 * 1024 * 1024 / (p.tile_size as usize).pow(2) / 3).min(256),
            )
            .ok()
            .map(|tiles| (z, b, tiles))
        })
        .ok_or("Satellite area exceeds live image limits".into())
}
pub fn prepare(
    service: &TileService,
    terrain: &super::source::TerrainPayload,
    region: super::region::MapRegion,
    desired: u8,
    job: &Job,
    uvs: bool,
) -> Result<LiveTexture, String> {
    let p = service.provider();
    if !p.allow_online_3d {
        return Err("Satellite: 3D use not enabled".into());
    }
    let start = Instant::now();
    let (z, b, required) = coverage(service, region, desired)?;
    let mut tiles = std::collections::BTreeMap::new();
    for (i, t) in required.iter().enumerate() {
        job.check()?;
        // Queue the bounded coverage first so all four workers can make progress.
        service.request(*t);
        job.status(format!(
            "Satellite: Loading · {i} / {} tiles",
            required.len()
        ));
    }
    for (i, t) in required.iter().enumerate() {
        tiles.insert(*t, service.get(*t, job)?);
        job.status(format!(
            "Satellite: Loading · {} ready / {} pending tiles",
            i + 1,
            required.len() - i - 1
        ));
    }
    let received = start.elapsed();
    let image = super::builder::imagery::assemble(&tiles, z, p.tile_size, &b, job)?;
    let metadata = SatelliteMetadata {
        provider: p.name.clone(),
        zoom: u32::from(z),
        width: image.width(),
        height: image.height(),
        georeference: Georeference {
            zoom: u32::from(z),
            tile_size: p.tile_size,
            crop_pixel_bounds: b,
        },
        file: String::new(),
        media_type: "image/raw-rgba8".into(),
        attribution: p.attribution.clone(),
    };
    let uvs = if uvs {
        super::satellite::terrain_uvs(terrain, &metadata.georeference)
    } else {
        Vec::new()
    };
    job.check()?;
    let preparation_ms = start.elapsed().saturating_sub(received).as_secs_f64() * 1000.0;
    info!(
        "Live imagery z{z}: {} tiles in {:.3}s, preparation {:.2}ms, service {:?}",
        required.len(),
        received.as_secs_f64(),
        preparation_ms,
        service.stats()
    );
    Ok(LiveTexture {
        metadata,
        rgba: image::DynamicImage::ImageRgb8(image)
            .into_rgba8()
            .into_raw(),
        uvs,
        preparation_ms,
    })
}

pub fn update(
    mut picker: NonSendMut<super::picker::MapPicker>,
    active: Res<ActiveMapRegion>,
    mut live: ResMut<LiveImagery>,
    mut scene: ResMut<RealScene>,
    import: Res<super::satellite_scene::SatelliteImport>,
    mut assets: SceneAssets,
) {
    let service = picker.imagery.tiles.as_ref().map(|t| t.service.clone());
    let current = active
        .get()
        .and_then(|(revision, _)| {
            Some(RequestKey {
                revision,
                terrain: scene.fingerprint()?,
                generation: service.as_ref()?.generation(),
            })
        })
        .filter(|k| scene.revision == Some(k.revision));
    if live.attempted != current {
        live.pending = None;
        live.attempted = None;
        if scene
            .satellite
            .as_ref()
            .is_some_and(|s| matches!(s.source, ImagerySource::Live { .. }))
        {
            let old = scene.satellite.take().unwrap();
            assets.images.remove(old.image.id());
        }
    }
    if scene
        .satellite
        .as_ref()
        .is_some_and(|s| matches!(s.source, ImagerySource::Package(_)))
    {
        live.pending = None;
        picker.imagery.live_status = "Satellite: Local Package".into();
        return;
    }
    let Some(service) = service else {
        picker.imagery.live_status = "Satellite: Unavailable".into();
        return;
    };
    if !service.provider().allow_online_3d {
        picker.imagery.live_status = "Satellite: 3D use not enabled".into();
        return;
    }
    let Some(key) = current else {
        return;
    };
    if live.attempted != Some(key) && !import.loading() && import.pending.is_none() {
        live.attempted = Some(key);
        let service = service.clone();
        let terrain = scene.data.clone().unwrap();
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::default(),
        };
        let work = job.clone();
        let (send, reply) = mpsc::sync_channel(1);
        live.pending = Some(Pending {
            key,
            job,
            reply: Mutex::new(reply),
        });
        std::thread::spawn(move || {
            let p = service.provider();
            let desired = coverage::zoom(
                terrain.manifest.region(),
                p.min_zoom,
                p.max_zoom,
                p.tile_size,
                8.0,
            );
            let result = prepare(
                &service,
                &terrain,
                terrain.manifest.region(),
                desired,
                &work,
                true,
            );
            let _ = send.send(result);
        });
    }
    if let Some(pending) = &live.pending {
        picker.imagery.live_status = pending.job.progress.lock().unwrap().clone();
        let result = pending.reply.lock().unwrap().try_recv().ok();
        if let Some(result) = result {
            let key = pending.key;
            live.pending = None;
            if current != Some(key) {
                return;
            }
            match result {
                Ok(texture) => {
                    let start = Instant::now();
                    let result = satellite_scene::install_texture(
                        &mut scene,
                        &mut assets,
                        texture.metadata,
                        ImagerySource::Live {
                            generation: key.generation,
                        },
                        texture.rgba,
                        texture.uvs,
                        key.terrain,
                    );
                    picker.imagery.live_status = match result {
                        Ok(()) => "Satellite: Live".into(),
                        Err(e) => format!("Satellite: Unavailable · {e}"),
                    };
                    info!(
                        "Live imagery CPU installation {:.2}ms; GPU latency not measured",
                        start.elapsed().as_secs_f64() * 1000.0
                    );
                }
                Err(e) => picker.imagery.live_status = format!("Satellite: Unavailable · {e}"),
            }
        }
    }
}
