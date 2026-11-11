use std::time::{Duration, Instant};

use starter_ui::PlotPoint;

pub struct MetricsState {
    app_started: Instant,
    sample_started: Instant,
    chart_sampled: Instant,
    callbacks: u64,
    pub callback_hz: f32,
    pub callback_cpu_ms: f32,
    pub samples: Vec<PlotPoint>,
}

impl Default for MetricsState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            app_started: now,
            sample_started: now,
            chart_sampled: now,
            callbacks: 0,
            callback_hz: 0.0,
            callback_cpu_ms: 0.0,
            samples: Vec::with_capacity(120),
        }
    }
}

impl MetricsState {
    pub fn begin(&mut self) -> Instant {
        self.callbacks += 1;
        Instant::now()
    }

    pub fn finish(&mut self, started: Instant) {
        let now = Instant::now();
        self.callback_cpu_ms = started.elapsed().as_secs_f32() * 1_000.0;
        let elapsed = now.duration_since(self.sample_started).as_secs_f32();
        if elapsed >= 1.0 {
            self.callback_hz = self.callbacks as f32 / elapsed;
            self.callbacks = 0;
            self.sample_started = now;
        }
        if now.duration_since(self.chart_sampled) >= Duration::from_millis(100) {
            self.samples.push(PlotPoint::new(
                now.duration_since(self.app_started).as_secs_f64(),
                self.callback_cpu_ms as f64,
            ));
            if self.samples.len() > 120 {
                self.samples.remove(0);
            }
            self.chart_sampled = now;
        }
    }
}
