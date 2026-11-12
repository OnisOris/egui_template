//! Native GLB fixture benchmark in the existing Viewport. Optional second arg: mixed.
use bevy::{camera::visibility::RenderLayers, gltf::GltfAssetLabel, prelude::*};
use starter_bevy::{
    models::{files, geometry::ModelGeometry},
    viewport::ViewportState,
};
use std::time::Instant;
#[derive(Resource)]
struct Bench {
    count: usize,
    mixed: bool,
    started: Instant,
    reported: bool,
    scenes: Vec<Handle<WorldAsset>>,
    timing: Option<(f64, f64)>,
}
fn main() {
    let count = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .filter(|n| (1..=1000).contains(n))
        .unwrap_or(1);
    let mixed = std::env::args().nth(2).is_some_and(|s| s == "mixed");
    let mut app = starter_bevy::build();
    app.insert_resource(Bench {
        count,
        mixed,
        started: Instant::now(),
        reported: false,
        scenes: Vec::new(),
        timing: None,
    });
    app.add_systems(PostStartup, setup)
        .add_systems(Update, report);
    app.run();
}
fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut bench: ResMut<Bench>,
    mut viewport: ResMut<ViewportState>,
    mut ui: ResMut<starter_bevy::state::UiState>,
    mut runtime: ResMut<starter_bevy::map::RegionRuntime>,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for file in if bench.mixed {
        vec!["box.glb", "orange-box.glb"]
    } else {
        vec!["box.glb"]
    } {
        let prepared = files::prepare(
            &root,
            file,
            &files::cache_root().unwrap(),
            files::DEFAULT_MAX_BYTES,
        )
        .unwrap();
        bench.scenes.push(
            server
                .load_builder()
                .with_settings(starter_bevy::models::assets::settings)
                .load(
                    GltfAssetLabel::Scene(prepared.scene)
                        .from_asset(format!("models://{}", prepared.file)),
                ),
        );
    }
    for i in 0..bench.count {
        commands.spawn((
            WorldAssetRoot(bench.scenes[i % bench.scenes.len()].clone()),
            ModelGeometry::default(),
            Transform::from_xyz((i % 10) as f32 * 12.0, 0.0, (i / 10) as f32 * 12.0),
            RenderLayers::layer(3),
        ));
    }
    runtime.mode = starter_bevy::map::SceneMode::Real;
    viewport.camera.target = Vec3::new(
        (bench.count.min(10) - 1) as f32 * 6.0 + 4.0,
        8.0,
        (bench.count.div_ceil(10) - 1) as f32 * 6.0,
    );
    viewport.camera.distance = if bench.count == 1 { 30.0 } else { 200.0 };
    ui.workspace.activate(starter_ui::PanelId::Viewport);
    ui.notice = Some("GLB fixture benchmark · fixed local positions, no DEM".into());
}
fn report(world: &mut World) {
    let bench = world.resource::<Bench>();
    if bench.reported || bench.scenes.is_empty() {
        return;
    }
    if bench.timing.is_none() {
        if !bench.scenes.iter().all(|h| {
            world
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(h.id())
        }) {
            return;
        }
        let load = bench.started.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        // Run the standard schedule once here to isolate its CPU work from the render frame.
        world.run_schedule(SpawnScene);
        world.resource_mut::<Bench>().timing = Some((load, start.elapsed().as_secs_f64() * 1e6));
    }
    let count = world.resource::<Bench>().count;
    if world
        .query::<&ModelGeometry>()
        .iter(world)
        .filter(|r| r.bounds.is_some())
        .count()
        != count
    {
        return;
    }
    let meshes: Vec<_> = world
        .query::<&Mesh3d>()
        .iter(world)
        .map(|m| m.0.id())
        .collect();
    let unique: std::collections::HashSet<_> = meshes.iter().collect();
    let start = Instant::now();
    for mut transform in world
        .query_filtered::<&mut Transform, With<ModelGeometry>>()
        .iter_mut(world)
    {
        transform.translation.x += 0.000_001;
    }
    let writes = start.elapsed().as_secs_f64() * 1e6;
    let bench = world.resource::<Bench>();
    let (load, spawn) = bench.timing.unwrap();
    println!(
        "GLB_BENCH count={} mixed={} mesh_entities={} unique_mesh_assets={} load_ready_ms={load:.3} spawn_scene_us={spawn:.3} transform_writes_us={writes:.3}",
        count,
        bench.mixed,
        meshes.len(),
        unique.len()
    );
    world.resource_mut::<Bench>().reported = true;
}
