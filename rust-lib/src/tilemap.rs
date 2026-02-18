use bevy::prelude::*;

use crate::states::GameState;
use crate::assets::{GameAssets, LevelData};

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
        app.add_systems(OnEnter(GameState::InGame), (spawn_camera, spawn_tiles))
           .add_systems(Update, camera_follow.run_if(in_state(GameState::InGame)))
           .add_systems(OnExit(GameState::InGame), despawn_game_entities);
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, GameEntity));
}

fn spawn_tiles(
    mut commands: Commands,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
    mut layouts:  ResMut<Assets<TextureAtlasLayout>>,
) {
    const TILE_W: u32 = 16;
    const TILE_H: u32 = 16;
    const TILES_PER_ROW: u32 = 40;

    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(TILE_W, TILE_H),
        TILES_PER_ROW,
        60, // rows (2320 tiles / 40 per row = 58, round up)
        None,
        None,
    );
    let layout_handle = layouts.add(layout);

    for x in 0..256usize {
        for y in 0..30usize {
            let tile_id = level.stage.array[x][y];
            if tile_id == 0 { continue; }

            let atlas_index = tile_id as usize;
            let px = (x as f32) * TILE_W as f32;
            let py = -(y as f32) * TILE_H as f32; // Bevy Y-up: flip

            commands.spawn((
                GameEntity,
                Tile { tile_id },
                Sprite {
                    image: assets.tiles.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: layout_handle.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                Transform::from_xyz(px, py, 0.0),
            ));
        }
    }
}

/// Smooth camera follow — tracks the player entity's X position.
fn camera_follow(
    player_q: Query<&Transform, (With<crate::player::Player>, Without<Camera2d>)>,
    mut cam_q: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };

    let target_x = player_tf.translation.x;
    cam_tf.translation.x += (target_x - cam_tf.translation.x) * 0.1;
}

fn despawn_game_entities(
    mut commands: Commands,
    query: Query<Entity, With<GameEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
