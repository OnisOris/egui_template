//! GLB trust boundary; geometry is decoded only by the standard loaders.
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_BYTES: u64 = 100 * 1024 * 1024;
pub fn max_bytes() -> u64 {
    std::env::var("STARTER_MODEL_MAX_MIB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| (1..=1024).contains(v))
        .map(|v| v * 1024 * 1024)
        .unwrap_or(DEFAULT_MAX_BYTES)
}

pub fn validate(bytes: &[u8], limit: u64) -> Result<usize, String> {
    if bytes.len() as u64 > limit {
        return Err("Model exceeds the configured GLB size limit".into());
    }
    if bytes.len() < 12
        || &bytes[..4] != b"glTF"
        || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 2
        || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize != bytes.len()
    {
        return Err("Invalid GLB magic, version or declared length".into());
    }
    let glb =
        gltf::Gltf::from_slice(bytes).map_err(|e| format!("Unsupported or corrupt GLB: {e}"))?;
    if glb.document.buffers().any(|b| matches!(b.source(), gltf::buffer::Source::Uri(uri) if !uri.starts_with("data:")))
        || glb.document.images().any(|i| matches!(i.source(), gltf::image::Source::Uri { uri, .. } if !uri.starts_with("data:")))
    {
        return Err("GLB must embed its buffers and textures".into());
    }
    // Required extensions must never silently degrade into a different model.
    if let Some(extension) = glb.document.extensions_required().find(|name| {
        !matches!(
            *name,
            "KHR_materials_unlit"
                | "KHR_materials_transmission"
                | "KHR_materials_ior"
                | "KHR_materials_specular"
                | "KHR_materials_volume"
                | "KHR_materials_emissive_strength"
                | "KHR_texture_transform"
                | "KHR_lights_punctual"
        )
    }) {
        return Err(format!("Unsupported Model: required extension {extension}"));
    }
    if glb.document.meshes().next().is_none() {
        return Err("Unsupported Model: no mesh geometry".into());
    }
    glb.document
        .default_scene()
        .or_else(|| glb.document.scenes().next())
        .map(|s| s.index())
        .ok_or_else(|| "Unsupported Model: no scene".into())
}

pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("glb"))
    {
        return Err("Only self-contained glTF 2.0 .glb models are supported".into());
    }
    let bytes = crate::map::source::read_limited(
        path,
        usize::try_from(limit).map_err(|_| "Invalid model size limit")?,
    )?;
    validate(&bytes, limit)?;
    Ok(bytes)
}

pub fn cache_root() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "OnisOris", "starter-bevy")
        .map(|d| d.cache_dir().join("models"))
}

pub struct Prepared {
    pub file: String,
    pub scene: usize,
}
pub fn prepare(root: &Path, relative: &str, cache: &Path, limit: u64) -> Result<Prepared, String> {
    let source = crate::project::storage::asset_path(root, relative)?;
    let bytes = read(&source, limit)?;
    let scene = validate(&bytes, limit)?;
    let file = format!("{}.glb", blake3::hash(&bytes));
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let target = cache.join(&file);
    if crate::map::source::read_limited(
        &target,
        usize::try_from(limit).map_err(|_| "Invalid model size limit")?,
    )
    .ok()
    .as_deref()
        != Some(&bytes)
    {
        let stage = crate::map::builder::cache::Staging::new(cache)?;
        let temporary = stage.path.join("model.glb");
        std::fs::write(&temporary, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temporary, target).map_err(|e| e.to_string())?;
    }
    Ok(Prepared { file, scene })
}
