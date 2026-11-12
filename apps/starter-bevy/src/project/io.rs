use super::{Project, storage};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
    window::WindowCloseRequested,
};
use starter_core::{ProjectAsset, ProjectDocument, ProjectState};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectAction {
    New,
    Open,
    Save,
    SaveAs,
    Import,
    Refresh,
    Exit,
}
enum Completed {
    Opened(PathBuf, ProjectDocument, Vec<bool>, u64),
    Saved(PathBuf, u64, u64),
    Imported(ProjectAsset),
    Refreshed(Vec<bool>),
    Cancelled,
}
#[derive(Resource, Default)]
pub struct ProjectIo {
    pub path: Option<PathBuf>,
    pub generation: u64,
    pub file_size: Option<u64>,
    pub error: Option<String>,
    pub missing: Vec<bool>,
    pub benchmark: bool,
    pub return_mode: crate::map::SceneMode,
    pub reset_scene: bool,
    pub restore: bool,
    pub exit: bool,
    pub prompt: Option<ProjectAction>,
    continuation: Option<ProjectAction>,
    task: Option<Task<Result<Completed, String>>>,
}
impl ProjectIo {
    pub fn busy(&self) -> bool {
        self.task.is_some()
    }
    pub fn request(&mut self, action: ProjectAction, project: &mut ProjectState) {
        if self.busy() {
            return;
        }
        if project.dirty
            && matches!(
                action,
                ProjectAction::New | ProjectAction::Open | ProjectAction::Exit
            )
        {
            self.prompt = Some(action);
            return;
        }
        self.execute(action, project);
    }
    pub fn discard(&mut self, project: &mut ProjectState) {
        if let Some(action) = self.prompt.take() {
            self.execute(action, project);
        }
    }
    pub fn save_before(&mut self, project: &mut ProjectState) {
        self.continuation = self.prompt.take();
        self.execute(ProjectAction::Save, project);
    }
    fn execute(&mut self, action: ProjectAction, project: &mut ProjectState) {
        self.error = None;
        if action == ProjectAction::New {
            *project = ProjectState {
                revision: project.revision.wrapping_add(1),
                ..Default::default()
            };
            self.generation = self.generation.wrapping_add(1);
            self.path = None;
            self.file_size = None;
            self.missing.clear();
            self.reset_scene = true;
            self.benchmark = false;
            return;
        }
        if action == ProjectAction::Exit {
            self.exit = true;
            return;
        }
        let old = self.path.clone();
        let snapshot = project.document.clone();
        let revision = project.revision;
        self.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let dialog = || rfd::FileDialog::new().add_filter("Geographic project", &["json"]);
            match action {
                ProjectAction::Open => {
                    let Some(path) = dialog().set_title("Open Project").pick_file() else {
                        return Ok(Completed::Cancelled);
                    };
                    let document = storage::load(&path)?;
                    let root = path.parent().ok_or("Missing project directory")?;
                    let missing = document
                        .assets
                        .iter()
                        .map(|a| storage::asset_path(root, &a.path).is_err())
                        .collect();
                    let size = path.metadata().map_err(|e| e.to_string())?.len();
                    Ok(Completed::Opened(path, document, missing, size))
                }
                ProjectAction::Save | ProjectAction::SaveAs => {
                    let path = if action == ProjectAction::Save {
                        old.clone()
                    } else {
                        None
                    };
                    let path = path.or_else(|| {
                        dialog()
                            .set_title("Save Project")
                            .set_file_name("project.json")
                            .save_file()
                    });
                    let Some(path) = path else {
                        return Ok(Completed::Cancelled);
                    };
                    storage::save_as(&path, old.as_deref(), &snapshot)?;
                    let size = path.metadata().map_err(|e| e.to_string())?.len();
                    Ok(Completed::Saved(path, revision, size))
                }
                ProjectAction::Refresh => {
                    let path = old.ok_or("Save the project first")?;
                    let root = path.parent().ok_or("Missing project directory")?;
                    Ok(Completed::Refreshed(
                        snapshot
                            .assets
                            .iter()
                            .map(|a| storage::asset_path(root, &a.path).is_err())
                            .collect(),
                    ))
                }
                ProjectAction::Import => {
                    let path = old.ok_or("Save the project before importing assets")?;
                    let Some(source) = rfd::FileDialog::new()
                        .add_filter("GLB model / image", &["glb", "png", "jpg", "jpeg", "webp"])
                        .set_title("Import Asset")
                        .pick_file()
                    else {
                        return Ok(Completed::Cancelled);
                    };
                    Ok(Completed::Imported(storage::import_asset(
                        path.parent().ok_or("Missing project directory")?,
                        &source,
                    )?))
                }
                _ => unreachable!(),
            }
        }));
    }
}
pub fn poll(
    mut io: ResMut<ProjectIo>,
    mut project: ResMut<Project>,
    mut exits: MessageWriter<AppExit>,
) {
    let result = io
        .task
        .as_mut()
        .and_then(|t| future::block_on(future::poll_once(t)));
    if let Some(result) = result {
        io.task = None;
        match result {
            Ok(Completed::Opened(path, document, missing, size)) => {
                io.generation = io.generation.wrapping_add(1);
                io.file_size = Some(size);
                project.0 = ProjectState {
                    document,
                    revision: project.revision.wrapping_add(1),
                    ..Default::default()
                };
                io.path = Some(path);
                io.missing = missing;
                io.reset_scene = true;
                io.restore = true;
                io.benchmark = false;
            }
            Ok(Completed::Saved(path, revision, size)) => {
                io.file_size = Some(size);
                io.path = Some(path);
                if project.revision == revision {
                    project.dirty = false;
                }
                if let Some(action) = io.continuation.take() {
                    if project.dirty {
                        io.prompt = Some(action);
                    } else {
                        io.execute(action, &mut project);
                    }
                }
            }
            Ok(Completed::Imported(asset)) => {
                project.document.assets.push(asset);
                project.changed();
                io.missing.push(false);
            }
            Ok(Completed::Refreshed(missing)) => {
                io.generation = io.generation.wrapping_add(1);
                io.missing = missing;
            }
            Ok(Completed::Cancelled) => {
                io.continuation = None;
            }
            Err(e) => {
                io.error = Some(e);
                io.prompt = io.continuation.take();
            }
        }
    }
    if std::mem::take(&mut io.exit) {
        exits.write(AppExit::Success);
    }
}
pub fn close_requested(
    mut close: MessageReader<WindowCloseRequested>,
    mut io: ResMut<ProjectIo>,
    mut project: ResMut<Project>,
) {
    if close.read().next().is_some() {
        io.request(ProjectAction::Exit, &mut project);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dirty_new_and_exit_wait_for_explicit_discard() {
        let mut state = ProjectState::default();
        state.document.name = "Keep me".into();
        state.changed();
        let mut io = ProjectIo::default();
        io.request(ProjectAction::New, &mut state);
        assert_eq!(state.document.name, "Keep me");
        assert_eq!(io.prompt, Some(ProjectAction::New));
        io.prompt = None;
        assert!(state.dirty);
        io.request(ProjectAction::Exit, &mut state);
        assert!(!io.exit);
        io.discard(&mut state);
        assert!(io.exit);
    }
}
