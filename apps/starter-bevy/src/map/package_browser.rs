//! Bounded, read-only discovery using the same satellite validator/decoder as import.
use super::{
    real_scene::RealScene,
    region::ActiveMapRegion,
    satellite::{self, MapManifest},
    satellite_scene::SatelliteImport,
};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_egui::egui::{self, Ui};
use std::path::{Path, PathBuf};
type Entries = Vec<(PathBuf, MapManifest)>;
#[derive(Default)]
pub struct PackageBrowser {
    pub open: bool,
    pub directory: String,
    entries: Entries,
    task: Option<Task<Result<Entries, String>>>,
    error: Option<String>,
}
pub fn default_directory() -> PathBuf {
    std::env::var_os("UAV_MAP_DATA_CACHE_DIR")
        .map(|p| PathBuf::from(p).join("packages"))
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|d| {
                    d.cache_dir()
                        .join("uav-hit-estimator/map-data-cache/packages")
                })
                .unwrap_or_default()
        })
}
pub fn scan(root: &Path) -> Result<Vec<(PathBuf, MapManifest)>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())?.take(64) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_ok_and(|t| t.is_dir())
            && let Ok(package) = satellite::read_package(&entry.path(), None)
        {
            entries.push((entry.path(), package.manifest));
        }
    }
    entries.sort_by(|a, b| a.1.map_data_id.cmp(&b.1.map_data_id));
    Ok(entries)
}
pub fn area(
    ui: &mut Ui,
    label: &str,
    bounds: super::region::GeoBounds,
    origin: super::region::GeoOrigin,
) {
    ui.label(format!(
        "{label}: W {:.6} S {:.6} E {:.6} N {:.6}; origin {:.7}, {:.7}",
        bounds.west, bounds.south, bounds.east, bounds.north, origin.longitude, origin.latitude
    ));
}
pub fn show(
    ui: &mut Ui,
    import: &mut SatelliteImport,
    active: &mut ActiveMapRegion,
    scene: &RealScene,
) {
    if !import.packages.open {
        return;
    }
    let loading = import.loading();
    let mut open = true;
    let mut selected = None;
    let mut adopt = None;
    egui::Window::new("Existing Satellite Packages").open(&mut open).default_width(650.0).vscroll(true).show(ui.ctx(), |ui| {
        let browser = &mut import.packages;
        if browser.directory.is_empty() { browser.directory = default_directory().to_string_lossy().into_owned(); }
        if let Some(result) = browser.task.as_mut().and_then(|t| future::block_on(future::poll_once(t))) {
            browser.task = None;
            match result { Ok(entries) => {browser.entries = entries;browser.error = None;}, Err(e) => browser.error = Some(e) }
        }
        ui.label("Existing Map Package Directory (read only; at most 64 entries)");
        ui.text_edit_singleline(&mut browser.directory);
        ui.horizontal(|ui| {
            if ui.add_enabled(browser.task.is_none(), egui::Button::new("Scan Directory")).clicked() {
                let root = PathBuf::from(&browser.directory);
                browser.task = Some(AsyncComputeTaskPool::get().spawn(async move { scan(&root) }));
            }
            if ui.add_enabled(!loading, egui::Button::new("Import Existing Package…")).clicked() { selected = Some(None); }
        });
        if browser.task.is_some() {ui.spinner();}
        if let Some(error) = &browser.error {ui.colored_label(ui.visuals().error_fg_color,error);}
        if let Some(payload) = &import.pending {
            ui.separator();
            ui.strong(if scene.data.is_some() {"Satellite package does not match the current terrain region"} else {"Satellite package is waiting for DEM"});
            area(ui, "Package", payload.manifest.bbox, payload.manifest.origin);
            let s = &payload.manifest.satellite;
            ui.label(format!("{} × {} · z{} · {}",s.width,s.height,s.zoom,s.provider));
            if let Some(data) = &scene.data {
                let m = &data.manifest;
                area(ui,"DEM",m.bbox,m.origin);
            } else {ui.label("No DEM loaded. The package is waiting for a compatible surface.");}
            ui.label("Load a DEM with matching bbox AND origin, or explicitly adopt this package's region and load DEM.");
            if ui.button("Use Package Region (including origin)").clicked() {
                adopt = Some(super::region::MapRegion::new(payload.manifest.bbox,payload.manifest.origin));
            }
        }
        ui.separator();
        for (path, manifest) in &browser.entries {
            let s = &manifest.satellite;
            ui.strong(&manifest.map_data_id);
            area(ui,"Area",manifest.bbox,manifest.origin);
            ui.label(format!("{} × {} · z{} · {}",s.width,s.height,s.zoom,s.provider));
            ui.label(if let Some(data) = &scene.data {if manifest.validate(Some(data)).is_ok() {"Compatible with DEM"} else {"Different bbox/origin"}} else {"DEM not loaded"});
            if ui.add_enabled(!loading,egui::Button::new("Open Package")).clicked() {selected = Some(Some(path.clone()));}
            ui.separator();
        }
    });
    import.packages.open = open;
    if let Some(result) = adopt {
        match result.and_then(|r| active.confirm(r)) {
            Ok(()) => import.error = None,
            Err(e) => import.error = Some(e.into()),
        }
    }
    if let Some(path) = selected {
        import.start(active, scene, path);
    }
}

/// Resolve only the application's missing cached map-data reference, never arbitrary paths.
pub(super) fn external_package(
    directory: &Path,
    cache: &Path,
    packages: &Path,
) -> Option<(PathBuf, String)> {
    let relative = directory.strip_prefix(cache.join("packages")).ok()?;
    let mut parts = relative.components();
    let id = parts.next()?.as_os_str().to_str()?;
    if id.is_empty()
        || id.len() > 256
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || parts.next()?.as_os_str() != "map-data"
        || parts.next().is_some()
    {
        return None;
    }
    Some((packages.join(id), id.to_owned()))
}
