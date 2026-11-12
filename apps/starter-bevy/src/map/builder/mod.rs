//! Data preparation only. Rendering and UI remain in their existing adapters.
pub mod cache;
pub mod coverage;
pub mod dem;
pub mod download;
pub mod provider;
pub mod runtime;

use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub step_m: f64,
    pub max_edge: u32,
    pub max_samples: usize,
    pub max_tiles: usize,
    pub http_parallelism: usize,
    pub terrain_tilejson: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            step_m: 25.0,
            max_edge: 1025,
            max_samples: 2_097_152,
            max_tiles: 256,
            http_parallelism: 4,
            terrain_tilejson: "https://tiles.mapterhorn.com/tilejson.json".into(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !self.step_m.is_finite()
            || self.step_m < 0.01
            || !(2..=1025).contains(&self.max_edge)
            || !(4..=2_097_152).contains(&self.max_samples)
            || !(1..=256).contains(&self.max_tiles)
            || !(1..=6).contains(&self.http_parallelism)
        {
            return Err("Invalid package builder limits".into());
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct Job {
    pub cancelled: Arc<AtomicBool>,
    pub progress: Arc<Mutex<String>>,
}
impl Job {
    pub fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Request cancelled".into())
        } else {
            Ok(())
        }
    }
    pub fn status(&self, message: String) {
        *self.progress.lock().unwrap() = message;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod network_tests;
