use super::{data::VectorData, mesh, package, source};
use crate::map::{
    RegionRuntime,
    builder::{Job, cache},
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
};
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource, Default)]
pub struct VectorScene {
    task: Option<Task<Result<Option<VectorData>, String>>>,
    mesh_task: Option<Task<Vec<mesh::Chunk>>>,
    job: Option<Job>,
    revision: Option<u64>,
    terrain: Option<blake3::Hash>,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
    pub data: Option<Arc<VectorData>>,
}
impl VectorScene {
    fn clear(&mut self, commands: &mut Commands, assets: &mut SceneAssets) {
        for e in self.entities.drain(..) {
            commands.entity(e).despawn();
        }
        for h in self.meshes.drain(..) {
            assets.meshes.remove(h.id());
        }
        for h in self.materials.drain(..) {
            assets.materials.remove(h.id());
        }
    }
    fn cancel(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancelled.store(true, Ordering::Relaxed)
        }
        self.task = None;
        self.mesh_task = None;
    }
    fn start(&mut self, region: starter_core::MapRegion, manual: bool) {
        self.cancel();
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new("Vectors: Loading…".into())),
        };
        self.job = Some(job.clone());
        self.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let root = cache::root()?;
            let data = if manual {
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Import map-data v2 vectors")
                    .pick_folder()
                else {
                    return Ok(None);
                };
                let data = package::load(&path, region)?;
                source::store_cache(&data, &root.join("vectors").join(&data.package_id))?;
                data
            } else {
                source::load(region, &root, source::TILEJSON, &job)?
            };
            job.check()?;
            Ok(Some(data))
        }));
    }
}
pub fn update(
    mut commands: Commands,
    active: Res<ActiveMapRegion>,
    mut runtime: ResMut<RegionRuntime>,
    mut scene: ResMut<RealScene>,
    mut vectors: ResMut<VectorScene>,
    mut assets: SceneAssets,
) {
    let revision = active.get().map(|(r, _)| r);
    if vectors.revision != revision || scene.data.is_none() && vectors.terrain.is_some() {
        vectors.cancel();
        vectors.clear(&mut commands, &mut assets);
        vectors.data = None;
        vectors.revision = revision;
        vectors.terrain = None;
        scene.vector_status = "Vectors: Not loaded".into();
    }
    let manual = std::mem::take(&mut scene.vector_import_requested);
    let automatic = std::mem::take(&mut runtime.vector_requested);
    if (manual || automatic)
        && let Some((_, region)) = active.get()
    {
        vectors.start(region, manual);
    }
    if let Some(job) = &vectors.job
        && vectors.task.is_some()
    {
        scene.vector_status = job.progress.lock().unwrap().clone();
    }
    if let Some(task) = vectors.task.as_mut()
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        vectors.task = None;
        vectors.job = None;
        match result {
            Err(e) => scene.vector_status = format!("Vectors: Failed · {e}"),
            Ok(None) => scene.vector_status = "Vectors: Import cancelled".into(),
            Ok(Some(data)) => {
                vectors.clear(&mut commands, &mut assets);
                vectors.data = Some(Arc::new(data));
                vectors.terrain = None;
                scene.vector_status = "Vectors: Ready · waiting for matching terrain".into();
            }
        }
    }
    let terrain = scene.data.as_ref().filter(|_| scene.revision == revision);
    if let (Some(data), Some(terrain)) = (vectors.data.clone(), terrain.cloned())
        && data.region == terrain.manifest.region()
        && vectors.terrain != Some(terrain.fingerprint)
    {
        vectors.mesh_task = None;
        vectors.clear(&mut commands, &mut assets);
        vectors.terrain = Some(terrain.fingerprint);
        vectors.mesh_task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let started = std::time::Instant::now();
            let chunks = mesh::build(&data, &terrain, false);
            info!(
                "Vector meshes: {} buildings, {} roads, {} chunks in {:.3} ms",
                data.buildings.len(),
                data.roads.len(),
                chunks.len(),
                started.elapsed().as_secs_f64() * 1000.
            );
            chunks
        }));
        scene.vector_status = "Vectors: Building geometry…".into();
    }
    if vectors.terrain != scene.fingerprint() {
        vectors.mesh_task = None;
        return;
    }
    let Some(task) = vectors.mesh_task.as_mut() else {
        return;
    };
    let Some(chunks) = future::block_on(future::poll_once(task)) else {
        return;
    };
    vectors.mesh_task = None;
    let road = assets.materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.66, 0.68),
        perceptual_roughness: 0.95,
        ..default()
    });
    let building = assets.materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.81, 0.82),
        perceptual_roughness: 0.9,
        ..default()
    });
    for c in chunks {
        let h = assets.meshes.add(c.mesh);
        vectors.entities.push(
            commands
                .spawn((
                    Mesh3d(h.clone()),
                    MeshMaterial3d(if c.buildings {
                        building.clone()
                    } else {
                        road.clone()
                    }),
                    RenderLayers::layer(3),
                ))
                .id(),
        );
        vectors.meshes.push(h);
    }
    vectors.materials = vec![road, building];
    if let Some(data) = &vectors.data {
        scene.vector_status = format!(
            "Vectors: Ready · {} buildings · {} roads · {}",
            data.buildings.len(),
            data.roads.len(),
            data.attribution
        );
    }
}
