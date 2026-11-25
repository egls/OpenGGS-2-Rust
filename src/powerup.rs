use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera};
use crate::audio::{SfxEvent, SfxKind};
use crate::enemy::{Enemy, EnemyBehavior, EnemyCollider, EnemyFrames};
use crate::hud::HudState;
use crate::player::{aabb, is_solid, Direction, Player, PlayerState, PLAYER_H, PLAYER_W};
use crate::states::{not_paused, GameState};
use crate::tilemap::GameEntity;

pub type Frame = (i32, i32, u32, u32);

// ── Resources & Events ────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PowerUpFrames {
    /// 8 power-up sprites (index 0..8 correspond to levels 1..=8).
    pub items: Vec<Frame>,
    /// Bullet / fireball sprite (line 9 of PowerUp.txt).
    pub bullet: Frame,
}

#[derive(Event)]
pub struct SpawnPowerUpEvent {
    pub x: f32,
    pub y: f32,
}

#[derive(Event)]
pub struct SpawnCoinPopEvent {
    pub x: f32,
    pub y: f32,
}

#[derive(Event)]
pub struct FireBulletEvent {
    pub x: f32,
    pub y: f32,
    pub dir: Direction,
    pub power_up: u8,
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct PowerUpItem {
    pub level: u8,
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Component)]
pub struct CoinPop {
    pub vy: f32,
    pub timer: f32,
}

#[derive(Component)]
pub struct Bullet {
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
    pub h: f32,
    pub power_up: u8,
    pub lifetime: f32,
    pub anim_tick: u32,
}

/// 2-frame bullet impact explosion (`PowerUp.png` at y=272, 32x32 each; matches `GRAPHICS_Sprites_Draw.cpp:138-156`).
#[derive(Component)]
pub struct BulletExplosion {
    pub timer:    f32,
    pub duration: f32,
}

pub fn bullet_explosion_rect(timer: f32, duration: f32) -> Rect {
    let x = if timer < duration * 0.5 { 32.0 } else { 0.0 };
    Rect::new(x, 272.0, x + 32.0, 304.0)
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PowerUpPlugin;

impl Plugin for PowerUpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerUpFrames>()
           .add_event::<SpawnPowerUpEvent>()
           .add_event::<SpawnCoinPopEvent>()
           .add_event::<FireBulletEvent>()
           .add_systems(Startup, load_powerup_sprites_system)
           .add_systems(
               FixedUpdate,
               (
                   handle_spawn_powerup,
                   handle_spawn_coin_pop,
                   handle_fire_bullet,
                   move_powerups,
                   collect_powerups,
                   move_coin_pops,
                   move_bullets,
                   tick_bullet_explosions,
               )
                   .chain()
                   .run_if(in_state(GameState::InGame).and(not_paused)),
           );
    }
}

// ── Frame loading ─────────────────────────────────────────────────────────────

fn load_powerup_sprites_system(mut commands: Commands, paths: Res<crate::paths::AssetPaths>) {
    let frames = load_powerup_frames(paths.c64_file("PowerUp.txt").to_str().unwrap_or(""));
    commands.insert_resource(frames);
}

pub(crate) fn load_powerup_frames(path: &str) -> PowerUpFrames {
    let Ok(content) = std::fs::read_to_string(path) else {
        eprintln!("PowerUp: failed to read {}", path);
        return PowerUpFrames {
            items: Vec::new(),
            bullet: (0, 256, 16, 16),
        };
    };

    let nums: Vec<i32> = content
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let mut all: Vec<Frame> = Vec::new();
    for chunk in nums.chunks(4) {
        if chunk.len() == 4 {
            all.push((chunk[0], chunk[1], chunk[2] as u32, chunk[3] as u32));
        }
    }

    let bullet = all.get(8).copied().unwrap_or((0, 256, 16, 16));
    let items = all.into_iter().take(8).collect();
    PowerUpFrames { items, bullet }
}

// ── Spawning ──────────────────────────────────────────────────────────────────

fn handle_spawn_powerup(
    mut commands: Commands,
    mut events:   EventReader<SpawnPowerUpEvent>,
    assets:       Res<GameAssets>,
    frames:       Res<PowerUpFrames>,
    hud:          Res<HudState>,
    existing:     Query<Entity, With<PowerUpItem>>,
) {
    for ev in events.read() {
        // C++ SPRITE_PowerUp_Activate keeps one active power-up at a time.
        for e in &existing {
            commands.entity(e).despawn();
        }

        let level = (hud.power_up + 1).min(8);
        let idx = (level as usize).saturating_sub(1);
        let (sx, sy, sw, sh) = frames.items.get(idx).copied().unwrap_or((0, 0, 32, 32));
        let w = sw as f32;
        let h = sh as f32;

        commands.spawn((
            GameEntity,
            PowerUpItem {
                level,
                vx: 1.8,
                vy: 4.5,
                w,
                h,
            },
            Sprite {
                image: assets.powerups.clone(),
                rect: Some(Rect::new(
                    sx as f32,
                    sy as f32,
                    (sx + sw as i32) as f32,
                    (sy + sh as i32) as f32,
                )),
                custom_size: Some(Vec2::new(w, h)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ev.x, ev.y, 0.8),
        ));
    }
}

fn handle_spawn_coin_pop(
    mut commands: Commands,
    mut events:   EventReader<SpawnCoinPopEvent>,
    assets:       Res<GameAssets>,
) {
    // Small coin tile in Tiles.png (Tile_SmallCoin = 45 in C++ CONTENT_Tiles_and_Sprites.h)
    let rect = crate::tilemap::tile_source_rect(45);
    for ev in events.read() {
        commands.spawn((
            GameEntity,
            CoinPop { vy: 6.5, timer: 0.45 },
            Sprite {
                image: assets.tiles.clone(),
                rect: Some(rect),
                custom_size: Some(Vec2::new(16.0, 16.0)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ev.x, ev.y, 0.9),
        ));
    }
}

fn handle_fire_bullet(
    mut commands: Commands,
    mut events:   EventReader<FireBulletEvent>,
    assets:       Res<GameAssets>,
    frames:       Res<PowerUpFrames>,
    existing:     Query<Entity, With<Bullet>>,
    mut sfx:      EventWriter<SfxEvent>,
) {
    let mut active_count = existing.iter().count();
    for ev in events.read() {
        if active_count >= 1 {
            continue;
        }
        active_count += 1;

        let (sx, sy, sw, sh) = frames.bullet;
        let vx = match ev.dir {
            Direction::Right => 6.0,
            Direction::Left  => -6.0,
        };

        sfx.send(SfxEvent(SfxKind::Shoot));
        commands.spawn((
            GameEntity,
            Bullet {
                vx,
                vy: -2.0,
                w: sw as f32,
                h: sh as f32,
                power_up: ev.power_up,
                lifetime: 3.0,
                anim_tick: 0,
            },
            Sprite {
                image: assets.powerups.clone(),
                rect: Some(Rect::new(
                    sx as f32,
                    sy as f32,
                    (sx + sw as i32) as f32,
                    (sy + sh as i32) as f32,
                )),
                custom_size: Some(Vec2::new(sw as f32, sh as f32)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ev.x, ev.y, 0.9),
        ));
    }
}

// ── Simulation ────────────────────────────────────────────────────────────────

fn move_powerups(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut PowerUpItem)>,
    level: Res<LevelData>,
) {
    const GRAVITY: f32 = 0.45;
    const TERMINAL_VEL: f32 = 10.0;

    for (entity, mut tf, mut item) in &mut q {
        item.vy = (item.vy - GRAVITY).max(-TERMINAL_VEL);

        // X axis + wall bounce
        tf.translation.x += item.vx;
        let px = tf.translation.x;
        let py = -tf.translation.y;
        let mut hit_x = false;
        for tx in (px / 16.0).floor() as i32 ..= ((px + item.w) / 16.0).floor() as i32 {
            for ty in (py / 16.0).floor() as i32 ..= ((py + item.h) / 16.0).floor() as i32 {
                if !is_solid(tx, ty, &level) { continue; }
                let (bx, by) = (tx as f32 * 16.0, ty as f32 * 16.0);
                if aabb(px, py, item.w, item.h, bx, by, 16.0, 16.0) {
                    if item.vx > 0.0 {
                        tf.translation.x = bx - item.w;
                    } else {
                        tf.translation.x = bx + 16.0;
                    }
                    item.vx = -item.vx;
                    hit_x = true;
                    break;
                }
            }
            if hit_x { break; }
        }

        // Y axis
        tf.translation.y += item.vy;
        let px = tf.translation.x;
        let py = -tf.translation.y;
        let mut hit_y = false;
        for tx in (px / 16.0).floor() as i32 ..= ((px + item.w) / 16.0).floor() as i32 {
            for ty in (py / 16.0).floor() as i32 ..= ((py + item.h) / 16.0).floor() as i32 {
                if !is_solid(tx, ty, &level) { continue; }
                let (bx, by) = (tx as f32 * 16.0, ty as f32 * 16.0);
                if aabb(px, py, item.w, item.h, bx, by, 16.0, 16.0) {
                    if item.vy < 0.0 {
                        tf.translation.y = -(by - item.h);
                        item.vy = 0.0;
                    } else if item.vy > 0.0 {
                        tf.translation.y = -(by + 16.0);
                        item.vy = 0.0;
                    }
                    hit_y = true;
                    break;
                }
            }
            if hit_y { break; }
        }

        if tf.translation.y < -700.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn collect_powerups(
    mut commands: Commands,
    mut player_q: Query<(&Transform, &mut PlayerState), With<Player>>,
    items_q:      Query<(Entity, &Transform, &PowerUpItem)>,
    mut hud:      ResMut<HudState>,
    mut sfx:      EventWriter<SfxEvent>,
) {
    let Ok((ptf, mut pstate)) = player_q.get_single_mut() else { return };
    if !pstate.alive {
        return;
    }

    let px = ptf.translation.x;
    let py = -ptf.translation.y;

    for (entity, itf, item) in &items_q {
        let ix = itf.translation.x;
        let iy = -itf.translation.y;
        if aabb(px, py, PLAYER_W, PLAYER_H, ix, iy, item.w, item.h) {
            commands.entity(entity).despawn();
            hud.score += 250;

            if item.level >= 8 {
                hud.lives += 1;
                hud.power_up = hud.power_up.max(7);
                sfx.send(SfxEvent(SfxKind::Ding));
            } else {
                let was_normal = hud.power_up == 0;
                hud.power_up = hud.power_up.max(item.level).min(7);
                if was_normal && hud.power_up >= 1 {
                    pstate.morph_timer = 0.5;
                    sfx.send(SfxEvent(SfxKind::Morph));
                } else {
                    sfx.send(SfxEvent(SfxKind::Ding));
                }
            }
        }
    }
}

fn move_coin_pops(
    mut commands: Commands,
    time:         Res<Time<Fixed>>,
    mut q:        Query<(Entity, &mut Transform, &mut CoinPop)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tf, mut pop) in &mut q {
        pop.vy -= 0.45;
        tf.translation.y += pop.vy;
        pop.timer -= dt;
        if pop.timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_bullet_explosion(commands: &mut Commands, assets: &GameAssets, x: f32, y: f32) {
    const DURATION: f32 = 0.22;
    commands.spawn((
        GameEntity,
        BulletExplosion {
            timer: DURATION,
            duration: DURATION,
        },
        Sprite {
            image: assets.powerups.clone(),
            rect: Some(bullet_explosion_rect(DURATION, DURATION)),
            custom_size: Some(Vec2::new(32.0, 32.0)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(x, y + 8.0, 0.95),
    ));
}

fn tick_bullet_explosions(
    mut commands: Commands,
    time:         Res<Time<Fixed>>,
    mut q:        Query<(Entity, &mut BulletExplosion, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (ent, mut exp, mut sprite) in &mut q {
        exp.timer -= dt;
        if exp.timer <= 0.0 {
            commands.entity(ent).despawn();
        } else {
            sprite.rect = Some(bullet_explosion_rect(exp.timer, exp.duration));
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn move_bullets(
    mut commands: Commands,
    time:         Res<Time<Fixed>>,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
    pframes:      Res<PowerUpFrames>,
    cam_q:        Query<&Transform, (With<PersistentCamera>, Without<Bullet>, Without<Enemy>)>,
    mut bullet_q: Query<(Entity, &mut Transform, &mut Bullet, &mut Sprite), Without<Enemy>>,
    mut enemy_q:  Query<(&Transform, &EnemyCollider, &EnemyBehavior, &mut Enemy, &mut crate::enemy::EnemyVelocity, &mut Sprite), Without<Bullet>>,
    frames:       Res<EnemyFrames>,
    mut hud:      ResMut<HudState>,
    mut sfx:      EventWriter<SfxEvent>,
) {
    let dt = time.delta_secs();
    let cam_x = cam_q.get_single().map(|t| t.translation.x).unwrap_or(400.0);

    for (b_ent, mut btf, mut bullet, mut bsprite) in &mut bullet_q {
        bullet.lifetime -= dt;
        if bullet.lifetime <= 0.0 {
            commands.entity(b_ent).despawn();
            continue;
        }

        // Animate bullet across 4 horizontal frames (matches C++ GRAPHICS_Sprites_Draw.cpp:121)
        bullet.anim_tick = bullet.anim_tick.wrapping_add(1);
        let frame_idx = ((bullet.anim_tick / 4) % 4) as i32;
        let (bsx, bsy, bsw, bsh) = pframes.bullet;
        let fx = (bsx + frame_idx * bsw as i32) as f32;
        bsprite.rect = Some(Rect::new(fx, bsy as f32, fx + bsw as f32, (bsy + bsh as i32) as f32));

        // Homing behavior for PowerUp >= 4 (matches C++ SPRITE_Bullet_Move)
        if bullet.power_up >= 4 {
            let mut best_target: Option<Vec2> = None;
            let mut best_dist = 360.0f32 * 360.0f32;
            for (etf, ecol, beh, enemy, _, _) in &enemy_q {
                if !enemy.alive || !beh.shootable { continue; }
                let ex = etf.translation.x + ecol.w * 0.5;
                let ey = etf.translation.y - ecol.h * 0.5;
                let dx = ex - btf.translation.x;
                let dy = ey - btf.translation.y;
                let d2 = dx * dx + dy * dy;
                if d2 < best_dist {
                    best_dist = d2;
                    best_target = Some(Vec2::new(ex, ey));
                }
            }
            if let Some(target) = best_target {
                let dir = (target - btf.translation.truncate()).normalize_or_zero();
                bullet.vx = dir.x * 6.0;
                bullet.vy = dir.y * 6.0;
            }
        }

        let mut despawn = false;
        let mut explode = false;

        // X axis movement + solid wall check
        btf.translation.x += bullet.vx;
        let bx = btf.translation.x;
        let by = -btf.translation.y;
        'x_loop: for tx in (bx / 16.0).floor() as i32 ..= ((bx + bullet.w) / 16.0).floor() as i32 {
            for ty in (by / 16.0).floor() as i32 ..= ((by + bullet.h) / 16.0).floor() as i32 {
                if !is_solid(tx, ty, &level) { continue; }
                let (wx, wy) = (tx as f32 * 16.0, ty as f32 * 16.0);
                if aabb(bx, by, bullet.w, bullet.h, wx, wy, 16.0, 16.0) {
                    if bullet.power_up >= 3 {
                        // Ricochet off wall
                        if bullet.vx > 0.0 {
                            btf.translation.x = wx - bullet.w;
                        } else {
                            btf.translation.x = wx + 16.0;
                        }
                        bullet.vx = -bullet.vx;
                    } else {
                        despawn = true;
                        explode = true;
                    }
                    break 'x_loop;
                }
            }
        }

        if despawn {
            if explode {
                spawn_bullet_explosion(&mut commands, &assets, btf.translation.x, btf.translation.y);
            }
            commands.entity(b_ent).despawn();
            continue;
        }

        // Y axis movement + bounce off floor/ceiling
        btf.translation.y += bullet.vy;
        let bx = btf.translation.x;
        let by = -btf.translation.y;
        'y_loop: for tx in (bx / 16.0).floor() as i32 ..= ((bx + bullet.w) / 16.0).floor() as i32 {
            for ty in (by / 16.0).floor() as i32 ..= ((by + bullet.h) / 16.0).floor() as i32 {
                if !is_solid(tx, ty, &level) { continue; }
                let (wx, wy) = (tx as f32 * 16.0, ty as f32 * 16.0);
                if aabb(bx, by, bullet.w, bullet.h, wx, wy, 16.0, 16.0) {
                    if bullet.vy < 0.0 {
                        btf.translation.y = -(wy - bullet.h);
                    } else {
                        btf.translation.y = -(wy + 16.0);
                    }
                    bullet.vy = -bullet.vy;
                    break 'y_loop;
                }
            }
        }

        // Check enemy hits
        let bx = btf.translation.x;
        let by = -btf.translation.y;
        for (etf, ecol, beh, mut enemy, mut evel, mut sprite) in &mut enemy_q {
            if !enemy.alive || !beh.shootable { continue; }
            let ex = etf.translation.x;
            let ey = -etf.translation.y;
            if aabb(bx, by, bullet.w, bullet.h, ex, ey, ecol.w, ecol.h) {
                enemy.alive = false;
                evel.vx = 0.0;
                evel.vy = 2.5;
                if let Some(etype) = frames.0.get(enemy.enemy_type) {
                    if let Some(&(fx, fy, fw, fh)) = etype.frames.get(4) {
                        sprite.rect = Some(Rect::new(
                            fx as f32,
                            fy as f32,
                            (fx + fw as i32) as f32,
                            (fy + fh as i32) as f32,
                        ));
                        sprite.custom_size = Some(Vec2::new(fw as f32, fh as f32));
                    }
                }
                hud.score += 100;
                sfx.send(SfxEvent(SfxKind::Stomp));
                despawn = true;
                explode = true;
                break;
            }
        }

        // Off-screen cleanup
        if despawn
            || btf.translation.y > 32.0
            || btf.translation.y < -520.0
            || (btf.translation.x - cam_x).abs() > 460.0
        {
            if explode {
                spawn_bullet_explosion(&mut commands, &assets, btf.translation.x, btf.translation.y);
            }
            commands.entity(b_ent).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_powerup_frames_parses_items_and_bullet() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/c64/PowerUp.txt");
        let parsed = load_powerup_frames(path.to_str().unwrap());
        assert_eq!(parsed.items.len(), 8);
        assert!(parsed.items.iter().all(|&(_, _, w, h)| w > 0 && h > 0));
        assert_eq!(parsed.bullet, (0, 256, 16, 16));
    }

    #[test]
    fn bullet_explosion_switches_frames_at_midpoint() {
        let first = bullet_explosion_rect(0.20, 0.22);
        let second = bullet_explosion_rect(0.05, 0.22);
        assert_eq!(first, Rect::new(0.0, 272.0, 32.0, 304.0));
        assert_eq!(second, Rect::new(32.0, 272.0, 64.0, 304.0));
    }
}
