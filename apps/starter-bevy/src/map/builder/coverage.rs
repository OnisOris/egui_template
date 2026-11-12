use super::Config;
use crate::map::{
    coordinates::wgs84_to_local,
    region::{EARTH_RADIUS, MapRegion},
    satellite::PixelBounds,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

pub fn pixel(lon: f64, lat: f64, z: u8, size: u32) -> [f64; 2] {
    let p = walkers::mercator::project(walkers::Position::new(lon, lat), f64::from(z));
    let scale = f64::from(size) / 256.0;
    let world = f64::from(size) * 2f64.powi(i32::from(z));
    [
        (p.x() * scale).clamp(0.0, world),
        (p.y() * scale).clamp(0.0, world),
    ]
}
pub fn bounds(region: MapRegion, z: u8, size: u32) -> PixelBounds {
    let b = region.bounds;
    let [left, top] = pixel(b.west, b.north, z, size);
    let [right, bottom] = pixel(b.east, b.south, z, size);
    PixelBounds {
        left,
        top,
        right,
        bottom,
    }
}
pub fn tiles(
    b: &PixelBounds,
    z: u8,
    size: u32,
    halo: bool,
    limit: usize,
) -> Result<Vec<Tile>, String> {
    let world = (1u64 << z) * u64::from(size);
    let range = |a: f64, b: f64| {
        let (a, b) = if halo {
            ((a - 0.5).floor(), (b - 0.5).floor() + 1.0)
        } else {
            (a.floor(), b.ceil() - 1.0)
        };
        (
            (a.max(0.0) as u64 / u64::from(size)) as u32,
            (b.max(0.0).min((world - 1) as f64) as u64 / u64::from(size)) as u32,
        )
    };
    let (x0, x1) = range(b.left, b.right);
    let (y0, y1) = range(b.top, b.bottom);
    let count = u64::from(x1 - x0 + 1) * u64::from(y1 - y0 + 1);
    if count > limit as u64 {
        return Err("Tile limit exceeded; reduce the region".into());
    }
    Ok((y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| Tile { z, x, y }))
        .collect())
}
pub fn resolution(region: MapRegion, z: u8, size: u32) -> f64 {
    2.0 * std::f64::consts::PI * EARTH_RADIUS * region.bounds.center().latitude.to_radians().cos()
        / (f64::from(size) * 2f64.powi(i32::from(z)))
}
pub fn zoom(region: MapRegion, min: u8, max: u8, size: u32, step: f64) -> u8 {
    (min..=max)
        .find(|&z| resolution(region, z, size) <= step)
        .unwrap_or(max)
}
pub fn grid(region: MapRegion, config: &Config) -> Result<(u32, u32), String> {
    let nw = wgs84_to_local(region.origin, region.bounds.west, region.bounds.north);
    let se = wgs84_to_local(region.origin, region.bounds.east, region.bounds.south);
    if nw.into_iter().chain(se).any(|v| v.abs() > 1_000_000.0) {
        return Err("Region/origin exceeds local coordinate limits; reduce the region".into());
    }
    let mut step = config
        .step_m
        .max((se[0] - nw[0]) / f64::from(config.max_edge - 1))
        .max((se[1] - nw[1]) / f64::from(config.max_edge - 1));
    loop {
        let w = (((se[0] - nw[0]) / step).ceil() as u32 + 1).clamp(2, config.max_edge);
        let h = (((se[1] - nw[1]) / step).ceil() as u32 + 1).clamp(2, config.max_edge);
        let count = u64::from(w) * u64::from(h);
        if count <= config.max_samples as u64 {
            return Ok((w, h));
        }
        step *= ((count as f64 / config.max_samples as f64).sqrt()).max(1.01);
    }
}
