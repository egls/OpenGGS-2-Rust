use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
use crate::player::{
    is_solid, KillPlayerEvent, Player, PlayerState, Velocity as PlayerVelocity, PLAYER_H, PLAYER_W,
};
use crate::states::GameState;
use crate::tilemap::GameEntity;

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Enemy {
    /// 0-indexed into `EnemyFrames` (C++ enemy type 1..=15 -> 0..=14).
    pub enemy_type: usize,
    pub alive:      bool,
    pub active:     bool,
    pub start_x:    f32,
    pub start_y:    f32,
    pub phase:      f32,
}

/// Per-enemy-type behavior profile matching C++ `ENEMIES.cpp`.
#[derive(Component, Clone, Copy, Debug)]
pub struct EnemyBehavior {
    pub walker:          bool,
    pub flying:          bool,
    pub fall_off_cliffs: bool,
    pub jump_strength:   f32,
    pub stompable:       bool,
    pub shootable:       bool,
    pub special_dragon:  bool,
    pub special_spider:  bool,
}

impl EnemyBehavior {
    /// Maps 1-indexed C++ enemy type (`1..=15`) to its behavior profile.
    pub fn from_cpp_type(cpp_type: i32) -> Self {
        let mut b = Self {
            walker: true,
            flying: false,
            fall_off_cliffs: false,
            jump_strength: 0.0,
            stompable: true,
            shootable: true,
            special_dragon: false,
            special_spider: false,
        };
        match cpp_type {
            1 => b.fall_off_cliffs = true, // Owl
            2 => b.fall_off_cliffs = true, // Eye
            3 => b.fall_off_cliffs = false, // Jelly (stops at edges)
            4 => b.fall_off_cliffs = true, // Crab
            5 => {
                // Bee (flies)
                b.walker = false;
                b.flying = true;
            }
            6 => b.fall_off_cliffs = false, // Bug
            7 => {
                // Ball (bouncing hazard)
                b.walker = false;
                b.jump_strength = 9.0;
                b.stompable = false;
                b.shootable = false;
            }
            8 | 9 => b.fall_off_cliffs = true, // Hook 1 & 2
            10 => b.fall_off_cliffs = false,   // Hook 3
            11 => b.fall_off_cliffs = false,   // Robin Hood
            12 => {
                // Dragon
                b.walker = false;
                b.stompable = false;
                b.shootable = false;
                b.special_dragon = true;
            }
            13 => {
                // Spider
                b.walker = false;
                b.stompable = false;
                b.shootable = false;
                b.special_spider = true;
            }
            14 => {
                // Piranha (leaping hazard)
                b.walker = false;
                b.jump_strength = 12.5;
                b.stompable = false;
                b.shootable = false;
            }
            15 => {
                // Pipe
                b.fall_off_cliffs = true;
                b.stompable = false;
                b.shootable = false;
            }
            _ => {}
        }
        b
    }
}

#[derive(Component)]
pub struct EnemyDir(pub i32); // 0 = left, 1 = right

#[derive(Component, Default)]
pub struct EnemyAnim {
    pub frame: usize, // 0..3 walk cycle
    pub timer: u32,
}

#[derive(Component)]
pub struct EnemyCollider {
    pub w: f32,
    pub h: f32,
}

#[derive(Component, Default)]
pub struct EnemyVelocity {
    pub vx: f32,
    pub vy: f32,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[derive(Event)]
pub struct StompEvent {
    pub enemy: Entity,
}

// ── Frame data ────────────────────────────────────────────────────────────────

/// One frame: pixel rect on the spritesheet
pub type Frame = (i32, i32, u32, u32);

/// Per-enemy-type data parsed from Enemies.txt
/// Each block: col_w, col_h, then 11 frames (x,y,w,h each)
/// Frames 0-3: walk right, frame 4: dead, frames 5-8: walk left, 9-10: extra
#[derive(Clone)]
pub struct EnemyType {
    pub col_w:  f32,
    pub col_h:  f32,
    pub frames: Vec<Frame>,
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
               FixedUpdate,
               (enemy_ai, player_enemy_collision, handle_stomp)
                   .chain()
                   .run_if(in_state(GameState::InGame)),
           );
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_enemies(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    let types = load_enemy_frames("../base/c64/Enemies.txt");
    commands.insert_resource(EnemyFrames(types.clone()));

    for i in 0..50usize {
        if level.stage.enemy_in_use[i] == 0 { continue; }

        let cpp_type = level.stage.enemy_type[i];
        let etype = (cpp_type as usize).saturating_sub(1);
        let behavior = EnemyBehavior::from_cpp_type(cpp_type);

        let (col_w, col_h, initial_rect) = if etype < types.len() {
            let t = &types[etype];
            let rect = t.frames.first().map(|&(x, y, w, h)| {
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
            Enemy {
                enemy_type: etype,
                alive: true,
                active: false,
                start_x: ex,
                start_y: ey,
                phase: i as f32 * 0.7,
            },
            behavior,
            EnemyDir(level.stage.enemy_direction[i]),
            EnemyAnim::default(),
            EnemyVelocity::default(),
            EnemyCollider { w: col_w, h: col_h },
            Sprite {
                image: assets.enemies.clone(),
                rect: initial_rect,
                custom_size: Some(Vec2::new(col_w, col_h)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ex, ey, 0.5),
        ));
    }
}

// ── AI ────────────────────────────────────────────────────────────────────────

const ENEMY_SPEED:  f32 = 1.6;
const GRAVITY:      f32 = 0.6;
const TERMINAL_VEL: f32 = 15.0;

fn enemy_ai(
    cam_q: Query<&Transform, (With<PersistentCamera>, Without<Enemy>)>,
    mut q: Query<(
        &mut Transform,
        &mut EnemyVelocity,
        &mut EnemyDir,
        &mut EnemyAnim,
        &mut Sprite,
        &mut Enemy,
        &EnemyBehavior,
        &EnemyCollider,
    )>,
    frames_res: Res<EnemyFrames>,
    level:      Res<LevelData>,
) {
    let cam_x = cam_q.get_single().map(|t| t.translation.x).unwrap_or(400.0);

    for (mut tf, mut vel, mut dir, mut anim, mut sprite, mut enemy, beh, col) in &mut q {
        if !enemy.alive { continue; }

        // Activate when within screen window ahead of camera, deactivate far behind
        let dx_cam = tf.translation.x - cam_x;
        enemy.active = dx_cam > -520.0 && dx_cam < 560.0;
        if !enemy.active {
            continue;
        }

        enemy.phase += 0.06;

        if beh.special_dragon {
            // Dragon hovers in a sinusoidal patrol near start position
            tf.translation.x = enemy.start_x + (enemy.phase * 0.7).sin() * 64.0;
            tf.translation.y = enemy.start_y + (enemy.phase * 1.4).cos() * 24.0;
            dir.0 = if (enemy.phase * 0.7).cos() >= 0.0 { 1 } else { 0 };
        } else if beh.special_spider {
            // Spider moves up and down on its web thread
            tf.translation.y = enemy.start_y - ((enemy.phase * 0.9).sin() * 0.5 + 0.5) * 96.0;
        } else if beh.flying {
            // Bee flies horizontally with gentle bobbing, reversing on walls
            vel.vx = if dir.0 == 1 { ENEMY_SPEED * 1.1 } else { -ENEMY_SPEED * 1.1 };
            tf.translation.x += vel.vx;
            let old_vx = vel.vx;
            resolve_enemy_x(&mut tf, &mut vel, col, &level);
            if old_vx.abs() > 0.0 && vel.vx == 0.0 {
                dir.0 = 1 - dir.0;
            }
            tf.translation.y = enemy.start_y + enemy.phase.sin() * 14.0;
        } else if beh.jump_strength > 0.0 && !beh.walker {
            // Ball / Piranha: periodic vertical jump from start_y
            vel.vy -= GRAVITY * 0.75;
            if vel.vy < -TERMINAL_VEL { vel.vy = -TERMINAL_VEL; }
            tf.translation.y += vel.vy;
            if tf.translation.y <= enemy.start_y {
                tf.translation.y = enemy.start_y;
                vel.vy = beh.jump_strength;
            }
        } else if beh.walker {
            // Gravity
            vel.vy -= GRAVITY;
            if vel.vy < -TERMINAL_VEL { vel.vy = -TERMINAL_VEL; }

            // Edge-stopping check for enemies with !fall_off_cliffs (Jelly, Bug, Hook3, RobinHood)
            if !beh.fall_off_cliffs {
                let probe_x = if dir.0 == 1 {
                    tf.translation.x + col.w + 2.0
                } else {
                    tf.translation.x - 2.0
                };
                let feet_y = -tf.translation.y + col.h;
                let tx_below = (probe_x / 16.0).floor() as i32;
                let ty_below = ((feet_y + 4.0) / 16.0).floor() as i32;
                let on_floor = is_solid(((tf.translation.x + col.w * 0.5) / 16.0).floor() as i32, ty_below, &level);
                if on_floor && !is_solid(tx_below, ty_below, &level) {
                    dir.0 = 1 - dir.0;
                }
            }

            vel.vx = if dir.0 == 1 { ENEMY_SPEED } else { -ENEMY_SPEED };

            // X movement + wall reversal
            tf.translation.x += vel.vx;
            let old_vx = vel.vx;
            resolve_enemy_x(&mut tf, &mut vel, col, &level);
            if old_vx.abs() > 0.0 && vel.vx == 0.0 {
                dir.0 = 1 - dir.0;
            }

            // Stage boundary reversal
            if tf.translation.x < 16.0 {
                tf.translation.x = 16.0;
                dir.0 = 1;
            } else if tf.translation.x > 256.0 * 16.0 - col.w - 16.0 {
                dir.0 = 0;
            }

            // Y movement
            tf.translation.y += vel.vy;
            resolve_enemy_y(&mut tf, &mut vel, col, &level);

            if tf.translation.y < -600.0 {
                enemy.alive = false;
                continue;
            }
        }

        // Animation tick
        anim.timer += 1;
        if anim.timer >= 10 {
            anim.timer = 0;
            anim.frame = (anim.frame + 1) % 4;
        }

        // Update sprite rect
        if enemy.enemy_type < frames_res.0.len() {
            let etype = &frames_res.0[enemy.enemy_type];
            let frame_idx = if dir.0 == 1 { anim.frame } else { 5 + anim.frame };
            if let Some(&(x, y, w, h)) = etype.frames.get(frame_idx) {
                if w > 0 && h > 0 {
                    sprite.rect = Some(Rect::new(
                        x as f32,
                        y as f32,
                        (x + w as i32) as f32,
                        (y + h as i32) as f32,
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
    for tx in (px / 16.0).floor() as i32 ..= ((px + w - 0.01) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32 ..= ((py + h - 0.01) / 16.0).floor() as i32 {
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
    for tx in (px / 16.0).floor() as i32 ..= ((px + w - 0.01) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32 ..= ((py + h - 0.01) / 16.0).floor() as i32 {
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

// ── Player-Enemy Collision (Stomp vs. Damage) ─────────────────────────────────

fn player_enemy_collision(
    player_q:    Query<(&Transform, &PlayerVelocity, &PlayerState), With<Player>>,
    enemy_q:     Query<(Entity, &Transform, &EnemyCollider, &EnemyBehavior, &Enemy)>,
    mut stomps:  EventWriter<StompEvent>,
    mut kill_ev: EventWriter<KillPlayerEvent>,
) {
    let Ok((ptf, pvel, pstate)) = player_q.get_single() else { return };
    if !pstate.alive {
        return;
    }

    let px = ptf.translation.x;
    let py = ptf.translation.y;
    let player_left   = px + 2.0;
    let player_right  = px + PLAYER_W - 2.0;
    let player_top    = py - 2.0;
    let player_bottom = py - PLAYER_H;

    for (entity, etf, ecol, beh, enemy) in &enemy_q {
        if !enemy.alive { continue; }
        let ex = etf.translation.x;
        let ey = etf.translation.y;
        let enemy_left   = ex + 2.0;
        let enemy_right  = ex + ecol.w - 2.0;
        let enemy_top    = ey;
        let enemy_bottom = ey - ecol.h;

        let overlap_x = player_left < enemy_right && player_right > enemy_left;
        let overlap_y = player_bottom < enemy_top && player_top > enemy_bottom;

        if overlap_x && overlap_y {
            let landing_from_above = pvel.vy < 0.0 && player_bottom >= enemy_top - ecol.h * 0.55;
            if beh.stompable && landing_from_above {
                stomps.send(StompEvent { enemy: entity });
            } else if pstate.invuln_timer <= 0.0 {
                kill_ev.send(KillPlayerEvent);
                return;
            }
        }
    }
}

fn handle_stomp(
    mut stomps:  EventReader<StompEvent>,
    mut enemies: Query<(&mut Enemy, &mut Sprite)>,
    mut player:  Query<&mut PlayerVelocity, With<Player>>,
    mut hud:     ResMut<HudState>,
    mut sfx:     EventWriter<SfxEvent>,
    frames:      Res<EnemyFrames>,
) {
    let mut bounced = false;
    for ev in stomps.read() {
        if let Ok((mut enemy, mut sprite)) = enemies.get_mut(ev.enemy) {
            if !enemy.alive { continue; }
            enemy.alive = false;

            // Show dead frame (index 4)
            if enemy.enemy_type < frames.0.len() {
                let etype = &frames.0[enemy.enemy_type];
                if let Some(&(x, y, w, h)) = etype.frames.get(4) {
                    sprite.rect = Some(Rect::new(
                        x as f32,
                        y as f32,
                        (x + w as i32) as f32,
                        (y + h as i32) as f32,
                    ));
                    sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
                }
            }

            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.5);
            hud.score += 100;
            sfx.send(SfxEvent(SfxKind::Stomp));
            bounced = true;
        }
    }
    if bounced {
        if let Ok(mut vel) = player.get_single_mut() {
            vel.vy = 8.5;
        }
    }
}

// ── Frame parsing ─────────────────────────────────────────────────────────────

pub(crate) fn load_enemy_frames(path: &str) -> Vec<EnemyType> {
    let mut result = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("Enemy: failed to read {}", path);
        return result;
    };

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

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_enemy_frames_parses_all_types() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/c64/Enemies.txt");
        let types = load_enemy_frames(path.to_str().unwrap());
        assert_eq!(types.len(), 15, "expected 15 enemy types in Enemies.txt");

        for (i, etype) in types.iter().enumerate() {
            assert!(etype.col_w > 0.0 && etype.col_h > 0.0, "enemy type {} has invalid collider", i);
            assert!(etype.frames.len() >= 9, "enemy type {} missing walk/dead frames", i);
        }
    }

    #[test]
    fn enemy_behavior_matches_cpp_definitions() {
        // Owl falls off cliffs and is stompable
        let owl = EnemyBehavior::from_cpp_type(1);
        assert!(owl.walker && owl.fall_off_cliffs && owl.stompable && owl.shootable);

        // Jelly stops at cliffs
        let jelly = EnemyBehavior::from_cpp_type(3);
        assert!(jelly.walker && !jelly.fall_off_cliffs && jelly.stompable);

        // Bee flies
        let bee = EnemyBehavior::from_cpp_type(5);
        assert!(bee.flying && !bee.walker && bee.stompable);

        // Ball, Dragon, Spider, Piranha, Pipe are non-stompable & non-shootable
        for t in [7, 12, 13, 14, 15] {
            let b = EnemyBehavior::from_cpp_type(t);
            assert!(!b.stompable && !b.shootable, "enemy type {} should be non-stompable/shootable", t);
        }
    }
}
