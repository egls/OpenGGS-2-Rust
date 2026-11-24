use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera, MAX_CLASSIC_STAGE, MAX_SUBSTAGE_INDEX};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
use crate::states::{not_paused, GamePaused, GameState};
use crate::tile_properties::TileSheetInfo;

pub const TILE_W: f32 = 16.0;
pub const TILE_H: f32 = 16.0;
pub const TILES_PER_ROW: i32 = 40;
pub const TILESHEET_W: f32 = 40.0 * 16.0;
pub const TILESHEET_H: f32 = 58.0 * 16.0;
pub const STAGE_COLS: usize = 256;
pub const STAGE_ROWS: usize = 30;

// ── Marker components ─────────────────────────────────────────────────────────

/// Marks all entities that belong to the in-game world (tiles, player, enemies, items, platforms).
/// Used for bulk despawn on state exit.
#[derive(Component)]
pub struct GameEntity;

/// A single map tile.
#[derive(Component)]
pub struct Tile {
    pub tile_id: i32,
}

/// Tag for tiles whose sprite rect animates on `TileAnimClock`.
#[derive(Component)]
pub struct AnimatedTile;

/// Tracks any tile undergoing a timed C++ `SwitchTileToNextFrame` transition
/// (crumbling ground, coin-block blink, 5-coin step, breakable brick crack).
#[derive(Component)]
pub struct TimedTileSwitch {
    pub tx:    usize,
    pub ty:    usize,
    pub timer: f32,
}

/// Falling solid stone dislodged when Punk Giana head-bumps a `DropStone` block
/// (matches C++ `Sprite_DropStone` in `GRAPHICS_Sprites.cpp` & `GRAPHICS_Sprites_Move.cpp`).
#[derive(Component)]
pub struct DropStone {
    pub vy:     f32,
    pub w:      f32,
    pub h:      f32,
    pub landed: bool,
}

#[derive(Event)]
pub struct SpawnDropStoneEvent {
    pub x: f32,
    pub y: f32,
}

/// Parallax scenery element positioned at `base_x + stage_scroll_x * factor`
/// (matches C++ half-speed scrolling clouds, tubes, and indoor bricks in `STAGE_Draw.cpp`).
#[derive(Component)]
pub struct ParallaxLayer {
    pub base_x: f32,
    pub factor: f32,
}

/// Computes world X for a parallax background element so it scrolls at `(1 - factor)` of camera speed.
/// With `factor = 0.5`, screen X moves at half camera speed (`base_x - stage_scroll_x / 2`).
pub fn parallax_world_x(base_x: f32, stage_scroll_x: f32, factor: f32) -> f32 {
    base_x + stage_scroll_x * factor
}

/// Vertical moving platform (matches C++ `Platform` in `GRAPHICS_Sprites_Move.cpp`).
#[derive(Component)]
pub struct MovingPlatform {
    pub start_y:   f32,
    pub end_y:     f32,
    pub direction: i32, // 1 = moving toward end_y, 0 = moving toward start_y
    pub width:     f32,
    pub height:    f32,
    pub delta_y:   f32,
}

// ── Resources ─────────────────────────────────────────────────────────────────

/// O(1) lookup from tile grid `(x, y)` to spawned tile `Entity`.
#[derive(Resource)]
pub struct TileGrid {
    pub entities: Box<[[Option<Entity>; STAGE_ROWS]; STAGE_COLS]>,
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            entities: Box::new([[None; STAGE_ROWS]; STAGE_COLS]),
        }
    }
}

/// Shared tile animation counters matching C++ `ME.Frame_3`, `ME.Frame_4`, `ME.Frame_6`.
#[derive(Resource)]
pub struct TileAnimClock {
    pub timer:   Timer,
    pub frame_3: i32,
    pub frame_4: i32,
    pub frame_6: i32,
}

impl Default for TileAnimClock {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.13, TimerMode::Repeating),
            frame_3: 0,
            frame_4: 0,
            frame_6: 0,
        }
    }
}

#[derive(Resource)]
struct StageTransitionTimer(Timer);

#[derive(Component)]
struct TransitionBannerRoot;

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct PauseOptionText(usize);

#[derive(Resource, Default)]
struct PauseSelection(usize);

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn tile_source_rect(tile_id: i32) -> Rect {
    let col = tile_id % TILES_PER_ROW;
    let row = tile_id / TILES_PER_ROW;
    let src_x = col as f32 * TILE_W;
    let src_y = row as f32 * TILE_H;
    Rect::new(src_x, src_y, src_x + TILE_W, src_y + TILE_H)
}

pub fn is_animated_tile(tile_id: i32, tile_info: &TileSheetInfo) -> bool {
    if tile_id <= 0 || (tile_id as usize) >= 2320 {
        return false;
    }
    let id = tile_id as usize;
    tile_info.coin[id] != 0
        || tile_info.coin_block[id] != 0
        || tile_info.power_up_block[id] != 0
        || (10..=13).contains(&tile_id)
        || (6..=9).contains(&tile_id)
        || (46..=49).contains(&tile_id)
        || (250..=252).contains(&tile_id)
}

/// Computes the source rect for a tile including C++ animation and stage colour-theme offsets
/// (matches `src/GRAPHICS_Tiles_Draw.cpp:44-96`).
pub fn animated_tile_source_rect(
    tile_id:   i32,
    tile_type: i32,
    tile_info: &TileSheetInfo,
    frame_3:   i32,
    frame_4:   i32,
    frame_6:   i32,
) -> Rect {
    let mut sx = (tile_id % TILES_PER_ROW) * 16;
    let mut sy = (tile_id / TILES_PER_ROW) * 16;

    if tile_id > 0 && (tile_id as usize) < 2320 {
        let uid = tile_id as usize;

        if tile_info.coin[uid] != 0 {
            sx += (2 * frame_6 - 3) * 16;
            sy += 52 * 16;
        }
        if tile_info.coin_block[uid] != 0 {
            sx += (3 * frame_6) * 16;
            sy -= 2 * 16;
        }
        if tile_info.power_up_block[uid] != 0 {
            sx += (3 * frame_6) * 16;
        }
        // Little flames
        if (10..=13).contains(&tile_id) {
            sx += (4 * frame_4 + 2) * 16;
            sy += 57 * 16;
        }
        // Big flames
        if (6..=9).contains(&tile_id) || (46..=49).contains(&tile_id) {
            sx += (4 * frame_3 - 6) * 16;
            sy += 55 * 16;
        }
        // Lava
        if (250..=252).contains(&tile_id) {
            sx += (3 * frame_3 - 10) * 16;
            sy += 48 * 16;
        }
        // Colour-changing stage theme tiles (underground, castle, ice, etc.)
        if tile_info.colour_changing[uid] != 0 && tile_type > 0 {
            sy += tile_type * 10 * 16;
        }
    }

    let x = (sx as f32).clamp(0.0, TILESHEET_W - TILE_W);
    let y = (sy as f32).clamp(0.0, TILESHEET_H - TILE_H);
    Rect::new(x, y, x + TILE_W, y + TILE_H)
}

/// Mutates tile `(tx, ty)` in both `LevelData.stage.array` and the live ECS `Sprite`/`TileGrid`.
#[allow(clippy::too_many_arguments)]
pub fn set_tile<F: bevy::ecs::query::QueryFilter>(
    commands:  &mut Commands,
    tile_grid: &mut TileGrid,
    level:     &mut LevelData,
    tile_q:    &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), F>,
    assets:    &GameAssets,
    tx:        usize,
    ty:        usize,
    new_tile_id: i32,
) {
    if tx >= STAGE_COLS || ty >= STAGE_ROWS {
        return;
    }
    level.stage.array[tx][ty] = new_tile_id;

    if new_tile_id == 0 {
        if let Some(entity) = tile_grid.entities[tx][ty].take() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let rect = animated_tile_source_rect(
        new_tile_id,
        level.stage.tile_type,
        &level.tile_info,
        0,
        0,
        0,
    );

    if let Some(entity) = tile_grid.entities[tx][ty] {
        if let Ok((mut tile, mut sprite, mut vis)) = tile_q.get_mut(entity) {
            tile.tile_id = new_tile_id;
            sprite.rect = Some(rect);
            *vis = Visibility::Inherited;
            if !is_animated_tile(new_tile_id, &level.tile_info) {
                commands.entity(entity).remove::<AnimatedTile>();
            }
            return;
        }
    }

    let px = tx as f32 * TILE_W;
    let py = -(ty as f32 * TILE_H);
    let mut ec = commands.spawn((
        GameEntity,
        Tile { tile_id: new_tile_id },
        Sprite {
            image: assets.tiles.clone(),
            rect: Some(rect),
            custom_size: Some(Vec2::new(TILE_W, TILE_H)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(px, py, 0.0),
    ));
    if is_animated_tile(new_tile_id, &level.tile_info) {
        ec.insert(AnimatedTile);
    }
    tile_grid.entities[tx][ty] = Some(ec.id());
}

/// Computes the trimmed stage width in pixels from the rightmost non-empty tile column
/// (matches C++ `CONTENT_Stages_Load_Write.cpp:329-344`).
pub fn compute_stage_width_pixels(stage: &crate::level::ImportStage, min_width: f32) -> f32 {
    let mut last_col = 0usize;
    for x in 0..STAGE_COLS {
        for y in 0..STAGE_ROWS {
            if stage.array[x][y] != 0 {
                last_col = x;
                break;
            }
        }
    }
    ((last_col + 1) as f32 * TILE_W).max(min_width)
}

/// Maps an animated transition tile ID to its next frame ID
/// (matches C++ `SwitchTileToNextFrame` in `src/GRAPHICS_Tiles_Animations.cpp`).
pub fn next_tile_switch_id(tile_id: i32) -> Option<i32> {
    match tile_id {
        // Crumbling ground: 568->570->572->0, 569->571->573->0
        568..=571 => Some(tile_id + 2),
        572 | 573 => Some(0),

        // 1-coin block & warp stone blink -> spent block (587..589 / 627..629)
        563..=565 | 603..=605 => Some(tile_id + 24),

        // Breakable block crack frames: 504..506->507..509->0, 544..546->547..549->0
        504..=506 | 544..=546 => Some(tile_id + 3),
        507..=509 | 547..=549 => Some(0),

        // 5-coin multi-hit block blink frames -> next interactive coin-block state
        643..=645
        | 683..=685
        | 649..=651
        | 689..=691
        | 655..=657
        | 695..=697
        | 661..=663
        | 701..=703 => Some(tile_id + 3),

        // 5-coin block final hit blink -> spent block
        667 => Some(587),
        668 => Some(588),
        669 => Some(589),
        707 => Some(627),
        708 => Some(628),
        709 => Some(629),

        _ => None,
    }
}

/// Attaches a `TimedTileSwitch` component to tile `(tx, ty)` if its current tile ID has a follow-up frame.
pub fn schedule_tile_switch(
    commands:  &mut Commands,
    tile_grid: &TileGrid,
    tx:        usize,
    ty:        usize,
    delay:     f32,
) {
    if tx >= STAGE_COLS || ty >= STAGE_ROWS {
        return;
    }
    if let Some(ent) = tile_grid.entities[tx][ty] {
        commands.entity(ent).insert(TimedTileSwitch { tx, ty, timer: delay });
    }
}

/// Triggers a crumbling ground tile (`566` or `567`) when the player steps on it.
pub fn trigger_crumble_tile<F: bevy::ecs::query::QueryFilter>(
    commands:  &mut Commands,
    tile_grid: &mut TileGrid,
    level:     &mut LevelData,
    tile_q:    &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), F>,
    assets:    &GameAssets,
    tx:        usize,
    ty:        usize,
) {
    if tx >= STAGE_COLS || ty >= STAGE_ROWS {
        return;
    }
    let next_id = match level.stage.array[tx][ty] {
        566 => 568,
        567 => 569,
        _   => return,
    };
    set_tile(commands, tile_grid, level, tile_q, assets, tx, ty, next_id);
    schedule_tile_switch(commands, tile_grid, tx, ty, 0.14);
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileGrid>()
           .init_resource::<TileAnimClock>()
           .init_resource::<GamePaused>()
           .init_resource::<PauseSelection>()
           .add_event::<SpawnDropStoneEvent>()
           .add_systems(OnEnter(GameState::StageTransition), enter_stage_transition)
           .add_systems(
               Update,
               tick_stage_transition.run_if(in_state(GameState::StageTransition)),
           )
           .add_systems(OnExit(GameState::StageTransition), exit_stage_transition)
           .add_systems(
               OnEnter(GameState::InGame),
               (clear_pause_state, spawn_tiles, spawn_parallax_background, spawn_platform, reset_camera),
           )
           .add_systems(
               FixedUpdate,
               (
                   move_platforms,
                   handle_spawn_drop_stone,
                   move_drop_stones,
                   tick_timed_tile_switches,
               )
                   .chain()
                   .run_if(in_state(GameState::InGame).and(not_paused)),
           )
           .add_systems(
               Update,
               (
                   toggle_pause,
                   pause_menu_input,
                   (animate_tiles, camera_follow, update_parallax, ingame_shortcuts)
                       .chain()
                       .run_if(not_paused),
               )
                   .run_if(in_state(GameState::InGame)),
           )
           .add_systems(OnExit(GameState::InGame), (despawn_pause_overlay, despawn_game_entities));
    }
}

fn clear_pause_state(mut paused: ResMut<GamePaused>, mut sel: ResMut<PauseSelection>) {
    paused.0 = false;
    sel.0 = 0;
}

// ── Stage transition systems ──────────────────────────────────────────────────

fn enter_stage_transition(
    mut commands: Commands,
    mut level:    ResMut<LevelData>,
    module_bank:  Option<Res<crate::random_gen::RandomModuleBank>>,
    hud:          Res<HudState>,
    mut next:     ResMut<NextState<GameState>>,
) {
    let target_stage = hud.stage.clamp(1, MAX_SUBSTAGE_INDEX);
    let checkpoint = level.checkpoint;
    if let Err(err) = level.load_stage_number(target_stage, checkpoint, module_bank.as_deref()) {
        eprintln!("Failed to load stage {}: {}", target_stage, err);
        next.set(GameState::GameOver);
        return;
    }

    let warping = level.is_warping;
    level.is_warping = false;

    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.12)));
    commands.insert_resource(StageTransitionTimer(Timer::from_seconds(
        if warping { 1.1 } else { 0.75 },
        TimerMode::Once,
    )));

    let headline = if warping {
        format!("WARPING!!!  STAGE {:02}", target_stage)
    } else if target_stage > MAX_CLASSIC_STAGE {
        format!("BONUS STAGE {}", target_stage - MAX_CLASSIC_STAGE)
    } else {
        format!("STAGE {:02}", target_stage)
    };
    let stage_name = stage_display_name(&level.stage.name, target_stage);

    commands
        .spawn((
            TransitionBannerRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap:         Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.04, 0.12)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(headline),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));
            parent.spawn((
                Text::new(format!("{} — {}", level.pack_name(), stage_name)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.85, 0.85, 0.95)),
            ));
            parent.spawn((
                Text::new(format!("LIVES: {}", hud.lives)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn stage_display_name(raw: &[u8; 100], stage_num: usize) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    let name = String::from_utf8_lossy(&raw[..end]).trim().to_string();
    if name.eq_ignore_ascii_case("noname") || name.is_empty() {
        format!("Stage {}", stage_num)
    } else {
        name
    }
}

fn tick_stage_transition(
    time:      Res<Time>,
    keys:      Res<ButtonInput<KeyCode>>,
    mut timer: ResMut<StageTransitionTimer>,
    mut next:  ResMut<NextState<GameState>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() || keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        next.set(GameState::InGame);
    }
}

fn exit_stage_transition(mut commands: Commands, q: Query<Entity, With<TransitionBannerRoot>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

// ── In-game tilemap, platforms, animations & crumbling ────────────────────────

fn reset_camera(
    level:   Res<LevelData>,
    win_q:   Query<&Window>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
) {
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };
    let viewport_w = win_q.get_single().map(|w| w.width()).unwrap_or(800.0);
    let start = crate::player::player_start_position(&level);
    cam_tf.translation.x = clamp_camera_x(start.x, level.stage_width_px, viewport_w);
    cam_tf.translation.y = -240.0;
}

pub fn clamp_camera_x(x: f32, stage_width_px: f32, viewport_w: f32) -> f32 {
    let half = viewport_w * 0.5;
    let max_x = (stage_width_px - half).max(half);
    x.clamp(half, max_x)
}

fn spawn_tiles(
    mut commands:  Commands,
    mut tile_grid: ResMut<TileGrid>,
    level:         Res<LevelData>,
    assets:        Res<GameAssets>,
) {
    *tile_grid = TileGrid::default();

    let bg_color = match level.stage.background_colour {
        0 => Color::srgb(0.4, 0.6, 1.0), // Blue sky (outdoors)
        _ => Color::BLACK,               // Indoors / underground
    };
    commands.insert_resource(ClearColor(bg_color));

    for x in 0..STAGE_COLS {
        for y in 0..STAGE_ROWS {
            let tile_id = level.stage.array[x][y];
            if tile_id == 0 { continue; }

            let px = x as f32 * TILE_W;
            let py = -(y as f32 * TILE_H);
            let rect = animated_tile_source_rect(
                tile_id,
                level.stage.tile_type,
                &level.tile_info,
                0,
                0,
                0,
            );

            // Warp stones are invisible until bumped (matches C++ GRAPHICS_Tiles_Draw.cpp:109)
            let is_hidden_warp = tile_id > 0
                && (tile_id as usize) < 2320
                && level.tile_info.warp_stone[tile_id as usize] != 0;

            let mut ec = commands.spawn((
                GameEntity,
                Tile { tile_id },
                Sprite {
                    image: assets.tiles.clone(),
                    rect: Some(rect),
                    custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                if is_hidden_warp { Visibility::Hidden } else { Visibility::Inherited },
                Transform::from_xyz(px, py, 0.0),
            ));

            if is_animated_tile(tile_id, &level.tile_info) {
                ec.insert(AnimatedTile);
            }
            tile_grid.entities[x][y] = Some(ec.id());
        }
    }
}

/// Deterministic LCG for placing stage parallax clouds & tubes without external RNG deps.
fn bg_lcg(seed: &mut u64) -> u32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 32) as u32
}

fn spawn_parallax_background(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    let mut seed = (level.stage_number as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(42);

    if level.stage.background_colour == 0 {
        // Outdoors: 12 scrolling clouds (tile 104: 5x2 tiles = 80x32 px)
        let cloud_rect = Rect::new(
            (104 % TILES_PER_ROW) as f32 * TILE_W,
            (104 / TILES_PER_ROW) as f32 * TILE_H,
            ((104 % TILES_PER_ROW) + 5) as f32 * TILE_W,
            ((104 / TILES_PER_ROW) + 2) as f32 * TILE_H,
        );
        for _ in 0..12 {
            let cx = (bg_lcg(&mut seed) % 2400) as f32;
            let cy = -((32 + (bg_lcg(&mut seed) % 118)) as f32);
            commands.spawn((
                GameEntity,
                ParallaxLayer { base_x: cx, factor: 0.5 },
                Sprite {
                    image: assets.tiles.clone(),
                    rect: Some(cloud_rect),
                    custom_size: Some(Vec2::new(80.0, 32.0)),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                Transform::from_xyz(cx, cy, -2.0),
            ));
        }

        // Outdoors: 4 semi-transparent background tubes (tile 514: 6x3 tiles = 96x48 px, 2 stacked)
        // Matches C++ STAGE_Draw.cpp:121-198
        let tube_rect = Rect::new(
            (514 % TILES_PER_ROW) as f32 * TILE_W,
            (514 / TILES_PER_ROW) as f32 * TILE_H,
            ((514 % TILES_PER_ROW) + 6) as f32 * TILE_W,
            ((514 / TILES_PER_ROW) + 3) as f32 * TILE_H,
        );
        let tube_color = Color::srgba(1.0, 1.0, 1.0, 100.0 / 255.0);
        for i in 0..4 {
            let tx = (i * 500 + (bg_lcg(&mut seed) % 360)) as f32;
            let has_fire = (bg_lcg(&mut seed) % 2) == 1;
            for stack in 0..2 {
                let ty = -(440.0 - (stack as f32 * 48.0));
                commands.spawn((
                    GameEntity,
                    ParallaxLayer { base_x: tx, factor: 0.5 },
                    Sprite {
                        image: assets.tiles.clone(),
                        rect: Some(tube_rect),
                        custom_size: Some(Vec2::new(96.0, 48.0)),
                        color: tube_color,
                        anchor: bevy::sprite::Anchor::TopLeft,
                        ..default()
                    },
                    Transform::from_xyz(tx, ty, -1.5),
                ));
            }
            if has_fire {
                let fire_top_y = -(440.0 - 48.0 - 32.0);
                for row in 0..2 {
                    for col in 0..4 {
                        let fid = if row == 0 { 6 + col } else { 46 + col };
                        let fx = tx + (col + 1) as f32 * TILE_W;
                        let fy = fire_top_y - row as f32 * TILE_H;
                        let rect = animated_tile_source_rect(fid, level.stage.tile_type, &level.tile_info, 0, 0, 0);
                        commands.spawn((
                            GameEntity,
                            Tile { tile_id: fid },
                            AnimatedTile,
                            ParallaxLayer { base_x: fx, factor: 0.5 },
                            Sprite {
                                image: assets.tiles.clone(),
                                rect: Some(rect),
                                custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.55),
                                anchor: bevy::sprite::Anchor::TopLeft,
                                ..default()
                            },
                            Transform::from_xyz(fx, fy, -1.4),
                        ));
                    }
                }
            }
        }
    } else {
        // Indoors: semi-transparent repeating 2x2 brick backdrop (tile 672, matches STAGE_Draw.cpp:204-242)
        let brick_rect = Rect::new(
            (672 % TILES_PER_ROW) as f32 * TILE_W,
            (672 / TILES_PER_ROW) as f32 * TILE_H,
            ((672 % TILES_PER_ROW) + 2) as f32 * TILE_W,
            ((672 / TILES_PER_ROW) + 2) as f32 * TILE_H,
        );
        let brick_color = Color::srgba(1.0, 1.0, 1.0, 80.0 / 255.0);
        let max_cols = ((level.stage_width_px * 0.65) / 32.0).ceil() as usize + 16;
        for bx_idx in 0..max_cols.min(84) {
            let bx = bx_idx as f32 * 32.0;
            for by_idx in 4..15 {
                let by = -(by_idx as f32 * 32.0);
                commands.spawn((
                    GameEntity,
                    ParallaxLayer { base_x: bx, factor: 0.5 },
                    Sprite {
                        image: assets.tiles.clone(),
                        rect: Some(brick_rect),
                        custom_size: Some(Vec2::new(32.0, 32.0)),
                        color: brick_color,
                        anchor: bevy::sprite::Anchor::TopLeft,
                        ..default()
                    },
                    Transform::from_xyz(bx, by, -2.0),
                ));
            }
        }
    }
}

fn update_parallax(
    win_q: Query<&Window>,
    cam_q: Query<&Transform, (With<PersistentCamera>, Without<ParallaxLayer>)>,
    mut q: Query<(&mut Transform, &ParallaxLayer)>,
) {
    let Ok(cam_tf) = cam_q.get_single() else { return };
    let viewport_w = win_q.get_single().map(|w| w.width()).unwrap_or(800.0);
    let stage_scroll_x = (cam_tf.translation.x - viewport_w * 0.5).max(0.0);

    for (mut tf, layer) in &mut q {
        tf.translation.x = parallax_world_x(layer.base_x, stage_scroll_x, layer.factor);
    }
}

fn spawn_platform(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    if level.stage.platform_in_use == 0 {
        return;
    }

    let start_x = level.stage.platform_x as f32 * TILE_W;
    let start_y = -(level.stage.platform_y as f32 * TILE_H);
    let path_px = level.stage.platform_path_length.max(1) as f32 * TILE_H;
    // In C++ End_Y = Start_Y - path_length (higher on screen) => higher Y in Bevy Y-up
    let end_y = start_y + path_px;

    // Platform sprite rect in Tiles.png from C++ GRAPHICS_Sprites.cpp:87-90
    const PLAT_W: f32 = 144.0;
    const PLAT_H: f32 = 16.0;

    commands.spawn((
        GameEntity,
        MovingPlatform {
            start_y,
            end_y,
            direction: 1,
            width: PLAT_W,
            height: PLAT_H,
            delta_y: 0.0,
        },
        Sprite {
            image: assets.tiles.clone(),
            rect: Some(Rect::new(192.0, 240.0, 192.0 + PLAT_W, 240.0 + PLAT_H)),
            custom_size: Some(Vec2::new(PLAT_W, PLAT_H)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(start_x, start_y, 0.4),
    ));
}

fn move_platforms(mut q: Query<(&mut Transform, &mut MovingPlatform)>) {
    const SPEED: f32 = 1.25;
    for (mut tf, mut plat) in &mut q {
        let old_y = tf.translation.y;
        let target = if plat.direction == 1 { plat.end_y } else { plat.start_y };
        let diff = target - tf.translation.y;
        if diff.abs() <= SPEED {
            tf.translation.y = target;
            plat.direction = 1 - plat.direction;
        } else {
            tf.translation.y += diff.signum() * SPEED;
        }
        plat.delta_y = tf.translation.y - old_y;
    }
}

fn animate_tiles(
    time:      Res<Time>,
    mut clock: ResMut<TileAnimClock>,
    level:     Res<LevelData>,
    mut q:     Query<(&Tile, &mut Sprite), With<AnimatedTile>>,
) {
    clock.timer.tick(time.delta());
    if !clock.timer.just_finished() {
        return;
    }

    clock.frame_3 = (clock.frame_3 + 1) % 3;
    clock.frame_4 = (clock.frame_4 + 1) % 4;
    clock.frame_6 = (clock.frame_6 + 1) % 6;

    for (tile, mut sprite) in &mut q {
        sprite.rect = Some(animated_tile_source_rect(
            tile.tile_id,
            level.stage.tile_type,
            &level.tile_info,
            clock.frame_3,
            clock.frame_4,
            clock.frame_6,
        ));
    }
}

fn handle_spawn_drop_stone(
    mut commands: Commands,
    mut events:   EventReader<SpawnDropStoneEvent>,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    // Matches C++ GRAPHICS_Sprites.cpp:95-98 & GRAPHICS_Sprites_Draw.cpp:166-168
    const STONE_W: f32 = 48.0;
    const STONE_H: f32 = 32.0;
    let src_y = (224.0 + level.stage.tile_type.max(0) as f32 * 10.0 * TILE_H)
        .clamp(0.0, TILESHEET_H - STONE_H);

    for ev in events.read() {
        commands.spawn((
            GameEntity,
            DropStone {
                vy: 0.0,
                w: STONE_W,
                h: STONE_H,
                landed: false,
            },
            Sprite {
                image: assets.tiles.clone(),
                rect: Some(Rect::new(384.0, src_y, 384.0 + STONE_W, src_y + STONE_H)),
                custom_size: Some(Vec2::new(STONE_W, STONE_H)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ev.x, ev.y, 0.45),
        ));
    }
}

fn move_drop_stones(
    mut commands: Commands,
    level:        Res<LevelData>,
    mut q:        Query<(Entity, &mut Transform, &mut DropStone)>,
) {
    const GRAVITY: f32 = 0.6;
    const TERMINAL_VEL: f32 = 15.0;

    for (ent, mut tf, mut stone) in &mut q {
        stone.vy = (stone.vy - GRAVITY).max(-TERMINAL_VEL);
        tf.translation.y += stone.vy;
        stone.landed = false;

        let px = tf.translation.x;
        let py = -tf.translation.y;
        let bottom_ty = ((py + stone.h) / TILE_H).floor() as i32;
        let left_tx = ((px + 1.0) / TILE_W).floor() as i32;
        let mid_tx = ((px + stone.w * 0.5) / TILE_W).floor() as i32;
        let right_tx = ((px + stone.w - 1.0) / TILE_W).floor() as i32;

        if crate::player::is_solid(left_tx, bottom_ty, &level)
            || crate::player::is_solid(mid_tx, bottom_ty, &level)
            || crate::player::is_solid(right_tx, bottom_ty, &level)
        {
            tf.translation.y = -(bottom_ty as f32 * TILE_H - stone.h);
            stone.vy = 0.0;
            stone.landed = true;
        }

        if tf.translation.y < -600.0 {
            commands.entity(ent).despawn();
        }
    }
}

fn tick_timed_tile_switches(
    mut commands:  Commands,
    time:          Res<Time<Fixed>>,
    mut level:     ResMut<LevelData>,
    mut tile_grid: ResMut<TileGrid>,
    assets:        Res<GameAssets>,
    mut switch_q:  Query<(Entity, &mut TimedTileSwitch)>,
    mut tile_q:    Query<(&mut Tile, &mut Sprite, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    let mut updates: Vec<(Entity, usize, usize, i32)> = Vec::new();
    let mut finished: Vec<Entity> = Vec::new();

    for (ent, mut sw) in &mut switch_q {
        sw.timer -= dt;
        if sw.timer <= 0.0 {
            let cur = level.stage.array[sw.tx][sw.ty];
            if let Some(next_id) = next_tile_switch_id(cur) {
                sw.timer = 0.14;
                updates.push((ent, sw.tx, sw.ty, next_id));
            } else {
                finished.push(ent);
            }
        }
    }

    for (ent, tx, ty, next_id) in updates {
        set_tile(&mut commands, &mut tile_grid, &mut level, &mut tile_q, &assets, tx, ty, next_id);
        if next_id != 0 && next_tile_switch_id(next_id).is_none() {
            commands.entity(ent).remove::<TimedTileSwitch>();
        }
    }

    for ent in finished {
        commands.entity(ent).remove::<TimedTileSwitch>();
    }
}

/// Smooth camera follow — tracks the player entity's X position within trimmed stage bounds.
fn camera_follow(
    level:    Res<LevelData>,
    win_q:    Query<&Window>,
    player_q: Query<&Transform, (With<crate::player::Player>, Without<PersistentCamera>)>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };
    let viewport_w = win_q.get_single().map(|w| w.width()).unwrap_or(800.0);

    let target_x = clamp_camera_x(player_tf.translation.x, level.stage_width_px, viewport_w);
    let target_y = -240.0;
    cam_tf.translation.x += (target_x - cam_tf.translation.x) * 0.12;
    cam_tf.translation.y += (target_y - cam_tf.translation.y) * 0.12;
}

/// Convenience shortcuts matching C++ LOOP_Gameloop: F2 reload, F3/F4 next/prev stage.
fn ingame_shortcuts(
    keys:      Res<ButtonInput<KeyCode>>,
    mut hud:   ResMut<HudState>,
    mut level: ResMut<LevelData>,
    mut next:  ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::F2) {
        level.checkpoint = 0;
        next.set(GameState::StageTransition);
    } else if keys.just_pressed(KeyCode::F3) {
        hud.stage = if hud.stage >= MAX_CLASSIC_STAGE { 1 } else { hud.stage + 1 };
        level.checkpoint = 0;
        level.sub_stage_entry_stage = None;
        next.set(GameState::StageTransition);
    } else if keys.just_pressed(KeyCode::F4) {
        hud.stage = if hud.stage <= 1 { MAX_CLASSIC_STAGE } else { hud.stage - 1 };
        level.checkpoint = 0;
        level.sub_stage_entry_stage = None;
        next.set(GameState::StageTransition);
    }
}

// ── Pause Overlay ─────────────────────────────────────────────────────────────

const PAUSE_OPTIONS: [&str; 3] = ["RESUME", "RESTART STAGE", "MAIN MENU"];

fn toggle_pause(
    mut commands: Commands,
    keys:         Res<ButtonInput<KeyCode>>,
    gamepads:     Query<&Gamepad>,
    mut paused:   ResMut<GamePaused>,
    mut sel:      ResMut<PauseSelection>,
    mut sfx:      EventWriter<SfxEvent>,
    overlay_q:    Query<Entity, With<PauseMenuRoot>>,
) {
    if keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::KeyP)
        || crate::input::any_gamepad_just_pause(&gamepads)
    {
        paused.0 = !paused.0;
        sel.0 = 0;
        sfx.send(SfxEvent(SfxKind::Click));
        if paused.0 {
            spawn_pause_overlay(&mut commands);
        } else {
            for e in &overlay_q {
                commands.entity(e).despawn_recursive();
            }
        }
    }
}

fn spawn_pause_overlay(commands: &mut Commands) {
    commands
        .spawn((
            PauseMenuRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap:         Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.08, 0.72)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));
            for (i, label) in PAUSE_OPTIONS.iter().enumerate() {
                parent.spawn((
                    PauseOptionText(i),
                    Text::new(if i == 0 { format!("> {}", label) } else { (*label).to_string() }),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(if i == 0 { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }),
                ));
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn pause_menu_input(
    mut commands: Commands,
    keys:         Res<ButtonInput<KeyCode>>,
    gamepads:     Query<&Gamepad>,
    mut paused:   ResMut<GamePaused>,
    mut sel:      ResMut<PauseSelection>,
    mut level:    ResMut<LevelData>,
    mut next:     ResMut<NextState<GameState>>,
    mut sfx:      EventWriter<SfxEvent>,
    overlay_q:    Query<Entity, With<PauseMenuRoot>>,
    mut text_q:   Query<(&PauseOptionText, &mut Text, &mut TextColor)>,
) {
    if !paused.0 {
        return;
    }

    let mut changed = false;
    if keys.just_pressed(KeyCode::ArrowDown) || crate::input::any_gamepad_nav_down(&gamepads) {
        sel.0 = (sel.0 + 1) % PAUSE_OPTIONS.len();
        changed = true;
    } else if keys.just_pressed(KeyCode::ArrowUp) || crate::input::any_gamepad_nav_up(&gamepads) {
        sel.0 = (sel.0 + PAUSE_OPTIONS.len() - 1) % PAUSE_OPTIONS.len();
        changed = true;
    }

    if changed {
        sfx.send(SfxEvent(SfxKind::Click));
        for (opt, mut text, mut color) in &mut text_q {
            let label = PAUSE_OPTIONS[opt.0];
            if opt.0 == sel.0 {
                **text = format!("> {}", label);
                color.0 = Color::srgb(1.0, 0.9, 0.3);
            } else {
                **text = label.to_string();
                color.0 = Color::WHITE;
            }
        }
    }

    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space)
        || crate::input::any_gamepad_confirm(&gamepads)
    {
        sfx.send(SfxEvent(SfxKind::Click));
        paused.0 = false;
        for e in &overlay_q {
            commands.entity(e).despawn_recursive();
        }
        match sel.0 {
            0 => {} // Resume
            1 => {
                level.checkpoint = 0;
                next.set(GameState::StageTransition);
            }
            _ => {
                next.set(GameState::Menu);
            }
        }
    }
}

fn despawn_pause_overlay(
    mut commands: Commands,
    mut paused:   ResMut<GamePaused>,
    q:            Query<Entity, With<PauseMenuRoot>>,
) {
    paused.0 = false;
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn despawn_game_entities(
    mut commands:  Commands,
    mut tile_grid: ResMut<TileGrid>,
    query:         Query<Entity, With<GameEntity>>,
) {
    *tile_grid = TileGrid::default();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn animated_tile_rects_stay_within_tilesheet_bounds() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/Tilesheetinfo.tsi");
        let info = crate::tile_properties::load_tile_properties(path.to_str().unwrap()).unwrap();

        for tile_id in 1..2320i32 {
            for tile_type in 0..=2 {
                for f in 0..6 {
                    let r = animated_tile_source_rect(tile_id, tile_type, &info, f % 3, f % 4, f);
                    assert!(r.min.x >= 0.0 && r.max.x <= TILESHEET_W);
                    assert!(r.min.y >= 0.0 && r.max.y <= TILESHEET_H);
                    assert_eq!(r.width(), TILE_W);
                    assert_eq!(r.height(), TILE_H);
                }
            }
        }
    }

    #[test]
    fn five_coin_block_and_breakable_switch_chains_match_cpp() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/Tilesheetinfo.tsi");
        let info = crate::tile_properties::load_tile_properties(path.to_str().unwrap()).unwrap();

        // 5-coin block starts at 640 (top-left) and steps +3 on bump -> +3 on timer 4 times,
        // remaining a coin_block for 5 total hits before becoming spent (587).
        let mut id = 640i32;
        for hit in 0..5 {
            assert!(
                info.coin_block[id as usize] != 0 || info.coin_block_helmet[id as usize] != 0,
                "hit {} expected coin_block flag on tile {}",
                hit,
                id
            );
            id += 3; // player bump
            id = next_tile_switch_id(id).expect("expected timed switch after bump");
        }
        assert_eq!(id, 587, "5-coin block should settle on spent tile 587 after 5 hits");
        assert_eq!(next_tile_switch_id(587), None);

        // Breakable block: 501 -> bump(+3)=504 -> 507 -> 0
        assert_eq!(next_tile_switch_id(504), Some(507));
        assert_eq!(next_tile_switch_id(507), Some(0));

        // Crumbling ground: 568 -> 570 -> 572 -> 0
        assert_eq!(next_tile_switch_id(568), Some(570));
        assert_eq!(next_tile_switch_id(570), Some(572));
        assert_eq!(next_tile_switch_id(572), Some(0));
    }

    #[test]
    fn stage_width_trims_short_substages_for_camera_clamp() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/stages/classic.lvl");
        let stage1 = crate::level::load_stage(path.to_str().unwrap(), 1).unwrap();
        let substage = crate::level::load_stage(path.to_str().unwrap(), 34).unwrap();

        let w1 = compute_stage_width_pixels(&stage1, 800.0);
        let ws = compute_stage_width_pixels(&substage, 800.0);
        assert!(w1 >= 800.0 && w1 <= STAGE_COLS as f32 * TILE_W);
        assert!(ws >= 800.0 && ws <= w1);

        // On an 800px-wide stage with an 800px viewport, camera stays fixed at x=400.
        assert_eq!(clamp_camera_x(0.0, 800.0, 800.0), 400.0);
        assert_eq!(clamp_camera_x(999.0, 800.0, 800.0), 400.0);
        // On a 2000px-wide stage, camera clamps between 400 and 1600.
        assert_eq!(clamp_camera_x(100.0, 2000.0, 800.0), 400.0);
        assert_eq!(clamp_camera_x(1900.0, 2000.0, 800.0), 1600.0);
    }

    #[test]
    fn parallax_scrolls_at_half_camera_speed() {
        let base_x = 120.0;
        let scroll = 200.0;
        let world_x = parallax_world_x(base_x, scroll, 0.5);
        // Apparent screen X = world_x - scroll = base_x - scroll/2
        let screen_x = world_x - scroll;
        assert_eq!(screen_x, base_x - 100.0);
    }
}
