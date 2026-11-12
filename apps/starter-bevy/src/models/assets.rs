use super::files;
use crate::project::{Project, ProjectIo};
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    gltf::{Gltf, GltfAssetLabel, GltfLoaderSettings},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelStatus {
    Loading,
    Ready,
    MissingAsset,
    UnsupportedModel(String),
    Failed(String),
}
impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading => write!(f, "Loading"),
            Self::Ready => write!(f, "Ready"),
            Self::MissingAsset => write!(f, "Missing Asset"),
            Self::UnsupportedModel(e) => write!(f, "Unsupported Model: {e}"),
            Self::Failed(e) => write!(f, "Failed: {e}"),
        }
    }
}
pub struct ModelAsset {
    pub status: ModelStatus,
    pub scene: Option<Handle<WorldAsset>>,
    pub gltf: Option<Handle<Gltf>>,
    pub(super) task: Option<Task<Result<files::Prepared, String>>>,
}
#[derive(Resource)]
pub struct ModelAssets {
    pub entries: HashMap<String, ModelAsset>,
    pub max_bytes: u64,
    root: Option<PathBuf>,
    generation: u64,
}
impl Default for ModelAssets {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            max_bytes: files::max_bytes(),
            root: None,
            generation: 0,
        }
    }
}
pub fn sync(
    project: Res<Project>,
    io: Res<ProjectIo>,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut models: ResMut<ModelAssets>,
) {
    let root = io
        .path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.to_owned());
    if models.root != root || models.generation != io.generation {
        models.entries.clear();
        models.root = root.clone();
        models.generation = io.generation;
    }
    models
        .entries
        .retain(|path, _| project.document.assets.iter().any(|a| &a.path == path));
    let Some(root) = root else {
        return;
    };
    let Some(cache) = files::cache_root() else {
        return;
    };
    let mut running = models.entries.values().filter(|m| m.task.is_some()).count();
    for asset in project
        .document
        .assets
        .iter()
        .filter(|a| a.path.to_lowercase().ends_with(".glb"))
    {
        if models.entries.contains_key(&asset.path) || running >= 4 {
            continue;
        }
        let (root, cache, path, limit) = (
            root.clone(),
            cache.clone(),
            asset.path.clone(),
            models.max_bytes,
        );
        let task = AsyncComputeTaskPool::get()
            .spawn(async move { files::prepare(&root, &path, &cache, limit) });
        models.entries.insert(
            asset.path.clone(),
            ModelAsset {
                status: ModelStatus::Loading,
                scene: None,
                gltf: None,
                task: Some(task),
            },
        );
        running += 1;
    }
    for model in models.entries.values_mut() {
        if let Some(result) = model
            .task
            .as_mut()
            .and_then(|t| future::block_on(future::poll_once(t)))
        {
            model.task = None;
            match result {
                Ok(prepared) => {
                    let path = format!("models://{}", prepared.file);
                    model.gltf = Some(
                        server
                            .load_builder()
                            .with_settings(settings)
                            .load(path.clone()),
                    );
                    model.scene = Some(
                        server
                            .load_builder()
                            .with_settings(settings)
                            .load(GltfAssetLabel::Scene(prepared.scene).from_asset(path)),
                    );
                }
                Err(e) => {
                    model.status = if e.contains("Missing project resource") {
                        ModelStatus::MissingAsset
                    } else {
                        ModelStatus::UnsupportedModel(e)
                    }
                }
            }
        }
        if let (Some(gltf), Some(scene)) = (&model.gltf, &model.scene) {
            if let Some(LoadState::Failed(e)) = server.get_load_state(gltf.id()) {
                model.status = ModelStatus::Failed(e.to_string());
            } else if let Some(LoadState::Failed(e)) = server.get_load_state(scene.id()) {
                model.status = ModelStatus::Failed(e.to_string());
            } else if let Some(RecursiveDependencyLoadState::Failed(e)) =
                server.get_recursive_dependency_load_state(gltf.id())
            {
                model.status = ModelStatus::Failed(e.to_string());
            } else if gltfs.contains(gltf) && server.is_loaded_with_dependencies(scene.id()) {
                model.status = ModelStatus::Ready;
            }
        }
    }
}

pub fn settings(settings: &mut GltfLoaderSettings) {
    settings.load_cameras = false;
    settings.load_lights = false;
    settings.load_animations = false;
}
