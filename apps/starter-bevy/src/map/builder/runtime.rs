//! Single lazy worker with one replaceable pending request; never wait for HTTP in Update.
use super::{Config, Job, cache, dem, download::Download, imagery};
use crate::map::{
    RegionRuntime, dem_mesh,
    loading::{PreparedTerrain, TerrainImport},
    region::{ActiveMapRegion, MapRegion},
};
use bevy::prelude::*;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RequestKey {
    pub request_id: u64,
    pub region_revision: u64,
    pub region: MapRegion,
}
struct Request {
    key: RequestKey,
    job: Job,
}
#[derive(Default)]
struct Pending {
    request: Option<Request>,
    shutdown: bool,
}
enum Output {
    Terrain(PreparedTerrain),
    Satellite(Box<crate::map::satellite::SatellitePayload>, blake3::Hash),
    Done(String),
}
struct Worker {
    pending: Arc<(Mutex<Pending>, Condvar)>,
    results: Mutex<mpsc::Receiver<(RequestKey, Result<Output, String>)>>,
}
#[derive(Default)]
pub struct MapPackageBuilder {
    worker: Option<Worker>,
    current: Option<RequestKey>,
    job: Option<Job>,
    serial: u64,
    pub loading: bool,
}
impl MapPackageBuilder {
    pub fn status(&self) -> String {
        self.job
            .as_ref()
            .map(|j| j.progress.lock().unwrap().clone())
            .unwrap_or_default()
    }
    pub fn cancel(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        self.current = None;
        self.loading = false;
    }
    pub fn start(&mut self, active: &ActiveMapRegion) {
        let Some((revision, region)) = active.get() else {
            return;
        };
        self.cancel();
        if self.worker.is_none() {
            self.worker = Some(Worker::new());
        }
        self.serial += 1;
        let key = RequestKey {
            request_id: self.serial,
            region_revision: revision,
            region,
        };
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new("Checking cache".into())),
        };
        let worker = self.worker.as_ref().unwrap();
        worker.pending.0.lock().unwrap().request = Some(Request {
            key,
            job: job.clone(),
        });
        worker.pending.1.notify_one();
        self.current = Some(key);
        self.job = Some(job);
        self.loading = true;
    }
    fn poll(&mut self, active: &ActiveMapRegion) -> Option<Result<Output, String>> {
        if self
            .current
            .is_some_and(|k| active.get() != Some((k.region_revision, k.region)))
        {
            self.cancel();
        }
        let worker = self.worker.as_ref()?;
        while let Ok((key, result)) = worker.results.lock().unwrap().try_recv() {
            if self.current == Some(key) && active.get() == Some((key.region_revision, key.region))
            {
                self.loading = matches!(&result, Ok(Output::Terrain(_)));
                return Some(result);
            }
        }
        None
    }
}
impl Worker {
    fn new() -> Self {
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let queue = pending.clone();
        let (send, results) = mpsc::channel();
        std::thread::Builder::new()
            .name("map-package-builder".into())
            .spawn(move || {
                let mut http = Download::new(4);
                loop {
                    let request = {
                        let (lock, wake) = &*queue;
                        let mut pending = lock.lock().unwrap();
                        while pending.request.is_none() && !pending.shutdown {
                            pending = wake.wait(pending).unwrap();
                        }
                        if pending.shutdown {
                            break;
                        }
                        pending.request.take().unwrap()
                    };
                    let Request { key, job } = request;
                    let result = (|| {
                        let config =
                            if let Some(path) = std::env::var_os("STARTER_MAP_BUILDER_CONFIG") {
                                serde_json::from_slice::<Config>(&crate::map::source::read_limited(
                                    std::path::Path::new(&path),
                                    1_048_576,
                                )?)
                                .map_err(|_| "Invalid builder configuration")?
                            } else {
                                Config::default()
                            };
                        config.validate()?;
                        let http = http.as_mut().map_err(|e| e.clone())?;
                        http.parallelism = config.http_parallelism;
                        let root = cache::root()?;
                        let data = dem::build(key.region, &config, &root, http, &job)?;
                        job.check()?;
                        let meshes = dem_mesh::chunks(&data);
                        let grid = Some(dem_mesh::grid(&data));
                        job.status(format!(
                            "Terrain ready · {:.1} × {:.1} m DEM · {}",
                            data.manifest.spacing_x_m,
                            data.manifest.spacing_y_m,
                            data.manifest.reference_elevation_source
                        ));
                        Ok((PreparedTerrain { data, meshes, grid }, config, root))
                    })();
                    match result {
                        Ok((prepared, config, root)) => {
                            let data = prepared.data.clone();
                            if send.send((key, Ok(Output::Terrain(prepared)))).is_err() {
                                break;
                            }
                            let satellite = imagery::configured().and_then(|provider| {
                                provider
                                    .map(|p| {
                                        imagery::build(
                                            &data,
                                            &p,
                                            &config,
                                            &root,
                                            http.as_ref().map_err(Clone::clone)?,
                                            &job,
                                        )
                                    })
                                    .transpose()
                            });
                            let output = match satellite {
                                Ok(Some(payload)) => {
                                    Output::Satellite(Box::new(payload), data.fingerprint)
                                }
                                Ok(None) => {
                                    Output::Done("Partial · Satellite source not configured".into())
                                }
                                Err(e) => Output::Done(format!("Partial · {e}")),
                            };
                            if send.send((key, Ok(output))).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            job.status("Failed".into());
                            if send.send((key, Err(e))).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
            .expect("Cannot start map package worker");
        Self {
            pending,
            results: Mutex::new(results),
        }
    }
}
impl Drop for MapPackageBuilder {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = &self.worker {
            worker.pending.0.lock().unwrap().shutdown = true;
            worker.pending.1.notify_one();
        }
    }
}
pub fn poll_builder(
    active: Res<ActiveMapRegion>,
    mut import: ResMut<TerrainImport>,
    mut runtime: ResMut<RegionRuntime>,
    mut scene: ResMut<crate::map::real_scene::RealScene>,
    mut assets: crate::map::real_scene::SceneAssets,
) {
    if let Some(result) = import.builder.poll(&active) {
        match result {
            Ok(Output::Terrain(prepared)) => {
                import.prepared = Some(prepared);
                import.error = None;
                runtime.real_requested = true;
            }
            Ok(Output::Satellite(payload, terrain)) => {
                if scene.fingerprint() == Some(terrain)
                    && scene.revision == active.get().map(|(r, _)| r)
                {
                    match crate::map::satellite_scene::install(&mut scene, &mut assets, *payload) {
                        Ok(()) => {
                            if let Some(job) = &import.builder.job {
                                job.status("Ready".into());
                            }
                        }
                        Err(e) => {
                            if let Some(job) = &import.builder.job {
                                job.status(format!("Partial · {e}"));
                            }
                        }
                    }
                }
            }
            Ok(Output::Done(message)) => {
                if let Some(job) = &import.builder.job {
                    job.status(message);
                }
            }
            Err(error) => import.error = Some(error),
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
