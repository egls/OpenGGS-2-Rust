use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData};
use crate::tilemap::GameEntity;

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

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { #[default] Right, Left }

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance { #[default] Stand, Walk1, Walk2, Jump, Run1, Run2 }

#[derive(Component, Default)]
pub struct PlayerAnimation {
    pub direction:  Direction,
    pub stance:     Stance,
    pub walk_timer: u32,
}

// ── Frame data ────────────────────────────────────────────────────────────────

/// (src_x, src_y, src_w, src_h) in pixels on the spritesheet
pub type Frame = (i32, i32, u32, u32);

#[derive(Resource)]
pub struct PlayerFrames(pub Vec<Frame>);

/// Player.txt layout (0-indexed):
/// Row 0 (y=0):   Stand-R, Walk1-R, Walk2-R, Run1-R, Run2-R
/// Row 1 (y=48):  Stand-L, Walk1-L, Walk2-L, Run1-L, Run2-L
/// Row 2 (y=96):  Jump-R variants
/// Row 3 (y=144): Jump-L variants
/// Row 4 (y=192): Crouch / other
///
/// We use: Stand=0, Walk1=1, Walk2=2, Jump=10 (row2 frame0), mirrored for left
fn frame_for_stance(frames: &[Frame], stance: Stance, dir: Direction) -> Option<Frame> {
    let row_offset = if dir == Direction::Right { 0 } else { 5 };
    let idx = match stance {
        Stance::Stand => row_offset,
        Stance::Walk1 => row_offset + 1,
        Stance::Walk2 => row_offset + 2,
        Stance::Run1  => row_offset + 3,
        Stance::Run2  => row_offset + 4,
        Stance::Jump  => {
            // Jump frames start at index 10 (row 2)
            if dir == Direction::Right { 10 } else { 15 }
        }
    };
    frames.get(idx).copied()
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
           .add_systems(
               FixedUpdate,
               (
                   player_input,
                   apply_gravity,
                   apply_movement,
               )
                   .chain()
                   .run_if(in_state(GameState::InGame)),
           )
           .add_systems(
               Update,
               (
                   update_animation,
                   update_player_sprite,
                   check_death,
               )
                   .run_if(in_state(GameState::InGame)),
           );
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_player(
    mut commands: Commands,
    level:   Res<LevelData>,
    assets:  Res<GameAssets>,
) {
    let frames = load_player_frames("../base/c64/Player.txt");

    // Start position from level data
    let sx = level.stage.start_position_x[0] as f32;
    let sy = level.stage.start_position_y[0] as f32;
    let start_x = if sx > 0.0 { sx } else { 100.0 };
    let start_y = if sy > 0.0 { -sy } else { -300.0 };

    // Initial sprite rect from first frame
    let initial_rect = frames.first().map(|&(x, y, w, h)| {
        Rect::new(x as f32, y as f32, (x + w as i32) as f32, (y + h as i32) as f32)
    });

    commands.insert_resource(PlayerFrames(frames));

    commands.spawn((
        GameEntity,
        Player,
        Velocity::default(),
        OnGround::default(),
        PlayerAnimation::default(),
        Sprite {
            image: assets.player.clone(),
            rect: initial_rect,
            ..default()
        },
        Transform::from_xyz(start_x, start_y, 1.0),
    ));
}

fn load_player_frames(path: &str) -> Vec<Frame> {
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
    println!("Player: loaded {} frames", frames.len());
    frames
}

// ── Physics constants ─────────────────────────────────────────────────────────

const GRAVITY:       f32 = 0.6;
const TERMINAL_VEL:  f32 = 15.0;
const FRICTION:      f32 = 0.8;
const MAX_RUN_SPEED: f32 = 4.0;
const ACCELERATION:  f32 = 0.5;
const JUMP_STRENGTH: f32 = 12.0;

// Player AABB (pixels)
pub const PLAYER_W: f32 = 24.0;
pub const PLAYER_H: f32 = 42.0;

// ── Systems ───────────────────────────────────────────────────────────────────

fn player_input(
    keys:  Res<ButtonInput<KeyCode>>,
    mut q: Query<(&mut Velocity, &mut PlayerAnimation, &OnGround), With<Player>>,
) {
    let Ok((mut vel, mut anim, on_ground)) = q.get_single_mut() else { return };

    let left  = keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::ArrowRight);

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

    if keys.just_pressed(KeyCode::Space) && on_ground.0 {
        vel.vy = JUMP_STRENGTH;
    }
}

fn apply_gravity(mut q: Query<&mut Velocity, With<Player>>) {
    let Ok(mut vel) = q.get_single_mut() else { return };
    vel.vy -= GRAVITY;
    if vel.vy < -TERMINAL_VEL { vel.vy = -TERMINAL_VEL; }
}

fn apply_movement(
    mut q: Query<(&mut Transform, &mut Velocity, &mut OnGround), With<Player>>,
    level: Res<LevelData>,
) {
    let Ok((mut tf, mut vel, mut on_ground)) = q.get_single_mut() else { return };
    on_ground.0 = false;

    tf.translation.x += vel.vx;
    resolve_x(&mut tf, &mut vel, &level);

    tf.translation.y += vel.vy;
    resolve_y(&mut tf, &mut vel, &mut on_ground, &level);
}

fn resolve_x(tf: &mut Transform, vel: &mut Velocity, level: &LevelData) {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    for tx in tile_range(px, PLAYER_W) {
        for ty in tile_range(py, PLAYER_H) {
            if !is_solid(tx, ty, level) { continue; }
            let (tx1, ty1, tx2, ty2) = tile_rect(tx, ty);
            if aabb(px, py, PLAYER_W, PLAYER_H, tx1, ty1, 16.0, 16.0) {
                if vel.vx > 0.0 { tf.translation.x = tx1 - PLAYER_W; vel.vx = 0.0; }
                else if vel.vx < 0.0 { tf.translation.x = tx2; vel.vx = 0.0; }
                let _ = ty2; return;
            }
        }
    }
}

fn resolve_y(tf: &mut Transform, vel: &mut Velocity, on_ground: &mut OnGround, level: &LevelData) {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    for tx in tile_range(px, PLAYER_W) {
        for ty in tile_range(py, PLAYER_H) {
            if !is_solid(tx, ty, level) { continue; }
            let (tx1, ty1, tx2, _) = tile_rect(tx, ty);
            if aabb(px, py, PLAYER_W, PLAYER_H, tx1, ty1, 16.0, 16.0) {
                if vel.vy < 0.0 { // falling
                    tf.translation.y = -(ty1 - PLAYER_H);
                    vel.vy = 0.0;
                    on_ground.0 = true;
                } else if vel.vy > 0.0 { // head bump
                    tf.translation.y = -(ty1 + 16.0);
                    vel.vy = 0.0;
                }
                let _ = tx2; return;
            }
        }
    }
}

fn tile_range(pos: f32, size: f32) -> std::ops::RangeInclusive<i32> {
    (pos / 16.0).floor() as i32 ..= ((pos + size) / 16.0).floor() as i32
}

fn tile_rect(tx: i32, ty: i32) -> (f32, f32, f32, f32) {
    let x = tx as f32 * 16.0;
    let y = ty as f32 * 16.0;
    (x, y, x + 16.0, y + 16.0)
}

fn aabb(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn is_solid(tx: i32, ty: i32, level: &LevelData) -> bool {
    if tx < 0 || tx >= 256 || ty < 0 || ty >= 30 { return false; }
    let id = level.stage.array[tx as usize][ty as usize] as usize;
    if id == 0 || id >= 2320 { return false; }
    level.tile_info.solid[id] != 0
}

fn update_animation(
    mut q: Query<(&Velocity, &OnGround, &mut PlayerAnimation), With<Player>>,
) {
    let Ok((vel, on_ground, mut anim)) = q.get_single_mut() else { return };
    if !on_ground.0 {
        anim.stance = Stance::Jump;
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
    mut q:  Query<(&PlayerAnimation, &mut Sprite), With<Player>>,
) {
    let Ok((anim, mut sprite)) = q.get_single_mut() else { return };
    if let Some((x, y, w, h)) = frame_for_stance(&frames.0, anim.stance, anim.direction) {
        sprite.rect = Some(Rect::new(
            x as f32,
            y as f32,
            (x + w as i32) as f32,
            (y + h as i32) as f32,
        ));
        // Use custom_size to render at actual pixel size
        sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
    }
}

fn check_death(
    player_q: Query<&Transform, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    level: Res<LevelData>,
) {
    if let Ok(tf) = player_q.get_single() {
        if tf.translation.y < -600.0 { // Fell off screen
            next_state.set(GameState::GameOver);
        }

        // Check if on exit tile
        let px = tf.translation.x;
        let py = -tf.translation.y;
        for tx in tile_range(px, PLAYER_W) {
            for ty in tile_range(py, PLAYER_H) {
                if is_exit(tx, ty, &level) {
                    next_state.set(GameState::GameOver);
                    return;
                }
            }
        }
    }
}

fn is_exit(tx: i32, ty: i32, level: &LevelData) -> bool {
    if tx < 0 || tx >= 256 || ty < 0 || ty >= 30 { return false; }
    let id = level.stage.array[tx as usize][ty as usize] as usize;
    if id == 0 || id >= 2320 { return false; }
    level.tile_info.exit[id] != 0
}
