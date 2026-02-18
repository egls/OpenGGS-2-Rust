use bevy::prelude::*;

use crate::states::GameState;
use crate::level::{self, ImportStage};
use crate::tile_properties::{self, TileSheetInfo};

// ── Resources ────────────────────────────────────────────────────────────────

/// Handles to every texture used in-game.
#[derive(Resource)]
pub struct GameAssets {
    pub tiles:     Handle<Image>,
    pub player:    Handle<Image>,
    pub enemies:   Handle<Image>,
    pub jump_sfx:  Handle<AudioSource>,
    pub stomp_sfx: Handle<AudioSource>,
}

/// Loaded level data (binary structs from C++ format).
#[derive(Resource)]
pub struct LevelData {
    pub stage:     ImportStage,
    pub tile_info: TileSheetInfo,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), start_loading)
           .add_systems(Update, check_loading.run_if(in_state(GameState::Loading)));
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn start_loading(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Resolve absolute paths for synchronous binary file reads.
    // The binary is run from rust-lib/, so we go up one level to reach base/.
    let base = std::path::Path::new("..").join("base");

    let stage = level::load_stage(base.join("stages/classic.lvl").to_str().unwrap(), 1)
        .expect("Failed to load classic.lvl");
    let tile_info = tile_properties::load_tile_properties(
        base.join("Tilesheetinfo.tsi").to_str().unwrap(),
    )
    .expect("Failed to load Tilesheetinfo.tsi");

    commands.insert_resource(LevelData { stage, tile_info });

    // Bevy AssetServer resolves relative to rust-lib/assets/ (symlink → ../base)
    let assets = GameAssets {
        tiles:     asset_server.load("c64/Tiles.png"),
        player:    asset_server.load("c64/Player.png"),
        enemies:   asset_server.load("c64/Enemies.png"),
        jump_sfx:  asset_server.load("audio/jump.wav"),
        stomp_sfx: asset_server.load("audio/stomp.wav"),
    };
    commands.insert_resource(assets);
}

fn check_loading(
    assets:       Res<GameAssets>,
    asset_server: Res<AssetServer>,
    mut next:     ResMut<NextState<GameState>>,
) {
    use bevy::asset::LoadState;
    let all_loaded = [
        asset_server.get_load_state(assets.tiles.id()),
        asset_server.get_load_state(assets.player.id()),
        asset_server.get_load_state(assets.enemies.id()),
    ]
    .iter()
    .all(|s| matches!(s, Some(LoadState::Loaded)));

    if all_loaded {
        next.set(GameState::Menu);
    }
}
