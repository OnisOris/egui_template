//! Native GLB fixture benchmark, using the application's existing Viewport.
//! cargo run -p starter-bevy --example model_smoke --release -- 10
use bevy::{camera::visibility::RenderLayers, gltf::GltfAssetLabel, prelude::*};
use starter_bevy::{
    models::{files, geometry::ModelGeometry},
    viewport::ViewportState,
};
use std::time::Instant;
#[derive(Resource)]
struct Bench {
    count: usize,
    started: Instant,
    reported: bool,
}
fn main() {
    let count = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .filter(|count| (1..=1000).contains(count))
        .unwrap_or(1);
    let mut app = starter_bevy::build();
    app.insert_resource(Bench {
        count,
        started: Instant::now(),
        reported: false,
    });
    app.add_systems(PostStartup, setup)
        .add_systems(Update, report);
    app.run();
}
fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    bench: Res<Bench>,
    mut viewport: ResMut<ViewportState>,
    mut ui: ResMut<starter_bevy::state::UiState>,
    mut runtime: ResMut<starter_bevy::map::RegionRuntime>,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let prepared = files::prepare(
        &root,
        "box.glb",
        &files::cache_root().unwrap(),
        files::DEFAULT_MAX_BYTES,
    )
    .unwrap();
    let scene: Handle<WorldAsset> = server
        .load_builder()
        .with_settings(starter_bevy::models::assets::settings)
        .load(
            GltfAssetLabel::Scene(prepared.scene).from_asset(format!("models://{}", prepared.file)),
        );
    for i in 0..bench.count {
        commands.spawn((
            WorldAssetRoot(scene.clone()),
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
}
fn report(
    mut bench: ResMut<Bench>,
    roots: Query<&ModelGeometry>,
    meshes: Query<&Mesh3d>,
    mut transforms: Query<&mut Transform, With<ModelGeometry>>,
) {
    if !bench.reported && roots.iter().filter(|r| r.bounds.is_some()).count() == bench.count {
        let unique: std::collections::HashSet<_> = meshes.iter().map(|m| m.0.id()).collect();
        let start = Instant::now();
        for mut transform in &mut transforms {
            transform.translation.x += 0.000_001;
        }
        println!(
            "GLB_BENCH count={} mesh_entities={} unique_mesh_assets={} ready_ms={:.3} transform_writes_us={:.3} bounds={:?}",
            bench.count,
            meshes.iter().count(),
            unique.len(),
            bench.started.elapsed().as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 1e6,
            roots.iter().next().unwrap().bounds
        );
        bench.reported = true;
    }
}
