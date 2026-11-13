use super::{data::VectorData, package, source};
use crate::map::builder::{Job, cache};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
pub enum Load {
    Import,
    Online,
    Cached(String),
}
pub type ResultTask = Task<Result<Option<VectorData>, String>>;
pub fn start(region: starter_core::MapRegion, request: Load) -> (Job, ResultTask) {
    let job = Job {
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(Mutex::new("Vectors: Loading…".into())),
    };
    let worker = job.clone();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let root = cache::root()?;
        let data = match request {
            Load::Import => {
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Import map-data v2 vectors")
                    .pick_folder()
                else {
                    return Ok(None);
                };
                let data = package::load(&path, region)?;
                source::store_cache(&data, &root.join("vectors").join(&data.package_id))?;
                data
            }
            Load::Online => source::load(region, &root, source::TILEJSON, &worker)?,
            Load::Cached(id) => source::read_cache(&root.join("vectors").join(&id), region, &id)
                .map_err(
                    |_| "Missing/incompatible vector cache · Reload vectors or Import Vectors",
                )?,
        };
        worker.check()?;
        Ok(Some(data))
    });
    (job, task)
}
