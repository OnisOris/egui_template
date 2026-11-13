use super::{
    data::VectorData,
    loading::{self, Load},
    mesh,
};
use crate::map::{
    RegionRuntime,
    builder::Job,
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
};
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use std::sync::{Arc, atomic::Ordering};

#[derive(Resource, Default)]
pub struct VectorScene {
    task: Option<loading::ResultTask>,
    restore_attempted: bool,
    project_generation: u64,
    imported: bool,
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
    fn start(&mut self, region: starter_core::MapRegion, request: Load) {
        self.cancel();
        self.imported = matches!(request, Load::Import);
        let (job, task) = loading::start(region, request);
        self.job = Some(job);
        self.task = Some(task);
        self.restore_attempted = true;
    }
}
pub fn update(
    mut commands: Commands,
    active: Res<ActiveMapRegion>,
    mut runtime: ResMut<RegionRuntime>,
    mut scene: ResMut<RealScene>,
    mut vectors: ResMut<VectorScene>,
    mut assets: SceneAssets,
    mut project: ResMut<crate::project::Project>,
    io: Res<crate::project::ProjectIo>,
) {
    let revision = active.get().map(|(r, _)| r);
    if vectors.revision != revision
        || vectors.project_generation != io.generation
        || scene.data.is_none() && vectors.terrain.is_some()
    {
        vectors.project_generation = io.generation;
        vectors.restore_attempted = false;
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
        let id = project
            .document
            .layer(starter_core::LayerKind::Roads)
            .package_id
            .clone();
        if manual {
            vectors.start(region, Load::Import)
        } else if project.document.vector_source == starter_core::VectorSource::OpenFreeMap {
            vectors.start(region, Load::Online)
        } else if let Some(id) = id {
            vectors.start(region, Load::Cached(id))
        } else {
            scene.vector_status = "Vectors: Unavailable · Import Vectors for this region".into();
        }
    }
    if !vectors.restore_attempted
        && vectors.task.is_none()
        && scene.revision == revision
        && let Some((_, region)) = active.get()
    {
        vectors.restore_attempted = true;
        if let Some(id) = project
            .document
            .layer(starter_core::LayerKind::Roads)
            .package_id
            .clone()
        {
            vectors.start(region, Load::Cached(id));
        }
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
                let mut changed = false;
                if vectors.imported
                    && project.document.vector_source != starter_core::VectorSource::LocalPackage
                {
                    changed = true;
                    project.document.vector_source = starter_core::VectorSource::LocalPackage;
                }
                for kind in [
                    starter_core::LayerKind::Roads,
                    starter_core::LayerKind::Buildings,
                ] {
                    if project.document.layer(kind).package_id.as_ref() != Some(&data.package_id) {
                        project.document.layer_mut(kind).package_id = Some(data.package_id.clone());
                        changed = true;
                    }
                }
                if changed {
                    project.changed();
                }
                if vectors
                    .data
                    .as_ref()
                    .is_some_and(|d| d.fingerprint == data.fingerprint && d.region == data.region)
                {
                    scene.vector_status = ready(&data);
                } else {
                    vectors.clear(&mut commands, &mut assets);
                    vectors.data = Some(Arc::new(data));
                    vectors.terrain = None;
                    scene.vector_status = "Vectors: Loading · waiting for matching terrain".into();
                }
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
            let mut chunks = mesh::build(&data, &terrain, false);
            chunks.extend(mesh::build(&data, &terrain, true));
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
                    VectorChunk {
                        flat: c.flat,
                        buildings: c.buildings,
                    },
                    RenderLayers::layer(3),
                ))
                .id(),
        );
        vectors.meshes.push(h);
    }
    vectors.materials = vec![road, building];
    if let Some(data) = &vectors.data {
        scene.vector_status = ready(data);
    }
}

#[derive(Component)]
pub struct VectorChunk {
    pub flat: bool,
    pub buildings: bool,
}
pub fn visibility(
    project: Res<crate::project::Project>,
    mut chunks: Query<(&VectorChunk, &mut Visibility)>,
) {
    for (chunk, mut current) in &mut chunks {
        let mode = project.document.geo_view_mode;
        let layer = if chunk.buildings {
            starter_core::LayerKind::Buildings
        } else {
            starter_core::LayerKind::Roads
        };
        let shown = mode != starter_core::GeoViewMode::Terrain
            && chunk.flat == (mode == starter_core::GeoViewMode::City)
            && project.document.layer(layer).visible;
        let next = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *current != next {
            *current = next;
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

fn ready(data: &VectorData) -> String {
    format!(
        "Vectors: Ready · {} buildings · {} roads · {} · {}",
        data.buildings.len(),
        data.roads.len(),
        data.source,
        data.attribution
    )
}
