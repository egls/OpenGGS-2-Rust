use bevy::asset::AssetPlugin;
use bevy::prelude::*;

mod assets;
mod audio;
mod enemy;
mod game_over;
mod hud;
mod input;
mod level;
mod menu;
mod options;
mod paths;
mod player;
mod powerup;
mod random_gen;
mod screens;
mod states;
mod tile_properties;
mod tilemap;

use assets::AssetsPlugin;
use enemy::EnemyPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use paths::AssetPaths;
use player::PlayerPlugin;
use powerup::PowerUpPlugin;
use states::GameState;
use tilemap::TileMapPlugin;

fn main() {
    let asset_paths = AssetPaths::resolve();
    let asset_root = asset_paths.base_dir.to_string_lossy().into_owned();

    App::new()
        .insert_resource(asset_paths)
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "OpenGGS (Bevy)".into(),
                        resolution: (800.0, 600.0).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<GameState>()
        .add_plugins((
            AssetsPlugin,
            MenuPlugin,
            options::OptionsPlugin,
            screens::ScreensPlugin,
            TileMapPlugin,
            PlayerPlugin,
            EnemyPlugin,
            PowerUpPlugin,
            HudPlugin,
            audio::AudioPlugin,
            game_over::GameOverPlugin,
        ))
        .run();
}
