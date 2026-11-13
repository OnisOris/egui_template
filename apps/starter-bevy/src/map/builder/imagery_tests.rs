use super::*;
use crate::map::{satellite::PixelBounds, source};
use std::collections::BTreeMap;
fn provider(url: String) -> provider::ImageryProvider {
    provider::ImageryProvider {
        name: "Fixture".into(),
        url,
        min_zoom: 1,
        max_zoom: 1,
        tile_size: 256,
        attribution: "Fixture license".into(),
        allow_permanent_packages: true,
        allow_online_3d: false,
        bearer_env: None,
    }
}
#[test]
fn stitch_crop_north_up_and_uv() {
    let mut tiles = BTreeMap::new();
    for y in 0..2 {
        for x in 0..2 {
            tiles.insert(
                coverage::Tile { z: 1, x, y },
                image::RgbImage::from_pixel(
                    256,
                    256,
                    image::Rgb([x as u8 * 100, y as u8 * 100, 0]),
                ),
            );
        }
    }
    let b = PixelBounds {
        left: 250.0,
        top: 250.0,
        right: 270.0,
        bottom: 280.0,
    };
    let image = imagery::assemble(&tiles, 1, 256, &b, &network_tests::job()).unwrap();
    assert_eq!(image.dimensions(), (20, 30));
    assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0]);
    assert_eq!(image.get_pixel(19, 29).0, [100, 100, 0]);
    let geo = crate::map::satellite::Georeference {
        zoom: 1,
        tile_size: 256,
        crop_pixel_bounds: b,
    };
    let uv = geo.uv(0.0, 0.0);
    assert!((uv[0] - 0.3).abs() < 1e-6);
    assert!((uv[1] - 0.2).abs() < 1e-6);
    tiles.clear();
    assert!(
        imagery::assemble(
            &tiles,
            1,
            256,
            &geo.crop_pixel_bounds,
            &network_tests::job()
        )
        .is_err()
    );
}
#[test]
fn jpeg_package_roundtrip_cache_and_permissions() {
    assert!(imagery::configured(None).unwrap().is_none());
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(256, 256, image::Rgb([30, 120, 60]))
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .unwrap();
    let server = network_tests::Server::new(bytes.into_inner());
    let p = provider(format!("{}/{{z}}/{{x}}/{{y}}.jpg", server.url));
    p.validate().unwrap();
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = source::load_terrain(dir.path()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let http = download::Download::new(2).unwrap();
    let job = network_tests::job();
    let first = imagery::build(&terrain, &p, &Config::default(), root.path(), &http, &job).unwrap();
    let s = &first.manifest.satellite;
    let b = &s.georeference.crop_pixel_bounds;
    assert_eq!(f64::from(s.width), b.right - b.left);
    assert_eq!(f64::from(s.height), b.bottom - b.top);
    assert_eq!(
        imagery::build(&terrain, &p, &Config::default(), root.path(), &http, &job)
            .unwrap()
            .fingerprint,
        first.fingerprint
    );
    assert!(
        provider::ImageryProvider {
            allow_permanent_packages: false,
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        provider::ImageryProvider {
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".into(),
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        provider::ImageryProvider { max_zoom: 30, ..p }
            .validate()
            .is_err()
    );
}

#[test]
fn imagery_failure_preserves_successful_dem() {
    let server = network_tests::Server::new(b"corrupt jpeg".to_vec());
    let p = provider(format!("{}/{{z}}/{{x}}/{{y}}.jpg", server.url));
    let dir = crate::map::dem_tests::package(3, 3);
    let terrain = source::load_terrain(dir.path()).unwrap();
    let fingerprint = terrain.fingerprint;
    let root = tempfile::tempdir().unwrap();
    assert!(
        imagery::build(
            &terrain,
            &p,
            &Config::default(),
            root.path(),
            &download::Download::new(2).unwrap(),
            &network_tests::job()
        )
        .is_err()
    );
    assert_eq!(
        source::load_terrain(dir.path()).unwrap().fingerprint,
        fingerprint
    );
}
