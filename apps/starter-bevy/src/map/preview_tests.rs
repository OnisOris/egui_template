use super::{
    builder::{Job, download::Download},
    imagery_tiles::ImageryTiles,
    sources,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
use walkers::{TileId, Tiles};
#[test]
fn esri_preset_is_preview_only_and_configuration_roundtrips_without_secrets() {
    let p = sources::esri();
    p.validate_preview().unwrap();
    assert!(p.validate().is_err());
    assert!(p.url.ends_with("/{z}/{y}/{x}"));
    let settings = sources::MapSources {
        provider: Some(p),
        online_authorized: true,
        build_packages: false,
        package_directory: Some("packages".into()),
    };
    let json = serde_json::to_string(&settings).unwrap();
    let copy: sources::MapSources = serde_json::from_str(&json).unwrap();
    assert!(!copy.provider.unwrap().allow_permanent_packages);
    assert!(!json.contains("secret"));
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        serde_json::to_vec(settings.provider.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert!(
        super::builder::imagery::configured(Some(file.path().as_os_str().to_owned()))
            .unwrap()
            .is_none()
    );
}
#[test]
fn preview_uses_esri_xyz_order_and_never_reads_or_writes_persistent_tiles() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut p = sources::esri();
    p.url = format!(
        "http://{}/{{z}}/{{y}}/{{x}}",
        listener.local_addr().unwrap()
    );
    let root = tempfile::tempdir().unwrap();
    let cached = root
        .path()
        .join("tiles")
        .join(super::builder::cache::id(&p).unwrap())
        .join("2/3/1.tile");
    std::fs::create_dir_all(cached.parent().unwrap()).unwrap();
    std::fs::write(&cached, b"existing cache must remain untouched").unwrap();
    let server = std::thread::spawn(move || {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(256, 256, image::Rgb([10, 90, 200]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = [0; 4096];
        let n = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..n]).starts_with("GET /2/1/3 "));
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.get_ref().len()
        )
        .unwrap();
        stream.write_all(bytes.get_ref()).unwrap();
    });
    let mut tiles = ImageryTiles::with_token(
        p,
        bevy_egui::egui::Context::default(),
        None,
        root.path().to_owned(),
    );
    let id = TileId {
        zoom: 2,
        x: 3,
        y: 1,
    };
    assert!(tiles.at(id).is_none());
    let start = Instant::now();
    loop {
        tiles.poll();
        if tiles.at(id).is_some() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            tiles.error
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(tiles.at(id).is_some());
    server.join().unwrap();
    assert_eq!(
        std::fs::read(&cached).unwrap(),
        b"existing cache must remain untouched"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[test]
fn http_access_and_rate_limit_errors_are_bounded_and_diagnostic() {
    for status in [401, 403, 404, 429] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/tile", listener.local_addr().unwrap());
        let count = if status == 429 { 3 } else { 1 };
        let server = std::thread::spawn(move || {
            for _ in 0..count {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = [0; 2048];
                assert!(stream.read(&mut request).unwrap() > 0);
                write!(
                    stream,
                    "HTTP/1.1 {status} Fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        });
        let job = Job {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(String::new())),
        };
        let error = Download::new(4).unwrap().get(&url, None, &job).unwrap_err();
        assert!(error.contains(&status.to_string()), "{error}");
        server.join().unwrap();
    }
}

#[test]
fn failed_preview_recovers_when_the_adapter_is_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut provider = sources::esri();
    provider.url = format!(
        "http://{}/{{z}}/{{y}}/{{x}}",
        listener.local_addr().unwrap()
    );
    let server = std::thread::spawn(move || {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(256, 256, image::Rgb([20, 100, 40]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        for status in [403, 200] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = [0; 2048];
            assert!(stream.read(&mut request).unwrap() > 0);
            let body = if status == 200 {
                bytes.get_ref().as_slice()
            } else {
                &[]
            };
            write!(
                stream,
                "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        }
    });
    let root = tempfile::tempdir().unwrap();
    let id = TileId {
        zoom: 2,
        x: 1,
        y: 0,
    };
    let mut tiles = ImageryTiles::with_token(
        provider.clone(),
        bevy_egui::egui::Context::default(),
        None,
        root.path().to_owned(),
    );
    tiles.at(id);
    let start = Instant::now();
    while tiles.error.is_none() {
        tiles.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(tiles.error.as_ref().unwrap().contains("403"));
    drop(tiles);
    let mut tiles = ImageryTiles::with_token(
        provider,
        bevy_egui::egui::Context::default(),
        None,
        root.path().to_owned(),
    );
    tiles.at(id);
    let start = Instant::now();
    loop {
        tiles.poll();
        if tiles.at(id).is_some() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(tiles.error.is_none());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    server.join().unwrap();
}
