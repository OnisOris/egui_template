use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Mutex, atomic::AtomicUsize},
    thread,
};

pub(super) struct Server {
    pub(super) url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    hits: Arc<AtomicUsize>,
}
impl Server {
    pub(super) fn new(tile: Vec<u8>) -> Self {
        Self::with_missing(tile, 0, None)
    }
    fn with_missing(tile: Vec<u8>, maxzoom: u8, missing: Option<&str>) -> Self {
        let missing = missing.map(str::to_owned);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let template = format!("{url}/{{z}}/{{x}}/{{y}}.webp");
        let meta=serde_json::to_vec(&serde_json::json!({"tiles":[template],"attribution":"test","minzoom":0,"maxzoom":maxzoom,"tileSize":512,"encoding":"terrarium"})).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let hits = Arc::new(AtomicUsize::new(0));
        let count = hits.clone();
        let thread = thread::spawn(move || {
            while !signal.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                    .unwrap();
                let mut request = [0; 4096];
                let n = stream.read(&mut request).unwrap_or(0);
                let request = String::from_utf8_lossy(&request[..n]);
                if request.contains("/timeout") {
                    thread::sleep(std::time::Duration::from_millis(200));
                }

                count.fetch_add(1, Ordering::Relaxed);
                let (status, body) =
                    if request.contains("/retry") && count.load(Ordering::Relaxed) == 1 {
                        ("503 Service Unavailable", b"retry".as_slice())
                    } else if missing.as_ref().is_some_and(|path| request.contains(path)) {
                        ("404 Not Found", b"missing".as_slice())
                    } else if request.contains("/tilejson") {
                        ("200 OK", meta.as_slice())
                    } else {
                        ("200 OK", tile.as_slice())
                    };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
            hits,
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
pub(super) fn job() -> Job {
    let progress = Arc::new(Mutex::new(String::new()));
    Job {
        cancelled: Arc::new(AtomicBool::new(false)),
        progress,
    }
}
fn tile() -> Vec<u8> {
    let image = image::RgbImage::from_fn(512, 512, |_, y| image::Rgb([128, (y / 2) as u8, 0]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::WebP)
        .unwrap();
    bytes.into_inner()
}
#[test]
fn build_roundtrip_cache_corruption_and_cancel() {
    let server = Server::new(tile());
    let root = tempfile::tempdir().unwrap();
    let config = Config {
        terrain_tilejson: format!("{}/tilejson", server.url),
        ..Config::default()
    };
    let region = super::tests::region();
    let job = job();
    let http = download::Download::new(2).unwrap();
    let first = dem::build(region, &config, root.path(), &http, &job).unwrap();
    assert_eq!(first.manifest.reference_elevation_source, "origin-sample");
    assert!(first.samples_msl[0] <= *first.samples_msl.last().unwrap());
    let m = &first.manifest;
    assert!(
        m.reference_elevation_msl >= m.min_elevation_msl
            && m.reference_elevation_msl <= m.max_elevation_msl
    );
    let path = root
        .path()
        .join("packages")
        .join(&m.terrain_id)
        .join("terrain");
    let raw = std::fs::read(path.join("heights.f32")).unwrap();
    assert_eq!(&raw[..4], &first.samples_msl[0].to_le_bytes());
    let hits = server.hits.load(Ordering::Relaxed);
    assert_eq!(
        dem::build(region, &config, root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        first.fingerprint
    );
    assert_eq!(server.hits.load(Ordering::Relaxed), hits); // Fully offline package reuse.
    let different_imagery = Config {
        imagery_step_m: 10.0,
        http_parallelism: 1,
        ..config.clone()
    };
    assert_eq!(
        dem::build(region, &different_imagery, root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        first.fingerprint
    );
    std::fs::write(path.join("heights.f32"), b"bad").unwrap();
    assert_eq!(
        dem::build(region, &config, root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        first.fingerprint
    );
    let outside = crate::map::region::MapRegion {
        origin: crate::map::region::GeoOrigin {
            longitude: 1.0,
            latitude: 1.0,
        },
        ..region
    };
    let external = dem::build(outside, &config, root.path(), &http, &job).unwrap();
    assert_eq!(
        external.manifest.reference_elevation_source,
        "origin-sample"
    );
    assert!(external.manifest.reference_elevation_msl < external.manifest.min_elevation_msl);
    job.cancelled.store(true, Ordering::Relaxed);
    assert!(
        dem::build(region, &config, root.path(), &http, &job)
            .unwrap_err()
            .contains("cancelled")
    );
}
#[test]
fn retry_and_atomic_publication() {
    let server = Server::new(b"ok".to_vec());
    let http = download::Download::new(1).unwrap();
    assert_eq!(
        http.get(&format!("{}/retry", server.url), None, &job())
            .unwrap(),
        b"ok"
    );
    assert_eq!(server.hits.load(Ordering::Relaxed), 2);
    let root = tempfile::tempdir().unwrap();
    let guard = Mutex::new(());
    let _guard = guard.lock().unwrap();
    let stage = cache::Staging::new(root.path()).unwrap();
    let path = stage.path.clone();
    drop(stage);
    assert!(!path.exists());
}

#[test]
fn bounded_timeout() {
    let server = Server::new(b"ok".to_vec());
    let http = download::Download::with_timeout(1, std::time::Duration::from_millis(40)).unwrap();
    let start = std::time::Instant::now();
    assert!(
        http.get(&format!("{}/timeout", server.url), None, &job())
            .unwrap_err()
            .contains("timed out")
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn lower_zoom_cache_works_offline_and_origin_failure_is_explicit() {
    let server = Server::with_missing(tile(), 5, Some("/5/"));
    let root = tempfile::tempdir().unwrap();
    let config = Config {
        terrain_tilejson: format!("{}/tilejson", server.url),
        ..Config::default()
    };
    let http = download::Download::new(2).unwrap();
    let job = job();
    let data = dem::build(super::tests::region(), &config, root.path(), &http, &job).unwrap();
    drop(server); // No server: cached lower zoom must be found before another high-zoom probe.
    assert_eq!(
        dem::build(super::tests::region(), &config, root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        data.fingerprint
    );
    let server = Server::with_missing(tile(), 5, Some("/5/18/14.webp"));
    let config = Config {
        terrain_tilejson: format!("{}/tilejson", server.url),
        ..Config::default()
    };
    let bounds = crate::map::region::GeoBounds {
        west: 20.0,
        east: 20.01,
        south: 20.0,
        north: 20.01,
    };
    let region = crate::map::region::MapRegion::new(
        bounds,
        crate::map::region::GeoOrigin {
            longitude: 23.0,
            latitude: 20.0,
        },
    )
    .unwrap();
    let data = dem::build(region, &config, root.path(), &http, &job).unwrap();
    assert_eq!(data.manifest.reference_elevation_source, "fallback-mean");
    let mean =
        data.samples_msl.iter().map(|h| f64::from(*h)).sum::<f64>() / data.samples_msl.len() as f64;
    assert!((f64::from(data.manifest.reference_elevation_msl) - mean).abs() < 0.001);
}
