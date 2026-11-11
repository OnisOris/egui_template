//! One bounded Bevy task: directory dialog, read/validate, then prepare CPU meshes off-thread.
use super::{
    dem_mesh,
    region::ActiveMapRegion,
    source::{self, TerrainPayload},
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use std::{path::PathBuf, sync::Arc};

pub struct PreparedTerrain {
    pub data: Arc<TerrainPayload>,
    pub meshes: Vec<Mesh>,
    pub grid: Option<Mesh>,
}

#[derive(Resource, Default)]
pub struct TerrainImport {
    task: Option<Task<Result<Option<PreparedTerrain>, String>>>,
    requested_revision: Option<u64>,
    pub prepared: Option<PreparedTerrain>,
    pub error: Option<String>,
}

impl TerrainImport {
    pub fn loading(&self) -> bool {
        self.task.is_some()
    }

    pub fn start(
        &mut self,
        active: &ActiveMapRegion,
        directory: Option<PathBuf>,
        cached: Option<blake3::Hash>,
    ) {
        self.requested_revision = active.get().map(|(r, _)| r);
        self.error = None;
        self.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let directory = directory.or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Import UAV terrain package v1")
                    .pick_folder()
            });
            let Some(directory) = directory else {
                return Ok(None);
            };
            let data = Arc::new(source::load_terrain(&directory)?);
            let reuse = cached == Some(data.fingerprint);
            let meshes = if reuse {
                Vec::new()
            } else {
                dem_mesh::chunks(&data)
            };
            let grid = (!reuse).then(|| dem_mesh::grid(&data));
            Ok(Some(PreparedTerrain { data, meshes, grid }))
        }));
    }

    pub fn is_current(&self, active: &ActiveMapRegion) -> bool {
        self.requested_revision == active.get().map(|(r, _)| r)
    }
}

pub fn poll_import(active: Res<ActiveMapRegion>, mut import: ResMut<TerrainImport>) {
    if !import.is_current(&active) {
        import.task = None; // Dropping the handle cancels/discards an obsolete result.
        return;
    }
    let Some(task) = import.task.as_mut() else {
        return;
    };
    if let Some(result) = future::block_on(future::poll_once(task)) {
        import.task = None;
        match result {
            Ok(Some(prepared)) => {
                import.prepared = Some(prepared);
                import.error = None;
            }
            Ok(None) => {} // Dialog cancellation keeps the last correct scene/package.
            Err(error) => import.error = Some(error),
        }
    }
}

pub fn import_from_env(active: Res<ActiveMapRegion>, mut import: ResMut<TerrainImport>) {
    if let Some(directory) = std::env::var_os("STARTER_TERRAIN_PACKAGE") {
        import.start(&active, Some(directory.into()), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{ecs::schedule::Schedule, tasks::TaskPool};
    #[test]
    fn obsolete_task_result_is_discarded_before_it_can_replace_the_scene() {
        let pool = AsyncComputeTaskPool::get_or_init(TaskPool::new);
        let dir = super::super::dem_tests::package(3, 3);
        let data = Arc::new(source::load_terrain(dir.path()).unwrap());
        let region = data.manifest.region();
        let task = pool.spawn(async move {
            Ok(Some(PreparedTerrain {
                data,
                meshes: Vec::new(),
                grid: None,
            }))
        });
        let import = TerrainImport {
            task: Some(task),
            requested_revision: None,
            ..Default::default()
        };
        let mut world = World::new();
        let mut active = ActiveMapRegion::default();
        active.confirm(region).unwrap();
        world.insert_resource(active);
        world.insert_resource(import);
        let mut schedule = Schedule::default();
        schedule.add_systems(poll_import);
        schedule.run(&mut world);
        let result = world.resource::<TerrainImport>();
        assert!(!result.loading());
        assert!(result.prepared.is_none());
    }
}
