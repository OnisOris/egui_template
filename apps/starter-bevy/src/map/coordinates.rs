//! Demo local tangent approximation: domain east/south/up → Bevy east/up/south.
use super::region::{EARTH_RADIUS, GeoOrigin};
use bevy::prelude::Vec3;

pub fn wgs84_to_local(origin: GeoOrigin, longitude: f64, latitude: f64) -> [f64; 3] {
    [
        EARTH_RADIUS
            * (longitude - origin.longitude).to_radians()
            * origin.latitude.to_radians().cos(),
        EARTH_RADIUS * (origin.latitude - latitude).to_radians(),
        0.0,
    ]
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
        assert!((local[0] - 55_659.745).abs() < 0.01);
        assert!((local[1] - 111_319.491).abs() < 0.01);
        assert_eq!(local[2], 0.0);
    }
}
