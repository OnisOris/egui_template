mod document;
mod state;

pub use document::{
    Document, DocumentError, GroupId, MAX_NAME_LENGTH, MAX_OBJECT_SIZE, MAX_WORLD_COORDINATE,
    ObjectId, Position, SceneObject, TEST_LOADS,
};
pub use state::{Command, DocumentState};

mod geo;
pub mod project;
pub use geo::{EARTH_RADIUS, GeoBounds, GeoOrigin, MERCATOR_LATITUDE, MapRegion};
pub use project::{
    LayerKind, MapLayer, MapMarker, ProjectAsset, ProjectDocument, ProjectSelection, ProjectState,
};

mod project_models;
pub use project_models::{ModelInstance, ObjectRow};
#[cfg(test)]
mod project_model_tests;

mod geo_view;
pub use geo_view::{GeoViewMode, VectorSource};
