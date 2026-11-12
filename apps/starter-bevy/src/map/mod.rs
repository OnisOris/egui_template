pub mod builder;
pub mod controls;
pub mod coordinates;
pub mod dem_mesh;
pub mod demo;
pub mod loading;
mod marker_overlay;
pub mod objects;
pub mod picker;
pub mod real_scene;
pub mod region;
pub mod satellite;
pub mod satellite_scene;
pub mod selection;
pub mod source;
mod terrain;
mod tiles;
pub mod ui;

use bevy::prelude::*;
use picker::MapPicker;
use region::ActiveMapRegion;

/// Carries a command, never a second copy of the confirmed geographic region.
#[derive(Resource, Default)]
pub struct RegionRuntime {
    pub open_requested: bool,
    pub mode: SceneMode,
    pub real_requested: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneMode {
    #[default]
    Editor,
    Demo,
    Real,
}

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send(MapPicker::default())
            .init_resource::<objects::MarkerScene>()
            .init_resource::<ActiveMapRegion>()
            .init_resource::<RegionRuntime>()
            .init_resource::<demo::DemoScene>()
            .init_resource::<loading::TerrainImport>()
            .init_resource::<real_scene::RealScene>()
            .init_resource::<satellite_scene::SatelliteImport>()
            .add_systems(Startup, loading::import_from_env)
            .add_systems(
                Update,
                (
                    loading::poll_import,
                    builder::runtime::poll_builder,
                    real_scene::open_real,
                    demo::open_region,
                    satellite_scene::poll_satellite,
                    satellite_scene::update_material,
                    real_scene::grid_visibility,
                    objects::sync,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod interaction_tests;

#[cfg(test)]
mod dem_tests;

#[cfg(test)]
mod satellite_tests;

#[cfg(test)]
mod object_tests;
