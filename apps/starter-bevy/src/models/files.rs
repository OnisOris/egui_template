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
    let mut parents = vec![0_u32; glb.document.nodes().len()];
    for node in glb.document.nodes() {
        for child in node.children() {
            parents[child.index()] += 1;
            if parents[child.index()] > 1 {
                return Err("Unsupported Model: node has multiple parents".into());
            }
        }
    }
    let mut forest: Vec<_> = glb
        .document
        .nodes()
        .filter(|n| parents[n.index()] == 0)
        .map(|n| (n, 1))
        .collect();
    let mut visited_count = 0;
    while let Some((node, depth)) = forest.pop() {
        if depth > 256 {
            return Err("Unsupported Model: node hierarchy exceeds 256 levels".into());
        }
        visited_count += 1;
        forest.extend(node.children().map(|n| (n, depth + 1)));
    }
    if visited_count != parents.len() {
        return Err("Unsupported Model: cyclic node hierarchy".into());
    }
    let scene = glb
        .document
        .default_scene()
        .or_else(|| glb.document.scenes().next())
        .ok_or("Unsupported Model: no scene")?;
    let mut nodes: Vec<_> = scene.nodes().collect();
    let mut visited = std::collections::HashSet::new();
    let mut geometry = false;
    while let Some(node) = nodes.pop() {
        if !visited.insert(node.index()) {
            return Err("Unsupported Model: invalid node hierarchy".into());
        }
        geometry |= node.mesh().is_some_and(|m| m.primitives().next().is_some());
        nodes.extend(node.children());
    }
    if !geometry {
        return Err("Unsupported Model: selected scene is empty".into());
    }
    Ok(scene.index())
}

pub fn read(path: &Path, limit: u64) -> Result<(Vec<u8>, usize), String> {
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
    let scene = validate(&bytes, limit)?;
    Ok((bytes, scene))
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
    let (bytes, scene) = read(&source, limit)?;
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
