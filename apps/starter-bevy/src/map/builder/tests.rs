use super::*;
use crate::map::region::{GeoBounds, MapRegion};
use std::collections::BTreeMap;
pub(super) fn region() -> MapRegion {
    let bounds = GeoBounds {
        west: 0.0,
        east: 0.01,
        south: 0.0,
        north: 0.01,
    };
    MapRegion::new(bounds, bounds.center()).unwrap()
}
#[test]
fn projection_and_exact_tile_edges() {
    assert_eq!(coverage::pixel(0.0, 0.0, 1, 512), [512.0, 512.0]);
    let b = crate::map::satellite::PixelBounds {
        left: 0.0,
        top: 0.0,
        right: 512.0,
        bottom: 512.0,
    };
    assert_eq!(
        coverage::tiles(&b, 1, 512, false, 256).unwrap(),
        vec![coverage::Tile { z: 1, x: 0, y: 0 }]
    );
    assert_eq!(coverage::tiles(&b, 1, 512, true, 256).unwrap().len(), 4);
    assert!(coverage::tiles(&b, 1, 512, true, 1).is_err());
    let mut r = region();
    let equator = coverage::resolution(r, 10, 512);
    r.bounds.north = 60.01;
    r.bounds.south = 59.99;
    assert!((coverage::resolution(r, 10, 512) / equator - 0.5).abs() < 1e-6);
}
#[test]
fn terrarium_and_interpolation_across_tiles() {
    assert_eq!(dem::decode_terrarium(128, 0, 0), 0.0);
    assert_eq!(dem::decode_terrarium(127, 255, 128), -0.5);
    let mut tiles = BTreeMap::new();
    for y in 0..2 {
        for x in 0..2 {
            tiles.insert(
                coverage::Tile { z: 1, x, y },
                image::RgbImage::from_pixel(2, 2, image::Rgb([128, (x * 20 + y * 40) as u8, 0])),
            );
        }
    }
    assert_eq!(dem::sample(&tiles, 1, 2, [2.0, 2.0]).unwrap(), 30.0);
    tiles.remove(&coverage::Tile { z: 1, x: 1, y: 1 });
    assert!(dem::sample(&tiles, 1, 2, [2.0, 2.0]).is_err());
    assert!(dem::decode(b"broken", 512, true).is_err());
}
#[test]
fn config_and_deterministic_identity() {
    let c = Config::default();
    c.validate().unwrap();
    assert_eq!(
        cache::id(&(region(), &c)).unwrap(),
        cache::id(&(region(), &c)).unwrap()
    );
    assert!(
        Config {
            max_edge: 1,
            ..c.clone()
        }
        .validate()
        .is_err()
    );
    assert!(coverage::grid(region(), &c).is_ok());
}
