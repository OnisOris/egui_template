use starter_core::{ProjectDocument, project::relative_path};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn load(path: &Path) -> Result<ProjectDocument, String> {
    let bytes = crate::map::source::read_limited(path, 32 * 1024 * 1024)?;
    let document: ProjectDocument =
        serde_json::from_slice(&bytes).map_err(|_| "Corrupt project JSON")?;
    document.validate()?;
    Ok(document)
}
pub fn save(path: &Path, document: &ProjectDocument) -> Result<(), String> {
    document.validate()?;
    let parent = path.parent().ok_or("Project has no root directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = crate::map::builder::cache::Staging::new(parent)?;
    let temp = stage.path.join("project.json");
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(document).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
pub fn asset_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    if !relative_path(path) {
        return Err("Asset must use a relative project path".into());
    }
    let resolved = root
        .join(path)
        .canonicalize()
        .map_err(|_| "Missing project resource")?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(&root) {
        return Err("Resource symlink leaves project directory".into());
    }
    Ok(resolved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_project_roundtrip_and_corruption() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.json");
        let doc = ProjectDocument::default();
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
        let mut bad = doc.clone();
        bad.schema_version = 100;
        assert!(save(&path, &bad).is_err());
        assert_eq!(load(&path).unwrap(), doc);
        fs::write(&path, b"broken").unwrap();
        assert!(load(&path).is_err());
    }
}

pub fn save_as(path: &Path, old: Option<&Path>, document: &ProjectDocument) -> Result<(), String> {
    let root = path.parent().ok_or("Missing project directory")?;
    for directory in ["Assets/Models", "Assets/Textures", "Geodata"] {
        fs::create_dir_all(root.join(directory)).map_err(|e| e.to_string())?;
    }
    if let Some(old) = old
        && old.parent() != Some(root)
    {
        let previous = old.parent().ok_or("Missing previous project directory")?;
        for a in &document.assets {
            let source = asset_path(previous, &a.path)?;
            let target = root.join(&a.path);
            if target.exists() {
                return Err("Save As would overwrite an existing resource".into());
            }
            fs::create_dir_all(target.parent().ok_or("Invalid asset directory")?)
                .map_err(|e| e.to_string())?;
            fs::copy(source, target).map_err(|e| e.to_string())?;
        }
    }
    save(path, document)
}
pub fn import_asset(root: &Path, source: &Path) -> Result<starter_core::ProjectAsset, String> {
    let file = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Unsupported asset filename")?;
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let texture = matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "webp");
    let directory = if texture {
        "Assets/Textures"
    } else {
        "Assets/Models"
    };
    fs::create_dir_all(root.join(directory)).map_err(|e| e.to_string())?;
    let relative = format!("{directory}/{file}");
    let target = root.join(&relative);
    if target.exists() {
        return Err("An asset with that name already exists".into());
    }
    let stage = crate::map::builder::cache::Staging::new(&root.join(directory))?;
    fs::copy(source, stage.path.join("asset")).map_err(|e| e.to_string())?;
    fs::hard_link(stage.path.join("asset"), &target).map_err(|e| e.to_string())?;
    let size = target.metadata().map_err(|e| e.to_string())?.len();
    Ok(starter_core::ProjectAsset {
        path: relative,
        media_type: if texture { "Texture" } else { "Model / file" }.into(),
        size,
    })
}
