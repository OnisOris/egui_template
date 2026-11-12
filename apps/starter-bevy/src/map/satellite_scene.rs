//! One asynchronous satellite task and one image owned by the visible DEM scene.
use super::{
    real_scene::{RealScene, SceneAssets},
    region::ActiveMapRegion,
    satellite::{self, MapManifest, SatellitePayload},
};
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use std::path::PathBuf;

pub struct InstalledSatellite {
    pub image: Handle<Image>,
    pub manifest: MapManifest,
    pub fingerprint: blake3::Hash,
}
#[derive(Resource, Default)]
pub struct SatelliteImport {
    task: Option<Task<Result<Option<SatellitePayload>, String>>>,
    revision: Option<u64>,
    terrain: Option<blake3::Hash>,
    pub error: Option<String>,
    env_attempted: bool,
    pub pending: Option<SatellitePayload>,
    pub packages: super::package_browser::PackageBrowser,
}
impl SatelliteImport {
    pub fn cancel(&mut self) {
        self.pending = None;
        self.task = None;
        self.error = None;
    }
    pub fn loading(&self) -> bool {
        self.task.is_some()
    }
    pub fn start(
        &mut self,
        active: &ActiveMapRegion,
        scene: &RealScene,
        directory: Option<PathBuf>,
    ) {
        let data = scene.data.clone();
        if data.is_some() && scene.revision != active.get().map(|(r, _)| r) {
            self.error =
                Some("Open the confirmed region's DEM before importing satellite imagery.".into());
            return;
        }
        self.revision = active.get().map(|(r, _)| r);
        self.terrain = data.as_ref().map(|d| d.fingerprint);
        self.pending = None;
        self.error = None;
        self.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let path = directory.or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Import matching UAV map-data v2 package")
                    .pick_folder()
            });
            path.map(|p| {
                let mut payload = satellite::read_package(&p, None)?;
                if let Some(data) = &data
                    && payload.manifest.validate(Some(data)).is_ok()
                {
                    satellite::prepare_uvs(&mut payload, data)?;
                }
                Ok(payload)
            })
            .transpose()
        }));
    }
    pub fn is_current(&self, active: &ActiveMapRegion, scene: &RealScene) -> bool {
        self.revision == active.get().map(|(r, _)| r)
            && (self.terrain.is_none()
                || (self.revision == scene.revision && self.terrain == scene.fingerprint()))
    }
}

pub fn poll_satellite(
    active: Res<ActiveMapRegion>,
    mut import: ResMut<SatelliteImport>,
    mut scene: ResMut<RealScene>,
    mut assets: SceneAssets,
) {
    if !import.env_attempted && scene.data.is_some() {
        import.env_attempted = true;
        if let Some(path) = std::env::var_os("STARTER_SATELLITE_PACKAGE") {
            import.start(&active, &scene, Some(path.into()));
        }
    }
    if import.task.is_none()
        && scene.revision == active.get().map(|(r, _)| r)
        && let Some(data) = scene.data.clone()
        && import
            .pending
            .as_ref()
            .is_some_and(|p| p.manifest.validate(Some(&data)).is_ok())
    {
        let mut payload = import.pending.take().unwrap();
        import.revision = scene.revision;
        import.terrain = scene.fingerprint();
        import.error = None;
        import.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            satellite::prepare_uvs(&mut payload, &data)?;
            Ok(Some(payload))
        }));
    }
    if !import.is_current(&active, &scene) {
        import.task = None;
        return;
    }
    let Some(task) = import.task.as_mut() else {
        return;
    };
    if let Some(result) = future::block_on(future::poll_once(task)) {
        import.task = None;
        match result {
            Ok(Some(payload)) => {
                if payload.uvs.is_empty() {
                    import.error = scene.data.as_ref().map(|_| {
                        "Satellite package does not match the current terrain region".into()
                    });
                    import.pending = Some(payload);
                    import.packages.open = true;
                    return;
                }
                if let Err(error) = install(&mut scene, &mut assets, payload) {
                    import.error = Some(error);
                }
            }
            Ok(None) => {}
            Err(error) => import.error = Some(error),
        }
    }
}

pub fn install(
    scene: &mut RealScene,
    assets: &mut SceneAssets,
    payload: SatellitePayload,
) -> Result<(), String> {
    if scene
        .satellite
        .as_ref()
        .is_some_and(|s| s.fingerprint == payload.fingerprint)
    {
        scene.satellite_enabled = true;
        return Ok(());
    }
    // The last mesh is the line grid. Validate everything before modifying any asset.
    if payload.uvs.len() + 1 != scene.meshes.len()
        || payload.uvs.iter().zip(&scene.meshes).any(|(uv, h)| {
            assets
                .meshes
                .get(h.id())
                .is_none_or(|m| m.count_vertices() != uv.len())
        })
    {
        return Err("Satellite UV layout differs from the loaded terrain chunks.".into());
    }
    let s = &payload.manifest.satellite;
    let mut image = Image::new(
        Extent3d {
            width: s.width,
            height: s.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        payload.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    let image = assets.images.add(image);
    for (uv, handle) in payload.uvs.into_iter().zip(&scene.meshes) {
        assets
            .meshes
            .get_mut(handle.id())
            .unwrap()
            .insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    if let Some(old) = scene.satellite.take() {
        assets.images.remove(old.image.id());
    }
    scene.satellite = Some(InstalledSatellite {
        image,
        manifest: payload.manifest,
        fingerprint: payload.fingerprint,
    });
    scene.satellite_enabled = true;
    scene.grid_enabled = false;
    Ok(())
}

pub fn update_material(scene: Res<RealScene>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Some(handle) = scene.materials.first() else {
        return;
    };
    let texture = scene
        .satellite
        .as_ref()
        .filter(|_| scene.satellite_enabled)
        .map(|s| s.image.clone());
    if let Some(mut material) = materials.get_mut(handle.id())
        && material.base_color_texture != texture
    {
        material.base_color = if texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.5, 0.57, 0.49)
        };
        material.base_color_texture = texture;
    }
}
