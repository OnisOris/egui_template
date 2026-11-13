use bevy::prelude::*;
use starter_core::{Command, Position};
use starter_ui::PlotPoint;

use crate::state::DocumentResource;

#[derive(Resource)]
pub struct EditorMetrics {
    pub fps: f32,
    pub frame_ms: f32,
    pub cpu_frame_ms: f32,
    cpu_started: Option<std::time::Instant>,
    pub ui_ms: f32,
    pub sync_ms: f32,
    pub scene_build_ms: f32,
    pub objects: usize,
    pub projected_entities: usize,
    pub signal: Vec<PlotPoint>,
    elapsed: f32,
    frames: u32,
    accumulated: f32,
}

impl Default for EditorMetrics {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_ms: 0.0,
            cpu_frame_ms: 0.0,
            cpu_started: None,
            ui_ms: 0.0,
            sync_ms: 0.0,
            scene_build_ms: 0.0,
            objects: 0,
            projected_entities: 0,
            signal: Vec::new(),
            elapsed: 0.0,
            frames: 0,
            accumulated: 0.0,
        }
    }
}

pub fn begin_frame(mut metrics: ResMut<EditorMetrics>) {
    metrics.cpu_started = Some(std::time::Instant::now());
}

pub fn end_frame(mut metrics: ResMut<EditorMetrics>) {
    if let Some(start) = metrics.cpu_started.take() {
        metrics.cpu_frame_ms = start.elapsed().as_secs_f32() * 1000.0;
    }
}

pub fn log_bench(time: Res<Time>, metrics: Res<EditorMetrics>, mut elapsed: Local<f32>) {
    if std::env::var_os("STARTER_BENCH").is_none() {
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < 1.0 {
        return;
    }
    *elapsed = 0.0;
    println!(
        "BENCH objects={} entities={} fps={:.1} frame_ms={:.2} main_cpu_ms={:.2} ui_ms={:.2} sync_ms={:.3} build_ms={:.2}",
        metrics.objects,
        metrics.projected_entities,
        metrics.fps,
        metrics.frame_ms,
        metrics.cpu_frame_ms,
        metrics.ui_ms,
        metrics.sync_ms,
        metrics.scene_build_ms,
    );
}

pub fn bench_edit(
    time: Res<Time>,
    mut document: ResMut<DocumentResource>,
    mut elapsed: Local<f32>,
) {
    if std::env::var_os("STARTER_BENCH_EDIT").is_none() {
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < 1.0 {
        return;
    }
    *elapsed = 0.0;
    if let Some(object) = document.document().objects().first() {
        let id = object.id();
        let position = object.position();
        let _ = document.dispatch(Command::SetPosition(
            id,
            Position {
                x: position.x + 8.0,
                y: position.y,
            },
        ));
    }
}

pub fn update_metrics(
    time: Res<Time>,
    mut metrics: ResMut<EditorMetrics>,
    project: Res<crate::project::Project>,
    io: Res<crate::project::ProjectIo>,
    markers: Res<crate::map::objects::MarkerScene>,
) {
    if !io.benchmark {
        metrics.objects = project.document.objects.len();
        metrics.projected_entities = markers.entities.len();
    }
    let delta = time.delta_secs();
    metrics.elapsed += delta;
    metrics.frames += 1;
    metrics.accumulated += delta;
    if metrics.accumulated < 0.25 {
        return;
    }
    let frames = metrics.frames as f32;
    let accumulated = metrics.accumulated;
    metrics.fps = frames / accumulated;
    metrics.frame_ms = accumulated * 1000.0 / frames;
    metrics.frames = 0;
    metrics.accumulated = 0.0;
    let point = PlotPoint::new(metrics.elapsed as f64, metrics.frame_ms as f64);
    metrics.signal.push(point);
    if metrics.signal.len() > 240 {
        metrics.signal.remove(0);
    }
}
