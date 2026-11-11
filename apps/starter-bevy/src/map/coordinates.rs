//! UAV v1 local mapping parity: east/south/up → Bevy east/up/south.
use super::region::GeoOrigin;

// UAV package v1 contract, deliberately not Earth-radius-derived.
const METERS_PER_DEGREE: f64 = 111_320.0;
use bevy::prelude::Vec3;

pub fn wgs84_to_local(origin: GeoOrigin, longitude: f64, latitude: f64) -> [f64; 3] {
    [
        METERS_PER_DEGREE * (longitude - origin.longitude) * origin.latitude.to_radians().cos(),
        METERS_PER_DEGREE * (origin.latitude - latitude),
        0.0,
    ]
}

pub fn local_to_wgs84(origin: GeoOrigin, east: f64, south: f64) -> (f64, f64) {
    (
        origin.longitude + east / (METERS_PER_DEGREE * origin.latitude.to_radians().cos()),
        origin.latitude - south / METERS_PER_DEGREE,
    )
}

pub fn to_bevy([east, south, up]: [f64; 3]) -> Vec3 {
    Vec3::new(east as f32, up as f32, south as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_and_external_origin() {
        let origin = GeoOrigin {
            longitude: 30.0,
            latitude: 60.0,
        };
        assert_eq!(to_bevy([2.0, 3.0, 4.0]), Vec3::new(2.0, 4.0, 3.0));
        assert_eq!(wgs84_to_local(origin, 30.0, 60.0), [0.0; 3]);
        let local = wgs84_to_local(origin, 31.0, 59.0);
        assert!((local[0] - 55_660.0).abs() < 0.01);
        assert!((local[1] - 111_320.0).abs() < 0.01);
        assert_eq!(local[2], 0.0);
    }
}
