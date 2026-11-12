use super::{builder::provider::ImageryProvider, imagery_tiles::ImageryTiles};
use bevy_egui::egui::Context;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use walkers::{TileId, Tiles};
#[test]
fn walkers_satellite_uses_bearer_auth_and_reuses_decoded_tiles() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let authenticated = Arc::new(AtomicBool::new(false));
    let checked = authenticated.clone();
    let mut buffer = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(256, 256, image::Rgb([30, 100, 70]))
        .write_to(&mut buffer, image::ImageFormat::Png)
        .unwrap();
    let bytes = buffer.into_inner();
    let server = std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 8192];
                let n = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..n]);
                checked.store(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer fixture-token"),
                    Ordering::Relaxed,
                );
                assert!(request.starts_with("GET /2/1/1.png "));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )
                .unwrap();
                stream.write_all(&bytes).unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("no tile request");
    });
    let provider = ImageryProvider {
        name: "Fixture".into(),
        url: format!("http://{address}/{{z}}/{{x}}/{{y}}.png"),
        min_zoom: 2,
        max_zoom: 2,
        tile_size: 256,
        attribution: "Fixture".into(),
        allow_permanent_packages: true,
        bearer_env: Some("TEST_ONLY_EXPLICIT_TOKEN".into()),
    };
    let root = tempfile::tempdir().unwrap();
    let mut tiles = ImageryTiles::with_token(
        provider,
        Context::default(),
        Some("fixture-token".into()),
        root.path().to_owned(),
    );
    let id = TileId {
        zoom: 2,
        x: 1,
        y: 1,
    };
    assert!(tiles.at(id).is_none());
    let start = Instant::now();
    let piece = loop {
        tiles.poll();
        if let Some(piece) = tiles.at(id) {
            break piece;
        }
        assert!(tiles.error.is_none(), "{:?}", tiles.error);
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(piece.uv.width(), 1.0);
    assert!(authenticated.load(Ordering::Relaxed));
    server.join().unwrap();
    assert!(tiles.at(id).is_some());
    assert!(tiles.at(TileId { zoom: 3, ..id }).is_none());
    assert_eq!(tiles.stats().in_progress, 0);
}
