//! Application settings only. Secrets remain in environment variables, never project JSON.
use super::builder::provider::ImageryProvider;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MapSources {
    pub provider: Option<ImageryProvider>,
    pub online_authorized: bool,
    pub package_directory: Option<String>,
}
pub fn path() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("org", "OnisOris", "starter-bevy")
        .map(|d| d.config_dir().join("map-sources.json"))
        .ok_or("Application settings directory unavailable".into())
}
pub fn load_saved() -> Result<MapSources, String> {
    let path = path()?;
    if !path.exists() {
        return Ok(MapSources::default());
    }
    let bytes = super::source::read_limited(&path, 1_048_576)?;
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Map Sources settings".into())
}
pub fn load() -> Result<MapSources, String> {
    let mut settings = load_saved()?;
    if let Some(path) = std::env::var_os("STARTER_IMAGERY_CONFIG") {
        let bytes = super::source::read_limited(std::path::Path::new(&path), 1_048_576)?;
        settings.provider =
            Some(serde_json::from_slice(&bytes).map_err(|_| "Invalid satellite configuration")?);
        settings.online_authorized = true;
    }
    if let Some(p) = &settings.provider {
        p.validate_preview()?;
    }
    Ok(settings)
}
pub fn save(settings: &MapSources) -> Result<(), String> {
    if let Some(p) = &settings.provider {
        p.validate_preview()?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    super::builder::cache::write_atomic(&path()?, &bytes)
}
pub fn esri() -> ImageryProvider {
    ImageryProvider {
        name:"Esri World Imagery".into(),
        url:"https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}".into(),
        min_zoom:0,max_zoom:19,tile_size:256,
        attribution:"Source: Esri, Maxar, Earthstar Geographics, and the GIS User Community".into(),
        allow_permanent_packages:false,allow_online_3d:false,bearer_env:None,
    }
}
