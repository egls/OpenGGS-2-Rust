use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera};
use crate::audio::{SfxEvent, SfxKind};
use crate::paths::AssetPaths;
use crate::states::GameState;
use crate::tile_properties::{save_tile_properties, TileFlag, NUMBER_OF_TILES};
use crate::tilemap::{TILESHEET_H, TILESHEET_W, TILES_PER_ROW, TILE_H, TILE_W};

const TOOLBAR_HEIGHT: f32 = 64.0;
const SHEET_ORIGIN_X: f32 = 0.0;
const SHEET_ORIGIN_Y: f32 = 0.0;

#[derive(Resource)]
pub struct TileEditorState {
    pub active_flag_idx: usize,
    pub scroll_y:        f32,
    pub hovered_tile:    Option<usize>,
    pub status_msg:      String,
    pub status_timer:    f32,
    pub dirty_markers:   bool,
}

impl Default for TileEditorState {
    fn default() -> Self {
        Self {
            active_flag_idx: 0,
            scroll_y: 0.0,
            hovered_tile: None,
            status_msg: String::new(),
            status_timer: 0.0,
            dirty_markers: true,
        }
    }
}

impl TileEditorState {
    pub fn active_flag(&self) -> TileFlag {
        TileFlag::ALL[self.active_flag_idx % TileFlag::ALL.len()]
    }
}

#[derive(Component)]
struct TileEditorSheet;

#[derive(Component)]
struct TileEditorMarker;

#[derive(Component)]
struct TileEditorHudRoot;

#[derive(Component)]
struct TileEditorHudText;

/// Converts window cursor coordinates to a tile ID (`0..2320`) on the `40×58` tilesheet.
pub fn cursor_to_tilesheet_id(
    cursor:   Vec2,
    win_size: Vec2,
    cam_pos:  Vec2,
) -> Option<usize> {
    if cursor.y < TOOLBAR_HEIGHT {
        return None;
    }
    let world_x = cam_pos.x + (cursor.x - win_size.x * 0.5);
    let world_y = cam_pos.y - (cursor.y - win_size.y * 0.5);
    let col = ((world_x - SHEET_ORIGIN_X) / TILE_W).floor() as i32;
    let row = (-(world_y - SHEET_ORIGIN_Y) / TILE_H).floor() as i32;
    if (0..TILES_PER_ROW).contains(&col) && (0..58).contains(&row) {
        let id = (row * TILES_PER_ROW + col) as usize;
        if id < NUMBER_OF_TILES {
            return Some(id);
        }
    }
    None
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TileEditorPlugin;

impl Plugin for TileEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileEditorState>()
           .add_systems(OnEnter(GameState::TileEditor), enter_tile_editor)
           .add_systems(
               Update,
               (tile_editor_input, rebuild_tile_markers, update_tile_editor_hud)
                   .chain()
                   .run_if(in_state(GameState::TileEditor)),
           )
           .add_systems(OnExit(GameState::TileEditor), exit_tile_editor);
    }
}

fn enter_tile_editor(
    mut commands: Commands,
    mut state:    ResMut<TileEditorState>,
    assets:       Res<GameAssets>,
    mut cam_q:    Query<&mut Transform, With<PersistentCamera>>,
) {
    state.active_flag_idx = 0;
    state.scroll_y = 0.0;
    state.hovered_tile = None;
    state.dirty_markers = true;
    state.status_msg = "Click any tile to toggle flag | Ctrl+S to save Tilesheetinfo.tsi".to_string();
    state.status_timer = 3.0;

    commands.insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.16)));

    if let Ok(mut cam_tf) = cam_q.get_single_mut() {
        cam_tf.translation.x = TILESHEET_W * 0.5;
        cam_tf.translation.y = -240.0;
    }

    // Spawn the full 640x928 tiles portion of Tiles.png
    commands.spawn((
        TileEditorSheet,
        Sprite {
            image: assets.tiles.clone(),
            rect: Some(Rect::new(0.0, 0.0, TILESHEET_W, TILESHEET_H)),
            custom_size: Some(Vec2::new(TILESHEET_W, TILESHEET_H)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(SHEET_ORIGIN_X, SHEET_ORIGIN_Y, 0.0),
    ));

    commands
        .spawn((
            TileEditorHudRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(TOOLBAR_HEIGHT),
                flex_direction:  FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding:         UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.07, 0.14, 0.94)),
        ))
        .with_children(|p| {
            p.spawn((
                TileEditorHudText,
                Text::new("TILE PROPERTIES EDITOR"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(1.0, 0.92, 0.35)),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn tile_editor_input(
    time:      Res<Time>,
    keys:      Res<ButtonInput<KeyCode>>,
    mouse:     Res<ButtonInput<MouseButton>>,
    win_q:     Query<&Window>,
    mut cam_q: Query<&mut Transform, With<PersistentCamera>>,
    mut state: ResMut<TileEditorState>,
    mut level: ResMut<LevelData>,
    paths:     Res<AssetPaths>,
    mut sfx:   EventWriter<SfxEvent>,
    mut next:  ResMut<NextState<GameState>>,
) {
    if state.status_timer > 0.0 {
        state.status_timer = (state.status_timer - time.delta_secs()).max(0.0);
    }

    if keys.just_pressed(KeyCode::Escape) {
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::Options);
        return;
    }

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        let path_str = paths.tilesheet_info().to_string_lossy().into_owned();
        match save_tile_properties(&path_str, &level.tile_info) {
            Ok(()) => {
                sfx.send(SfxEvent(SfxKind::Disk));
                state.status_msg = "SAVED Tilesheetinfo.tsi".to_string();
                state.status_timer = 3.0;
            }
            Err(err) => {
                state.status_msg = format!("Save failed: {}", err);
                state.status_timer = 4.0;
            }
        }
        return;
    }

    if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyE) {
        state.active_flag_idx = (state.active_flag_idx + 1) % TileFlag::ALL.len();
        state.dirty_markers = true;
        sfx.send(SfxEvent(SfxKind::Click));
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyQ) {
        state.active_flag_idx = (state.active_flag_idx + TileFlag::ALL.len() - 1) % TileFlag::ALL.len();
        state.dirty_markers = true;
        sfx.send(SfxEvent(SfxKind::Click));
    }

    let scroll_speed = 320.0 * time.delta_secs();
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        state.scroll_y += scroll_speed;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        state.scroll_y -= scroll_speed;
    }
    state.scroll_y = state.scroll_y.clamp(0.0, (TILESHEET_H - 360.0).max(0.0));

    let cam_pos = Vec2::new(TILESHEET_W * 0.5, -240.0 - state.scroll_y);
    if let Ok(mut cam_tf) = cam_q.get_single_mut() {
        cam_tf.translation.x = cam_pos.x;
        cam_tf.translation.y = cam_pos.y;
    }

    let win = win_q.get_single().ok();
    let win_size = win.map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(800.0, 600.0));
    state.hovered_tile = win
        .and_then(|w| w.cursor_position())
        .and_then(|c| cursor_to_tilesheet_id(c, win_size, cam_pos));

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(tile_id) = state.hovered_tile {
            let flag = state.active_flag();
            let now = flag.toggle(&mut level.tile_info, tile_id);
            state.dirty_markers = true;
            state.status_msg = format!("Tile #{} {} = {}", tile_id, flag.label(), now);
            state.status_timer = 2.0;
            sfx.send(SfxEvent(SfxKind::Click));
        }
    }
}

fn flag_overlay_color(flag: TileFlag) -> Color {
    match flag {
        TileFlag::Solid            => Color::srgba(0.2, 0.7, 1.0, 0.42),
        TileFlag::Exit             => Color::srgba(0.2, 1.0, 0.3, 0.5),
        TileFlag::Lethal           => Color::srgba(1.0, 0.15, 0.15, 0.5),
        TileFlag::Fire             => Color::srgba(1.0, 0.55, 0.1, 0.5),
        TileFlag::Coin             => Color::srgba(1.0, 0.9, 0.1, 0.5),
        TileFlag::CoinBlock        => Color::srgba(1.0, 0.78, 0.2, 0.5),
        TileFlag::Breakable        => Color::srgba(0.8, 0.45, 0.2, 0.5),
        TileFlag::PowerUpBlock     => Color::srgba(0.85, 0.2, 1.0, 0.5),
        TileFlag::CoinBlockHelmet  => Color::srgba(0.95, 0.65, 0.3, 0.5),
        TileFlag::ColourChanging   => Color::srgba(0.2, 0.95, 0.9, 0.42),
        TileFlag::SubStageEntrance => Color::srgba(0.3, 0.4, 1.0, 0.5),
        TileFlag::DropStone        => Color::srgba(0.75, 0.75, 0.85, 0.5),
        TileFlag::WarpStone        => Color::srgba(1.0, 0.25, 0.8, 0.55),
    }
}

fn rebuild_tile_markers(
    mut commands: Commands,
    mut state:    ResMut<TileEditorState>,
    level:        Res<LevelData>,
    old_q:        Query<Entity, With<TileEditorMarker>>,
) {
    if !state.dirty_markers {
        return;
    }
    state.dirty_markers = false;

    for e in &old_q {
        commands.entity(e).despawn();
    }

    let flag = state.active_flag();
    let color = flag_overlay_color(flag);

    for id in 0..NUMBER_OF_TILES {
        if !flag.get(&level.tile_info, id) {
            continue;
        }
        let col = (id as i32) % TILES_PER_ROW;
        let row = (id as i32) / TILES_PER_ROW;
        commands.spawn((
            TileEditorMarker,
            Sprite {
                color,
                custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(col as f32 * TILE_W, -(row as f32 * TILE_H), 0.5),
        ));
    }
}

fn update_tile_editor_hud(
    state:      Res<TileEditorState>,
    mut text_q: Query<&mut Text, With<TileEditorHudText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return };
    let hovered = state
        .hovered_tile
        .map(|id| format!("#{}", id))
        .unwrap_or_else(|| "-".to_string());
    let status = if state.status_timer > 0.0 && !state.status_msg.is_empty() {
        format!("  |  {}", state.status_msg)
    } else {
        String::new()
    };

    **text = format!(
        "TILE EDITOR | Active Flag [Tab/Q/E]: {} ({}/{}) | Hover Tile: {}\nLMB: Toggle Flag | Up/Down: Scroll Sheet | Ctrl+S: Save Tilesheetinfo.tsi | ESC: Back{}",
        state.active_flag().label(),
        state.active_flag_idx + 1,
        TileFlag::ALL.len(),
        hovered,
        status,
    );
}

fn exit_tile_editor(
    mut commands: Commands,
    sheet_q:      Query<Entity, With<TileEditorSheet>>,
    marker_q:     Query<Entity, With<TileEditorMarker>>,
    hud_q:        Query<Entity, With<TileEditorHudRoot>>,
) {
    for e in &sheet_q {
        commands.entity(e).despawn();
    }
    for e in &marker_q {
        commands.entity(e).despawn();
    }
    for e in &hud_q {
        commands.entity(e).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_to_tilesheet_id_maps_tiles_and_rejects_toolbar() {
        let win = Vec2::new(800.0, 600.0);
        let cam = Vec2::new(320.0, -240.0);

        // Inside toolbar -> None
        assert_eq!(cursor_to_tilesheet_id(Vec2::new(320.0, 30.0), win, cam), None);

        // Screen center (400, 300) maps to world (320, -240) -> col=20, row=15 -> id=620
        assert_eq!(cursor_to_tilesheet_id(Vec2::new(400.0, 300.0), win, cam), Some(15 * 40 + 20));
    }
}
