use super::{
    builder::{imagery, provider::ImageryProvider},
    imagery_tiles::ImageryTiles,
};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_egui::egui::{self, Ui};
#[derive(Default)]
pub struct ImageryView {
    pub enabled: bool,
    pub tiles: Option<ImageryTiles>,
    pub message: Option<String>,
    attempted: bool,
    task: Option<Task<Result<Option<ImageryProvider>, String>>>,
}
impl ImageryView {
    pub fn retry(&mut self) {
        self.tiles = None;
        self.attempted = false;
        self.task = None;
        self.message = None;
    }
    pub fn controls(&mut self, ui: &mut Ui) {
        if !self.attempted {
            self.attempted = true;
            self.task = Some(AsyncComputeTaskPool::get().spawn(async {
                let provider = imagery::configured(std::env::var_os("STARTER_IMAGERY_CONFIG"))?;
                if let Some(p) = &provider
                    && let Some(name) = &p.bearer_env
                {
                    std::env::var(name)
                        .map_err(|_| "Satellite authorization variable is missing")?;
                }
                Ok(provider)
            }));
        }
        if let Some(result) = self
            .task
            .as_mut()
            .and_then(|t| future::block_on(future::poll_once(t)))
        {
            self.task = None;
            match result {
                Ok(Some(provider)) => match ImageryTiles::new(provider, ui.ctx().clone()) {
                    Ok(tiles) => {
                        self.tiles = Some(tiles);
                        self.message = None;
                    }
                    Err(e) => self.message = Some(e),
                },
                Ok(None) => self.message = Some("Satellite source not configured".into()),
                Err(e) => self.message = Some(e),
            }
        }
        if let Some(tiles) = &mut self.tiles {
            tiles.poll();
        }
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.enabled, false, "Standard");
            ui.add_enabled_ui(self.tiles.is_some(), |ui| {
                ui.selectable_value(&mut self.enabled, true, "Satellite");
            });
            if self.tiles.is_none() {
                ui.small(
                    self.message
                        .as_deref()
                        .unwrap_or("Checking satellite source…"),
                );
            }
            if ui.small_button("Reload source").clicked() {
                self.tiles = None;
                self.enabled = false;
                self.attempted = false;
            }
        });
    }
    pub fn attribution(&self, ui: &mut Ui) {
        if self.enabled {
            if let Some(tiles) = &self.tiles {
                ui.label(&tiles.provider.attribution);
                if let Some(error) = &tiles.error {
                    ui.colored_label(egui::Color32::YELLOW, error);
                }
            }
        } else {
            ui.hyperlink_to(
                "© OpenStreetMap contributors",
                "https://www.openstreetmap.org/copyright",
            );
        }
    }
}
