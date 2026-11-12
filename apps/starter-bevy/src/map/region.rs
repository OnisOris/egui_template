//! WGS84 contract. No map widget, screen pixels or Bevy types belong here.
use bevy::prelude::Resource;

pub use starter_core::{EARTH_RADIUS, GeoBounds, GeoOrigin, MERCATOR_LATITUDE, MapRegion};

#[derive(Resource, Default)]
pub struct ActiveMapRegion {
    region: Option<MapRegion>,
    revision: u64,
}

impl ActiveMapRegion {
    pub fn clear(&mut self) {
        self.region = None;
        self.revision += 1;
    }

    pub fn get(&self) -> Option<(u64, MapRegion)> {
        self.region.map(|region| (self.revision, region))
    }

    pub fn confirm(&mut self, region: MapRegion) -> Result<(), &'static str> {
        MapRegion::new(region.bounds, region.origin)?;
        self.revision += 1;
        self.region = Some(region);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn bounds() -> GeoBounds {
        GeoBounds {
            west: 0.0,
            east: 1.0,
            south: 0.0,
            north: 1.0,
        }
    }

    #[test]
    fn rejects_invalid_bounds_and_accepts_projection_edges() {
        for b in [
            GeoBounds {
                west: f64::NAN,
                ..bounds()
            },
            GeoBounds {
                east: f64::INFINITY,
                ..bounds()
            },
            GeoBounds {
                west: 2.0,
                ..bounds()
            },
            GeoBounds {
                north: 0.0,
                ..bounds()
            },
            GeoBounds {
                north: 90.0,
                ..bounds()
            },
            GeoBounds {
                east: 181.0,
                ..bounds()
            },
        ] {
            assert!(b.validate().is_err());
        }
        assert!(
            GeoBounds {
                west: -180.0,
                east: 180.0,
                south: -MERCATOR_LATITUDE,
                north: MERCATOR_LATITUDE
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn measurements_and_origin_are_stable() {
        let b = bounds();
        let (w, h) = b.dimensions();
        assert!((h - 111_319.49).abs() < 0.01);
        assert!((w - 111_315.25).abs() < 0.1);
        assert!((b.area() / 1e6 - 12_391.4).abs() < 0.1);
        let origin = GeoOrigin {
            latitude: 50.0,
            longitude: 50.0,
        };
        let mut active = ActiveMapRegion::default();
        active.confirm(MapRegion::new(b, origin).unwrap()).unwrap();
        assert_eq!(active.get().unwrap().1.origin, origin);
        let invalid = MapRegion {
            bounds: GeoBounds { west: 2.0, ..b },
            origin,
        };
        assert!(active.confirm(invalid).is_err());
        assert_eq!(active.get().unwrap().0, 1);
    }
}
