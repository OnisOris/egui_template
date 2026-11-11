//! Application-specific layout persistence; desktop continues to use eframe storage.
use crate::state::UiState;
use bevy::prelude::*;
use starter_ui::Workspace;
use std::path::PathBuf;

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "OnisOris", "starter-bevy")
        .map(|dirs| dirs.config_dir().join("layout.json"))
}

pub fn load() -> Option<Workspace> {
    path()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn save_on_exit(mut exits: MessageReader<AppExit>, state: Res<UiState>) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(path) = path() else {
        return;
    };
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec(&state.workspace)?)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    })();
    if let Err(error) = result {
        warn!("Could not save docking layout: {error}");
    }
}
