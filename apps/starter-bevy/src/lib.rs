use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};
use starter_core::{Command, TEST_LOADS};

mod layout;
pub mod map;
pub mod metrics;
pub mod scene;
pub mod state;
pub mod ui;
pub mod viewport;

use metrics::EditorMetrics;
use scene::ScenePlugin;
use state::{DocumentResource, UiState};
use viewport::ViewportPlugin;

pub fn run() -> AppExit {
    build().run()
}

pub fn build() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy + egui Starter".into(),
            resolution: (1440, 900).into(),
            present_mode: if std::env::var_os("STARTER_BENCH").is_some() {
                PresentMode::AutoNoVsync
            } else {
                PresentMode::AutoVsync
            },
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(EguiPlugin::default());
    app.world_mut()
        .resource_mut::<EguiGlobalSettings>()
        .auto_create_primary_context = false;
    let mut document = DocumentResource::default();
    if let Ok(value) = std::env::var("STARTER_LOAD")
        && let Ok(count) = value.parse::<usize>()
        && TEST_LOADS.contains(&count)
    {
        let _ = document.dispatch(Command::LoadTestScene(count));
    }
    app.insert_resource(document)
        .init_resource::<EditorMetrics>()
        .init_resource::<UiState>()
        .add_plugins((ScenePlugin, ViewportPlugin, map::MapPlugin))
        .add_systems(
            Update,
            (
                metrics::bench_edit,
                metrics::update_metrics,
                metrics::log_bench,
            ),
        )
        .add_systems(EguiPrimaryContextPass, ui::editor_ui)
        .add_systems(Last, layout::save_on_exit.after(bevy::window::ExitSystems));
    app
}
