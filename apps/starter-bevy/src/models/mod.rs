//! Native Bevy assets; the project document owns the instance parameters.
pub mod assets;
pub mod files;
pub mod geometry;
pub mod instances;
pub mod picking;
mod selection;
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
            .init_resource::<instances::ModelScene>()
            .init_resource::<picking::ModelPick>()
            .init_gizmo_group::<selection::ModelGizmos>()
            .add_systems(Startup, selection::configure)
            .add_systems(
                Update,
                (
                    instances::sync
                        .after(assets::sync)
                        .after(crate::project::scene::sync_project),
                    picking::pick,
                ),
            )
            .add_systems(
                PostUpdate,
                selection::show.after(bevy::transform::TransformSystems::Propagate),
            )
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
mod instance_tests;
#[cfg(test)]
mod tests;
