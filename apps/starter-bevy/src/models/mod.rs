//! Native Bevy assets; the project document owns the instance parameters.
pub mod assets;
pub mod files;
pub mod geometry;
use bevy::{
    asset::{
        AssetApp,
        io::{AssetSourceBuilder, file::FileAssetReader},
    },
    prelude::*,
};

pub struct ModelsPlugin;
impl Plugin for ModelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<assets::ModelAssets>()
            .add_systems(Update, assets::sync)
            .add_observer(geometry::ready);
    }
}

/// Register once, before AssetPlugin builds its sources.
pub fn register_source(app: &mut App) {
    if let Some(root) = files::cache_root() {
        app.register_asset_source(
            "models",
            AssetSourceBuilder::new(move || Box::new(FileAssetReader::new(root.clone()))),
        );
    }
}

#[cfg(test)]
mod tests;
