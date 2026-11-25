use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData, PersistentCamera, MAX_SUBSTAGE_INDEX};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
use crate::level::{save_stage, ImportStage};
use crate::random_gen::{element_dimensions, generate_random_stage, stamp_element, RandomModuleBank};
use crate::states::GameState;
use crate::tilemap::{
    animated_tile_source_rect, STAGE_COLS, STAGE_ROWS, TILE_H, TILE_W,
};

const MAX_NUM_ENEMIES: usize = 50;

/// Curated palette of classic C++ OpenGGS Maker multi-tile elements (`GRAPHICS_Tiles_Elements.cpp`).
pub const EDITOR_ELEMENTS: &[(i32, &str)] = &[
    (775, "Ground 4x3"),
    (783, "Ground Alt 4x3"),
    (672, "Bricks 2x2"),
    (501, "Breakable 3x2"),
    (560, "Coin Block 3x2"),
    (640, "5-Coin Block 3x2"),
    (480, "PowerUp Block 3x2"),
    (587, "Solid Block 3x2"),
    (566, "Crumble 2x1"),
    (630, "Bridge 3x1"),
    (3,   "Coin 2x2"),
    (32,  "Spikes 3x3"),
    (250, "Lava 3x1"),
    (6,   "Big Fire 4x2"),
    (514, "Barrel 6x3"),
    (720, "Exit Gate 6x4"),
    (80,  "Warp Block 3x2"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Element,
    SingleTile,
    Enemy,
    StartPos,
}

impl EditorTool {
    pub fn next(self) -> Self {
        match self {
            Self::Element    => Self::SingleTile,
            Self::SingleTile => Self::Enemy,
            Self::Enemy      => Self::StartPos,
            Self::StartPos   => Self::Element,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Element    => "ELEMENT",
            Self::SingleTile => "SINGLE TILE",
            Self::Enemy      => "ENEMY",
            Self::StartPos   => "START POS",
        }
    }
}

#[derive(Resource)]
pub struct StageEditorState {
    pub stage_num:     usize,
    pub camera_x:      f32,
    pub tool:          EditorTool,
    pub element_idx:   usize,
    pub single_tile:   i32,
    pub enemy_type:    i32,
    pub status_msg:    String,
    pub status_timer:  f32,
    pub dirty_visuals: bool,
}

impl Default for StageEditorState {
    fn default() -> Self {
        Self {
            stage_num: 1,
            camera_x: 400.0,
            tool: EditorTool::Element,
            element_idx: 0,
            single_tile: 501,
            enemy_type: 1,
            status_msg: String::new(),
            status_timer: 0.0,
            dirty_visuals: true,
        }
    }
}

#[derive(Component)]
struct StageEditorEntity;

#[derive(Component)]
struct StageEditorHudRoot;

#[derive(Component)]
struct StageEditorHudText;

// ── Pure testable editing helpers ─────────────────────────────────────────────

/// Converts window cursor coordinates to stage tile grid `(tx, ty)` given camera center & window size.
pub fn cursor_to_stage_tile(
    cursor:   Vec2,
    win_size: Vec2,
    cam_pos:  Vec2,
) -> Option<(usize, usize)> {
    // Top 56px reserved for editor toolbar HUD
    if cursor.y < 56.0 {
        return None;
    }
    let world_x = cam_pos.x + (cursor.x - win_size.x * 0.5);
    let world_y = cam_pos.y - (cursor.y - win_size.y * 0.5);
    let tx = (world_x / TILE_W).floor() as i32;
    let ty = (-world_y / TILE_H).floor() as i32;
    if (0..STAGE_COLS as i32).contains(&tx) && (0..STAGE_ROWS as i32).contains(&ty) {
        Some((tx as usize, ty as usize))
    } else {
        None
    }
}

pub fn place_editor_element(stage: &mut ImportStage, base_tile: i32, tx: usize, ty: usize) {
    stamp_element(stage, base_tile, tx, ty);
}

pub fn place_editor_single_tile(stage: &mut ImportStage, tile_id: i32, tx: usize, ty: usize) {
    if tx < STAGE_COLS && ty < STAGE_ROWS {
        stage.array[tx][ty] = tile_id;
    }
}

pub fn erase_editor_tile(stage: &mut ImportStage, tx: usize, ty: usize) {
    if tx < STAGE_COLS && ty < STAGE_ROWS {
        stage.array[tx][ty] = 0;
    }
}

/// Places an enemy into the first free slot (`0..50`). Returns `Some(slot)` on success.
pub fn place_editor_enemy(
    stage:      &mut ImportStage,
    enemy_type: i32,
    tx:         usize,
    ty:         usize,
) -> Option<usize> {
    let slot = (0..MAX_NUM_ENEMIES).find(|&i| stage.enemy_in_use[i] == 0)?;
    stage.enemy_in_use[slot] = 1;
    stage.enemy_type[slot] = enemy_type.clamp(1, 15);
    stage.enemy_direction[slot] = 0;
    stage.enemy_pos_x[slot] = (tx * 16) as i32;
    stage.enemy_pos_y[slot] = (ty * 16) as i32;
    Some(slot)
}

/// Removes the closest active enemy within 2 tiles of `(tx, ty)`. Returns `true` if removed.
pub fn remove_editor_enemy_near(stage: &mut ImportStage, tx: usize, ty: usize) -> bool {
    let px = (tx * 16) as i32;
    let py = (ty * 16) as i32;
    for i in 0..MAX_NUM_ENEMIES {
        if stage.enemy_in_use[i] != 0
            && (stage.enemy_pos_x[i] - px).abs() <= 32
            && (stage.enemy_pos_y[i] - py).abs() <= 32
        {
            stage.enemy_in_use[i] = 0;
            return true;
        }
    }
    false
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct StageEditorPlugin;

impl Plugin for StageEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StageEditorState>()
           .add_systems(OnEnter(GameState::StageEditor), enter_stage_editor)
           .add_systems(
               Update,
               (stage_editor_input, rebuild_editor_world, update_editor_hud)
                   .chain()
                   .run_if(in_state(GameState::StageEditor)),
           )
           .add_systems(OnExit(GameState::StageEditor), exit_stage_editor);
    }
}

fn enter_stage_editor(
    mut commands: Commands,
    mut editor:   ResMut<StageEditorState>,
    mut level:    ResMut<LevelData>,
    mut cam_q:    Query<&mut Transform, With<PersistentCamera>>,
) {
    let target = editor.stage_num.clamp(1, MAX_SUBSTAGE_INDEX);
    editor.stage_num = target;
    let _ = level.load_stage_number(target, 0, None);
    editor.camera_x = 400.0;
    editor.dirty_visuals = true;
    editor.status_msg = format!("Loaded Stage {:02} from {}", target, level.pack_name());
    editor.status_timer = 2.5;

    if let Ok(mut cam_tf) = cam_q.get_single_mut() {
        cam_tf.translation.x = editor.camera_x;
        cam_tf.translation.y = -240.0;
    }

    commands
        .spawn((
            StageEditorHudRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(54.0),
                flex_direction:  FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding:         UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.14, 0.92)),
        ))
        .with_children(|p| {
            p.spawn((
                StageEditorHudText,
                Text::new("STAGE EDITOR"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(1.0, 0.92, 0.35)),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn stage_editor_input(
    time:        Res<Time>,
    keys:        Res<ButtonInput<KeyCode>>,
    mouse:       Res<ButtonInput<MouseButton>>,
    win_q:       Query<&Window>,
    mut cam_q:   Query<&mut Transform, With<PersistentCamera>>,
    mut editor:  ResMut<StageEditorState>,
    mut level:   ResMut<LevelData>,
    module_bank: Option<Res<RandomModuleBank>>,
    mut hud:     ResMut<HudState>,
    mut sfx:     EventWriter<SfxEvent>,
    mut next:    ResMut<NextState<GameState>>,
) {
    if editor.status_timer > 0.0 {
        editor.status_timer = (editor.status_timer - time.delta_secs()).max(0.0);
    }

    if keys.just_pressed(KeyCode::Escape) {
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::Menu);
        return;
    }

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Save current stage to .lvl pack (Ctrl+S)
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        let path_str = level.stage_file.to_string_lossy().into_owned();
        match save_stage(&path_str, editor.stage_num, &level.stage) {
            Ok(()) => {
                sfx.send(SfxEvent(SfxKind::Disk));
                editor.status_msg = format!("SAVED Stage {:02} to {}", editor.stage_num, level.pack_name());
                editor.status_timer = 3.0;
            }
            Err(err) => {
                editor.status_msg = format!("Save error: {}", err);
                editor.status_timer = 4.0;
            }
        }
        return;
    }

    // Test-play current edited stage directly (F5)
    if keys.just_pressed(KeyCode::F5) {
        hud.reset();
        hud.stage = editor.stage_num;
        level.stage_number = editor.stage_num;
        level.checkpoint = 0;
        level.refresh_stage_width();
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::InGame);
        return;
    }

    // Switch stage slot (F3 / F4)
    if keys.just_pressed(KeyCode::F3) {
        editor.stage_num = if editor.stage_num >= MAX_SUBSTAGE_INDEX { 1 } else { editor.stage_num + 1 };
        let _ = level.load_stage_number(editor.stage_num, 0, None);
        editor.dirty_visuals = true;
        editor.status_msg = format!("Loaded Stage {:02}", editor.stage_num);
        editor.status_timer = 2.0;
        sfx.send(SfxEvent(SfxKind::Click));
    } else if keys.just_pressed(KeyCode::F4) {
        editor.stage_num = if editor.stage_num <= 1 { MAX_SUBSTAGE_INDEX } else { editor.stage_num - 1 };
        let _ = level.load_stage_number(editor.stage_num, 0, None);
        editor.dirty_visuals = true;
        editor.status_msg = format!("Loaded Stage {:02}", editor.stage_num);
        editor.status_timer = 2.0;
        sfx.send(SfxEvent(SfxKind::Click));
    }

    // Cycle tool mode (Tab)
    if keys.just_pressed(KeyCode::Tab) {
        editor.tool = editor.tool.next();
        sfx.send(SfxEvent(SfxKind::Click));
    }

    // Cycle active item (Q / E)
    if keys.just_pressed(KeyCode::KeyQ) {
        match editor.tool {
            EditorTool::Element => {
                editor.element_idx = (editor.element_idx + EDITOR_ELEMENTS.len() - 1) % EDITOR_ELEMENTS.len();
            }
            EditorTool::SingleTile => {
                editor.single_tile = if editor.single_tile <= 1 { 2319 } else { editor.single_tile - 1 };
            }
            EditorTool::Enemy => {
                editor.enemy_type = if editor.enemy_type <= 1 { 15 } else { editor.enemy_type - 1 };
            }
            EditorTool::StartPos => {}
        }
        sfx.send(SfxEvent(SfxKind::Click));
    } else if keys.just_pressed(KeyCode::KeyE) {
        match editor.tool {
            EditorTool::Element => {
                editor.element_idx = (editor.element_idx + 1) % EDITOR_ELEMENTS.len();
            }
            EditorTool::SingleTile => {
                editor.single_tile = if editor.single_tile >= 2319 { 1 } else { editor.single_tile + 1 };
            }
            EditorTool::Enemy => {
                editor.enemy_type = if editor.enemy_type >= 15 { 1 } else { editor.enemy_type + 1 };
            }
            EditorTool::StartPos => {}
        }
        sfx.send(SfxEvent(SfxKind::Click));
    }

    // Toggle background colour (B) and tile theme (T)
    if keys.just_pressed(KeyCode::KeyB) {
        level.stage.background_colour = if level.stage.background_colour == 0 { 1 } else { 0 };
        editor.dirty_visuals = true;
        sfx.send(SfxEvent(SfxKind::Click));
    }
    if keys.just_pressed(KeyCode::KeyT) {
        level.stage.tile_type = (level.stage.tile_type + 1) % 4;
        editor.dirty_visuals = true;
        sfx.send(SfxEvent(SfxKind::Click));
    }

    // Generate random stage into current editor slot (G)
    if keys.just_pressed(KeyCode::KeyG) {
        if let Some(bank) = module_bank.as_deref() {
            level.random_seed = level.random_seed.wrapping_add(97);
            if let Some(gen) = generate_random_stage(
                bank,
                editor.stage_num,
                level.stage.background_colour,
                &level.tile_info,
                level.random_seed,
            ) {
                level.stage = gen;
                level.refresh_stage_width();
                editor.dirty_visuals = true;
                editor.status_msg = "Generated random stage (Ctrl+S to save, F5 to test)".to_string();
                editor.status_timer = 3.0;
                sfx.send(SfxEvent(SfxKind::Disk));
            }
        }
    }

    // Scroll camera horizontally
    let scroll_speed = if shift { 700.0 } else { 320.0 } * time.delta_secs();
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        editor.camera_x += scroll_speed;
    }
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        editor.camera_x -= scroll_speed;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        editor.camera_x = 400.0;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        editor.camera_x = STAGE_COLS as f32 * TILE_W - 400.0;
    }
    let win = win_q.get_single().ok();
    let win_size = win.map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(800.0, 600.0));
    let half_w = win_size.x * 0.5;
    editor.camera_x = editor.camera_x.clamp(half_w, (STAGE_COLS as f32 * TILE_W - half_w).max(half_w));

    let cam_pos = Vec2::new(editor.camera_x, -240.0);
    if let Ok(mut cam_tf) = cam_q.get_single_mut() {
        cam_tf.translation.x = cam_pos.x;
        cam_tf.translation.y = cam_pos.y;
    }

    // Mouse painting & erasing
    if let Some(cursor) = win.and_then(|w| w.cursor_position()) {
        if let Some((tx, ty)) = cursor_to_stage_tile(cursor, win_size, cam_pos) {
            if mouse.just_pressed(MouseButton::Left) {
                match editor.tool {
                    EditorTool::Element => {
                        let (base_id, _) = EDITOR_ELEMENTS[editor.element_idx];
                        place_editor_element(&mut level.stage, base_id, tx, ty);
                    }
                    EditorTool::SingleTile => {
                        place_editor_single_tile(&mut level.stage, editor.single_tile, tx, ty);
                    }
                    EditorTool::Enemy => {
                        if place_editor_enemy(&mut level.stage, editor.enemy_type, tx, ty).is_none() {
                            editor.status_msg = "Max 50 enemies reached!".to_string();
                            editor.status_timer = 2.0;
                        }
                    }
                    EditorTool::StartPos => {
                        level.stage.start_position_x[0] = (tx * 16) as i32;
                        level.stage.start_position_y[0] = (ty * 16) as i32;
                    }
                }
                editor.dirty_visuals = true;
            } else if mouse.just_pressed(MouseButton::Right) {
                match editor.tool {
                    EditorTool::Enemy => {
                        if remove_editor_enemy_near(&mut level.stage, tx, ty) {
                            editor.dirty_visuals = true;
                        }
                    }
                    _ => {
                        erase_editor_tile(&mut level.stage, tx, ty);
                        editor.dirty_visuals = true;
                    }
                }
            }
        }
    }
}

fn rebuild_editor_world(
    mut commands: Commands,
    mut editor:   ResMut<StageEditorState>,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
    old_q:        Query<Entity, With<StageEditorEntity>>,
) {
    if !editor.dirty_visuals {
        return;
    }
    editor.dirty_visuals = false;

    for e in &old_q {
        commands.entity(e).despawn();
    }

    let bg = if level.stage.background_colour == 0 {
        Color::srgb(0.35, 0.55, 0.92)
    } else {
        Color::srgb(0.06, 0.06, 0.1)
    };
    commands.insert_resource(ClearColor(bg));

    // Render all tiles in stage
    for x in 0..STAGE_COLS {
        for y in 0..STAGE_ROWS {
            let id = level.stage.array[x][y];
            if id == 0 {
                continue;
            }
            let rect = animated_tile_source_rect(id, level.stage.tile_type, &level.tile_info, 0, 0, 0);
            commands.spawn((
                StageEditorEntity,
                Sprite {
                    image: assets.tiles.clone(),
                    rect: Some(rect),
                    custom_size: Some(Vec2::new(TILE_W, TILE_H)),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                Transform::from_xyz(x as f32 * TILE_W, -(y as f32 * TILE_H), 0.0),
            ));
        }
    }

    // Render start position marker
    let sx = level.stage.start_position_x[0] as f32;
    let sy = -(level.stage.start_position_y[0] as f32);
    commands.spawn((
        StageEditorEntity,
        Sprite {
            color: Color::srgba(0.2, 1.0, 0.35, 0.75),
            custom_size: Some(Vec2::new(24.0, 42.0)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(sx, sy, 1.0),
    ));

    // Render enemy markers
    for i in 0..MAX_NUM_ENEMIES {
        if level.stage.enemy_in_use[i] == 0 {
            continue;
        }
        let ex = level.stage.enemy_pos_x[i] as f32;
        let ey = -(level.stage.enemy_pos_y[i] as f32);
        commands.spawn((
            StageEditorEntity,
            Sprite {
                color: Color::srgba(1.0, 0.25, 0.25, 0.8),
                custom_size: Some(Vec2::new(24.0, 24.0)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(ex, ey, 0.9),
        ));
    }
}

fn update_editor_hud(
    editor:    Res<StageEditorState>,
    level:     Res<LevelData>,
    mut text_q: Query<&mut Text, With<StageEditorHudText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return };
    let selection = match editor.tool {
        EditorTool::Element => {
            let (id, name) = EDITOR_ELEMENTS[editor.element_idx];
            let (w, h) = element_dimensions(id);
            format!("{} (#{}, {}x{})", name, id, w, h)
        }
        EditorTool::SingleTile => format!("Tile #{}", editor.single_tile),
        EditorTool::Enemy => format!("Enemy Type {}", editor.enemy_type),
        EditorTool::StartPos => "Player Spawn".to_string(),
    };
    let bg_label = if level.stage.background_colour == 0 { "Outdoors" } else { "Indoors" };
    let status = if editor.status_timer > 0.0 && !editor.status_msg.is_empty() {
        format!("  |  {}", editor.status_msg)
    } else {
        String::new()
    };

    **text = format!(
        "STAGE {:02} ({}) | Tool [Tab]: {} -> {} [Q/E] | BG [B]: {} | Theme [T]: {}\nLMB: Place | RMB: Erase | Arrows/AD: Scroll | G: Random | Ctrl+S: Save | F5: Play | F3/F4: Stage | ESC: Menu{}",
        editor.stage_num,
        level.pack_name(),
        editor.tool.label(),
        selection,
        bg_label,
        level.stage.tile_type,
        status,
    );
}

fn exit_stage_editor(
    mut commands: Commands,
    world_q:      Query<Entity, With<StageEditorEntity>>,
    hud_q:        Query<Entity, With<StageEditorHudRoot>>,
) {
    for e in &world_q {
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
    fn editor_element_stamp_and_erase_work_on_stage_grid() {
        let mut stage = ImportStage::new();
        // 3x2 breakable block at (10, 15)
        place_editor_element(&mut stage, 501, 10, 15);
        assert_eq!(stage.array[10][15], 501);
        assert_eq!(stage.array[11][15], 502);
        assert_eq!(stage.array[12][15], 503);
        assert_eq!(stage.array[10][16], 541);
        assert_eq!(stage.array[12][16], 543);

        erase_editor_tile(&mut stage, 11, 15);
        assert_eq!(stage.array[11][15], 0);
    }

    #[test]
    fn editor_enemy_place_and_remove_manage_slots() {
        let mut stage = ImportStage::new();
        let slot = place_editor_enemy(&mut stage, 5, 20, 12).expect("slot 0 should be free");
        assert_eq!(slot, 0);
        assert_eq!(stage.enemy_in_use[0], 1);
        assert_eq!(stage.enemy_type[0], 5);
        assert_eq!(stage.enemy_pos_x[0], 320);
        assert_eq!(stage.enemy_pos_y[0], 192);

        assert!(remove_editor_enemy_near(&mut stage, 21, 12));
        assert_eq!(stage.enemy_in_use[0], 0);
    }

    #[test]
    fn cursor_to_stage_tile_respects_toolbar_and_camera_offset() {
        let win = Vec2::new(800.0, 600.0);
        let cam = Vec2::new(400.0, -240.0);

        // Inside top toolbar -> None
        assert_eq!(cursor_to_stage_tile(Vec2::new(100.0, 20.0), win, cam), None);

        // Screen center (400, 300) maps to world (400, -240) -> tile (25, 15)
        assert_eq!(cursor_to_stage_tile(Vec2::new(400.0, 300.0), win, cam), Some((25, 15)));
    }
}
