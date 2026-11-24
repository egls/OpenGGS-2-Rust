use std::io;
use std::path::Path;

use bevy::prelude::*;

use crate::level::{self, ImportStage};
use crate::states::GameState;
use crate::tile_properties::{self, TileSheetInfo};

pub const MAX_CLASSIC_STAGE: usize = 33;

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameAssets {
    pub tiles:     Handle<Image>,
    pub player:    Handle<Image>,
    pub enemies:   Handle<Image>,
    pub powerups:  Handle<Image>,
    pub jump_sfx:  Handle<AudioSource>,
    pub stomp_sfx: Handle<AudioSource>,
    pub died_sfx:  Handle<AudioSource>,
    pub ding_sfx:  Handle<AudioSource>,
    pub crack_sfx: Handle<AudioSource>,
    pub morph_sfx: Handle<AudioSource>,
    pub shoot_sfx: Handle<AudioSource>,
    pub portal_sfx: Handle<AudioSource>,
}

#[derive(Resource)]
pub struct LevelData {
    pub stage_file:   String,
    pub stage_number: usize,
    pub checkpoint:   usize,
    pub stage:        ImportStage,
    pub tile_info:    TileSheetInfo,
}

impl LevelData {
    pub fn load_stage_number(&mut self, stage_number: usize, checkpoint: usize) -> io::Result<()> {
        let loaded = level::load_stage(&self.stage_file, stage_number)?;
        self.stage = loaded;
        self.stage_number = stage_number;
        self.checkpoint = checkpoint.min(3);
        Ok(())
    }
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_persistent_camera)
           .add_systems(OnEnter(GameState::Loading), start_loading)
           .add_systems(Update, check_loading.run_if(in_state(GameState::Loading)));
    }
}

// ── Persistent camera (exists across all states for UI) ───────────────────────

/// Marker so we don't accidentally despawn this camera with GameEntity cleanup.
#[derive(Component)]
pub struct PersistentCamera;

fn spawn_persistent_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PersistentCamera));
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn start_loading(mut commands: Commands, asset_server: Res<AssetServer>) {
    let base = Path::new("..").join("base");
    let stage_file = base.join("stages/classic.lvl").to_string_lossy().into_owned();

    let stage = level::load_stage(&stage_file, 1)
        .expect("Failed to load classic.lvl");
    let tile_info = tile_properties::load_tile_properties(
        base.join("Tilesheetinfo.tsi").to_str().unwrap(),
    )
    .expect("Failed to load Tilesheetinfo.tsi");

    commands.insert_resource(LevelData {
        stage_file,
        stage_number: 1,
        checkpoint: 0,
        stage,
        tile_info,
    });

    // Bevy resolves these relative to rust-lib/assets/ (symlink -> ../base)
    let assets = GameAssets {
        tiles:      asset_server.load("c64/Tiles.png"),
        player:     asset_server.load("c64/Player.png"),
        enemies:    asset_server.load("c64/Enemies.png"),
        powerups:   asset_server.load("c64/PowerUp.png"),
        jump_sfx:   asset_server.load("audio/jump.wav"),
        stomp_sfx:  asset_server.load("audio/stomp.wav"),
        died_sfx:   asset_server.load("audio/died.wav"),
        ding_sfx:   asset_server.load("audio/ding.wav"),
        crack_sfx:  asset_server.load("audio/crack.wav"),
        morph_sfx:  asset_server.load("audio/morphing.wav"),
        shoot_sfx:  asset_server.load("audio/shoot.wav"),
        portal_sfx: asset_server.load("audio/portal.wav"),
    };
    commands.insert_resource(assets);
}

fn check_loading(
    assets_opt:   Option<Res<GameAssets>>,
    asset_server: Res<AssetServer>,
    mut images:   ResMut<Assets<Image>>,
    mut next:     ResMut<NextState<GameState>>,
) {
    use bevy::asset::LoadState;

    // GameAssets may not be inserted yet on the very first frame
    let Some(assets) = assets_opt else { return };

    let states = [
        asset_server.get_load_state(assets.tiles.id()),
        asset_server.get_load_state(assets.player.id()),
        asset_server.get_load_state(assets.enemies.id()),
        asset_server.get_load_state(assets.powerups.id()),
        asset_server.get_load_state(assets.jump_sfx.id()),
        asset_server.get_load_state(assets.stomp_sfx.id()),
        asset_server.get_load_state(assets.died_sfx.id()),
        asset_server.get_load_state(assets.ding_sfx.id()),
        asset_server.get_load_state(assets.crack_sfx.id()),
        asset_server.get_load_state(assets.morph_sfx.id()),
        asset_server.get_load_state(assets.shoot_sfx.id()),
        asset_server.get_load_state(assets.portal_sfx.id()),
    ];

    // Log any failures
    for s in &states {
        if matches!(s, Some(LoadState::Failed(_))) {
            eprintln!("Asset failed to load! Check that rust-lib/assets symlink exists.");
            process_transparency(&assets, &mut images);
            next.set(GameState::Menu);
            return;
        }
    }

    let all_loaded = states.iter().all(|s| matches!(s, Some(LoadState::Loaded)));
    if all_loaded {
        process_transparency(&assets, &mut images);
        next.set(GameState::Menu);
    }
}

fn process_transparency(assets: &GameAssets, images: &mut Assets<Image>) {
    let handles = [&assets.tiles, &assets.player, &assets.enemies, &assets.powerups];
    for handle in handles {
        if let Some(image) = images.get_mut(handle) {
            // Assume RGBA8
            for pixel in image.data.chunks_exact_mut(4) {
                // Magenta: 255, 0, 255
                if pixel[0] == 255 && pixel[1] == 0 && pixel[2] == 255 {
                    pixel[0] = 0;
                    pixel[1] = 0;
                    pixel[2] = 0;
                    pixel[3] = 0;
                }
            }
        }
    }
}
