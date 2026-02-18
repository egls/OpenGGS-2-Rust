use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData};
use crate::tilemap::GameEntity;

// ── Marker / Components ───────────────────────────────────────────────────────

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
pub enum Stance { #[default] Stand, Walk1, Walk2, Jump }

#[derive(Component, Default)]
pub struct PlayerAnimation {
    pub direction:  Direction,
    pub stance:     Stance,
    pub walk_timer: u32,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
           .add_systems(
               Update,
               (
                   player_input,
                   apply_gravity,
                   apply_movement,
                   update_animation,
               )
                   .chain()
                   .run_if(in_state(GameState::InGame)),
           );
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_player(
    mut commands: Commands,
    level:   Res<LevelData>,
    assets:  Res<GameAssets>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Parse Player.txt for frame data
    let frames = load_player_frames("../base/c64/Player.txt");

    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(32, 44),
        5, // columns per row (stand+walk1+walk2+jump+extra, right then left)
        5,
        None,
        None,
    );
    let layout_handle = layouts.add(layout);

    // Start position from level data
    let sx = level.stage.start_position_x[0] as f32;
    let sy = level.stage.start_position_y[0] as f32;
    let start_x = if sx > 0.0 { sx } else { 100.0 };
    let start_y = if sy > 0.0 { -sy } else { -300.0 }; // Bevy Y-up

    commands.insert_resource(PlayerFrames(frames));

    commands.spawn((
        GameEntity,
        Player,
        Velocity::default(),
        OnGround::default(),
        PlayerAnimation::default(),
        Sprite {
            image: assets.player.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle,
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(start_x, start_y, 1.0),
    ));
}

// ── Frame data ────────────────────────────────────────────────────────────────

/// (src_x, src_y, src_w, src_h) in pixels
pub type Frame = (i32, i32, u32, u32);

#[derive(Resource)]
pub struct PlayerFrames(pub Vec<Frame>);

fn load_player_frames(path: &str) -> Vec<Frame> {
    let mut frames = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("Player: failed to read {}", path);
        return frames;
    };
    let nums: Vec<i32> = content
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
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

const GRAVITY:          f32 = 1.0;
const TERMINAL_VEL:     f32 = 15.0;
const FRICTION:         f32 = 1.0;
const MAX_RUN_SPEED:    f32 = 4.0;
const ACCELERATION:     f32 = 1.0;
const JUMP_STRENGTH:    f32 = 12.0;

// ── Systems ───────────────────────────────────────────────────────────────────

fn player_input(
    keys:     Res<ButtonInput<KeyCode>>,
    mut q:    Query<(&mut Velocity, &mut PlayerAnimation, &OnGround), With<Player>>,
) {
    let Ok((mut vel, mut anim, on_ground)) = q.get_single_mut() else { return };

    let left  = keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::ArrowRight);

    if right {
        vel.vx += ACCELERATION;
        if vel.vx > MAX_RUN_SPEED { vel.vx = MAX_RUN_SPEED; }
        anim.direction = Direction::Right;
    }
    if left {
        vel.vx -= ACCELERATION;
        if vel.vx < -MAX_RUN_SPEED { vel.vx = -MAX_RUN_SPEED; }
        anim.direction = Direction::Left;
    }
    if !left && !right {
        if vel.vx > 0.0 { vel.vx = (vel.vx - FRICTION).max(0.0); }
        else if vel.vx < 0.0 { vel.vx = (vel.vx + FRICTION).min(0.0); }
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

    // X axis
    tf.translation.x += vel.vx;
    resolve_tile_collision_x(&mut tf, &mut vel, &level);

    // Y axis
    tf.translation.y += vel.vy;
    resolve_tile_collision_y(&mut tf, &mut vel, &mut on_ground, &level);
}

fn resolve_tile_collision_x(tf: &mut Transform, vel: &mut Velocity, level: &LevelData) {
    let w = 28.0f32;
    let h = 42.0f32;
    let px = tf.translation.x;
    let py = -tf.translation.y; // back to C++ Y-down for tile lookup

    let left_tile   = (px / 16.0).floor() as i32;
    let right_tile  = ((px + w) / 16.0).floor() as i32;
    let top_tile    = (py / 16.0).floor() as i32;
    let bottom_tile = ((py + h) / 16.0).floor() as i32;

    for tx in left_tile..=right_tile {
        for ty in top_tile..=bottom_tile {
            if !is_solid(tx, ty, level) { continue; }
            let tile_x = tx as f32 * 16.0;
            let tile_y = ty as f32 * 16.0;
            if px < tile_x + 16.0 && px + w > tile_x && py < tile_y + 16.0 && py + h > tile_y {
                if vel.vx > 0.0 { tf.translation.x = tile_x - w; vel.vx = 0.0; }
                else if vel.vx < 0.0 { tf.translation.x = tile_x + 16.0; vel.vx = 0.0; }
                return;
            }
        }
    }
}

fn resolve_tile_collision_y(
    tf: &mut Transform,
    vel: &mut Velocity,
    on_ground: &mut OnGround,
    level: &LevelData,
) {
    let w = 28.0f32;
    let h = 42.0f32;
    let px = tf.translation.x;
    let py = -tf.translation.y;

    let left_tile   = (px / 16.0).floor() as i32;
    let right_tile  = ((px + w) / 16.0).floor() as i32;
    let top_tile    = (py / 16.0).floor() as i32;
    let bottom_tile = ((py + h) / 16.0).floor() as i32;

    for tx in left_tile..=right_tile {
        for ty in top_tile..=bottom_tile {
            if !is_solid(tx, ty, level) { continue; }
            let tile_x = tx as f32 * 16.0;
            let tile_y = ty as f32 * 16.0;
            if px < tile_x + 16.0 && px + w > tile_x && py < tile_y + 16.0 && py + h > tile_y {
                if vel.vy < 0.0 { // falling (Bevy: vy negative = down)
                    tf.translation.y = -(tile_y - h);
                    vel.vy = 0.0;
                    on_ground.0 = true;
                } else if vel.vy > 0.0 { // jumping (head bump)
                    tf.translation.y = -(tile_y + 16.0);
                    vel.vy = 0.0;
                }
                return;
            }
        }
    }
}

fn is_solid(tx: i32, ty: i32, level: &LevelData) -> bool {
    if tx < 0 || tx >= 256 || ty < 0 || ty >= 30 { return false; }
    let tile_id = level.stage.array[tx as usize][ty as usize] as usize;
    if tile_id >= 2320 { return false; }
    level.tile_info.solid[tile_id] != 0
}

fn update_animation(
    mut q: Query<(&Velocity, &OnGround, &mut PlayerAnimation), With<Player>>,
) {
    let Ok((vel, on_ground, mut anim)) = q.get_single_mut() else { return };
    if !on_ground.0 {
        anim.stance = Stance::Jump;
    } else if vel.vx.abs() > 0.1 {
        anim.walk_timer += 1;
        if anim.walk_timer >= 16 { anim.walk_timer = 0; }
        anim.stance = if anim.walk_timer < 8 { Stance::Walk1 } else { Stance::Walk2 };
    } else {
        anim.stance = Stance::Stand;
        anim.walk_timer = 0;
    }
}
