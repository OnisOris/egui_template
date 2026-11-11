//! WGS84 contract. No map widget, screen pixels or Bevy types belong here.
use bevy::prelude::Resource;

pub const MERCATOR_LATITUDE: f64 = 85.051_128_779_806_6;
pub const EARTH_RADIUS: f64 = 6_378_137.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoOrigin {
    pub longitude: f64,
    pub latitude: f64,
}

impl GeoOrigin {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.longitude.is_finite() || !self.latitude.is_finite() {
            return Err("Coordinates must be finite numbers.");
        }
        if !(-180.0..=180.0).contains(&self.longitude)
            || !(-MERCATOR_LATITUDE..=MERCATOR_LATITUDE).contains(&self.latitude)
        {
            return Err("Longitude must be −180…180°, latitude −85.05112878…85.05112878°.");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl GeoBounds {
    pub fn validate(self) -> Result<(), &'static str> {
        GeoOrigin {
            longitude: self.west,
            latitude: self.south,
        }
        .validate()?;
        GeoOrigin {
            longitude: self.east,
            latitude: self.north,
        }
        .validate()?;
        if self.west >= self.east || self.south >= self.north {
            return Err("Select a nonzero rectangle. Antimeridian crossing is not supported.");
        }
        Ok(())
    }

    pub fn center(self) -> GeoOrigin {
        GeoOrigin {
            longitude: (self.west + self.east) * 0.5,
            latitude: (self.south + self.north) * 0.5,
        }
    }

    /// Parallel/meridian lengths on a sphere; intended for approximate UI measurements.
    pub fn dimensions(self) -> (f64, f64) {
        (
            EARTH_RADIUS
                * (self.east - self.west).to_radians()
                * self.center().latitude.to_radians().cos(),
            EARTH_RADIUS * (self.north - self.south).to_radians(),
        )
    }

    pub fn area(self) -> f64 {
        EARTH_RADIUS.powi(2)
            * (self.east - self.west).to_radians()
            * (self.north.to_radians().sin() - self.south.to_radians().sin())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapRegion {
    pub bounds: GeoBounds,
    /// Independent of bbox: a future adapter can supply a fixed home outside the region.
    pub origin: GeoOrigin,
}

impl MapRegion {
    pub fn new(bounds: GeoBounds, origin: GeoOrigin) -> Result<Self, &'static str> {
        bounds.validate()?;
        origin.validate()?;
        Ok(Self { bounds, origin })
    }
}

#[derive(Resource, Default)]
pub struct ActiveMapRegion {
    region: Option<MapRegion>,
    revision: u64,
}

impl ActiveMapRegion {
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
