use super::{data::VectorData, mesh, package};
use crate::map::{
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
};
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};

type Prepared = (VectorData, Vec<mesh::Chunk>);
#[derive(Resource, Default)]
pub struct VectorScene {
    task: Option<Task<Result<Option<Prepared>, String>>>,
    key: Option<(u64, blake3::Hash)>,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
    pub data: Option<VectorData>,
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
        self.data = None;
    }
}
pub fn update(
    mut commands: Commands,
    active: Res<ActiveMapRegion>,
    mut scene: ResMut<RealScene>,
    mut vectors: ResMut<VectorScene>,
    mut assets: SceneAssets,
) {
    let key = active
        .get()
        .zip(scene.data.as_ref())
        .map(|((r, _), d)| (r, d.fingerprint));
    if vectors.key != key {
        vectors.task = None;
        vectors.clear(&mut commands, &mut assets);
        vectors.key = key;
        scene.vector_status = "Vectors: Not loaded".into();
    }
    if std::mem::take(&mut scene.vector_import_requested)
        && let (Some((_, region)), Some(terrain)) = (active.get(), scene.data.clone())
    {
        scene.vector_status = "Vectors: Loading package…".into();
        vectors.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Import map-data v2 vectors")
                .pick_folder()
            else {
                return Ok(None);
            };
            let data = package::load(&path, region)?;
            let chunks = mesh::build(&data, &terrain, false);
            Ok(Some((data, chunks)))
        }));
    }
    let Some(task) = vectors.task.as_mut() else {
        return;
    };
    let Some(result) = future::block_on(future::poll_once(task)) else {
        return;
    };
    vectors.task = None;
    match result {
        Err(e) => scene.vector_status = format!("Vectors: Failed · {e}"),
        Ok(None) => scene.vector_status = "Vectors: Import cancelled".into(),
        Ok(Some((data, chunks))) => {
            vectors.clear(&mut commands, &mut assets);
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
            scene.vector_status = format!(
                "Vectors: Ready · {} buildings · {} roads · {}",
                data.buildings.len(),
                data.roads.len(),
                data.attribution
            );
            vectors.data = Some(data);
        }
    }
}
