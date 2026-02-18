use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData};
use crate::tilemap::GameEntity;
use crate::player::{Player, Velocity as PlayerVelocity, OnGround, Frame};

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: u32,
    pub alive: bool,
}

#[derive(Component)]
pub struct EnemyDirection(pub i32); // 0 = left, 1 = right

#[derive(Component, Default)]
pub struct EnemyAnim {
    pub frame: usize,
    pub timer: u32,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[derive(Event)]
pub struct StompEvent {
    pub enemy: Entity,
}

// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct EnemyFrameData {
    pub col_w: f32,
    pub col_h: f32,
    pub frames_right: [Frame; 4],
    pub frames_left:  [Frame; 4],
    pub frame_dead:   Frame,
}

#[derive(Resource)]
pub struct EnemyFrames(pub Vec<EnemyFrameData>);

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StompEvent>()
           .add_systems(OnEnter(GameState::InGame), spawn_enemies)
           .add_systems(
               Update,
               (
                   enemy_ai,
                   stomp_detection,
                   handle_stomp,
               )
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
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let frames = load_enemy_frames("../base/c64/Enemies.txt");

    // Build a shared atlas layout (enemies sheet is one big image)
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(32, 32),
        10,
        20,
        None,
        None,
    );
    let layout_handle = layouts.add(layout);

    commands.insert_resource(EnemyFrames(frames.clone()));

    const MAX_ENEMIES: usize = 50;
    for i in 0..MAX_ENEMIES {
        if level.stage.enemy_in_use[i] == 0 { continue; }

        let etype = level.stage.enemy_type[i] as u32;
        let (col_w, col_h) = if etype >= 1 && (etype as usize - 1) < frames.len() {
            let ef = &frames[etype as usize - 1];
            (ef.col_w, ef.col_h)
        } else {
            (32.0, 32.0)
        };

        let ex = level.stage.enemy_pos_x[i] as f32;
        let ey = -(level.stage.enemy_pos_y[i] as f32); // Bevy Y-up

        commands.spawn((
            GameEntity,
            Enemy { enemy_type: etype, alive: true },
            EnemyDirection(level.stage.enemy_direction[i]),
            EnemyAnim::default(),
            Velocity { vx: 0.0, vy: 0.0 },
            EnemyCollider { width: col_w, height: col_h },
            Sprite {
                image: assets.enemies.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout_handle.clone(),
                    index: 0,
                }),
                ..default()
            },
            Transform::from_xyz(ex, ey, 0.5),
        ));
    }
    println!("Enemy: spawned enemies from level data");
}

// ── Enemy-specific components ─────────────────────────────────────────────────

#[derive(Component)]
pub struct Velocity { pub vx: f32, pub vy: f32 }

#[derive(Component)]
pub struct EnemyCollider { pub width: f32, pub height: f32 }

// ── AI ────────────────────────────────────────────────────────────────────────

const ENEMY_SPEED:    f32 = 2.5;
const GRAVITY:        f32 = 1.0;
const TERMINAL_VEL:   f32 = 15.0;

fn enemy_ai(
    mut q: Query<(&mut Transform, &mut Velocity, &mut EnemyDirection, &mut EnemyAnim, &Enemy)>,
    level: Res<LevelData>,
) {
    for (mut tf, mut vel, mut dir, mut anim, enemy) in &mut q {
        if !enemy.alive { continue; }

        // Gravity
        vel.vy -= GRAVITY;
        if vel.vy < -TERMINAL_VEL { vel.vy = -TERMINAL_VEL; }

        // Walk
        vel.vx = if dir.0 == 1 { ENEMY_SPEED } else { -ENEMY_SPEED };

        // X move + wall check
        tf.translation.x += vel.vx;
        let old_vx = vel.vx;
        resolve_enemy_x(&mut tf, &mut vel, &level);
        if old_vx != 0.0 && vel.vx == 0.0 {
            dir.0 = if dir.0 == 1 { 0 } else { 1 };
        }

        // Y move
        tf.translation.y += vel.vy;
        resolve_enemy_y(&mut tf, &mut vel, &level);

        // Animation
        anim.timer += 1;
        if anim.timer >= 8 {
            anim.timer = 0;
            anim.frame = (anim.frame + 1) % 4;
        }
    }
}

fn resolve_enemy_x(tf: &mut Transform, vel: &mut Velocity, level: &LevelData) {
    let w = 32.0f32;
    let h = 32.0f32;
    let px = tf.translation.x;
    let py = -tf.translation.y;
    for tx in (px / 16.0).floor() as i32..=((px + w) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32..=((py + h) / 16.0).floor() as i32 {
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

fn resolve_enemy_y(tf: &mut Transform, vel: &mut Velocity, level: &LevelData) {
    let w = 32.0f32;
    let h = 32.0f32;
    let px = tf.translation.x;
    let py = -tf.translation.y;
    for tx in (px / 16.0).floor() as i32..=((px + w) / 16.0).floor() as i32 {
        for ty in (py / 16.0).floor() as i32..=((py + h) / 16.0).floor() as i32 {
            if !is_solid(tx, ty, level) { continue; }
            let tile_x = tx as f32 * 16.0;
            let tile_y = ty as f32 * 16.0;
            if px < tile_x + 16.0 && px + w > tile_x && py < tile_y + 16.0 && py + h > tile_y {
                if vel.vy < 0.0 { tf.translation.y = -(tile_y - h); vel.vy = 0.0; }
                else if vel.vy > 0.0 { tf.translation.y = -(tile_y + 16.0); vel.vy = 0.0; }
                return;
            }
        }
    }
}

fn is_solid(tx: i32, ty: i32, level: &LevelData) -> bool {
    if tx < 0 || tx >= 256 || ty < 0 || ty >= 30 { return false; }
    let id = level.stage.array[tx as usize][ty as usize] as usize;
    if id >= 2320 { return false; }
    level.tile_info.solid[id] != 0
}

// ── Stomp detection ───────────────────────────────────────────────────────────

fn stomp_detection(
    player_q:  Query<(&Transform, &PlayerVelocity, &OnGround), With<Player>>,
    enemy_q:   Query<(Entity, &Transform, &EnemyCollider, &Enemy)>,
    mut stomps: EventWriter<StompEvent>,
) {
    let Ok((ptf, pvel, _)) = player_q.get_single() else { return };

    let pw = 28.0f32;
    let ph = 42.0f32;
    let px1 = ptf.translation.x;
    let py1 = ptf.translation.y;
    let px2 = px1 + pw;
    let py2 = py1 - ph; // Bevy Y-up: bottom of player is lower Y

    for (entity, etf, ecol, enemy) in &enemy_q {
        if !enemy.alive { continue; }
        let ex1 = etf.translation.x;
        let ey1 = etf.translation.y;
        let ex2 = ex1 + ecol.width;
        let ey2 = ey1 - ecol.height;

        // AABB overlap
        if px1 < ex2 && px2 > ex1 && py1 > ey2 && py2 < ey1 {
            // Stomp: player falling (vy < 0 in Bevy Y-up) and player bottom near enemy top
            if pvel.vy < 0.0 && py2 >= ey1 - 8.0 {
                stomps.send(StompEvent { enemy: entity });
            }
        }
    }
}

fn handle_stomp(
    mut stomps:  EventReader<StompEvent>,
    mut enemies: Query<(&mut Enemy, &mut Sprite)>,
    mut player:  Query<&mut PlayerVelocity, With<Player>>,
    mut hud:     ResMut<crate::hud::HudState>,
    assets:      Res<GameAssets>,
) {
    for ev in stomps.read() {
        if let Ok((mut enemy, mut sprite)) = enemies.get_mut(ev.enemy) {
            enemy.alive = false;
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.3); // fade out
            hud.score += 100;
        }
        if let Ok(mut vel) = player.get_single_mut() {
            vel.vy = 8.0; // bounce up
        }
        // Play stomp sound
        // (audio spawned in audio.rs via StompEvent listener)
    }
}

// ── Enemy frame parsing ───────────────────────────────────────────────────────

fn load_enemy_frames(path: &str) -> Vec<EnemyFrameData> {
    let mut result = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("Enemy: failed to read {}", path);
        return result;
    };
    let nums: Vec<i32> = content
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    // 46 ints per enemy type: 2 (col_w, col_h) + 4*4 (right frames) + 4*4 (left frames) + 4 (dead frame) + 4 (extra)
    for chunk in nums.chunks(46) {
        if chunk.len() < 46 { break; }
        let col_w = chunk[0] as f32;
        let col_h = chunk[1] as f32;

        let mut fr = [(0i32, 0i32, 0u32, 0u32); 4];
        let mut fl = [(0i32, 0i32, 0u32, 0u32); 4];
        for i in 0..4 {
            let base = 2 + i * 4;
            fr[i] = (chunk[base], chunk[base+1], chunk[base+2] as u32, chunk[base+3] as u32);
        }
        for i in 0..4 {
            let base = 18 + i * 4;
            fl[i] = (chunk[base], chunk[base+1], chunk[base+2] as u32, chunk[base+3] as u32);
        }
        let dead = (chunk[34], chunk[35], chunk[36] as u32, chunk[37] as u32);

        result.push(EnemyFrameData {
            col_w, col_h,
            frames_right: fr,
            frames_left:  fl,
            frame_dead:   dead,
        });
    }
    println!("Enemy: loaded {} types", result.len());
    result
}
