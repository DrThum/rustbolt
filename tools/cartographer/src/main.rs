use bevy::prelude::*;
use cartographer::{
    models::terrain::{TerrainBlockLoader, WrappedTerrainBlock},
    resources::terrain_handle::TerrainHandle,
};
use smooth_bevy_cameras::{controllers::orbit::OrbitCameraPlugin, LookTransformPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(LookTransformPlugin)
        .add_plugin(OrbitCameraPlugin::default())
        .add_asset::<WrappedTerrainBlock>()
        .init_asset_loader::<TerrainBlockLoader>()
        .init_resource::<TerrainHandle>()
        .add_startup_system(cartographer::setup)
        .add_system(cartographer::display_terrain)
        .run();
}
