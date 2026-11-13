//! Synthetic widget events. This is not a human/manual smoke test.
use super::{
    project_explorer::{self, ExplorerState},
    project_table,
};
use crate::{
    map::{loading::TerrainImport, real_scene::RealScene},
    project::{Project, ProjectIo},
};
use egui_kittest::{Harness, kittest::Queryable};
use starter_core::{GeoOrigin, ProjectSelection};
#[test]
fn explorer_and_virtual_table_select_the_same_document_markers() {
    let mut project = Project::default();
    let first = project
        .add_marker(GeoOrigin {
            latitude: 46.0,
            longitude: 8.0,
        })
        .unwrap();
    let second = project
        .add_marker(GeoOrigin {
            latitude: 46.1,
            longitude: 8.1,
        })
        .unwrap();
    project.document.assets.push(starter_core::ProjectAsset {
        path: "Assets/Models/box.glb".into(),
        media_type: "model/gltf-binary".into(),
        size: 1,
    });
    let model = project
        .add_model(
            "Assets/Models/box.glb".into(),
            GeoOrigin {
                latitude: 46.0,
                longitude: 8.0,
            },
        )
        .unwrap();
    project.selection = ProjectSelection::Project;
    let mut harness = Harness::new_ui_state(
        |ui,
         state: &mut (
            Project,
            ProjectIo,
            ExplorerState,
            project_table::TableState,
            RealScene,
            TerrainImport,
        )| {
            ui.columns(2, |columns| {
                project_explorer::show(
                    &mut columns[0],
                    &mut state.2,
                    &mut state.0,
                    &mut state.1,
                    &mut state.4,
                    &state.5,
                );
                project_table::show(&mut columns[1], &mut state.0, &mut state.3);
            });
        },
        (
            project,
            ProjectIo::default(),
            ExplorerState::default(),
            project_table::TableState::default(),
            RealScene::default(),
            TerrainImport::default(),
        ),
    );
    harness.run();
    assert!(harness.query_all_by_label("Group A").next().is_none());
    assert!(harness.query_all_by_label("Groups").next().is_none());
    assert!(harness.query_all_by_label("Map Markers").next().is_some());
    assert!(
        harness
            .query_all_by_label("Model Instances")
            .next()
            .is_some()
    );
    harness
        .query_all_by_label("Marker 1")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().0.selection, ProjectSelection::Marker(first));
    harness
        .query_all_by_label("Marker 2")
        .last()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        harness.state().0.selection,
        ProjectSelection::Marker(second)
    );
    harness
        .query_all_by_label("Model 3")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().0.selection, ProjectSelection::Model(model));
    harness
        .query_all_by_label("Model 3")
        .last()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().0.selection, ProjectSelection::Model(model));
}
