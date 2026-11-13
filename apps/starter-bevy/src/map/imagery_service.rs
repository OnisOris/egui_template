//! Source-scoped decoded RGB tiles. Both GPU adapters share this bounded memory-only service.
use super::builder::{
    Job, coverage::Tile, dem::decode, download::Download, provider::ImageryProvider,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
pub type RasterTile = image::ImageBuffer<image::Rgb<u8>, Arc<[u8]>>;
type TileResult = Result<RasterTile, String>;
struct Entry {
    result: Option<TileResult>,
    touched: Instant,
}
#[derive(Default)]
struct Cache {
    entries: BTreeMap<Tile, Entry>,
    order: VecDeque<Tile>,
    bytes: usize,
    hits: u64,
    decode_ms: f64,
}
struct Service {
    cache: Arc<Mutex<Cache>>,
    requests: mpsc::SyncSender<Tile>,
    job: Job,
    provider: ImageryProvider,
    pub generation: u64,
    http: Arc<Download>,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.job.cancelled.store(true, Ordering::Relaxed);
    }
}
#[derive(Clone)]
pub struct TileService(Arc<Service>);
impl TileService {
    pub fn new(provider: ImageryProvider, token: Option<String>) -> Result<Self, String> {
        provider.validate_preview()?;
        if provider.bearer_env.is_some() && token.is_none() {
            return Err("Satellite authorization variable is missing".into());
        }
        let http = Arc::new(Download::new(1)?);
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::default(),
        };
        let cache = Arc::new(Mutex::new(Cache::default()));
        let (send, receive) = mpsc::sync_channel::<Tile>(64);
        let receive = Arc::new(Mutex::new(receive));
        for _ in 0..4 {
            let (http, job, cache, receive, source, token) = (
                http.clone(),
                job.clone(),
                cache.clone(),
                receive.clone(),
                provider.clone(),
                token.clone(),
            );
            std::thread::Builder::new()
                .name("satellite-tiles".into())
                .spawn(move || {
                    loop {
                        let tile = match receive
                            .lock()
                            .unwrap()
                            .recv_timeout(Duration::from_millis(100))
                        {
                            Ok(tile) => tile,
                            Err(mpsc::RecvTimeoutError::Timeout) if job.check().is_ok() => continue,
                            Err(_) => break,
                        };
                        if job.check().is_err() {
                            break;
                        }
                        let url = source
                            .url
                            .replace("{z}", &tile.z.to_string())
                            .replace("{x}", &tile.x.to_string())
                            .replace("{y}", &tile.y.to_string());
                        let mut decode_ms = 0.0;
                        let result = http.get(&url, token.as_deref(), &job).and_then(|bytes| {
                            let start = Instant::now();
                            let image = decode(&bytes, source.tile_size, false)?;
                            decode_ms = start.elapsed().as_secs_f64() * 1000.0;
                            Ok(RasterTile::from_raw(
                                image.width(),
                                image.height(),
                                Arc::from(image.into_raw()),
                            )
                            .unwrap())
                        });
                        let mut state = cache.lock().unwrap();
                        state.decode_ms += decode_ms;
                        if result.is_ok() {
                            state.bytes += (source.tile_size as usize).pow(2) * 3;
                        }
                        if let Some(entry) = state.entries.get_mut(&tile) {
                            entry.result = Some(result);
                            entry.touched = Instant::now();
                        }
                    }
                })
                .map_err(|_| "Cannot start tile worker")?;
        }
        static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Ok(Self(Arc::new(Service {
            cache,
            requests: send,
            job,
            provider,
            generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            http,
        })))
    }
    pub fn generation(&self) -> u64 {
        self.0.generation
    }
    pub fn provider(&self) -> &ImageryProvider {
        &self.0.provider
    }
    pub fn request(&self, tile: Tile) -> Option<TileResult> {
        let p = self.provider();
        if tile.z < p.min_zoom
            || tile.z > p.max_zoom
            || tile.x >= 1 << tile.z
            || tile.y >= 1 << tile.z
        {
            return Some(Err("Tile outside source coverage".into()));
        }
        let mut cache = self.0.cache.lock().unwrap();
        if let Some(entry) = cache.entries.get(&tile) {
            if !matches!(&entry.result, Some(Err(_)))
                || entry.touched.elapsed() < Duration::from_secs(5)
            {
                let result = entry.result.clone();
                if matches!(&result, Some(Ok(_))) {
                    cache.hits += 1;
                }
                cache.order.retain(|t| *t != tile);
                cache.order.push_back(tile);
                return result;
            }
            cache.entries.remove(&tile);
            cache.order.retain(|t| *t != tile);
        }
        let tile_bytes = (p.tile_size as usize).pow(2) * 3;
        // Reserve memory for queued tiles too; pinned tiles cannot be evicted.
        while cache.entries.len() >= (128 * 1024 * 1024 / tile_bytes) {
            let old = cache.order.iter().copied().find(|t| {
                cache.entries.get(t).is_some_and(|e| {
                    e.result.as_ref().is_some_and(|r| {
                        r.as_ref()
                            .map_or(true, |i| Arc::strong_count(i.as_raw()) == 1)
                    })
                })
            });
            let old = old?;
            if matches!(cache.entries.remove(&old).unwrap().result, Some(Ok(_))) {
                cache.bytes -= tile_bytes;
            }
            cache.order.retain(|t| *t != old);
        }
        if self.0.requests.try_send(tile).is_ok() {
            cache.entries.insert(
                tile,
                Entry {
                    result: None,
                    touched: Instant::now(),
                },
            );
            cache.order.push_back(tile);
        }
        None
    }
    pub fn get(&self, tile: Tile, job: &Job) -> TileResult {
        loop {
            job.check()?;
            if let Some(result) = self.request(tile) {
                return result;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn stats(&self) -> (u64, u64, usize, f64) {
        let c = self.0.cache.lock().unwrap();
        (
            self.0.http.requests.load(Ordering::Relaxed),
            c.hits,
            c.bytes,
            c.decode_ms,
        )
    }
}
