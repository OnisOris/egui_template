//! Project lifecycle regression, not a manual GUI smoke test.
use super::{
    RegionRuntime, dem_mesh, dem_tests,
    loading::{PreparedTerrain, TerrainImport},
    objects::{self, MarkerScene},
    real_scene::{self, RealScene},
    region::ActiveMapRegion,
    source,
};
use crate::{
    project::{Project, ProjectAction, ProjectIo, storage},
    state::DocumentResource,
};
use bevy::{ecs::schedule::Schedule, prelude::*};
use starter_core::{LayerKind, ProjectState};
use std::sync::Arc;
#[test]
fn saved_project_restores_region_markers_layers_and_survives_missing_cache_then_clears_gpu_assets()
{
    let terrain = dem_tests::package(3, 3);
    let data = Arc::new(source::load_terrain(terrain.path()).unwrap());
    let region = data.manifest.region();
    let mut project = ProjectState::default();
    project.document.name = "Survey Project".into();
    project.document.map_region = Some(region);
    project.document.layer_mut(LayerKind::Terrain).package_id =
        Some(data.manifest.terrain_id.clone());
    project.document.layer_mut(LayerKind::Satellite).visible = false;
    let id = project.add_marker(region.bounds.center()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("project.json");
    storage::save_as(&path, None, &project.document).unwrap();
    let saved = storage::load(&path).unwrap();
    assert_eq!(saved, project.document);
    let mut world = dem_tests::world();
    world.insert_resource(Project(ProjectState {
        document: saved.clone(),
        ..Default::default()
    }));
    let mut io = ProjectIo::default();
    io.reset_scene = true;
    io.path = Some(path);
    world.insert_resource(io);
    world.insert_resource(DocumentResource::empty());
    world.init_resource::<super::satellite_scene::SatelliteImport>();
    world.init_resource::<MarkerScene>();
    world.init_resource::<super::demo::DemoScene>();
    world.insert_non_send(super::picker::MapPicker::default());
    let mut sync = Schedule::default();
    sync.add_systems((crate::project::scene::sync_project, objects::sync).chain());
    sync.run(&mut world);
    assert_eq!(world.resource::<ActiveMapRegion>().get().unwrap().1, region);
    assert_eq!(world.resource::<Project>().document, saved);
    assert!(!world.resource::<Project>().dirty);
    assert!(world.resource::<MarkerScene>().entities.is_empty()); // No invented ground at zero.
    for _ in 0..3 {
        world.resource_mut::<TerrainImport>().prepared = Some(PreparedTerrain {
            data: data.clone(),
            meshes: dem_mesh::chunks(&data),
            grid: Some(dem_mesh::grid(&data)),
        });
        world.resource_mut::<RegionRuntime>().real_requested = true;
        let mut open = Schedule::default();
        open.add_systems((real_scene::open_real, objects::sync).chain());
        open.run(&mut world);
        assert_eq!(world.resource::<MarkerScene>().entities.len(), 1);
        assert!(world.resource::<MarkerScene>().entities.contains_key(&id));
        assert!(
            !world
                .resource::<Project>()
                .document
                .layer(LayerKind::Satellite)
                .visible
        );
        let markers = world.resource::<MarkerScene>().entities.len();
        let terrain_entities = world.resource::<RealScene>().entities.len();
        assert_eq!(markers + terrain_entities, 3);
        sync.run(&mut world);
    }
    world.resource_mut::<Project>().dirty = false;
    world.resource_scope(|world, mut io: Mut<ProjectIo>| {
        io.request(ProjectAction::New, &mut world.resource_mut::<Project>());
    });
    sync.run(&mut world);
    assert!(world.resource::<Project>().document.objects.is_empty());
    assert!(world.resource::<RealScene>().entities.is_empty());
    assert!(world.resource::<MarkerScene>().entities.is_empty());
    assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 0);
    assert_eq!(world.resource::<Assets<Mesh>>().len(), 1); // One reusable marker mesh, no accumulation.
    assert_eq!(world.resource::<Assets<Image>>().len(), 1); // Existing viewport target only.
}

#[test]
fn documented_project_fixture_is_portable_and_can_be_saved_and_opened() {
    let document: starter_core::ProjectDocument =
        serde_json::from_str(include_str!("../../../../docs/fixtures/m4-project.json")).unwrap();
    document.validate().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("project.json");
    storage::save_as(&path, None, &document).unwrap();
    assert_eq!(storage::load(&path).unwrap(), document);
    assert_eq!(document.objects.len(), 1);
    assert_eq!(document.objects[0].altitude_agl, 12.0);
    assert!(document.map_region.is_some());
    assert!(!document.layer(LayerKind::Satellite).visible);
}

#[test]
fn explorer_does_not_report_previous_region_terrain_as_ready() {
    use egui_kittest::{Harness, kittest::Queryable};
    let package = dem_tests::package(3, 3);
    let data = Arc::new(source::load_terrain(package.path()).unwrap());
    let mut region = data.manifest.region();
    region.bounds.west += 1.0;
    region.bounds.east += 1.0;
    let mut project = Project::default();
    project.document.map_region = Some(region);
    let real = RealScene {
        data: Some(data),
        revision: Some(1),
        ..Default::default()
    };
    let mut io = ProjectIo::default();
    let mut state = crate::ui::project_explorer::ExplorerState::default();
    let import = TerrainImport::default();
    let mut harness = Harness::new_ui(|ui| {
        crate::ui::project_explorer::show(ui, &mut state, &mut project, &mut io, &real, &import)
    });
    harness.run_ok();
    assert!(
        harness
            .query_all_by_label("● Terrain · Previous region")
            .next()
            .is_some()
    );
    assert!(
        harness
            .query_all_by_label("● Terrain · Ready")
            .next()
            .is_none()
    );
}
