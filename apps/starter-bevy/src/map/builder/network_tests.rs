use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Mutex, atomic::AtomicUsize},
    thread,
};

struct Server {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    hits: Arc<AtomicUsize>,
}
impl Server {
    fn new(tile: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let template = format!("{url}/{{z}}/{{x}}/{{y}}.webp");
        let meta=serde_json::to_vec(&serde_json::json!({"tiles":[template],"attribution":"test","minzoom":0,"maxzoom":0,"tileSize":512,"encoding":"terrarium"})).unwrap();
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
                count.fetch_add(1, Ordering::Relaxed);
                let (status, body) =
                    if request.contains("/retry") && count.load(Ordering::Relaxed) == 1 {
                        ("503 Service Unavailable", b"retry".as_slice())
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
fn job() -> Job {
    let (progress, _) = mpsc::channel();
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
    std::fs::write(path.join("heights.f32"), b"bad").unwrap();
    assert_eq!(
        dem::build(region, &config, root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        first.fingerprint
    );
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
