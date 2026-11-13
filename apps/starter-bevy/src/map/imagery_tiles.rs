//! Walkers raster adapter: existing authorized downloader, no projection or pan/zoom code.
use super::{
    builder::{coverage::Tile as SourceTile, provider::ImageryProvider},
    imagery_service::TileService,
};
use bevy_egui::egui::{self, Context, Rect, pos2};
use std::collections::{HashMap, HashSet, VecDeque};
use walkers::{Tile, TileId, TilePiece, Tiles, sources::Attribution};
pub struct ImageryTiles {
    pub service: TileService,
    ctx: Context,
    pending: HashSet<TileId>,
    textures: HashMap<TileId, egui::TextureHandle>,
    order: VecDeque<TileId>,
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
        provider.validate_preview()?;
        if provider.bearer_env.is_some() && token.is_none() {
            return Err("Satellite authorization variable is missing".into());
        }
        let service = TileService::new(provider.clone(), token)?;
        Ok(Self::from_service(service, ctx))
    }
    #[cfg(test)]
    pub(super) fn with_token(
        provider: ImageryProvider,
        ctx: Context,
        token: Option<String>,
        _root: std::path::PathBuf,
    ) -> Self {
        Self::from_service(TileService::new(provider, token).unwrap(), ctx)
    }
    pub fn from_service(service: TileService, ctx: Context) -> Self {
        let provider = service.provider().clone();
        let capacity =
            ((64 * 1024 * 1024) / (provider.tile_size as usize).pow(2) / 4).clamp(4, 128);
        Self {
            service,
            ctx,
            pending: HashSet::new(),
            textures: HashMap::new(),
            order: VecDeque::new(),
            provider,
            error: None,
            capacity,
        }
    }
    pub fn poll(&mut self) {
        for id in self.pending.clone() {
            if let Some(result) = self.service.request(SourceTile {
                z: id.zoom,
                x: id.x,
                y: id.y,
            }) {
                self.pending.remove(&id);
                match result {
                    Ok(image) => {
                        let texture = self.ctx.load_texture(
                            format!("satellite/{}/{}/{}", id.zoom, id.x, id.y),
                            egui::ColorImage::from_rgb(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        );
                        if self.textures.len() >= self.capacity
                            && let Some(old) = self.order.pop_front()
                        {
                            self.textures.remove(&old);
                        }
                        self.textures.insert(id, texture);
                        self.order.push_back(id);
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
        }
        if !self.pending.is_empty() {
            self.ctx
                .request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
    pub fn ready(&self) -> bool {
        !self.textures.is_empty()
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
        self.service.request(SourceTile {
            z: id.zoom,
            x: id.x,
            y: id.y,
        });
        self.pending.insert(id);
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
