use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera, MAX_CLASSIC_STAGE};
use crate::hud::HudState;
use crate::states::GameState;

pub const TILE_W: f32 = 16.0;
pub const TILE_H: f32 = 16.0;
pub const TILES_PER_ROW: i32 = 40;
pub const STAGE_COLS: usize = 256;
pub const STAGE_ROWS: usize = 30;

// ── Marker components ─────────────────────────────────────────────────────────

/// Marks all entities that belong to the in-game world (tiles, player, enemies, items).
/// Used for bulk despawn on state exit.
#[derive(Component)]
pub struct GameEntity;

/// A single map tile.
#[derive(Component)]
pub struct Tile {
    pub tile_id: i32,
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

#[derive(Resource)]
struct StageTransitionTimer(Timer);

#[derive(Component)]
struct TransitionBannerRoot;

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn tile_source_rect(tile_id: i32) -> Rect {
    let col = tile_id % TILES_PER_ROW;
    let row = tile_id / TILES_PER_ROW;
    let src_x = col as f32 * TILE_W;
    let src_y = row as f32 * TILE_H;
    Rect::new(src_x, src_y, src_x + TILE_W, src_y + TILE_H)
}

/// Mutates tile `(tx, ty)` in both `LevelData.stage.array` and the live ECS `Sprite`/`TileGrid`.
pub fn set_tile<F: bevy::ecs::query::QueryFilter>(
    commands: &mut Commands,
    tile_grid: &mut TileGrid,
    level: &mut LevelData,
    tile_q: &mut Query<(&mut Tile, &mut Sprite), F>,
    assets: &GameAssets,
    tx: usize,
    ty: usize,
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

    if let Some(entity) = tile_grid.entities[tx][ty] {
        if let Ok((mut tile, mut sprite)) = tile_q.get_mut(entity) {
            tile.tile_id = new_tile_id;
            sprite.rect = Some(tile_source_rect(new_tile_id));
            return;
        }
    }

    let px = tx as f32 * TILE_W;
    let py = -(ty as f32 * TILE_H);
    let entity = commands
        .spawn((
            GameEntity,
            Tile {
                tile_id: new_tile_id,
            },
            Sprite {
                image: assets.tiles.clone(),
                rect: Some(tile_source_rect(new_tile_id)),
                custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(px, py, 0.0),
        ))
        .id();
    tile_grid.entities[tx][ty] = Some(entity);
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileGrid>()
           .add_systems(OnEnter(GameState::StageTransition), enter_stage_transition)
           .add_systems(
               Update,
               tick_stage_transition.run_if(in_state(GameState::StageTransition)),
           )
           .add_systems(OnExit(GameState::StageTransition), exit_stage_transition)
           .add_systems(OnEnter(GameState::InGame), (spawn_tiles, reset_camera))
           .add_systems(
               Update,
               (camera_follow, ingame_shortcuts).run_if(in_state(GameState::InGame)),
           )
           .add_systems(OnExit(GameState::InGame), despawn_game_entities);
    }
}

// ── Stage transition systems ──────────────────────────────────────────────────

fn enter_stage_transition(
    mut commands: Commands,
    mut level:    ResMut<LevelData>,
    hud:          Res<HudState>,
    mut next:     ResMut<NextState<GameState>>,
) {
    let target_stage = hud.stage.clamp(1, MAX_CLASSIC_STAGE);
    let checkpoint = level.checkpoint;
    if let Err(err) = level.load_stage_number(target_stage, checkpoint) {
        eprintln!("Failed to load stage {}: {}", target_stage, err);
        next.set(GameState::GameOver);
        return;
    }

    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.12)));
    commands.insert_resource(StageTransitionTimer(Timer::from_seconds(0.75, TimerMode::Once)));

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
                Text::new(format!("STAGE {}", target_stage)),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));
            if !stage_name.is_empty() {
                parent.spawn((
                    Text::new(stage_name),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(0.85, 0.85, 0.95)),
                ));
            }
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
        format!("Get ready for Stage {}", stage_num)
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

// ── In-game tilemap & camera systems ──────────────────────────────────────────

fn reset_camera(
    level: Res<LevelData>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
) {
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };
    let start = crate::player::player_start_position(&level);
    cam_tf.translation.x = clamp_camera_x(start.x);
    cam_tf.translation.y = -240.0;
}

fn clamp_camera_x(x: f32) -> f32 {
    let min_x = 400.0;
    let max_x = STAGE_COLS as f32 * TILE_W - 400.0;
    x.clamp(min_x, max_x)
}

fn spawn_tiles(
    mut commands:  Commands,
    mut tile_grid: ResMut<TileGrid>,
    level:         Res<LevelData>,
    assets:        Res<GameAssets>,
) {
    *tile_grid = TileGrid::default();

    let bg_color = match level.stage.background_colour {
        0 => Color::srgb(0.4, 0.6, 1.0), // Blue sky
        _ => Color::BLACK,
    };
    commands.insert_resource(ClearColor(bg_color));

    for x in 0..STAGE_COLS {
        for y in 0..STAGE_ROWS {
            let tile_id = level.stage.array[x][y];
            if tile_id == 0 { continue; }

            // Bevy Y-up: flip Y so row 0 is at the top of the screen
            let px = x as f32 * TILE_W;
            let py = -(y as f32 * TILE_H);

            let entity = commands
                .spawn((
                    GameEntity,
                    Tile { tile_id },
                    Sprite {
                        image: assets.tiles.clone(),
                        rect: Some(tile_source_rect(tile_id)),
                        custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                        anchor: bevy::sprite::Anchor::TopLeft,
                        ..default()
                    },
                    Transform::from_xyz(px, py, 0.0),
                ))
                .id();
            tile_grid.entities[x][y] = Some(entity);
        }
    }
}

/// Smooth camera follow — tracks the player entity's X position within stage bounds.
fn camera_follow(
    player_q: Query<&Transform, (With<crate::player::Player>, Without<PersistentCamera>)>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };

    let target_x = clamp_camera_x(player_tf.translation.x);
    let target_y = -240.0;
    cam_tf.translation.x += (target_x - cam_tf.translation.x) * 0.12;
    cam_tf.translation.y += (target_y - cam_tf.translation.y) * 0.12;
}

/// Convenience shortcuts matching C++ LOOP_Gameloop: ESC -> Menu, F2 -> reload, F3/F4 -> next/prev stage.
fn ingame_shortcuts(
    keys:      Res<ButtonInput<KeyCode>>,
    mut hud:   ResMut<HudState>,
    mut level: ResMut<LevelData>,
    mut next:  ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(GameState::Menu);
    } else if keys.just_pressed(KeyCode::F2) {
        level.checkpoint = 0;
        next.set(GameState::StageTransition);
    } else if keys.just_pressed(KeyCode::F3) {
        hud.stage = if hud.stage >= MAX_CLASSIC_STAGE { 1 } else { hud.stage + 1 };
        level.checkpoint = 0;
        next.set(GameState::StageTransition);
    } else if keys.just_pressed(KeyCode::F4) {
        hud.stage = if hud.stage <= 1 { MAX_CLASSIC_STAGE } else { hud.stage - 1 };
        level.checkpoint = 0;
        next.set(GameState::StageTransition);
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
