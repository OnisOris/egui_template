//! Bounded patch GPU resources; one cancellable preparation task, four retained patches.
use super::{
    imagery_3d::{self, LiveTexture},
    imagery_lod::{self, Plan},
    real_scene::{RealScene, SceneAssets},
    satellite_scene::{ImagerySource, create_image},
};
use bevy::prelude::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
struct Patch {
    image: Handle<Image>,
    material: Handle<StandardMaterial>,
    zoom: u8,
}
struct Pending {
    index: usize,
    zoom: u8,
    job: super::builder::Job,
    reply: Mutex<mpsc::Receiver<Result<LiveTexture, String>>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.job.cancelled.store(true, Ordering::Relaxed);
    }
}
#[derive(Resource)]
pub struct ImageryDetail {
    base: Option<bevy::asset::AssetId<Image>>,
    patches: BTreeMap<usize, Patch>,
    pending: Option<Pending>,
    wanted: Vec<(usize, u8)>,
    changed: Instant,
    failed: BTreeMap<(usize, u8), Instant>,
    pub material_updates: u64,
}
impl Default for ImageryDetail {
    fn default() -> Self {
        Self {
            base: None,
            patches: BTreeMap::new(),
            pending: None,
            wanted: Vec::new(),
            changed: Instant::now(),
            failed: BTreeMap::new(),
            material_updates: 0,
        }
    }
}
fn remove(
    index: usize,
    detail: &mut ImageryDetail,
    scene: &RealScene,
    assets: &mut SceneAssets,
    entities: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Some(patch) = detail.patches.remove(&index) {
        if let Some(&entity) = scene.entities.get(index)
            && let Ok(mut material) = entities.get_mut(entity)
            && material.0 == patch.material
            && let Some(base) = scene.materials.first()
        {
            material.0 = base.clone();
        }
        assets.images.remove(patch.image.id());
        assets.materials.remove(patch.material.id());
    }
}
pub fn update(
    mut picker: NonSendMut<super::picker::MapPicker>,
    scene: Res<RealScene>,
    viewport: Res<crate::viewport::ViewportState>,
    mut detail: ResMut<ImageryDetail>,
    mut assets: SceneAssets,
    mut entities: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let base = scene
        .satellite
        .as_ref()
        .filter(|s| scene.satellite_enabled && matches!(s.source, ImagerySource::Live { .. }));
    let base_id = base.map(|b| b.image.id());
    if detail.base != base_id {
        detail.pending = None;
        let old: Vec<_> = detail.patches.keys().copied().collect();
        for i in old {
            remove(i, &mut detail, &scene, &mut assets, &mut entities);
        }
        detail.base = base_id;
        detail.wanted.clear();
        detail.failed.clear();
    }
    let (Some(base), Some(terrain), Some(service)) = (base, &scene.data, &picker.imagery.service)
    else {
        return;
    };
    let service = service.clone();
    if !matches!(base.source, ImagerySource::Live { generation } if generation == service.generation())
    {
        return;
    }
    let installed = detail.patches.iter().map(|(&i, p)| (i, p.zoom)).collect();
    let plans = imagery_lod::plans(
        terrain,
        &service,
        &viewport,
        base.metadata.zoom as u8,
        &installed,
    );
    let mut wanted: Vec<_> = plans.iter().map(|p| (p.index, p.zoom)).collect();
    wanted.sort_unstable();
    if detail.wanted != wanted {
        detail.wanted = wanted;
        detail.changed = Instant::now();
        if detail
            .pending
            .as_ref()
            .is_some_and(|p| !detail.wanted.contains(&(p.index, p.zoom)))
        {
            detail.pending = None;
        }
    }
    let old: Vec<_> = detail
        .patches
        .keys()
        .copied()
        .filter(|i| !plans.iter().any(|p| p.index == *i))
        .collect();
    for i in old {
        remove(i, &mut detail, &scene, &mut assets, &mut entities);
    }
    if let Some(pending) = &detail.pending {
        let result = pending.reply.lock().unwrap().try_recv().ok();
        if let Some(result) = result {
            let (index, zoom) = (pending.index, pending.zoom);
            detail.pending = None;
            match result {
                Ok(texture) => {
                    if let Some(&entity) = scene.entities.get(index)
                        && let Ok(mut current) = entities.get_mut(entity)
                    {
                        let transform = imagery_lod::transform(
                            &base.metadata.georeference,
                            &texture.metadata.georeference,
                        );
                        let image = assets
                            .images
                            .add(create_image(&texture.metadata, texture.rgba));
                        let material = assets.materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            base_color_texture: Some(image.clone()),
                            uv_transform: transform,
                            perceptual_roughness: 0.95,
                            ..default()
                        });
                        current.0 = material.clone();
                        if let Some(old) = detail.patches.insert(
                            index,
                            Patch {
                                image,
                                material,
                                zoom,
                            },
                        ) {
                            assets.images.remove(old.image.id());
                            assets.materials.remove(old.material.id());
                        }
                        detail.material_updates += 1;
                        info!(
                            "Imagery chunk {index}: installed z{zoom}, {} patches, {} material updates",
                            detail.patches.len(),
                            detail.material_updates
                        );
                    }
                }
                Err(error) => {
                    warn!("Imagery detail unavailable; retaining previous texture: {error}");
                    detail.failed.insert((index, zoom), Instant::now());
                }
            }
        }
    }
    if detail.pending.is_none() && detail.changed.elapsed() >= Duration::from_millis(300) {
        let next = plans
            .iter()
            .find(|p| {
                detail
                    .patches
                    .get(&p.index)
                    .is_none_or(|old| old.zoom != p.zoom)
                    && detail
                        .failed
                        .get(&(p.index, p.zoom))
                        .is_none_or(|at| at.elapsed() > Duration::from_secs(5))
            })
            .cloned();
        if let Some(Plan {
            index,
            region,
            zoom,
        }) = next
        {
            let terrain = terrain.clone();
            let job = super::builder::Job {
                cancelled: Arc::new(AtomicBool::new(false)),
                progress: Arc::default(),
            };
            let work = job.clone();
            let (send, reply) = mpsc::sync_channel(1);
            detail.pending = Some(Pending {
                index,
                zoom,
                job,
                reply: Mutex::new(reply),
            });
            std::thread::spawn(move || {
                let _ = send.send(imagery_3d::prepare(
                    &service, &terrain, region, zoom, &work, false,
                ));
            });
        }
    }
    let zoom = detail
        .patches
        .values()
        .map(|p| p.zoom)
        .max()
        .unwrap_or(base.metadata.zoom as u8);
    picker.imagery.live_status = format!(
        "Satellite: Live · imagery zoom {zoom} · {} detail textures{}",
        detail.patches.len(),
        detail
            .pending
            .as_ref()
            .map(|p| format!(" · {}", p.job.progress.lock().unwrap()))
            .unwrap_or_default()
    );
}
