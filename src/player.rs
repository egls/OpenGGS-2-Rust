use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, MAX_CLASSIC_STAGE};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
use crate::powerup::{FireBulletEvent, SpawnCoinPopEvent, SpawnPowerUpEvent};
use crate::states::{not_paused, GameState};
use crate::tilemap::{
    next_tile_switch_id, schedule_tile_switch, set_tile, trigger_crumble_tile, DropStone,
    GameEntity, MovingPlatform, SpawnDropStoneEvent, Tile, TileGrid, STAGE_COLS, STAGE_ROWS,
    TILES_PER_ROW,
};

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[derive(Component, Default)]
pub struct OnGround(pub bool);

#[derive(Component)]
pub struct PlayerState {
    pub alive:        bool,
    pub death_timer:  f32,
    pub invuln_timer: f32,
    pub morph_timer:  f32,
    pub warp_timer:   f32,
    pub warp_target:  Option<usize>,
    pub coyote_timer: f32,
    pub furthest_x:   f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            alive: true,
            death_timer: 0.0,
            invuln_timer: 1.0, // brief spawn protection
            morph_timer: 0.0,
            warp_timer: 0.0,
            warp_target: None,
            coyote_timer: 0.0,
            furthest_x: 0.0,
        }
    }
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { #[default] Right, Left }

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance { #[default] Stand, Walk1, Walk2, Jump, Turn }

#[derive(Component, Default)]
pub struct PlayerAnimation {
    pub direction:  Direction,
    pub stance:     Stance,
    pub walk_timer: u32,
}

// ── Events & Input Buffer ─────────────────────────────────────────────────────

#[derive(Event)]
pub struct KillPlayerEvent;

#[derive(Resource, Default)]
struct PlayerInputBuffer {
    jump_requested:  bool,
    shoot_requested: bool,
}

// ── Frame data ────────────────────────────────────────────────────────────────

/// (src_x, src_y, src_w, src_h) in pixels on the spritesheet
pub type Frame = (i32, i32, u32, u32);

#[derive(Resource)]
pub struct PlayerFrames(pub Vec<Frame>);

/// Exact C++ Player.txt layout (23 frames, 0-indexed):
/// Row 0 (0..4):   Normal Right -> Stand, Walk1, Walk2, Jump, Turn
/// Row 1 (5..9):   Normal Left  -> Stand, Walk1, Walk2, Jump, Turn
/// Row 2 (10..14): Punk Right   -> Stand, Walk1, Walk2, Jump, Turn
/// Row 3 (15..19): Punk Left    -> Stand, Walk1, Walk2, Jump, Turn
/// Row 4 (20..22): Morph1 (20), Morph2 (21), Dead (22)
pub fn frame_for_state(
    frames: &[Frame],
    stance: Stance,
    dir: Direction,
    punk: bool,
    alive: bool,
    morph_timer: f32,
) -> Option<Frame> {
    if !alive {
        return frames.get(22).copied();
    }
    if morph_timer > 0.0 {
        let idx = if ((morph_timer * 20.0) as i32) % 2 == 0 { 20 } else { 21 };
        return frames.get(idx).copied();
    }
    let mode_offset = if punk { 10 } else { 0 };
    let dir_offset = if dir == Direction::Right { 0 } else { 5 };
    let stance_offset = match stance {
        Stance::Stand => 0,
        Stance::Walk1 => 1,
        Stance::Walk2 => 2,
        Stance::Jump  => 3,
        Stance::Turn  => 4,
    };
    frames.get(mode_offset + dir_offset + stance_offset).copied()
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KillPlayerEvent>()
           .init_resource::<PlayerInputBuffer>()
           .add_systems(OnEnter(GameState::InGame), (reset_input_buffer, spawn_player))
           .add_systems(
               FixedUpdate,
               (
                   player_input,
                   apply_gravity,
                   apply_movement,
                   check_non_solid_tiles,
                   tick_warp_sequence,
                   handle_player_kill,
                   tick_dead_player,
               )
                   .chain()
                   .run_if(in_state(GameState::InGame).and(not_paused)),
           )
           .add_systems(
               Update,
               (
                   buffer_player_input,
                   debug_powerup_keys,
                   update_animation,
                   update_player_sprite,
               )
                   .run_if(in_state(GameState::InGame).and(not_paused)),
           );
    }
}

fn reset_input_buffer(mut input_buf: ResMut<PlayerInputBuffer>) {
    input_buf.jump_requested = false;
    input_buf.shoot_requested = false;
}

fn buffer_player_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input_buf: ResMut<PlayerInputBuffer>,
) {
    if keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::ArrowUp)
        || crate::input::any_gamepad_just_jump(&gamepads)
    {
        input_buf.jump_requested = true;
    }
    if keys.just_pressed(KeyCode::ControlLeft)
        || keys.just_pressed(KeyCode::ControlRight)
        || keys.just_pressed(KeyCode::KeyX)
        || keys.just_pressed(KeyCode::KeyF)
        || crate::input::any_gamepad_just_shoot(&gamepads)
    {
        input_buf.shoot_requested = true;
    }
}

/// F7 / F8 power-up testing shortcuts (matching C++ LOOP_Gameloop.cpp).
fn debug_powerup_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut hud: ResMut<HudState>,
    mut sfx: EventWriter<SfxEvent>,
    mut player_q: Query<&mut PlayerState, With<Player>>,
) {
    if keys.just_pressed(KeyCode::F7) {
        let was_normal = hud.power_up == 0;
        hud.power_up = (hud.power_up + 1).min(7);
        if was_normal && hud.power_up >= 1 {
            if let Ok(mut st) = player_q.get_single_mut() {
                st.morph_timer = 0.5;
            }
            sfx.send(SfxEvent(SfxKind::Morph));
        }
    } else if keys.just_pressed(KeyCode::F8) {
        hud.power_up = hud.power_up.saturating_sub(1);
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

pub fn player_start_position(level: &LevelData) -> Vec2 {
    let cp = level.checkpoint.min(3);
    let sx = level.stage.start_position_x[cp] as f32;
    let sy = level.stage.start_position_y[cp] as f32;
    if sx > 0.0 || sy > 0.0 {
        return Vec2::new(sx.max(16.0), -sy);
    }
    let sx0 = level.stage.start_position_x[0] as f32;
    let sy0 = level.stage.start_position_y[0] as f32;
    let start_x = if sx0 > 0.0 { sx0 } else { 100.0 };
    let start_y = if sy0 > 0.0 { -sy0 } else { -300.0 };
    Vec2::new(start_x, start_y)
}

fn spawn_player(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
    paths:        Res<crate::paths::AssetPaths>,
    hud:          Res<HudState>,
) {
    let frames = load_player_frames(paths.c64_file("Player.txt").to_str().unwrap_or(""));
    let start = player_start_position(&level);

    let initial_rect = frame_for_state(
        &frames,
        Stance::Stand,
        Direction::Right,
        hud.power_up >= 1,
        true,
        0.0,
    )
    .map(|(x, y, w, h)| Rect::new(x as f32, y as f32, (x + w as i32) as f32, (y + h as i32) as f32));

    commands.insert_resource(PlayerFrames(frames));

    commands.spawn((
        GameEntity,
        Player,
        Velocity::default(),
        OnGround::default(),
        PlayerState {
            furthest_x: start.x,
            ..default()
        },
        PlayerAnimation::default(),
        Sprite {
            image: assets.player.clone(),
            rect: initial_rect,
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 1.0),
    ));
}

pub(crate) fn load_player_frames(path: &str) -> Vec<Frame> {
    let mut frames = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("Player: failed to read {}", path);
        return frames;
    };
    let nums: Vec<i32> = content
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    for chunk in nums.chunks(4) {
        if chunk.len() == 4 {
            frames.push((chunk[0], chunk[1], chunk[2] as u32, chunk[3] as u32));
        }
    }
    frames
}

// ── Physics constants ─────────────────────────────────────────────────────────

const GRAVITY:       f32 = 0.6;
const TERMINAL_VEL:  f32 = 15.0;
const FRICTION:      f32 = 0.8;
const MAX_RUN_SPEED: f32 = 4.2;
const ACCELERATION:  f32 = 0.55;
const JUMP_STRENGTH: f32 = 12.2;
const COYOTE_TIME:   f32 = 0.09; // ~80-90ms grace window after stepping off ledge

// Player AABB (pixels)
pub const PLAYER_W: f32 = 24.0;
pub const PLAYER_H: f32 = 42.0;

// ── Systems ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn player_input(
    time:            Res<Time<Fixed>>,
    keys:            Res<ButtonInput<KeyCode>>,
    gamepads:        Query<&Gamepad>,
    mut input_buf:   ResMut<PlayerInputBuffer>,
    hud:             Res<HudState>,
    mut sfx:         EventWriter<SfxEvent>,
    mut fire_events: EventWriter<FireBulletEvent>,
    mut q: Query<(&Transform, &mut Velocity, &mut PlayerAnimation, &OnGround, &mut PlayerState), With<Player>>,
) {
    let jump_requested = std::mem::take(&mut input_buf.jump_requested);
    let shoot_requested = std::mem::take(&mut input_buf.shoot_requested);
    let Ok((tf, mut vel, mut anim, on_ground, mut pstate)) = q.get_single_mut() else { return };

    let dt = time.delta_secs();
    if pstate.invuln_timer > 0.0 {
        pstate.invuln_timer = (pstate.invuln_timer - dt).max(0.0);
    }
    if pstate.morph_timer > 0.0 {
        pstate.morph_timer = (pstate.morph_timer - dt).max(0.0);
    }

    if !pstate.alive || pstate.warp_timer > 0.0 {
        return;
    }

    if on_ground.0 {
        pstate.coyote_timer = COYOTE_TIME;
    } else {
        pstate.coyote_timer = (pstate.coyote_timer - dt).max(0.0);
    }

    let left  = keys.pressed(KeyCode::ArrowLeft) || crate::input::any_gamepad_left_held(&gamepads);
    let right = keys.pressed(KeyCode::ArrowRight) || crate::input::any_gamepad_right_held(&gamepads);

    if right {
        vel.vx = (vel.vx + ACCELERATION).min(MAX_RUN_SPEED);
        anim.direction = Direction::Right;
    }
    if left {
        vel.vx = (vel.vx - ACCELERATION).max(-MAX_RUN_SPEED);
        anim.direction = Direction::Left;
    }
    if !left && !right {
        vel.vx *= FRICTION;
        if vel.vx.abs() < 0.1 { vel.vx = 0.0; }
    }

    if jump_requested && (on_ground.0 || pstate.coyote_timer > 0.0) {
        vel.vy = JUMP_STRENGTH;
        pstate.coyote_timer = 0.0;
        sfx.send(SfxEvent(SfxKind::Jump));
    }

    if shoot_requested && hud.power_up >= 2 {
        let bx = match anim.direction {
            Direction::Right => tf.translation.x + PLAYER_W + 2.0,
            Direction::Left  => tf.translation.x - 18.0,
        };
        let by = tf.translation.y - PLAYER_H * 0.35;
        fire_events.send(FireBulletEvent {
            x: bx,
            y: by,
            dir: anim.direction,
            power_up: hud.power_up,
        });
    }
}

/// Computes vertical velocity step with gravity, variable jump release cut, and ArrowDown fast-fall
/// (matches C++ `PLAYER_Jump.cpp:25-66`).
pub fn step_player_vertical_velocity(
    vy:         f32,
    alive:      bool,
    on_ground:  bool,
    jump_held:  bool,
    down_held:  bool,
) -> f32 {
    let mut next_vy = (vy - GRAVITY).max(-TERMINAL_VEL);
    if alive {
        if !jump_held && next_vy > 0.0 {
            next_vy -= 2.0 * GRAVITY;
        }
        if !on_ground && down_held {
            next_vy = -TERMINAL_VEL;
        }
    }
    next_vy
}

/// Computes destination stage for a warp stone bump (matches C++ `LOOP_Gameloop_Splashscreens.cpp:82-85`).
pub fn resolve_warp_target(current_stage: usize, stage_warp_to: i32) -> usize {
    if (1..=MAX_CLASSIC_STAGE as i32).contains(&stage_warp_to) {
        stage_warp_to as usize
    } else {
        (current_stage + 3).min(MAX_CLASSIC_STAGE)
    }
}

fn apply_gravity(
    keys:     Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut q:    Query<(&mut Velocity, &OnGround, &PlayerState), With<Player>>,
) {
    let Ok((mut vel, on_ground, pstate)) = q.get_single_mut() else { return };
    if pstate.warp_timer > 0.0 {
        vel.vx = 0.0;
        vel.vy = 0.0;
        return;
    }
    let jump_held = keys.pressed(KeyCode::Space)
        || keys.pressed(KeyCode::ArrowUp)
        || crate::input::any_gamepad_jump_held(&gamepads);
    let down_held = keys.pressed(KeyCode::ArrowDown)
        || crate::input::any_gamepad_down_held(&gamepads);
    vel.vy = step_player_vertical_velocity(vel.vy, pstate.alive, on_ground.0, jump_held, down_held);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_movement(
    mut commands:     Commands,
    mut q:            Query<(&mut Transform, &mut Velocity, &mut OnGround, &mut PlayerState), With<Player>>,
    plat_q:           Query<(&Transform, &MovingPlatform), Without<Player>>,
    stone_q:          Query<(&Transform, &DropStone), (Without<Player>, Without<MovingPlatform>)>,
    mut level:        ResMut<LevelData>,
    mut tile_grid:    ResMut<TileGrid>,
    mut tile_q:       Query<(&mut Tile, &mut Sprite, &mut Visibility), (Without<Player>, Without<MovingPlatform>, Without<DropStone>)>,
    assets:           Res<GameAssets>,
    mut hud:          ResMut<HudState>,
    mut sfx:          EventWriter<SfxEvent>,
    mut powerup_ev:   EventWriter<SpawnPowerUpEvent>,
    mut coin_pop_ev:  EventWriter<SpawnCoinPopEvent>,
    mut dropstone_ev: EventWriter<SpawnDropStoneEvent>,
) {
    let Ok((mut tf, mut vel, mut on_ground, mut pstate)) = q.get_single_mut() else { return };

    if pstate.warp_timer > 0.0 {
        return;
    }

    // Dead player falls off screen ignoring tile collisions (classic Giana/Mario death hop).
    if !pstate.alive {
        tf.translation.y += vel.vy;
        return;
    }

    on_ground.0 = false;

    tf.translation.x += vel.vx;
    // Clamp within trimmed stage left/right edges
    if tf.translation.x < 0.0 {
        tf.translation.x = 0.0;
        vel.vx = 0.0;
    }
    let max_x = (level.stage_width_px - PLAYER_W).max(0.0);
    if tf.translation.x > max_x {
        tf.translation.x = max_x;
        vel.vx = 0.0;
    }
    resolve_x(&mut tf, &mut vel, &level);

    // Horizontal collision against active drop stones (matches C++ PLAYER_Collision_Solids.cpp:360-371)
    for (stf, stone) in &stone_q {
        let px = tf.translation.x;
        let py = -tf.translation.y;
        let sx = stf.translation.x;
        let sy = -stf.translation.y;
        if aabb(px, py, PLAYER_W, PLAYER_H, sx, sy, stone.w, stone.h) {
            if vel.vx > 0.0 {
                tf.translation.x = sx - PLAYER_W;
                vel.vx = 0.0;
            } else if vel.vx < 0.0 {
                tf.translation.x = sx + stone.w;
                vel.vx = 0.0;
            }
        }
    }

    let prev_bottom_y = tf.translation.y - PLAYER_H;
    tf.translation.y += vel.vy;
    let head_bump_tile = resolve_y(&mut tf, &mut vel, &mut on_ground, &level);

    // Moving platform landing & riding (one-way top collision)
    if vel.vy <= 0.0 {
        let px = tf.translation.x;
        let new_bottom_y = tf.translation.y - PLAYER_H;
        for (ptf, plat) in &plat_q {
            let plat_left = ptf.translation.x;
            let plat_right = plat_left + plat.width;
            let plat_top = ptf.translation.y;
            let overlap_x = px + PLAYER_W > plat_left && px < plat_right;
            let was_above = prev_bottom_y >= plat_top - plat.delta_y - 6.0;
            let landed = new_bottom_y <= plat_top + 2.0 && new_bottom_y >= plat_top - plat.height - 6.0;
            if overlap_x && was_above && landed {
                tf.translation.y = plat_top + PLAYER_H;
                vel.vy = 0.0;
                on_ground.0 = true;
                break;
            }
        }
    }

    // Standing on top of (or lifted onto) a DropStone (matches C++ PC_Collision_Down_Stone)
    {
        let px = tf.translation.x;
        let new_bottom_y = tf.translation.y - PLAYER_H;
        for (stf, stone) in &stone_q {
            let sx_left = stf.translation.x;
            let sx_right = sx_left + stone.w;
            let stone_top = stf.translation.y;
            let stone_bottom = stone_top - stone.h;
            let overlap_x = px + PLAYER_W - 3.0 > sx_left && px + 3.0 < sx_right;
            let landing = vel.vy <= 0.0 && new_bottom_y <= stone_top + 2.0 && new_bottom_y >= stone_bottom;
            if overlap_x && landing {
                tf.translation.y = stone_top + PLAYER_H;
                vel.vy = 0.0;
                on_ground.0 = true;
                break;
            }
        }
    }

    // Trigger crumbling ground under player's feet (tiles 566 / 567)
    if on_ground.0 {
        let px = tf.translation.x;
        let feet_py = -tf.translation.y + PLAYER_H + 1.0;
        let ty = (feet_py / 16.0).floor() as i32;
        if ty >= 0 && ty < STAGE_ROWS as i32 {
            for tx in tile_range(px, PLAYER_W) {
                if tx >= 0 && tx < STAGE_COLS as i32 {
                    trigger_crumble_tile(
                        &mut commands,
                        &mut tile_grid,
                        &mut level,
                        &mut tile_q,
                        &assets,
                        tx as usize,
                        ty as usize,
                    );
                }
            }
        }
    }

    if let Some((btx, bty)) = head_bump_tile {
        handle_head_bump_special(
            &mut commands,
            &mut tile_grid,
            &mut level,
            &mut tile_q,
            &assets,
            &mut hud,
            &mut pstate,
            &mut vel,
            &mut sfx,
            &mut powerup_ev,
            &mut coin_pop_ev,
            &mut dropstone_ev,
            btx,
            bty,
        );
    }

    if tf.translation.x > pstate.furthest_x {
        pstate.furthest_x = tf.translation.x;
    }
}

fn resolve_x(tf: &mut Transform, vel: &mut Velocity, level: &LevelData) {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    for tx in tile_range(px, PLAYER_W) {
        for ty in tile_range(py, PLAYER_H) {
            if !is_solid(tx, ty, level) { continue; }
            let (tx1, ty1, tx2, _) = tile_rect(tx, ty);
            if aabb(px, py, PLAYER_W, PLAYER_H, tx1, ty1, 16.0, 16.0) {
                if vel.vx > 0.0 { tf.translation.x = tx1 - PLAYER_W; vel.vx = 0.0; }
                else if vel.vx < 0.0 { tf.translation.x = tx2; vel.vx = 0.0; }
                return;
            }
        }
    }
}

/// Resolves vertical tile collision and returns the `(tx, ty)` tile hit on an upward head bump.
fn resolve_y(
    tf:        &mut Transform,
    vel:       &mut Velocity,
    on_ground: &mut OnGround,
    level:     &LevelData,
) -> Option<(usize, usize)> {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    let center_x = px + PLAYER_W * 0.5;
    let mut best_bump: Option<(usize, usize, f32)> = None;

    for tx in tile_range(px, PLAYER_W) {
        for ty in tile_range(py, PLAYER_H) {
            if !is_solid(tx, ty, level) { continue; }
            let (tx1, ty1, _, _) = tile_rect(tx, ty);
            if aabb(px, py, PLAYER_W, PLAYER_H, tx1, ty1, 16.0, 16.0) {
                if vel.vy < 0.0 {
                    // Falling onto floor
                    tf.translation.y = -(ty1 - PLAYER_H);
                    vel.vy = 0.0;
                    on_ground.0 = true;
                    return None;
                } else if vel.vy > 0.0 {
                    // Head bump: pick the tile closest to player's horizontal center
                    let tile_center = tx1 + 8.0;
                    let dist = (tile_center - center_x).abs();
                    if best_bump.is_none_or(|(_, _, d)| dist < d) {
                        best_bump = Some((tx as usize, ty as usize, dist));
                    }
                }
            }
        }
    }

    if let Some((btx, bty, _)) = best_bump {
        let (_, ty1, _, _) = tile_rect(btx as i32, bty as i32);
        tf.translation.y = -(ty1 + 16.0);
        vel.vy = 0.0;
        return Some((btx, bty));
    }
    None
}

// ── Special block head-bump logic (matches C++ PLAYER_Collision_Solids_Specials.cpp) ──

/// Finds the left column of a 3×2 special block given any tile `(tx, ty)` on its bottom row.
fn find_3x2_block_left(
    level: &LevelData,
    tx: usize,
    ty: usize,
    predicate: impl Fn(&crate::tile_properties::TileSheetInfo, usize) -> bool,
) -> Option<usize> {
    if tx >= STAGE_COLS || ty == 0 || ty >= STAGE_ROWS {
        return None;
    }
    // Must not have a solid tile directly beneath it
    if is_solid(tx as i32, ty as i32 + 1, level) {
        return None;
    }

    let id = level.stage.array[tx][ty];
    if id <= 0 || (id as usize) >= 2320 || !predicate(&level.tile_info, id as usize) {
        return None;
    }

    let matches_at = |x: usize, expected_id: i32| -> bool {
        if x >= STAGE_COLS || expected_id <= 0 || (expected_id as usize) >= 2320 {
            return false;
        }
        level.stage.array[x][ty] == expected_id && predicate(&level.tile_info, expected_id as usize)
    };

    // Hit right tile of 3-wide block
    if tx >= 2 && matches_at(tx - 2, id - 2) && matches_at(tx - 1, id - 1) {
        return Some(tx - 2);
    }
    // Hit left tile of 3-wide block
    if tx + 2 < STAGE_COLS && matches_at(tx + 2, id + 2) && matches_at(tx + 1, id + 1) {
        return Some(tx);
    }
    // Hit center tile of 3-wide block
    if tx >= 1 && tx + 1 < STAGE_COLS && matches_at(tx - 1, id - 1) && matches_at(tx + 1, id + 1) {
        return Some(tx - 1);
    }
    None
}

fn replace_3x2_with_spent_block<F: bevy::ecs::query::QueryFilter>(
    commands:  &mut Commands,
    tile_grid: &mut TileGrid,
    level:     &mut LevelData,
    tile_q:    &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), F>,
    assets:    &GameAssets,
    left_x:    usize,
    bottom_y:  usize,
) {
    // Spent block tile IDs from C++ GRAPHICS_Tiles_Animations.cpp / PLAYER_Collision_Solids_Specials.cpp
    const SPENT_TOP: [i32; 3] = [587, 588, 589];
    const SPENT_BOT: [i32; 3] = [627, 628, 629];

    for dx in 0..3 {
        set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, bottom_y - 1, SPENT_TOP[dx]);
        set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, bottom_y, SPENT_BOT[dx]);
    }
}

/// Steps all 6 tiles of a 3×2 special block by `+3` and schedules timed frame switches
/// (matches C++ `PC_Collision_CoinBlock`, `PC_Collision_CoinBlockHelmet`, `PC_Collision_Breakable`).
#[allow(clippy::too_many_arguments)]
fn step_3x2_block_plus_3<F: bevy::ecs::query::QueryFilter>(
    commands:  &mut Commands,
    tile_grid: &mut TileGrid,
    level:     &mut LevelData,
    tile_q:    &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), F>,
    assets:    &GameAssets,
    left_x:    usize,
    bottom_y:  usize,
    fallback_clear: bool,
) {
    for dx in 0..3 {
        for dy in [bottom_y - 1, bottom_y] {
            let cur = level.stage.array[left_x + dx][dy];
            let next_id = cur + 3;
            if next_tile_switch_id(next_id).is_some() {
                set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, dy, next_id);
                schedule_tile_switch(commands, tile_grid, left_x + dx, dy, 0.14);
            } else if fallback_clear {
                set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, dy, 0);
            } else {
                // Non-5-coin / unexpected coin block ID -> settle cleanly to spent block
                let spent = if dy == bottom_y - 1 { 587 + dx as i32 } else { 627 + dx as i32 };
                set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, dy, spent);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_head_bump_special<F: bevy::ecs::query::QueryFilter>(
    commands:     &mut Commands,
    tile_grid:    &mut TileGrid,
    level:        &mut LevelData,
    tile_q:       &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), F>,
    assets:       &GameAssets,
    hud:          &mut HudState,
    pstate:       &mut PlayerState,
    vel:          &mut Velocity,
    sfx:          &mut EventWriter<SfxEvent>,
    powerup_ev:   &mut EventWriter<SpawnPowerUpEvent>,
    coin_pop_ev:  &mut EventWriter<SpawnCoinPopEvent>,
    dropstone_ev: &mut EventWriter<SpawnDropStoneEvent>,
    tx:           usize,
    ty:           usize,
) {
    // 0. Hidden warp stone block -> reveals warp tiles, freezes with morph, then warps
    // (matches C++ PC_Collision_WarpStone + LOOP_Gameloop_Warpscreen)
    if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.warp_stone[id] != 0) {
        const WARP_TOP: [i32; 3] = [563, 564, 565];
        const WARP_BOT: [i32; 3] = [603, 604, 605];
        for dx in 0..3 {
            set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, ty - 1, WARP_TOP[dx]);
            schedule_tile_switch(commands, tile_grid, left_x + dx, ty - 1, 0.14);
            set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, ty, WARP_BOT[dx]);
            schedule_tile_switch(commands, tile_grid, left_x + dx, ty, 0.14);
        }
        sfx.send(SfxEvent(SfxKind::Portal));
        vel.vx = 0.0;
        vel.vy = 0.0;
        pstate.morph_timer = 0.65;
        pstate.warp_timer = 0.65;
        pstate.warp_target = Some(resolve_warp_target(hud.stage, level.stage.warp_to_stage));
        return;
    }

    // 1. Power-up block
    if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.power_up_block[id] != 0) {
        replace_3x2_with_spent_block(commands, tile_grid, level, tile_q, assets, left_x, ty);
        sfx.send(SfxEvent(SfxKind::Ding));
        powerup_ev.send(SpawnPowerUpEvent {
            x: (left_x + 1) as f32 * 16.0,
            y: -(ty.saturating_sub(2) as f32 * 16.0),
        });
        return;
    }

    // 2. Standard coin block (single or 5-hit multi-coin block via +3 tile stepping)
    if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.coin_block[id] != 0) {
        step_3x2_block_plus_3(commands, tile_grid, level, tile_q, assets, left_x, ty, false);
        hud.add_coin();
        sfx.send(SfxEvent(SfxKind::Ding));
        coin_pop_ev.send(SpawnCoinPopEvent {
            x: (left_x + 1) as f32 * 16.0,
            y: -(ty.saturating_sub(2) as f32 * 16.0),
        });
        return;
    }

    // Punk-only special blocks (matches C++ PLAYER_Collision_Solids.cpp:303-307)
    if hud.power_up >= 1 {
        // 3. Helmet coin block (Punk Giana only)
        if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.coin_block_helmet[id] != 0) {
            step_3x2_block_plus_3(commands, tile_grid, level, tile_q, assets, left_x, ty, false);
            hud.add_coin();
            sfx.send(SfxEvent(SfxKind::Ding));
            coin_pop_ev.send(SpawnCoinPopEvent {
                x: (left_x + 1) as f32 * 16.0,
                y: -(ty.saturating_sub(2) as f32 * 16.0),
            });
            return;
        }

        // 4. Drop stone block (Punk Giana dislodges it into a falling solid sprite)
        if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.drop_stone[id] != 0) {
            for dx in 0..3 {
                set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, ty - 1, 0);
                set_tile(commands, tile_grid, level, tile_q, assets, left_x + dx, ty, 0);
            }
            dropstone_ev.send(SpawnDropStoneEvent {
                x: left_x as f32 * 16.0,
                y: -((ty - 1) as f32 * 16.0),
            });
            sfx.send(SfxEvent(SfxKind::Crack));
            return;
        }

        // 5. Breakable brick block (steps through crack frames 504->507->0)
        if let Some(left_x) = find_3x2_block_left(level, tx, ty, |ti, id| ti.breakable[id] != 0) {
            step_3x2_block_plus_3(commands, tile_grid, level, tile_q, assets, left_x, ty, true);
            hud.score += 50;
            sfx.send(SfxEvent(SfxKind::Crack));
        }
    }
}

fn tick_warp_sequence(
    time:           Res<Time<Fixed>>,
    mut q:          Query<&mut PlayerState, With<Player>>,
    mut hud:        ResMut<HudState>,
    mut level:      ResMut<LevelData>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(mut pstate) = q.get_single_mut() else { return };
    if pstate.warp_timer <= 0.0 {
        return;
    }
    pstate.warp_timer = (pstate.warp_timer - time.delta_secs()).max(0.0);
    if pstate.warp_timer <= 0.0 {
        if let Some(target) = pstate.warp_target.take() {
            hud.stage = target;
            level.checkpoint = 0;
            level.sub_stage_entry_stage = None;
            level.is_warping = true;
            next_state.set(GameState::StageTransition);
        }
    }
}

// ── Non-solid tile collisions: Coins, Lethal/Fire, Substage, Exit, Pitfall ───

#[allow(clippy::too_many_arguments)]
fn check_non_solid_tiles(
    mut commands:   Commands,
    player_q:       Query<(&Transform, &PlayerState), With<Player>>,
    mut level:      ResMut<LevelData>,
    mut tile_grid:  ResMut<TileGrid>,
    mut tile_q:     Query<(&mut Tile, &mut Sprite, &mut Visibility), Without<Player>>,
    assets:         Res<GameAssets>,
    mut hud:        ResMut<HudState>,
    mut sfx:        EventWriter<SfxEvent>,
    mut kill_ev:    EventWriter<KillPlayerEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((tf, pstate)) = player_q.get_single() else { return };
    if !pstate.alive || pstate.warp_timer > 0.0 {
        return;
    }

    // Fell off bottom of stage
    if tf.translation.y < -520.0 {
        kill_ev.send(KillPlayerEvent);
        return;
    }

    let px = tf.translation.x;
    let py = -tf.translation.y;
    let mut hit_lethal = false;
    let mut hit_exit = false;
    let mut hit_substage = false;
    let mut coins_to_collect: Vec<(usize, usize)> = Vec::new();

    for tx in tile_range(px, PLAYER_W) {
        for ty in tile_range(py, PLAYER_H) {
            if tx < 0 || tx >= STAGE_COLS as i32 || ty < 0 || ty >= STAGE_ROWS as i32 {
                continue;
            }
            let (ux, uy) = (tx as usize, ty as usize);
            let id = level.stage.array[ux][uy];
            if id <= 0 || (id as usize) >= 2320 {
                continue;
            }
            let uid = id as usize;
            if level.tile_info.coin[uid] != 0 {
                coins_to_collect.push((ux, uy));
            }
            if level.tile_info.lethal[uid] != 0 {
                let is_fire = level.tile_info.fire[uid] != 0;
                if !(is_fire && hud.power_up >= 7) {
                    hit_lethal = true;
                }
            }
            if level.tile_info.sub_stage_entrance[uid] != 0 {
                hit_substage = true;
            }
            if level.tile_info.exit[uid] != 0 {
                hit_exit = true;
            }
        }
    }

    for (cx, cy) in coins_to_collect {
        if level.stage.array[cx][cy] == 0 {
            continue;
        }
        collect_coin_cluster(&mut commands, &mut tile_grid, &mut level, &mut tile_q, &assets, cx, cy);
        hud.add_coin();
        sfx.send(SfxEvent(SfxKind::Ding));
    }

    // Enter bonus substage (stages 34..=37, matching C++ LOOP_Gameloop.cpp:167-176)
    if hit_substage && hud.stage <= MAX_CLASSIC_STAGE {
        sfx.send(SfxEvent(SfxKind::Portal));
        level.sub_stage_entry_stage = Some(hud.stage);
        let bonus_stage = 34 + level.sub_stage_counter;
        level.sub_stage_counter = (level.sub_stage_counter + 1) % 4;
        hud.stage = bonus_stage;
        level.checkpoint = 0;
        next_state.set(GameState::StageTransition);
        return;
    }

    if hit_exit {
        sfx.send(SfxEvent(SfxKind::Portal));
        hud.score += 500;
        if hud.stage > MAX_CLASSIC_STAGE {
            // Exiting a bonus substage returns to entry stage at checkpoint 3 (matches C++ LOOP_Gameloop.cpp:89-92)
            let return_stage = level.sub_stage_entry_stage.take().unwrap_or(1);
            hud.stage = return_stage;
            level.checkpoint = 3;
            next_state.set(GameState::StageTransition);
        } else if hud.stage >= MAX_CLASSIC_STAGE {
            hud.victory = true;
            next_state.set(GameState::GameOver);
        } else {
            hud.stage += 1;
            level.checkpoint = 0;
            next_state.set(GameState::StageTransition);
        }
        return;
    }

    if hit_lethal && pstate.invuln_timer <= 0.0 {
        kill_ev.send(KillPlayerEvent);
    }
}

/// Removes a coin and its adjacent multi-tile spritesheet pieces (matches C++ `Delete_Coin`).
fn collect_coin_cluster(
    commands:  &mut Commands,
    tile_grid: &mut TileGrid,
    level:     &mut LevelData,
    tile_q:    &mut Query<(&mut Tile, &mut Sprite, &mut Visibility), Without<Player>>,
    assets:    &GameAssets,
    cx:        usize,
    cy:        usize,
) {
    let base_id = level.stage.array[cx][cy];
    if base_id == 0 {
        return;
    }

    let offsets: [(isize, isize, i32); 8] = [
        (-1,  0, -1),
        ( 1,  0,  1),
        ( 0, -1, -TILES_PER_ROW),
        ( 0,  1,  TILES_PER_ROW),
        (-1, -1, -TILES_PER_ROW - 1),
        ( 1, -1, -TILES_PER_ROW + 1),
        (-1,  1,  TILES_PER_ROW - 1),
        ( 1,  1,  TILES_PER_ROW + 1),
    ];

    for (dx, dy, id_delta) in offsets {
        let nx = cx as isize + dx;
        let ny = cy as isize + dy;
        if nx >= 0 && nx < STAGE_COLS as isize && ny >= 0 && ny < STAGE_ROWS as isize {
            let (ux, uy) = (nx as usize, ny as usize);
            if level.stage.array[ux][uy] == base_id + id_delta {
                set_tile(commands, tile_grid, level, tile_q, assets, ux, uy, 0);
            }
        }
    }
    set_tile(commands, tile_grid, level, tile_q, assets, cx, cy, 0);
}

// ── Death, Lives & Checkpoint Respawn ─────────────────────────────────────────

fn handle_player_kill(
    mut kill_events: EventReader<KillPlayerEvent>,
    mut player_q:    Query<(&mut Velocity, &mut PlayerState), With<Player>>,
    mut hud:         ResMut<HudState>,
    mut sfx:         EventWriter<SfxEvent>,
) {
    if kill_events.read().next().is_none() {
        return;
    }
    let Ok((mut vel, mut pstate)) = player_q.get_single_mut() else { return };
    if !pstate.alive || pstate.warp_timer > 0.0 {
        return;
    }

    pstate.alive = false;
    pstate.death_timer = 0.0;
    vel.vx = 0.0;
    vel.vy = 11.0; // classic upward death hop
    hud.power_up = 0;
    hud.lives = hud.lives.saturating_sub(1);
    sfx.send(SfxEvent(SfxKind::Died));
}

fn tick_dead_player(
    time:           Res<Time<Fixed>>,
    mut player_q:   Query<(&Transform, &mut PlayerState), With<Player>>,
    hud:            Res<HudState>,
    mut level:      ResMut<LevelData>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((tf, mut pstate)) = player_q.get_single_mut() else { return };
    if pstate.alive {
        return;
    }

    pstate.death_timer += time.delta_secs();
    if tf.translation.y < -620.0 || pstate.death_timer > 1.4 {
        if hud.lives == 0 {
            next_state.set(GameState::GameOver);
        } else {
            // Pick furthest reached checkpoint (matches C++ PC_Death_Routine)
            let mut checkpoint = 0usize;
            for cp in 1..=2 {
                let cx = level.stage.start_position_x[cp] as f32;
                if cx > 0.0 && pstate.furthest_x > cx - 10.0 {
                    checkpoint = cp;
                }
            }
            level.checkpoint = checkpoint;
            next_state.set(GameState::StageTransition);
        }
    }
}

// ── Collision helpers ─────────────────────────────────────────────────────────

pub fn tile_range(pos: f32, size: f32) -> std::ops::RangeInclusive<i32> {
    (pos / 16.0).floor() as i32 ..= ((pos + size - 0.01) / 16.0).floor() as i32
}

fn tile_rect(tx: i32, ty: i32) -> (f32, f32, f32, f32) {
    let x = tx as f32 * 16.0;
    let y = ty as f32 * 16.0;
    (x, y, x + 16.0, y + 16.0)
}

#[allow(clippy::too_many_arguments)]
pub fn aabb(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn is_solid(tx: i32, ty: i32, level: &LevelData) -> bool {
    if tx < 0 || tx >= STAGE_COLS as i32 || ty < 0 || ty >= STAGE_ROWS as i32 { return false; }
    let id = level.stage.array[tx as usize][ty as usize];
    if id <= 0 || (id as usize) >= 2320 { return false; }
    level.tile_info.solid[id as usize] != 0
}

// ── Animation & Sprite Rendering ──────────────────────────────────────────────

fn update_animation(
    mut q: Query<(&Velocity, &OnGround, &PlayerState, &mut PlayerAnimation), With<Player>>,
) {
    let Ok((vel, on_ground, pstate, mut anim)) = q.get_single_mut() else { return };
    if !pstate.alive || pstate.warp_timer > 0.0 {
        return;
    }
    if !on_ground.0 {
        anim.stance = Stance::Jump;
    } else if (vel.vx > 0.5 && anim.direction == Direction::Left)
        || (vel.vx < -0.5 && anim.direction == Direction::Right)
    {
        anim.stance = Stance::Turn;
    } else if vel.vx.abs() > 0.1 {
        anim.walk_timer += 1;
        if anim.walk_timer >= 20 { anim.walk_timer = 0; }
        anim.stance = if anim.walk_timer < 10 { Stance::Walk1 } else { Stance::Walk2 };
    } else {
        anim.stance = Stance::Stand;
        anim.walk_timer = 0;
    }
}

fn update_player_sprite(
    frames: Res<PlayerFrames>,
    hud:    Res<HudState>,
    mut q:  Query<(&PlayerAnimation, &PlayerState, &mut Sprite), With<Player>>,
) {
    let Ok((anim, pstate, mut sprite)) = q.get_single_mut() else { return };
    if let Some((x, y, w, h)) = frame_for_state(
        &frames.0,
        anim.stance,
        anim.direction,
        hud.power_up >= 1,
        pstate.alive,
        pstate.morph_timer,
    ) {
        sprite.rect = Some(Rect::new(
            x as f32,
            y as f32,
            (x + w as i32) as f32,
            (y + h as i32) as f32,
        ));
        sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
    }

    // Visual blink while briefly invulnerable after spawning
    if pstate.alive && pstate.invuln_timer > 0.0 {
        let alpha = if ((pstate.invuln_timer * 16.0) as i32) % 2 == 0 { 0.45 } else { 1.0 };
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    } else {
        sprite.color = Color::WHITE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_player_frames_covers_normal_punk_morph_and_dead() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/c64/Player.txt");
        let frames = load_player_frames(path.to_str().unwrap());
        assert_eq!(frames.len(), 23, "expected 23 frames in Player.txt");

        for punk in [false, true] {
            for dir in [Direction::Right, Direction::Left] {
                for stance in [
                    Stance::Stand,
                    Stance::Walk1,
                    Stance::Walk2,
                    Stance::Jump,
                    Stance::Turn,
                ] {
                    let frame = frame_for_state(&frames, stance, dir, punk, true, 0.0);
                    assert!(frame.is_some(), "missing frame for punk={} {:?} {:?}", punk, stance, dir);
                    let (_, _, w, h) = frame.unwrap();
                    assert!(w > 0 && h > 0);
                }
            }
        }

        // Morphing frames (20, 21) and Dead frame (22)
        assert_eq!(frame_for_state(&frames, Stance::Stand, Direction::Right, true, true, 0.2), Some(frames[20]));
        assert_eq!(frame_for_state(&frames, Stance::Stand, Direction::Right, true, true, 0.16), Some(frames[21]));
        assert_eq!(frame_for_state(&frames, Stance::Stand, Direction::Right, false, false, 0.0), Some(frames[22]));
    }

    #[test]
    fn variable_jump_cut_and_fast_fall_behave_like_cpp() {
        let held = step_player_vertical_velocity(10.0, true, false, true, false);
        let released = step_player_vertical_velocity(10.0, true, false, false, false);
        assert!(released < held, "releasing jump should cut upward velocity");

        let fast_fall = step_player_vertical_velocity(2.0, true, false, false, true);
        assert_eq!(fast_fall, -TERMINAL_VEL, "holding down in air should trigger fast-fall");

        // On ground, holding down should not snap velocity to terminal fall beyond normal gravity
        let grounded_down = step_player_vertical_velocity(0.0, true, true, false, true);
        assert_eq!(grounded_down, -GRAVITY);
    }

    #[test]
    fn warp_target_respects_stage_override_or_advances_plus_three() {
        assert_eq!(resolve_warp_target(1, 0), 4);
        assert_eq!(resolve_warp_target(32, 0), 33);
        assert_eq!(resolve_warp_target(2, 12), 12);
    }

    #[test]
    fn find_3x2_block_left_detects_left_center_right_and_rejects_blocked_underneath() {
        let paths = crate::paths::AssetPaths::resolve();
        let tile_info = crate::tile_properties::load_tile_properties(
            paths.tilesheet_info().to_str().unwrap(),
        )
        .unwrap();
        let mut stage = crate::level::ImportStage::new();
        // Stamp 3x2 breakable block (501..503 / 541..543) at (10, 14..=15)
        crate::random_gen::stamp_element(&mut stage, 501, 10, 14);

        let mut level = LevelData {
            stage_packs: Vec::new(),
            pack_index: 0,
            stage_file: std::path::PathBuf::new(),
            stage_number: 1,
            checkpoint: 0,
            sub_stage_entry_stage: None,
            sub_stage_counter: 0,
            is_warping: false,
            random_levels: false,
            random_seed: 0,
            stage_width_px: 4096.0,
            stage,
            tile_info,
        };

        for hit_x in [10usize, 11, 12] {
            assert_eq!(
                find_3x2_block_left(&level, hit_x, 15, |ti, id| ti.breakable[id] != 0),
                Some(10),
                "hitting col {} should resolve left column 10",
                hit_x
            );
        }

        // Put a solid tile directly beneath center col (11, 16) -> bumping (11, 15) must be rejected
        level.stage.array[11][16] = 587;
        assert_eq!(
            find_3x2_block_left(&level, 11, 15, |ti, id| ti.breakable[id] != 0),
            None
        );
    }

    #[test]
    fn punk_mode_gates_drop_stone_helmet_coin_and_breakable_blocks() {
        let paths = crate::paths::AssetPaths::resolve();
        let tile_info = crate::tile_properties::load_tile_properties(
            paths.tilesheet_info().to_str().unwrap(),
        )
        .unwrap();

        // Find a real drop_stone bottom-row triple in Tilesheetinfo.tsi
        let drop_top = (1..2270i32)
            .find(|&id| {
                let b = (id + TILES_PER_ROW) as usize;
                tile_info.drop_stone[b] != 0
                    && tile_info.drop_stone[b + 1] != 0
                    && tile_info.drop_stone[b + 2] != 0
            })
            .expect("expected 3-wide drop_stone in Tilesheetinfo.tsi");

        // Find a real coin_block_helmet bottom-row triple in Tilesheetinfo.tsi
        let helmet_top = (1..2270i32)
            .find(|&id| {
                let b = (id + TILES_PER_ROW) as usize;
                tile_info.coin_block_helmet[b] != 0
                    && tile_info.coin_block_helmet[b + 1] != 0
                    && tile_info.coin_block_helmet[b + 2] != 0
                    && tile_info.coin_block[b] == 0
            })
            .expect("expected 3-wide coin_block_helmet in Tilesheetinfo.tsi");

        let mut world = World::new();
        world.init_resource::<TileGrid>();
        world.init_resource::<Events<SfxEvent>>();
        world.init_resource::<Events<SpawnPowerUpEvent>>();
        world.init_resource::<Events<SpawnCoinPopEvent>>();
        world.init_resource::<Events<SpawnDropStoneEvent>>();
        world.insert_resource(HudState::default());
        world.insert_resource(GameAssets {
            tiles: Handle::default(),
            player: Handle::default(),
            enemies: Handle::default(),
            powerups: Handle::default(),
            interface: Handle::default(),
            jump_sfx: Handle::default(),
            stomp_sfx: Handle::default(),
            died_sfx: Handle::default(),
            ding_sfx: Handle::default(),
            crack_sfx: Handle::default(),
            morph_sfx: Handle::default(),
            shoot_sfx: Handle::default(),
            portal_sfx: Handle::default(),
            click_sfx: Handle::default(),
            disk_sfx: Handle::default(),
        });

        let mut stage = crate::level::ImportStage::new();
        // Breakable 3x2 at (10, 14..=15)
        crate::random_gen::stamp_element(&mut stage, 501, 10, 14);
        // Drop stone 3x2 at (20, 14..=15)
        for dx in 0..3 {
            stage.array[20 + dx][14] = drop_top + dx as i32;
            stage.array[20 + dx][15] = drop_top + TILES_PER_ROW + dx as i32;
        }
        // Helmet coin block 3x2 at (30, 14..=15)
        for dx in 0..3 {
            stage.array[30 + dx][14] = helmet_top + dx as i32;
            stage.array[30 + dx][15] = helmet_top + TILES_PER_ROW + dx as i32;
        }

        world.insert_resource(LevelData {
            stage_packs: Vec::new(),
            pack_index: 0,
            stage_file: std::path::PathBuf::new(),
            stage_number: 1,
            checkpoint: 0,
            sub_stage_entry_stage: None,
            sub_stage_counter: 0,
            is_warping: false,
            random_levels: false,
            random_seed: 0,
            stage_width_px: 4096.0,
            stage,
            tile_info,
        });

        #[derive(Resource)]
        struct TestBump(usize, usize);

        #[allow(clippy::too_many_arguments)]
        fn run_bump(
            mut commands: Commands,
            bump: Res<TestBump>,
            mut tile_grid: ResMut<TileGrid>,
            mut level: ResMut<LevelData>,
            mut tile_q: Query<(&mut Tile, &mut Sprite, &mut Visibility), Without<Player>>,
            assets: Res<GameAssets>,
            mut hud: ResMut<HudState>,
            mut sfx: EventWriter<SfxEvent>,
            mut powerup_ev: EventWriter<SpawnPowerUpEvent>,
            mut coin_pop_ev: EventWriter<SpawnCoinPopEvent>,
            mut dropstone_ev: EventWriter<SpawnDropStoneEvent>,
        ) {
            let mut pstate = PlayerState::default();
            let mut vel = Velocity::default();
            handle_head_bump_special(
                &mut commands,
                &mut tile_grid,
                &mut level,
                &mut tile_q,
                &assets,
                &mut hud,
                &mut pstate,
                &mut vel,
                &mut sfx,
                &mut powerup_ev,
                &mut coin_pop_ev,
                &mut dropstone_ev,
                bump.0,
                bump.1,
            );
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(run_bump);

        // Normal Giana (power_up == 0): none of the 3 Punk-only blocks should trigger
        for tx in [11usize, 21, 31] {
            world.insert_resource(TestBump(tx, 15));
            schedule.run(&mut world);
        }
        assert_eq!(world.resource::<LevelData>().stage.array[10][14], 501);
        assert_eq!(world.resource::<LevelData>().stage.array[20][14], drop_top);
        assert_eq!(world.resource::<HudState>().coins, 0);
        assert!(world.resource::<Events<SpawnDropStoneEvent>>().is_empty());

        // Punk Giana (power_up == 1): all 3 blocks should trigger properly
        world.resource_mut::<HudState>().power_up = 1;
        for tx in [11usize, 21, 31] {
            world.insert_resource(TestBump(tx, 15));
            schedule.run(&mut world);
        }
        // Breakable stepped +3 into crack animation (501 -> 504)
        assert_eq!(world.resource::<LevelData>().stage.array[10][14], 504);
        // Drop stone cleared & event emitted
        assert_eq!(world.resource::<LevelData>().stage.array[20][14], 0);
        assert_eq!(world.resource::<Events<SpawnDropStoneEvent>>().len(), 1);
        // Helmet coin block awarded a coin
        assert_eq!(world.resource::<HudState>().coins, 1);
    }
}
