mod io;
mod scene;
pub mod storage;
pub mod ui;
use bevy::prelude::*;
pub use io::{ProjectAction, ProjectIo};
use starter_core::ProjectState;

#[derive(Resource, Default)]
pub struct Project(pub ProjectState);
impl std::ops::Deref for Project {
    type Target = ProjectState;
    fn deref(&self) -> &ProjectState {
        &self.0
    }
}
impl std::ops::DerefMut for Project {
    fn deref_mut(&mut self) -> &mut ProjectState {
        &mut self.0
    }
}

pub struct ProjectPlugin;
impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Project>()
            .init_resource::<ProjectIo>()
            .add_systems(
                Update,
                (io::poll, scene::sync_project)
                    .chain()
                    .before(crate::map::loading::poll_import),
            )
            .add_systems(Update, io::close_requested);
        app.world_mut().resource_mut::<ProjectIo>().benchmark =
            std::env::var_os("STARTER_LOAD").is_some();
    }
}
