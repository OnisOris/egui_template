use super::{mvt, source};
use crate::map::builder::{Job, coverage::Tile};
use starter_core::{GeoBounds, MapRegion};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
fn region() -> MapRegion {
    let b = GeoBounds {
        west: 0.,
        east: 360. / 16384.,
        north: 0.,
        south: -360. / 16384. * 0.999,
    };
    MapRegion::new(b, b.center()).unwrap()
}
#[test]
fn authored_mvt_is_decoded_clipped_deduplicated_and_uses_height() {
    let mut data = mvt::empty(region(), "fixture".into());
    let bytes = include_bytes!("../../../../../docs/fixtures/m53/city.pbf").to_vec();
    let tile = Tile {
        z: 14,
        x: 8192,
        y: 8192,
    };
    mvt::extract(bytes.clone(), tile, &mut data).unwrap();
    assert_eq!(data.buildings.len(), 1);
    assert_eq!(data.roads.len(), 1);
    assert_eq!(data.buildings[0].height_m, 24.);
    mvt::extract(bytes, tile, &mut data).unwrap();
    mvt::deduplicate(&mut data);
    assert_eq!(data.buildings.len(), 1);
    assert_eq!(data.roads.len(), 1);
    super::clip::validate(&data).unwrap();
    assert!(mvt::extract(vec![255; 32], tile, &mut data).is_err());
}
#[test]
fn local_tilejson_cache_and_cancellation_need_no_public_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let requests = seen.clone();
    let thread = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 2048];
            let n = stream.read(&mut buffer).unwrap();
            let path = String::from_utf8_lossy(&buffer[..n])
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_string();
            requests.lock().unwrap().push(path.clone());
            let bytes = if path == "/metadata" {
                format!(r#"{{"tiles":["http://{addr}/{{z}}/{{x}}/{{y}}.pbf"],"minzoom":14,"maxzoom":14}}"#).into_bytes()
            } else {
                include_bytes!("../../../../../docs/fixtures/m53/city.pbf").to_vec()
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
        }
    });
    let job = Job {
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(Mutex::new(String::new())),
    };
    let dir = tempfile::tempdir().unwrap();
    let url = format!("http://{addr}/metadata");
    let data = source::load(region(), dir.path(), &url, &job).unwrap();
    let n = seen.lock().unwrap().len();
    assert_eq!(data.buildings.len(), 1);
    assert_eq!(n, 2);
    let reused = source::load(region(), dir.path(), &url, &job).unwrap();
    assert_eq!(seen.lock().unwrap().len(), n);
    assert_eq!(reused.fingerprint, data.fingerprint);
    stop.store(true, Ordering::Relaxed);
    thread.join().unwrap();
    job.cancelled.store(true, Ordering::Relaxed);
    let other = tempfile::tempdir().unwrap();
    assert!(source::load(region(), other.path(), &url, &job).is_err());
}
#[test]
fn limits_respect_metadata_and_bounded_coverage() {
    let m = source::TileJson {
        tiles: vec!["fixture".into()],
        minzoom: Some(15),
        maxzoom: Some(16),
        attribution: String::new(),
    };
    assert!(source::plan(&m, region()).is_err());
    let m = source::TileJson {
        minzoom: Some(12),
        maxzoom: Some(14),
        ..m
    };
    assert!(source::plan(&m, region()).unwrap().len() <= 64);
}
