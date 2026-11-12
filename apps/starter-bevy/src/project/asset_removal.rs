//! Explicit removal of an unused imported resource, never an implicitly referenced file.
use std::{fs, path::Path};
pub fn remove(
    root: &Path,
    document: &starter_core::ProjectDocument,
    index: usize,
) -> Result<(), String> {
    let asset = document.assets.get(index).ok_or("Resource not found")?;
    if document.models.iter().any(|m| m.asset_path == asset.path) {
        return Err("Asset is used by model instances; delete those instances first".into());
    }
    if !(asset.path.starts_with("Assets/Models/") || asset.path.starts_with("Assets/Textures/")) {
        return Err("Only imported Assets resources can be removed".into());
    }
    if !starter_core::project::relative_path(&asset.path) {
        return Err("Invalid project asset path".into());
    }
    let target = root.join(&asset.path);
    match fs::symlink_metadata(&target) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(m) if m.file_type().is_symlink() => {
            return Err("Refusing to remove a symlink resource".into());
        }
        Ok(_) => {}
    }
    let resolved = super::storage::asset_path(root, &asset.path)?;
    if document.assets.iter().enumerate().any(|(i, a)| {
        i != index && super::storage::asset_path(root, &a.path).ok().as_ref() == Some(&resolved)
    }) {
        return Err("Another project resource refers to this file".into());
    }
    fs::remove_file(target).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removal_cancels_pending_placement_before_taking_its_snapshot() {
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let root = tempfile::tempdir().unwrap();
        let mut project = starter_core::ProjectState {
            placing_model: Some("Assets/Models/box.glb".into()),
            ..Default::default()
        };
        let mut io = crate::project::ProjectIo::default();
        io.path = Some(root.path().join("project.json"));
        io.request(crate::project::ProjectAction::RemoveAsset(0), &mut project);
        assert!(project.placing_model.is_none());
        assert!(io.busy());
    }
    #[test]
    fn used_models_cannot_be_removed_or_silently_omitted_from_save_as() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let file = source.path().join("box.glb");
        fs::write(&file, include_bytes!("../../tests/fixtures/box.glb")).unwrap();
        let asset = crate::project::storage::import_asset(root.path(), &file).unwrap();
        let mut project = starter_core::ProjectState::default();
        project.document.assets.push(asset.clone());
        project
            .add_model(
                asset.path.clone(),
                starter_core::GeoOrigin {
                    longitude: 8.0,
                    latitude: 46.0,
                },
            )
            .unwrap();
        assert!(remove(root.path(), &project.document, 0).is_err());
        assert!(root.path().join(&asset.path).is_file());
        let json = root.path().join("project.json");
        crate::project::storage::save(&json, &project.document).unwrap();
        crate::project::storage::save_as(
            &other.path().join("project.json"),
            Some(&json),
            &project.document,
        )
        .unwrap();
        assert_eq!(
            crate::project::storage::load(&other.path().join("project.json")).unwrap(),
            project.document
        );
        assert_eq!(
            fs::read(other.path().join(&asset.path)).unwrap(),
            include_bytes!("../../tests/fixtures/box.glb")
        );
        fs::remove_file(root.path().join(&asset.path)).unwrap();
        assert!(
            crate::project::storage::save_as(
                &other.path().join("project.json"),
                Some(&json),
                &project.document
            )
            .is_err()
        );
        assert_eq!(
            crate::project::storage::load(&json).unwrap(),
            project.document
        );
        project.delete_selected();
        assert!(remove(root.path(), &project.document, 0).is_ok());
        fs::write(
            root.path().join(&asset.path),
            include_bytes!("../../tests/fixtures/box.glb"),
        )
        .unwrap();
        remove(root.path(), &project.document, 0).unwrap();
        assert!(!root.path().join(&asset.path).exists());
    }
}
