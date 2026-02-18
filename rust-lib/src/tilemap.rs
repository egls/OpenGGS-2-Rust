use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData, PersistentCamera};

// ── Marker components ─────────────────────────────────────────────────────────

/// Marks all entities that belong to the in-game world (tiles, player, enemies).
/// Used for bulk despawn on state exit.
#[derive(Component)]
pub struct GameEntity;

/// A single map tile.
#[derive(Component)]
pub struct Tile {
    pub tile_id: i32,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_tiles)
           .add_systems(Update, camera_follow.run_if(in_state(GameState::InGame)))
           .add_systems(OnExit(GameState::InGame), despawn_game_entities);
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn spawn_tiles(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    const TILE_W: f32 = 16.0;
    const TILE_H: f32 = 16.0;
    const TILES_PER_ROW: i32 = 40;

    for x in 0..256usize {
        for y in 0..30usize {
            let tile_id = level.stage.array[x][y];
            if tile_id == 0 { continue; }

            let col = tile_id % TILES_PER_ROW;
            let row = tile_id / TILES_PER_ROW;
            let src_x = col as f32 * TILE_W;
            let src_y = row as f32 * TILE_H;

            // Bevy Y-up: flip Y so row 0 is at the top of the screen
            let px = x as f32 * TILE_W;
            let py = -(y as f32 * TILE_H);

            let bg_color = match level.stage.background_colour {
                0 => Color::srgb(0.4, 0.6, 1.0), // Blue sky
                _ => Color::BLACK,
            };
            commands.insert_resource(ClearColor(bg_color));

            commands.spawn((
                GameEntity,
                Tile { tile_id },
                Sprite {
                    image: assets.tiles.clone(),
                    rect: Some(Rect::new(src_x, src_y, src_x + TILE_W, src_y + TILE_H)),
                    custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                    ..default()
                },
                Transform::from_xyz(px, py, 0.0),
            ));
        }
    }
}

/// Smooth camera follow — tracks the player entity's X position.
fn camera_follow(
    player_q: Query<&Transform, (With<crate::player::Player>, Without<PersistentCamera>)>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };

    let target_x = player_tf.translation.x;
    let target_y = player_tf.translation.y + 50.0; // keep player in lower half
    cam_tf.translation.x += (target_x - cam_tf.translation.x) * 0.1;
    cam_tf.translation.y += (target_y - cam_tf.translation.y) * 0.1;
}

fn despawn_game_entities(
    mut commands: Commands,
    query: Query<Entity, With<GameEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
