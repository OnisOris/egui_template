pub mod coordinates;
pub mod demo;
pub mod picker;
pub mod region;
pub mod selection;
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
    pub showing_demo: bool,
}

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send(MapPicker::default())
            .init_resource::<ActiveMapRegion>()
            .init_resource::<RegionRuntime>()
            .init_resource::<demo::DemoScene>()
            .add_systems(Update, demo::open_region);
    }
}

#[cfg(test)]
mod interaction_tests;
