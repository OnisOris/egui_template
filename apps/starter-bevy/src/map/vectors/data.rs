//! UAV map-data v2 wire types. Field and enum order are part of the bincode contract.
use serde::{Deserialize, Serialize};
use starter_core::MapRegion;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPoint2 {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum RoadClass {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Minor,
    Service,
}
impl RoadClass {
    pub fn width(self) -> f64 {
        match self {
            Self::Motorway => 14.,
            Self::Trunk => 12.,
            Self::Primary => 10.,
            Self::Secondary => 8.,
            Self::Tertiary => 7.,
            Self::Minor => 5.,
            Self::Service => 3.,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadFeature {
    pub id: String,
    pub class: RoadClass,
    pub points: Vec<LocalPoint2>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingPolygon {
    pub rings: Vec<Vec<LocalPoint2>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFeature {
    pub id: String,
    pub polygons: Vec<BuildingPolygon>,
    pub height_m: f32,
    pub min_height_m: f32,
}
#[derive(Clone, Debug)]
pub struct VectorData {
    pub region: MapRegion,
    pub source: String,
    pub attribution: String,
    pub package_id: String,
    pub fingerprint: blake3::Hash,
    pub skipped_buildings: usize,
    pub metrics: VectorMetrics,
    pub roads: Vec<RoadFeature>,
    pub buildings: Vec<BuildingFeature>,
}
pub const MAX_FEATURES: usize = 100_000;
pub const MAX_POINTS: usize = 2_000_000;

pub fn valid_height(b: &BuildingFeature) -> bool {
    b.height_m.is_finite()
        && b.min_height_m.is_finite()
        && b.min_height_m >= 0.
        && b.height_m > b.min_height_m
        && b.height_m <= 2000.
}

#[derive(Clone, Debug, Default)]
pub struct VectorMetrics {
    pub tiles: usize,
    pub http_requests: u64,
    pub tile_cache_hits: usize,
    pub processed_cache_hit: bool,
    pub load_ms: f64,
    pub decode_ms: f64,
}
