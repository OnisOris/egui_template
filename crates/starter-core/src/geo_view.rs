//! Presentation settings never change saved geographic coordinates or the DEM.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeoViewMode {
    City,
    #[default]
    Terrain,
    Combined,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorSource {
    #[default]
    OpenFreeMap,
    LocalPackage,
}
impl crate::LayerKind {
    pub const ALL: [Self; 4] = [Self::Terrain, Self::Satellite, Self::Roads, Self::Buildings];
}
