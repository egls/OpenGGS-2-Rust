use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData};
use crate::tilemap::GameEntity;
use crate::player::{Player, Velocity as PlayerVelocity, OnGround, is_solid, PLAYER_W, PLAYER_H};

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: usize,
    pub alive: bool,
}

#[derive(Component)]
pub struct EnemyDir(pub i32); // 0 = left, 1 = right

#[derive(Component, Default)]
pub struct EnemyAnim {
    pub frame: usize, // 0..3 walk cycle
    pub timer: u32,
}

#[derive(Component)]
pub struct EnemyCollider { pub w: f32, pub h: f32 }

// ── Events ────────────────────────────────────────────────────────────────────

#[derive(Event)]
pub struct StompEvent { pub enemy: Entity }

// ── Frame data ────────────────────────────────────────────────────────────────

/// One frame: pixel rect on the spritesheet
pub type Frame = (i32, i32, u32, u32);

/// Per-enemy-type data parsed from Enemies.txt
/// Each block: col_w, col_h, then 11 frames (x,y,w,h each)
/// Frames 0-3: walk right, frames 4: dead, frames 5-8: walk left, 9-10: extra
#[derive(Clone)]
pub struct EnemyType {
    pub col_w: f32,
    pub col_h: f32,
    pub frames: Vec<Frame>, // 11 frames per type
}

#[derive(Resource)]
pub struct EnemyFrames(pub Vec<EnemyType>);

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StompEvent>()
           .add_systems(OnEnter(GameState::InGame), spawn_enemies)
           .add_systems(
               Update,
               (enemy_ai, stomp_detection, handle_stomp)
                   .chain()
                   .run_if(in_state(GameState::InGame)),
           );
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_enemies(
    mut commands: Commands,
    level:   Res<LevelData>,
    assets:  Res<GameAssets>,
) {
    let types = load_enemy_frames("../base/c64/Enemies.txt");
    commands.insert_resource(EnemyFrames(types.clone()));

    for i in 0..50usize {
        if level.stage.enemy_in_use[i] == 0 { continue; }

        let etype = (level.stage.enemy_type[i] as usize).saturating_sub(1);
        let (col_w, col_h, initial_rect) = if etype < types.len() {
            let t = &types[etype];
            let rect = types[etype].frames.first().map(|&(x, y, w, h)| {
                Rect::new(x as f32, y as f32, (x + w as i32) as f32, (y + h as i32) as f32)
            });
            (t.col_w, t.col_h, rect)
        } else {
            (32.0, 32.0, None)
        };

        let ex = level.stage.enemy_pos_x[i] as f32;
        let ey = -(level.stage.enemy_pos_y[i] as f32);

        commands.spawn((
            GameEntity,
            Enemy { enemy_type: etype, alive: true },
            EnemyDir(level.stage.enemy_direction[i]),
            EnemyAnim::default(),
            EnemyVelocity::default(),
            EnemyCollider { w: col_w, h: col_h },
            Sprite {
                image: assets.enemies.clone(),
                rect: initial_rect,
                custom_size: Some(Vec2::new(col_w, col_h)),
                ..default()
            },
            Transform::from_xyz(ex, ey, 0.5),
        ));
    }
}

// ── Enemy velocity (separate from player's) ───────────────────────────────────

#[derive(Component, Default)]
pub struct EnemyVelocity { pub vx: f32, pub vy: f32 }

// ── AI ────────────────────────────────────────────────────────────────────────

const ENEMY_SPEED:  f32 = 1.5;
const GRAVITY:      f32 = 0.6;
const TERMINAL_VEL: f32 = 15.0;

fn enemy_ai(
    mut q: Query<(
        &mut Transform, &mut EnemyVelocity, &mut EnemyDir,
        &mut EnemyAnim, &mut Sprite, &Enemy, &EnemyCollider,
    )>,
    frames_res: Res<EnemyFrames>,
    level: Res<LevelData>,
) {
    for (mut tf, mut vel, mut dir, mut anim, mut sprite, enemy, col) in &mut q {
        if !enemy.alive { continue; }

        // Gravity
        vel.vy -= GRAVITY;
        if vel.vy < -TERMINAL_VEL { vel.vy = -TERMINAL_VEL; }

        // Horizontal walk
        vel.vx = if dir.0 == 1 { ENEMY_SPEED } else { -ENEMY_SPEED };

        // X movement + wall reversal
        tf.translation.x += vel.vx;
        let old_vx = vel.vx;
        resolve_enemy_x(&mut tf, &mut vel, col, &level);
        if old_vx.abs() > 0.0 && vel.vx == 0.0 {
            dir.0 = 1 - dir.0; // reverse
        }

        // Y movement
        tf.translation.y += vel.vy;
        resolve_enemy_y(&mut tf, &mut vel, col, &level);

        // Animation tick
        anim.timer += 1;
        if anim.timer >= 10 {
            anim.timer = 0;
            anim.frame = (anim.frame + 1) % 4;
        }

        // Update sprite rect
        if enemy.enemy_type < frames_res.0.len() {
            let etype = &frames_res.0[enemy.enemy_type];
            // Walk-right frames: 0-3, walk-left frames: 5-8
            let frame_idx = if dir.0 == 1 { anim.frame } else { 5 + anim.frame };
            if let Some(&(x, y, w, h)) = etype.frames.get(frame_idx) {
                if w > 0 && h > 0 {
                    sprite.rect = Some(Rect::new(
                        x as f32, y as f32,
                        (x + w as i32) as f32, (y + h as i32) as f32,
                    ));
                    sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
                }
            }
        }
    }
}

fn resolve_enemy_x(tf: &mut Transform, vel: &mut EnemyVelocity, col: &EnemyCollider, level: &LevelData) {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    let (w, h) = (col.w, col.h);
    for tx in (px / 16.0).floor() as i32 ..= ((px + w) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32 ..= ((py + h) / 16.0).floor() as i32 {
            if !is_solid(tx, ty, level) { continue; }
            let (bx, by) = (tx as f32 * 16.0, ty as f32 * 16.0);
            if px < bx + 16.0 && px + w > bx && py < by + 16.0 && py + h > by {
                if vel.vx > 0.0 { tf.translation.x = bx - w; vel.vx = 0.0; }
                else if vel.vx < 0.0 { tf.translation.x = bx + 16.0; vel.vx = 0.0; }
                return;
            }
        }
    }
}

fn resolve_enemy_y(tf: &mut Transform, vel: &mut EnemyVelocity, col: &EnemyCollider, level: &LevelData) {
    let px = tf.translation.x;
    let py = -tf.translation.y;
    let (w, h) = (col.w, col.h);
    for tx in (px / 16.0).floor() as i32 ..= ((px + w) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32 ..= ((py + h) / 16.0).floor() as i32 {
            if !is_solid(tx, ty, level) { continue; }
            let (bx, by) = (tx as f32 * 16.0, ty as f32 * 16.0);
            if px < bx + 16.0 && px + w > bx && py < by + 16.0 && py + h > by {
                if vel.vy < 0.0 { tf.translation.y = -(by - h); vel.vy = 0.0; }
                else if vel.vy > 0.0 { tf.translation.y = -(by + 16.0); vel.vy = 0.0; }
                return;
            }
        }
    }
}

// ── Stomp detection ───────────────────────────────────────────────────────────

fn stomp_detection(
    player_q:   Query<(&Transform, &PlayerVelocity, &OnGround), With<Player>>,
    enemy_q:    Query<(Entity, &Transform, &EnemyCollider, &Enemy)>,
    mut stomps: EventWriter<StompEvent>,
) {
    let Ok((ptf, pvel, _)) = player_q.get_single() else { return };
    let px = ptf.translation.x;
    let py = ptf.translation.y;

    for (entity, etf, ecol, enemy) in &enemy_q {
        if !enemy.alive { continue; }
        let ex = etf.translation.x;
        let ey = etf.translation.y;

        // Both in Bevy Y-up: player bottom = py - PLAYER_H, enemy top = ey
        let player_left   = px;
        let player_right  = px + PLAYER_W;
        let player_bottom = py - PLAYER_H;
        let enemy_left    = ex;
        let enemy_right   = ex + ecol.w;
        let enemy_top     = ey;
        let enemy_bottom  = ey - ecol.h;

        let overlap_x = player_left < enemy_right && player_right > enemy_left;
        let overlap_y = player_bottom < enemy_top && py > enemy_bottom;

        if overlap_x && overlap_y && pvel.vy < 0.0 {
            stomps.send(StompEvent { enemy: entity });
        }
    }
}

fn handle_stomp(
    mut stomps:  EventReader<StompEvent>,
    mut enemies: Query<(&mut Enemy, &mut Sprite)>,
    mut player:  Query<&mut PlayerVelocity, With<Player>>,
    mut hud:     ResMut<crate::hud::HudState>,
) {
    for ev in stomps.read() {
        if let Ok((mut enemy, mut sprite)) = enemies.get_mut(ev.enemy) {
            if !enemy.alive { continue; }
            enemy.alive = false;
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0); // hide
            hud.score += 100;
        }
        if let Ok(mut vel) = player.get_single_mut() {
            vel.vy = 8.0; // bounce
        }
    }
}

// ── Frame parsing ─────────────────────────────────────────────────────────────

fn load_enemy_frames(path: &str) -> Vec<EnemyType> {
    let mut result = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("Enemy: failed to read {}", path);
        return result;
    };

    // Split into blocks separated by blank lines
    for block in content.split("\n\n") {
        let block = block.trim();
        if block.is_empty() { continue; }

        let nums: Vec<i32> = block
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if nums.len() < 2 { continue; }
        let col_w = nums[0] as f32;
        let col_h = nums[1] as f32;

        let mut frames = Vec::new();
        for chunk in nums[2..].chunks(4) {
            if chunk.len() == 4 {
                frames.push((chunk[0], chunk[1], chunk[2] as u32, chunk[3] as u32));
            }
        }

        result.push(EnemyType { col_w, col_h, frames });
    }

    println!("Enemy: loaded {} types", result.len());
    result
}
