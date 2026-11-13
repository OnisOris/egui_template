pub const MERCATOR_LATITUDE: f64 = 85.051_128_779_806_6;
pub const EARTH_RADIUS: f64 = 6_378_137.0;

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MapRegion {
    pub bounds: GeoBounds,
    /// Independent of bbox: a future adapter can supply a fixed home outside the region.
    pub origin: GeoOrigin,
}

impl MapRegion {
    /// Shared tolerance for JSON round-trip error, below 0.2 mm on Earth.
    pub fn matches(self, other: Self) -> bool {
        [
            self.bounds.west - other.bounds.west,
            self.bounds.east - other.bounds.east,
            self.bounds.south - other.bounds.south,
            self.bounds.north - other.bounds.north,
            self.origin.longitude - other.origin.longitude,
            self.origin.latitude - other.origin.latitude,
        ]
        .into_iter()
        .all(|d| d.abs() <= 1e-9)
    }

    pub fn new(bounds: GeoBounds, origin: GeoOrigin) -> Result<Self, &'static str> {
        bounds.validate()?;
        origin.validate()?;
        Ok(Self { bounds, origin })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn region_matching_accepts_roundtrip_noise_but_rejects_a_different_origin() {
        let b = super::GeoBounds {
            west: 37.615387061386684,
            south: 55.745090331002885,
            east: 37.6331015072041,
            north: 55.75536216322164,
        };
        let a = super::MapRegion::new(b, b.center()).unwrap();
        let mut noisy = a;
        noisy.origin.latitude += 1e-13;
        noisy.bounds.east -= 1e-13;
        assert!(a.matches(noisy));
        noisy.origin.latitude += 1e-5;
        assert!(!a.matches(noisy));
    }
}
