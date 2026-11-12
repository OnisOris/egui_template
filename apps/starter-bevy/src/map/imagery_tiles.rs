//! Walkers raster adapter: existing authorized downloader, no projection or pan/zoom code.
use super::builder::{
    Job, cache, coverage::Tile as SourceTile, dem::decode, download::Download,
    provider::ImageryProvider,
};
use bevy_egui::egui::{self, Context, Rect, pos2};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
use walkers::{Tile, TileId, TilePiece, Tiles, sources::Attribution};
type Reply = (TileId, Result<egui::TextureHandle, String>);
pub struct ImageryTiles {
    requests: mpsc::SyncSender<TileId>,
    replies: mpsc::Receiver<Reply>,
    pending: HashSet<TileId>,
    textures: HashMap<TileId, egui::TextureHandle>,
    order: VecDeque<TileId>,
    job: Job,
    pub provider: ImageryProvider,
    pub error: Option<String>,
    capacity: usize,
}
impl ImageryTiles {
    pub fn new(provider: ImageryProvider, ctx: Context) -> Result<Self, String> {
        let token = provider
            .bearer_env
            .as_ref()
            .and_then(|name| std::env::var(name).ok());
        provider.validate()?;
        Ok(Self::with_token(provider, ctx, token, cache::root()?))
    }
    pub(super) fn with_token(
        provider: ImageryProvider,
        ctx: Context,
        token: Option<String>,
        root: std::path::PathBuf,
    ) -> Self {
        let (requests, receive) = mpsc::sync_channel::<TileId>(32);
        let (reply, replies) = mpsc::sync_channel(8);
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(String::new())),
        };
        let work = job.clone();
        let source = provider.clone();
        std::thread::Builder::new()
            .name("walkers-satellite".into())
            .spawn(move || {
                let setup = (|| {
                    source.validate()?;
                    if source.bearer_env.is_some() && token.is_none() {
                        return Err("Satellite authorization variable is missing".into());
                    }
                    Ok::<_, String>((
                        Download::new(4)?,
                        root.join("tiles").join(cache::id(&source)?),
                        token,
                    ))
                })();
                while let Ok(first) = receive.recv() {
                    if work.check().is_err() {
                        break;
                    }
                    let ids: Vec<_> = std::iter::once(first)
                        .chain(receive.try_iter().take(7))
                        .collect();
                    let result =
                        setup
                            .as_ref()
                            .map_err(Clone::clone)
                            .and_then(|(http, root, token)| {
                                let tiles: Vec<_> = ids
                                    .iter()
                                    .map(|t| SourceTile {
                                        z: t.zoom,
                                        x: t.x,
                                        y: t.y,
                                    })
                                    .collect();
                                http.tiles(
                                    &source.url,
                                    token.as_deref(),
                                    &tiles,
                                    root,
                                    &work,
                                    "Downloading satellite map",
                                )
                            });
                    for id in ids {
                        let decoded = result.as_ref().map_err(Clone::clone).and_then(|tiles| {
                            let bytes = tiles
                                .get(&SourceTile {
                                    z: id.zoom,
                                    x: id.x,
                                    y: id.y,
                                })
                                .ok_or("Missing satellite tile")?;
                            let image =
                                decode(bytes, source.tile_size, false).inspect_err(|_| {
                                    if let Ok((_, root, _)) = &setup {
                                        let _ =
                                            std::fs::remove_file(root.join(format!(
                                                "{}/{}/{}.tile",
                                                id.zoom, id.x, id.y
                                            )));
                                    }
                                })?;
                            Ok(egui::ColorImage::from_rgb(
                                [source.tile_size as usize; 2],
                                image.as_raw(),
                            ))
                        });
                        let texture = decoded.map(|image| {
                            ctx.load_texture(
                                format!("satellite/{}/{}/{}", id.zoom, id.x, id.y),
                                image,
                                egui::TextureOptions::LINEAR,
                            )
                        });
                        if reply.send((id, texture)).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                    }
                }
            })
            .expect("Cannot start satellite tile worker");
        let capacity =
            ((64 * 1024 * 1024) / (provider.tile_size as usize).pow(2) / 4).clamp(4, 128);
        Self {
            requests,
            replies,
            pending: HashSet::new(),
            textures: HashMap::new(),
            order: VecDeque::new(),
            job,
            provider,
            error: None,
            capacity,
        }
    }
    pub fn poll(&mut self) {
        while let Ok((id, result)) = self.replies.try_recv() {
            self.pending.remove(&id);
            match result {
                Ok(texture) => {
                    if self.textures.len() >= self.capacity
                        && let Some(old) = self.order.pop_front()
                    {
                        self.textures.remove(&old);
                    }
                    self.textures.insert(id, texture);
                    self.order.push_back(id);
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
    pub fn zoom_limits(&self) -> (f64, f64) {
        let offset = (f64::from(self.provider.tile_size) / 256.0).log2();
        (
            f64::from(self.provider.min_zoom) + offset,
            f64::from(self.provider.max_zoom) + offset,
        )
    }
}
impl Tiles for ImageryTiles {
    fn at(&mut self, id: TileId) -> Option<TilePiece> {
        if id.zoom < self.provider.min_zoom || id.zoom > self.provider.max_zoom {
            return None;
        }
        if let Some(texture) = self.textures.get(&id) {
            if let Some(i) = self.order.iter().position(|t| *t == id) {
                self.order.remove(i);
            }
            self.order.push_back(id);
            return Some(TilePiece::new(
                Tile::Raster(texture.clone()),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            ));
        }
        if self.error.is_none() && !self.pending.contains(&id) && self.requests.try_send(id).is_ok()
        {
            self.pending.insert(id);
        }
        None
    }
    fn attribution(&self) -> Attribution {
        Attribution {
            text: "Configured satellite imagery",
            url: "",
            logo_light: None,
            logo_dark: None,
        }
    }
    fn tile_size(&self) -> u32 {
        self.provider.tile_size
    }
    fn stats(&self) -> walkers::Stats {
        walkers::Stats {
            in_progress: self.pending.len(),
        }
    }
}
impl Drop for ImageryTiles {
    fn drop(&mut self) {
        self.job.cancelled.store(true, Ordering::Relaxed);
    }
}
