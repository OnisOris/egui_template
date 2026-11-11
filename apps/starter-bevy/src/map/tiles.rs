//! Observes availability, delegates HTTP, cache, XYZ coverage and decoding to Walkers.
use bevy_egui::egui::Context;
use std::time::{Duration, Instant};
use walkers::{
    HttpOptions, HttpTiles, TileId, TilePiece, Tiles,
    sources::{Attribution, OpenStreetMap},
};

pub struct MapTiles {
    inner: HttpTiles,
    missing: usize,
    available: usize,
    waiting_since: Option<Instant>,
}

impl MapTiles {
    pub fn new(ctx: Context) -> Self {
        let cache = directories::ProjectDirs::from("org", "OnisOris", "starter-bevy")
            .map(|dirs| dirs.cache_dir().join("map-tiles"));
        let options = HttpOptions {
            cache,
            user_agent: Some(walkers::HeaderValue::from_static(
                "starter-bevy/0.1 (+https://github.com/OnisOris/egui_template)",
            )),
            ..Default::default()
        };
        Self {
            inner: HttpTiles::with_options(OpenStreetMap, options, ctx),
            missing: 0,
            available: 0,
            waiting_since: None,
        }
    }

    pub fn begin_frame(&mut self) {
        self.missing = 0;
        self.available = 0;
    }

    pub fn status(&mut self) -> Option<&'static str> {
        if self.missing == 0 {
            self.waiting_since = None;
            return None;
        }
        let elapsed = self
            .waiting_since
            .get_or_insert_with(Instant::now)
            .elapsed();
        if elapsed > Duration::from_secs(12) {
            Some("Some map tiles are unavailable. Check your connection and Retry tiles.")
        } else if self.available == 0 {
            Some("Loading map tiles…")
        } else {
            None
        }
    }
}

impl Tiles for MapTiles {
    fn at(&mut self, id: TileId) -> Option<TilePiece> {
        let tile = self.inner.at(id);
        if tile.is_some() {
            self.available += 1;
        } else {
            self.missing += 1;
        }
        tile
    }
    fn attribution(&self) -> Attribution {
        self.inner.attribution()
    }
    fn tile_size(&self) -> u32 {
        self.inner.tile_size()
    }
    fn stats(&self) -> walkers::Stats {
        self.inner.stats()
    }
}
