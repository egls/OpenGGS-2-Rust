use bevy::prelude::*;

mod assets;
mod audio;
mod enemy;
mod game_over;
mod hud;
mod level;
mod menu;
mod player;
mod states;
mod tile_properties;
mod tilemap;

use assets::AssetsPlugin;
use enemy::EnemyPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use player::PlayerPlugin;
use states::GameState;
use tilemap::TileMapPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
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
            TileMapPlugin,
            PlayerPlugin,
            EnemyPlugin,
            HudPlugin,
            audio::AudioPlugin,
            game_over::GameOverPlugin,
        ))
        .run();
}