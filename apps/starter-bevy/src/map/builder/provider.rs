use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageryProvider {
    pub name: String,
    pub url: String,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub tile_size: u32,
    pub attribution: String,
    pub allow_permanent_packages: bool,
    /// Optional bearer token environment variable name; never the token itself.
    pub bearer_env: Option<String>,
}
impl ImageryProvider {
    pub fn validate(&self) -> Result<(), String> {
        if !self.allow_permanent_packages {
            return Err("Satellite source does not permit permanent packages".into());
        }
        self.validate_preview()
    }
    pub fn validate_preview(&self) -> Result<(), String> {
        let url = reqwest::Url::parse(&self.url).map_err(|_| "Invalid imagery URL")?;
        if (url.scheme() != "https"
            && !(url.scheme() == "http" && url.host_str() == Some("127.0.0.1")))
            || url.host_str() == Some("tile.openstreetmap.org")
            || !url.username().is_empty()
            || url.password().is_some()
            || self.name.trim().is_empty()
            || self.attribution.trim().is_empty()
            || self.min_zoom > self.max_zoom
            || self.max_zoom > 22
            || !(256..=1024).contains(&self.tile_size)
            || !self.tile_size.is_power_of_two()
            || ["{z}", "{x}", "{y}"].iter().any(|v| !self.url.contains(v))
        {
            return Err("Invalid satellite source: HTTPS XYZ, attribution and supported dimensions required".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainProvider {
    pub tiles: Vec<String>,
    pub attribution: String,
    #[serde(default)]
    pub minzoom: u8,
    pub maxzoom: Option<u8>,
    pub tile_size: u32,
    pub encoding: String,
    pub bounds: Option<[f64; 4]>,
}
impl TerrainProvider {
    pub fn validate(&self) -> Result<(), String> {
        if self.encoding != "terrarium"
            || self.tile_size != 512
            || self.tiles.first().is_none_or(|s| {
                !s.starts_with("https://") && !s.starts_with("http://127.0.0.1:")
                    || ["{z}", "{x}", "{y}"].iter().any(|v| !s.contains(v))
            })
            || self.minzoom > self.max_zoom()
            || self.max_zoom() > 22
            || self.attribution.trim().is_empty()
        {
            return Err("Unsupported Mapterhorn TileJSON".into());
        }
        Ok(())
    }
    pub fn max_zoom(&self) -> u8 {
        // Mapterhorn documents global z0–12; current TileJSON omits zoom limits.
        self.maxzoom.unwrap_or(12)
    }
}
