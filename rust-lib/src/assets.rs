use std::io;
use std::path::Path;

use bevy::prelude::*;

use crate::level::{self, ImportStage};
use crate::states::GameState;
use crate::tile_properties::{self, TileSheetInfo};

pub const MAX_CLASSIC_STAGE: usize = 33;
pub const MAX_SUBSTAGE_INDEX: usize = 37;

pub const STAGE_PACKS: &[(&str, &str)] = &[
    ("Classic", "../base/stages/classic.lvl"),
    ("New",     "../base/stages/new.lvl"),
    ("Modules", "../base/stages/modules.lvl"),
];

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameAssets {
    pub tiles:      Handle<Image>,
    pub player:     Handle<Image>,
    pub enemies:    Handle<Image>,
    pub powerups:   Handle<Image>,
    pub jump_sfx:   Handle<AudioSource>,
    pub stomp_sfx:  Handle<AudioSource>,
    pub died_sfx:   Handle<AudioSource>,
    pub ding_sfx:   Handle<AudioSource>,
    pub crack_sfx:  Handle<AudioSource>,
    pub morph_sfx:  Handle<AudioSource>,
    pub shoot_sfx:  Handle<AudioSource>,
    pub portal_sfx: Handle<AudioSource>,
    pub click_sfx:  Handle<AudioSource>,
}

#[derive(Resource)]
pub struct LevelData {
    pub pack_index:            usize,
    pub stage_file:            String,
    pub stage_number:          usize,
    pub checkpoint:            usize,
    pub sub_stage_entry_stage: Option<usize>,
    pub sub_stage_counter:     usize,
    pub stage:                 ImportStage,
    pub tile_info:             TileSheetInfo,
}

impl LevelData {
    pub fn set_pack(&mut self, index: usize) {
        let idx = index % STAGE_PACKS.len();
        self.pack_index = idx;
        self.stage_file = STAGE_PACKS[idx].1.to_string();
        self.sub_stage_entry_stage = None;
        self.sub_stage_counter = 0;
        self.checkpoint = 0;
    }

    pub fn pack_name(&self) -> &'static str {
        STAGE_PACKS[self.pack_index % STAGE_PACKS.len()].0
    }

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
    let stage_file = STAGE_PACKS[0].1.to_string();

    let stage = level::load_stage(&stage_file, 1)
        .expect("Failed to load classic.lvl");
    let tile_info = tile_properties::load_tile_properties(
        base.join("Tilesheetinfo.tsi").to_str().unwrap(),
    )
    .expect("Failed to load Tilesheetinfo.tsi");

    commands.insert_resource(LevelData {
        pack_index: 0,
        stage_file,
        stage_number: 1,
        checkpoint: 0,
        sub_stage_entry_stage: None,
        sub_stage_counter: 0,
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
        click_sfx:  asset_server.load("audio/click.wav"),
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
        asset_server.get_load_state(assets.click_sfx.id()),
    ];

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
