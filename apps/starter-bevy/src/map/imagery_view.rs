use super::imagery_tiles::ImageryTiles;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_egui::egui::{self, Ui};
#[derive(Default)]
pub struct ImageryView {
    pub enabled: bool,
    pub tiles: Option<ImageryTiles>,
    pub message: Option<String>,
    attempted: bool,
    pub(super) task: Option<Task<Result<super::sources::MapSources, String>>>,
    pub settings: Option<super::sources::MapSources>,
    pub settings_open: bool,
    pub package_directory_update: Option<String>,
}
impl ImageryView {
    pub fn retry(&mut self) {
        self.tiles = None;
        self.attempted = false;
        self.task = None;
        self.message = None;
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        if !self.attempted {
            self.attempted = true;
            self.task = Some(AsyncComputeTaskPool::get().spawn(async { super::sources::load() }));
        }
        if let Some(result) = self
            .task
            .as_mut()
            .and_then(|t| future::block_on(future::poll_once(t)))
        {
            self.task = None;
            match result {
                Ok(settings) => {
                    let provider = settings
                        .provider
                        .clone()
                        .filter(|_| settings.online_authorized);
                    self.package_directory_update = settings.package_directory.clone();
                    self.settings = Some(settings);
                    match provider {
                        Some(provider) => match ImageryTiles::new(provider, ctx.clone()) {
                            Ok(tiles) => {
                                self.tiles = Some(tiles);
                                self.message = None;
                            }
                            Err(e) => self.message = Some(e),
                        },
                        None => {
                            self.message = Some(
                                "Satellite source not configured or online access not authorized"
                                    .into(),
                            )
                        }
                    }
                }
                Err(e) => self.message = Some(e),
            }
        }
        if let Some(tiles) = &mut self.tiles {
            tiles.poll();
        }
    }
    pub fn controls(&mut self, ui: &mut Ui) {
        self.poll(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            ui.label("Map style");
            ui.selectable_value(&mut self.enabled, false, "Standard (OSM)");
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
            if ui.small_button("Map Sources…").clicked() {
                self.settings_open = true;
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
