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
        allow_online_3d: false,
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
    let service = tiles.service.clone();
    let source_id = super::builder::coverage::Tile { z: 2, x: 1, y: 1 };
    service.request(source_id);
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
    let raster = service.request(source_id).unwrap().unwrap();
    assert_eq!(raster.get_pixel(0, 0).0, [30, 100, 70]);
    assert_eq!(
        service.stats().0,
        1,
        "two consumers share one HTTP tile request"
    );
    assert!(Arc::ptr_eq(
        raster.as_raw(),
        service.request(source_id).unwrap().unwrap().as_raw()
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    assert!(authenticated.load(Ordering::Relaxed));
    server.join().unwrap();
    assert!(tiles.at(id).is_some());
    assert!(tiles.at(TileId { zoom: 3, ..id }).is_none());
    assert_eq!(tiles.stats().in_progress, 0);
}

#[test]
fn source_and_configuration_generations_do_not_share_cache_or_credentials() {
    use super::{builder::coverage::Tile, imagery_service::TileService};
    let p = super::sources::esri();
    let first = TileService::new(p.clone(), Some("old-token".into())).unwrap();
    let mut changed = p.clone();
    changed.name = "Different source".into();
    let second = TileService::new(changed, None).unwrap();
    let renewed = TileService::new(p, Some("new-token".into())).unwrap();
    assert_ne!(first.generation(), second.generation());
    assert_ne!(first.generation(), renewed.generation());
    assert_eq!(second.stats().0, 0);
    assert_eq!(renewed.stats().2, 0);
    assert!(first.request(Tile { z: 23, x: 0, y: 0 }).unwrap().is_err());
    assert!(!first.provider().allow_online_3d);
    assert!(first.provider().validate().is_err());
}
