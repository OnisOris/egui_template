use super::*;
use bevy::{ecs::schedule::Schedule, tasks::TaskPool};
use starter_core::{GeoViewMode, LayerKind};
fn world() -> World {
    let mut w = World::new();
    w.init_resource::<crate::project::Project>();
    w.init_resource::<crate::project::ProjectIo>();
    w.init_resource::<Assets<Mesh>>();
    w.init_resource::<Assets<Image>>();
    w.init_resource::<Assets<StandardMaterial>>();
    w.init_resource::<RegionRuntime>();
    w.init_resource::<ActiveMapRegion>();
    w.init_resource::<RealScene>();
    w.init_resource::<VectorScene>();
    w
}
#[test]
fn new_region_cancels_obsolete_ready_task_and_releases_owned_assets() {
    let pool = AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = Arc::new(crate::map::source::load_terrain(dir.path()).unwrap());
    let mut w = world();
    w.resource_mut::<ActiveMapRegion>()
        .confirm(terrain.manifest.region())
        .unwrap();
    w.resource_mut::<RealScene>().data = Some(terrain.clone());
    w.resource_mut::<RealScene>().revision = Some(1);
    let material = w
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    let mesh = w.resource_mut::<Assets<Mesh>>().add(Cuboid::default());
    let entity = w
        .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
        .id();
    let result = super::super::mvt::empty(terrain.manifest.region(), "stale".into());
    w.insert_resource(VectorScene {
        revision: Some(1),
        terrain: Some(terrain.fingerprint),
        entities: vec![entity],
        meshes: vec![mesh],
        materials: vec![material],
        task: Some(pool.spawn(async move { Ok(Some(result)) })),
        ..default()
    });
    w.resource_mut::<ActiveMapRegion>()
        .confirm(terrain.manifest.region())
        .unwrap();
    let mut s = Schedule::default();
    s.add_systems(update);
    s.run(&mut w);
    assert!(w.get_entity(entity).is_err());
    assert_eq!(w.resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(w.resource::<Assets<StandardMaterial>>().len(), 0);
    let v = w.resource::<VectorScene>();
    assert!(v.task.is_none() && v.data.is_none() && v.mesh_task.is_none());
}
#[test]
fn modes_and_layer_visibility_reuse_entities_meshes_and_materials() {
    let mut w = world();
    let entities: Vec<_> = [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .map(|(flat, buildings)| {
            w.spawn((VectorChunk { flat, buildings }, Visibility::Inherited))
                .id()
        })
        .collect();
    let mut s = Schedule::default();
    s.add_systems(visibility);
    for mode in [
        GeoViewMode::City,
        GeoViewMode::Terrain,
        GeoViewMode::Combined,
    ]
    .into_iter()
    .cycle()
    .take(30)
    {
        w.resource_mut::<crate::project::Project>()
            .document
            .geo_view_mode = mode;
        s.run(&mut w);
        for e in &entities {
            let c = w.get::<VectorChunk>(*e).unwrap();
            assert_eq!(
                *w.get::<Visibility>(*e).unwrap() != Visibility::Hidden,
                mode != GeoViewMode::Terrain && c.flat == (mode == GeoViewMode::City)
            );
        }
    }
    w.resource_mut::<crate::project::Project>()
        .document
        .layer_mut(LayerKind::Buildings)
        .visible = false;
    s.run(&mut w);
    assert!(
        entities
            .iter()
            .filter(|e| w.get::<VectorChunk>(**e).unwrap().buildings)
            .all(|e| *w.get::<Visibility>(*e).unwrap() == Visibility::Hidden)
    );
    assert_eq!(w.query::<&VectorChunk>().iter(&w).count(), 4);
}

#[test]
fn failed_reload_keeps_previous_vectors_and_dem() {
    let pool = AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = Arc::new(crate::map::source::load_terrain(dir.path()).unwrap());
    let mut w = world();
    w.resource_mut::<ActiveMapRegion>()
        .confirm(terrain.manifest.region())
        .unwrap();
    w.resource_mut::<RealScene>().data = Some(terrain.clone());
    w.resource_mut::<RealScene>().revision = Some(1);
    let e = w.spawn_empty().id();
    w.insert_resource(VectorScene {
        revision: Some(1),
        terrain: Some(terrain.fingerprint),
        entities: vec![e],
        restore_attempted: true,
        data: Some(Arc::new(super::super::mvt::empty(
            terrain.manifest.region(),
            "retained".into(),
        ))),
        task: Some(pool.spawn(async { Err("fixture HTTP 503".into()) })),
        ..default()
    });
    let mut s = Schedule::default();
    s.add_systems(update);
    for _ in 0..100 {
        s.run(&mut w);
        if w.resource::<VectorScene>().task.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(w.resource::<RealScene>().vector_status.contains("Failed"));
    assert_eq!(
        w.resource::<RealScene>().fingerprint(),
        Some(terrain.fingerprint)
    );
    assert_eq!(
        w.resource::<VectorScene>()
            .data
            .as_ref()
            .unwrap()
            .package_id,
        "retained"
    );
    assert!(w.get_entity(e).is_ok());
}

#[test]
fn roundtrip_noise_installs_once_and_cache_restore_keeps_project_clean() {
    let pool = AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = Arc::new(crate::map::source::load_terrain(dir.path()).unwrap());
    let mut w = world();
    w.resource_mut::<ActiveMapRegion>()
        .confirm(terrain.manifest.region())
        .unwrap();
    w.resource_mut::<RealScene>().data = Some(terrain.clone());
    w.resource_mut::<RealScene>().revision = Some(1);
    for kind in [LayerKind::Roads, LayerKind::Buildings] {
        w.resource_mut::<crate::project::Project>()
            .document
            .layer_mut(kind)
            .package_id = Some("retained".into());
    }
    let mut data = super::super::mvt::empty(terrain.manifest.region(), "retained".into());
    data.region.origin.longitude += 1e-13;
    data.buildings.push(super::super::data::BuildingFeature {
        id: "fixture".into(),
        height_m: 12.,
        min_height_m: 0.,
        polygons: vec![super::super::data::BuildingPolygon {
            rings: vec![
                [(1., 1.), (5., 1.), (5., 5.), (1., 5.), (1., 1.)]
                    .into_iter()
                    .map(|(x, y)| super::super::data::LocalPoint2 { x, y })
                    .collect(),
            ],
        }],
    });
    w.insert_resource(VectorScene {
        revision: Some(1),
        restore_attempted: true,
        task: Some(pool.spawn(async move { Ok(Some(data)) })),
        ..default()
    });
    let mut s = Schedule::default();
    s.add_systems(update);
    for _ in 0..1000 {
        s.run(&mut w);
        if !w.resource::<VectorScene>().entities.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(w.resource::<VectorScene>().entities.len(), 2);
    assert!(!w.resource::<crate::project::Project>().dirty);
    let entities = w.resource::<VectorScene>().entities.clone();
    for _ in 0..30 {
        s.run(&mut w);
    }
    assert_eq!(w.resource::<VectorScene>().entities, entities);
    assert_eq!(w.resource::<Assets<Mesh>>().len(), 2);
    assert_eq!(w.resource::<Assets<StandardMaterial>>().len(), 2);
}

#[test]
fn source_change_rejects_download_but_preserves_current_mesh_preparation() {
    let pool = AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = Arc::new(crate::map::source::load_terrain(dir.path()).unwrap());
    let mut w = world();
    w.resource_mut::<ActiveMapRegion>()
        .confirm(terrain.manifest.region())
        .unwrap();
    w.resource_mut::<RealScene>().data = Some(terrain.clone());
    w.resource_mut::<RealScene>().revision = Some(1);
    w.resource_mut::<crate::project::Project>()
        .document
        .vector_source = starter_core::VectorSource::LocalPackage;
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stale = super::super::mvt::empty(terrain.manifest.region(), "obsolete".into());
    w.insert_resource(VectorScene {
        revision: Some(1),
        terrain: Some(terrain.fingerprint),
        restore_attempted: true,
        source_at_request: starter_core::VectorSource::OpenFreeMap,
        task: Some(pool.spawn(async move { Ok(Some(stale)) })),
        job: Some(Job {
            cancelled: cancelled.clone(),
            progress: Arc::new(std::sync::Mutex::new(String::new())),
        }),
        mesh_task: Some(pool.spawn(async { std::future::pending::<Vec<mesh::Chunk>>().await })),
        ..default()
    });
    let mut s = Schedule::default();
    s.add_systems(update);
    s.run(&mut w);
    let vectors = w.resource::<VectorScene>();
    assert!(vectors.task.is_none() && vectors.data.is_none());
    assert!(vectors.mesh_task.is_some());
    assert!(cancelled.load(Ordering::Relaxed));
    assert!(!w.resource::<crate::project::Project>().dirty);
    assert!(
        w.resource::<RealScene>()
            .vector_status
            .contains("Source changed")
    );
    assert_eq!(
        w.resource::<RealScene>().fingerprint(),
        Some(terrain.fingerprint)
    );
}
