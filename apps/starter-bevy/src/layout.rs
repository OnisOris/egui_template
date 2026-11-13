//! UI-only preferences: never stored in ProjectDocument. Legacy layouts still load.
use crate::state::UiState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use starter_ui::{Theme, Workspace, style::Appearance};
use std::path::{Path, PathBuf};
fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "OnisOris", "starter-bevy")
        .map(|dirs| dirs.config_dir().join("layout.json"))
}
pub fn load() -> Option<Workspace> {
    path()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub appearance: Appearance,
}
pub fn preferences() -> Preferences {
    path()
        .and_then(|p| std::fs::read(p.with_file_name("appearance.json")).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn write(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_vec(value)?)?;
    std::fs::rename(temp, path)?;
    Ok(())
}
pub fn save_on_exit(mut exits: MessageReader<AppExit>, state: Res<UiState>) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(path) = path() else {
        return;
    };
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let backup = path.with_file_name("layout.pre-m54.json");
        if path.exists() && !backup.exists() {
            std::fs::copy(&path, backup)?;
        }
        write(&path, &state.workspace)?;
        write(
            &path.with_file_name("appearance.json"),
            &Preferences {
                theme: state.theme,
                appearance: state.appearance,
            },
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        warn!("Could not save UI preferences: {error}");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_and_custom_docking_roundtrip_independently_of_projects() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("appearance.json");
        let settings = Preferences {
            theme: Theme::Light,
            appearance: Appearance {
                scale: 1.25,
                ..default()
            },
        };
        write(&p, &settings).unwrap();
        let restored: Preferences = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(restored.theme, Theme::Light);
        assert_eq!(restored.appearance.scale, 1.25);
        let mut workspace = Workspace::preset(starter_ui::WorkspacePreset::Analysis);
        workspace.set_visible(starter_ui::PanelId::Inspector, false);
        let p = dir.path().join("layout.json");
        write(&p, &workspace).unwrap();
        assert_eq!(
            serde_json::from_slice::<Workspace>(&std::fs::read(p).unwrap()).unwrap(),
            workspace
        );
    }
}
