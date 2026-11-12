use starter_core::{ProjectDocument, project::relative_path};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn load(path: &Path) -> Result<ProjectDocument, String> {
    let bytes = crate::map::source::read_limited(path, 32 * 1024 * 1024)?;
    let mut document: ProjectDocument =
        serde_json::from_slice(&bytes).map_err(|_| "Corrupt project JSON")?;
    document.migrate()?;
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

pub fn save_as(path: &Path, old: Option<&Path>, document: &ProjectDocument) -> Result<(), String> {
    let root = path.parent().ok_or("Missing project directory")?;
    for directory in ["Assets/Models", "Assets/Textures", "Geodata"] {
        controlled_directory(root, directory)?;
    }
    if let Some(old) = old
        && old.parent() != Some(root)
    {
        let previous = old.parent().ok_or("Missing previous project directory")?;
        for a in &document.assets {
            let source = match asset_path(previous, &a.path) {
                Ok(source) => source,
                Err(e) if document.models.iter().any(|m| m.asset_path == a.path) => {
                    return Err(format!("Cannot copy used model {}: {e}", a.path));
                }
                Err(_) => continue,
            };
            let target = root.join(&a.path);
            if target.symlink_metadata().is_ok() {
                return Err("Save As would overwrite an existing resource".into());
            }
            let directory = controlled_directory(
                root,
                Path::new(&a.path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .ok_or("Invalid asset directory")?,
            )?;
            let stage = crate::map::builder::cache::Staging::new(&directory)?;
            let staged = stage.path.join("asset");
            fs::copy(source, &staged).map_err(|e| e.to_string())?;
            fs::hard_link(staged, target).map_err(|e| e.to_string())?;
        }
    }
    save(path, document)
}
pub fn import_asset(root: &Path, source: &Path) -> Result<starter_core::ProjectAsset, String> {
    let file = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Unsupported asset filename")?;
    if !relative_path(file) {
        return Err("Unsupported asset filename".into());
    }
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
    let model_bytes = (!texture)
        .then(|| {
            crate::models::files::read(source, crate::models::files::max_bytes())
                .map(|(bytes, _)| bytes)
        })
        .transpose()?;
    controlled_directory(root, directory)?;
    let relative = format!("{directory}/{file}");
    let target = root.join(&relative);
    if target.exists() {
        return Err("An asset with that name already exists".into());
    }
    let stage = crate::map::builder::cache::Staging::new(&root.join(directory))?;
    if let Some(bytes) = model_bytes {
        fs::write(stage.path.join("asset"), bytes).map_err(|e| e.to_string())?;
    } else {
        fs::copy(source, stage.path.join("asset")).map_err(|e| e.to_string())?;
    }
    fs::hard_link(stage.path.join("asset"), &target).map_err(|e| e.to_string())?;
    let size = target.metadata().map_err(|e| e.to_string())?.len();
    Ok(starter_core::ProjectAsset {
        path: relative,
        media_type: if texture {
            "Texture"
        } else {
            "model/gltf-binary"
        }
        .into(),
        size,
    })
}

fn controlled_directory(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !relative_path(relative) {
        return Err("Invalid project directory".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut directory = root.clone();
    for component in Path::new(relative).components() {
        directory.push(component);
        if directory.exists() {
            directory = directory.canonicalize().map_err(|e| e.to_string())?;
            if !directory.starts_with(&root) || !directory.is_dir() {
                return Err("Asset directory leaves the project root".into());
            }
        } else {
            fs::create_dir(&directory).map_err(|e| e.to_string())?;
        }
    }
    Ok(directory)
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
    #[test]
    fn imported_resources_follow_save_as_and_missing_links_survive() {
        let source = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let image = source.path().join("survey.png");
        fs::write(&image, b"fixture asset").unwrap();
        let asset = import_asset(root.path(), &image).unwrap();
        assert_eq!(asset.path, "Assets/Textures/survey.png");
        assert!(import_asset(root.path(), &image).is_err());
        let mut doc = ProjectDocument::default();
        doc.assets.push(asset.clone());
        let path = root.path().join("project.json");
        save_as(&path, None, &doc).unwrap();
        let target = second.path().join("project.json");
        save_as(&target, Some(&path), &doc).unwrap();
        assert_eq!(
            fs::read(asset_path(second.path(), &asset.path).unwrap()).unwrap(),
            b"fixture asset"
        );
        fs::remove_file(asset_path(root.path(), &asset.path).unwrap()).unwrap();
        assert_eq!(load(&path).unwrap().assets, vec![asset.clone()]);
        assert!(asset_path(root.path(), &asset.path).is_err());
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
    }
    #[cfg(unix)]
    #[test]
    fn importing_into_symlinked_asset_directory_cannot_leave_the_project() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Assets")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("Assets/Models")).unwrap();
        let model = source.path().join("model.glb");
        fs::write(&model, include_bytes!("../../tests/fixtures/box.glb")).unwrap();
        assert!(import_asset(root.path(), &model).is_err());
        assert!(!outside.path().join("model.glb").exists());
    }
    #[cfg(unix)]
    #[test]
    fn save_as_cannot_follow_a_dangling_target_symlink_outside_the_project() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("Assets/Models")).unwrap();
        fs::create_dir_all(target.path().join("Assets/Models")).unwrap();
        let relative = "Assets/Models/box.glb";
        fs::write(
            source.path().join(relative),
            include_bytes!("../../tests/fixtures/box.glb"),
        )
        .unwrap();
        let mut document = ProjectDocument::default();
        document.assets.push(starter_core::ProjectAsset {
            path: relative.into(),
            media_type: "model/gltf-binary".into(),
            size: 1,
        });
        let original = source.path().join("project.json");
        save(&original, &document).unwrap();
        std::os::unix::fs::symlink(outside.path().join("box.glb"), target.path().join(relative))
            .unwrap();
        assert!(
            save_as(
                &target.path().join("project.json"),
                Some(&original),
                &document
            )
            .is_err()
        );
        assert!(!outside.path().join("box.glb").exists());
        assert_eq!(load(&original).unwrap(), document);
    }
}
