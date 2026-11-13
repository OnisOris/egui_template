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
