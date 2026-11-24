use std::io;
use std::path::PathBuf;

use bevy::prelude::*;

use crate::level::{self, ImportStage};
use crate::paths::{discover_stage_packs, AssetPaths, StagePack};
use crate::states::GameState;
use crate::tile_properties::{self, TileSheetInfo};

pub const MAX_CLASSIC_STAGE: usize = 33;
pub const MAX_SUBSTAGE_INDEX: usize = 37;

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameAssets {
    pub tiles:      Handle<Image>,
    pub player:     Handle<Image>,
    pub enemies:    Handle<Image>,
    pub powerups:   Handle<Image>,
    pub interface:  Handle<Image>,
    pub jump_sfx:   Handle<AudioSource>,
    pub stomp_sfx:  Handle<AudioSource>,
    pub died_sfx:   Handle<AudioSource>,
    pub ding_sfx:   Handle<AudioSource>,
    pub crack_sfx:  Handle<AudioSource>,
    pub morph_sfx:  Handle<AudioSource>,
    pub shoot_sfx:  Handle<AudioSource>,
    pub portal_sfx: Handle<AudioSource>,
    pub click_sfx:  Handle<AudioSource>,
    pub disk_sfx:   Handle<AudioSource>,
}

#[derive(Resource)]
pub struct LevelData {
    pub stage_packs:           Vec<StagePack>,
    pub pack_index:            usize,
    pub stage_file:            PathBuf,
    pub stage_number:          usize,
    pub checkpoint:            usize,
    pub sub_stage_entry_stage: Option<usize>,
    pub sub_stage_counter:     usize,
    pub is_warping:            bool,
    pub stage_width_px:        f32,
    pub stage:                 ImportStage,
    pub tile_info:             TileSheetInfo,
}

impl LevelData {
    pub fn pack_count(&self) -> usize {
        self.stage_packs.len().max(1)
    }

    pub fn set_pack(&mut self, index: usize) {
        if self.stage_packs.is_empty() {
            return;
        }
        let idx = index % self.stage_packs.len();
        self.pack_index = idx;
        self.stage_file = self.stage_packs[idx].path.clone();
        self.sub_stage_entry_stage = None;
        self.sub_stage_counter = 0;
        self.checkpoint = 0;
        self.is_warping = false;
    }

    pub fn pack_name(&self) -> &str {
        self.stage_packs
            .get(self.pack_index)
            .map(|p| p.name.as_str())
            .unwrap_or("Classic")
    }

    pub fn refresh_stage_width(&mut self) {
        self.stage_width_px = crate::tilemap::compute_stage_width_pixels(&self.stage, 800.0);
    }

    pub fn load_stage_number(&mut self, stage_number: usize, checkpoint: usize) -> io::Result<()> {
        let path_str = self.stage_file.to_string_lossy();
        let loaded = level::load_stage(&path_str, stage_number)?;
        self.stage = loaded;
        self.stage_number = stage_number;
        self.checkpoint = checkpoint.min(3);
        self.refresh_stage_width();
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

fn start_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    paths:        Res<AssetPaths>,
) {
    let mut stage_packs = discover_stage_packs(&paths.stages_dir());
    if stage_packs.is_empty() {
        stage_packs.push(StagePack {
            name: "Classic".to_string(),
            path: paths.stages_dir().join("classic.lvl"),
        });
    }
    let stage_file = stage_packs[0].path.clone();

    let stage = level::load_stage(stage_file.to_str().unwrap(), 1)
        .expect("Failed to load initial stage pack");
    let tile_info = tile_properties::load_tile_properties(
        paths.tilesheet_info().to_str().unwrap(),
    )
    .expect("Failed to load Tilesheetinfo.tsi");

    let stage_width_px = crate::tilemap::compute_stage_width_pixels(&stage, 800.0);

    commands.insert_resource(LevelData {
        stage_packs,
        pack_index: 0,
        stage_file,
        stage_number: 1,
        checkpoint: 0,
        sub_stage_entry_stage: None,
        sub_stage_counter: 0,
        is_warping: false,
        stage_width_px,
        stage,
        tile_info,
    });

    // Bevy AssetPlugin root is configured directly to resolved `base/`.
    let assets = GameAssets {
        tiles:      asset_server.load("c64/Tiles.png"),
        player:     asset_server.load("c64/Player.png"),
        enemies:    asset_server.load("c64/Enemies.png"),
        powerups:   asset_server.load("c64/PowerUp.png"),
        interface:  asset_server.load("Interface.png"),
        jump_sfx:   asset_server.load("audio/jump.wav"),
        stomp_sfx:  asset_server.load("audio/stomp.wav"),
        died_sfx:   asset_server.load("audio/died.wav"),
        ding_sfx:   asset_server.load("audio/ding.wav"),
        crack_sfx:  asset_server.load("audio/crack.wav"),
        morph_sfx:  asset_server.load("audio/morphing.wav"),
        shoot_sfx:  asset_server.load("audio/shoot.wav"),
        portal_sfx: asset_server.load("audio/portal.wav"),
        click_sfx:  asset_server.load("audio/click.wav"),
        disk_sfx:   asset_server.load("audio/disk.wav"),
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
        asset_server.get_load_state(assets.interface.id()),
        asset_server.get_load_state(assets.jump_sfx.id()),
        asset_server.get_load_state(assets.stomp_sfx.id()),
        asset_server.get_load_state(assets.died_sfx.id()),
        asset_server.get_load_state(assets.ding_sfx.id()),
        asset_server.get_load_state(assets.crack_sfx.id()),
        asset_server.get_load_state(assets.morph_sfx.id()),
        asset_server.get_load_state(assets.shoot_sfx.id()),
        asset_server.get_load_state(assets.portal_sfx.id()),
        asset_server.get_load_state(assets.click_sfx.id()),
        asset_server.get_load_state(assets.disk_sfx.id()),
    ];

    for s in &states {
        if matches!(s, Some(LoadState::Failed(_))) {
            eprintln!("Asset failed to load! Check base/ asset directory.");
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
    let handles = [
        &assets.tiles,
        &assets.player,
        &assets.enemies,
        &assets.powerups,
        &assets.interface,
    ];
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
