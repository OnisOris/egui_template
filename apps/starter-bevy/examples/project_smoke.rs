//! Native integration probe for a saved geographic project (no synthetic clicks).
//! cargo run -p starter-bevy --example project_smoke --release -- /path/project.json
use bevy::prelude::*;
use starter_bevy::{
    models::{
        assets::ModelAssets,
        instances::{ModelRoot, ModelScene},
    },
    project::{Project, ProjectIo, storage},
    state::UiState,
};
use starter_core::{ProjectSelection, ProjectState};
#[derive(Resource, Default)]
struct Probe(bool);
fn main() {
    let path =
        std::path::PathBuf::from(std::env::args().nth(1).expect("Pass a saved project JSON"));
    let path = path.canonicalize().expect("Project file must exist");
    let document = storage::load(&path).expect("Valid project required");
    let selection = document
        .models
        .first()
        .map(|m| ProjectSelection::Model(m.id))
        .unwrap_or_default();
    let missing = document
        .assets
        .iter()
        .map(|a| storage::asset_path(path.parent().unwrap(), &a.path).is_err())
        .collect();
    let mut app = starter_bevy::build();
    app.world_mut().resource_mut::<Project>().0 = ProjectState {
        document,
        selection,
        revision: 1,
        ..Default::default()
    };
    let mut io = app.world_mut().resource_mut::<ProjectIo>();
    io.path = Some(path);
    io.generation = 1;
    io.missing = missing;
    io.reset_scene = true;
    io.restore = true;
    app.init_resource::<Probe>().add_systems(
        PostUpdate,
        report.after(bevy::transform::TransformSystems::Propagate),
    );
    app.run();
}
fn report(world: &mut World) {
    if world.resource::<Probe>().0 {
        return;
    }
    let count = world.resource::<Project>().document.models.len();
    if count == 0
        || world
            .resource::<ModelScene>()
            .information
            .values()
            .filter(|i| i.center.is_some())
            .count()
            != count
    {
        return;
    }
    let poses: Vec<_> = world
        .query::<(&ModelRoot, &Transform)>()
        .iter(world)
        .map(|(id, t)| (id.0, *t))
        .collect();
    println!(
        "MODEL_PROJECT_SMOKE models={count} roots={} asset_entries={} poses={poses:?}",
        poses.len(),
        world.resource::<ModelAssets>().entries.len()
    );
    world.resource_mut::<Project>().focus_selected = true;
    world
        .resource_mut::<UiState>()
        .workspace
        .activate(starter_ui::PanelId::Viewport);
    world.resource_mut::<Probe>().0 = true;
}
