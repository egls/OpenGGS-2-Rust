use bevy::prelude::*;

mod assets;
mod audio;
mod enemy;
mod game_over;
mod hud;
mod level;
mod menu;
mod options;
mod player;
mod powerup;
mod states;
mod tile_properties;
mod tilemap;

use assets::AssetsPlugin;
use enemy::EnemyPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use player::PlayerPlugin;
use powerup::PowerUpPlugin;
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
            options::OptionsPlugin,
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
