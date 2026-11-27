use std::fs::File;
use std::io::{self, Read, Write};
use std::mem;

use bevy::prelude::*;
use bevy::window::WindowMode;

use crate::assets::LevelData;
use crate::audio::{SfxEvent, SfxKind};
use crate::input;
use crate::paths::AssetPaths;
use crate::states::GameState;

/// Exact C++ `SavedOptions_Type` binary layout (`base/Options.oi`, 32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SavedOptions {
    pub screen_width:  i32,
    pub screen_height: i32,
    pub volume_sounds: i32,
    pub volume_music:  i32,
    pub game_type:     i32,
    pub var4:          i32,
    pub var5:          i32,
    pub var6:          i32,
}

pub fn load_saved_options(path: &str) -> io::Result<SavedOptions> {
    let mut file = File::open(path)?;
    let size = mem::size_of::<SavedOptions>();
    let mut buf = vec![0u8; size];
    file.read_exact(&mut buf)?;
    let mut opts = SavedOptions::default();
    unsafe {
        std::ptr::copy_nonoverlapping(buf.as_ptr(), &mut opts as *mut SavedOptions as *mut u8, size);
    }
    Ok(opts)
}

pub fn save_saved_options(path: &str, opts: &SavedOptions) -> io::Result<()> {
    let mut file = File::create(path)?;
    let size = mem::size_of::<SavedOptions>();
    let bytes = unsafe { std::slice::from_raw_parts(opts as *const SavedOptions as *const u8, size) };
    file.write_all(bytes)?;
    Ok(())
}

// ── Display Settings ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayResolution {
    R640x480,
    #[default]
    R800x600,
    R1280x720,
}

impl DisplayResolution {
    pub fn from_dims(w: i32, h: i32) -> Self {
        match (w, h) {
            (640, 480) => Self::R640x480,
            (1280, 720) => Self::R1280x720,
            _ => Self::R800x600,
        }
    }

    pub fn dims(self) -> (u32, u32) {
        match self {
            Self::R640x480  => (640, 480),
            Self::R800x600  => (800, 600),
            Self::R1280x720 => (1280, 720),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::R640x480  => "640x480 (4:3)",
            Self::R800x600  => "800x600 (4:3)",
            Self::R1280x720 => "1280x720 (16:9)",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::R640x480  => Self::R800x600,
            Self::R800x600  => Self::R1280x720,
            Self::R1280x720 => Self::R640x480,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::R640x480  => Self::R1280x720,
            Self::R800x600  => Self::R640x480,
            Self::R1280x720 => Self::R800x600,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct DisplaySettings {
    pub resolution: DisplayResolution,
    pub fullscreen: bool,
    pub game_type:  i32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            resolution: DisplayResolution::R800x600,
            fullscreen: false,
            game_type: 1,
        }
    }
}

// ── Audio Settings Resource ───────────────────────────────────────────────────

#[derive(Resource, Debug, Clone)]
pub struct AudioSettings {
    /// Sound effects volume percentage (`0..=100`).
    pub sfx_volume:   u8,
    /// Background music volume percentage (`0..=100`).
    pub music_volume: u8,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            sfx_volume: 80,
            music_volume: 60,
        }
    }
}

impl AudioSettings {
    pub fn from_saved(saved: &SavedOptions) -> Self {
        let sfx = if (0..=100).contains(&saved.volume_sounds) {
            saved.volume_sounds as u8
        } else {
            80
        };
        let music = if (0..=100).contains(&saved.volume_music) {
            saved.volume_music as u8
        } else {
            60
        };
        Self {
            sfx_volume: sfx,
            music_volume: music,
        }
    }

    pub fn sfx_gain(&self) -> f32 {
        (self.sfx_volume as f32 / 100.0).clamp(0.0, 1.0)
    }

    pub fn music_gain(&self) -> f32 {
        (self.music_volume as f32 / 100.0).clamp(0.0, 1.0)
    }
}

pub fn build_saved_options(audio: &AudioSettings, display: &DisplaySettings) -> SavedOptions {
    let (w, h) = display.resolution.dims();
    SavedOptions {
        screen_width: w as i32,
        screen_height: h as i32,
        volume_sounds: audio.sfx_volume as i32,
        volume_music: audio.music_volume as i32,
        game_type: display.game_type,
        var4: if display.fullscreen { 1 } else { 0 },
        var5: 0,
        var6: 0,
    }
}

pub fn persist_options(paths: &AssetPaths, audio: &AudioSettings, display: &DisplaySettings) {
    let opts = build_saved_options(audio, display);
    if let Err(err) = save_saved_options(paths.options_file().to_str().unwrap_or(""), &opts) {
        eprintln!("Warning: failed to save Options.oi: {}", err);
    }
}

// ── UI Components & State ─────────────────────────────────────────────────────

#[derive(Component)]
struct OptionsRoot;

#[derive(Component)]
struct OptionRowText(usize);

#[derive(Resource, Default)]
struct OptionsSelection(usize);

const OPTION_COUNT: usize = 8;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        let paths = AssetPaths::resolve();
        let (audio, display) = match load_saved_options(paths.options_file().to_str().unwrap_or("")) {
            Ok(saved) => (
                AudioSettings::from_saved(&saved),
                DisplaySettings {
                    resolution: DisplayResolution::from_dims(saved.screen_width, saved.screen_height),
                    fullscreen: saved.var4 != 0,
                    game_type: saved.game_type,
                },
            ),
            Err(_) => (AudioSettings::default(), DisplaySettings::default()),
        };

        app.insert_resource(audio)
           .insert_resource(display)
           .init_resource::<OptionsSelection>()
           .add_systems(Startup, apply_display_settings_to_window)
           .add_systems(
               Update,
               (global_display_and_volume_hotkeys, sync_window_display_settings)
                   .run_if(not(in_state(GameState::Loading))),
           )
           .add_systems(OnEnter(GameState::Options), spawn_options)
           .add_systems(Update, options_input.run_if(in_state(GameState::Options)))
           .add_systems(OnExit(GameState::Options), (save_options_on_exit, despawn_options));
    }
}

fn apply_display_settings_to_window(
    display: Res<DisplaySettings>,
    mut win_q: Query<&mut Window>,
) {
    let Ok(mut win) = win_q.get_single_mut() else { return };
    let (w, h) = display.resolution.dims();
    win.resolution.set(w as f32, h as f32);
    win.mode = if display.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

fn sync_window_display_settings(
    display: Res<DisplaySettings>,
    win_q:   Query<&mut Window>,
) {
    if !display.is_changed() {
        return;
    }
    apply_display_settings_to_window(display, win_q);
}

/// Global hotkeys matching C++ `SYSTEM_Input_General.cpp` / `LOOP_Options.cpp`:
/// - `Alt+Enter`: toggle windowed / fullscreen
/// - `PageUp` / `PageDown`: music volume +/- 10%
/// - `NumpadAdd` / `NumpadSubtract`: SFX volume +/- 10%
fn global_display_and_volume_hotkeys(
    keys:        Res<ButtonInput<KeyCode>>,
    paths:       Res<AssetPaths>,
    mut audio:   ResMut<AudioSettings>,
    mut display: ResMut<DisplaySettings>,
    mut sfx:     EventWriter<SfxEvent>,
) {
    let mut changed = false;
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);

    if alt && keys.just_pressed(KeyCode::Enter) {
        display.fullscreen = !display.fullscreen;
        changed = true;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        audio.music_volume = (audio.music_volume + 10).min(100);
        changed = true;
    }
    if keys.just_pressed(KeyCode::PageDown) {
        audio.music_volume = audio.music_volume.saturating_sub(10);
        changed = true;
    }
    if keys.just_pressed(KeyCode::NumpadAdd) {
        audio.sfx_volume = (audio.sfx_volume + 10).min(100);
        sfx.send(SfxEvent(SfxKind::Click));
        changed = true;
    }
    if keys.just_pressed(KeyCode::NumpadSubtract) {
        audio.sfx_volume = audio.sfx_volume.saturating_sub(10);
        sfx.send(SfxEvent(SfxKind::Click));
        changed = true;
    }

    if changed {
        persist_options(&paths, &audio, &display);
    }
}

fn format_option_row(
    idx:      usize,
    selected: usize,
    audio:    &AudioSettings,
    display:  &DisplaySettings,
    level:    &LevelData,
) -> String {
    let prefix = if idx == selected { "> " } else { "  " };
    match idx {
        0 => format!("{}SFX VOLUME:    < {}% >", prefix, audio.sfx_volume),
        1 => format!("{}MUSIC VOLUME:  < {}% >", prefix, audio.music_volume),
        2 => format!("{}STAGE PACK:    < {} >", prefix, level.pack_name()),
        3 => format!("{}RANDOM LEVELS: < {} >", prefix, if level.random_levels { "ON" } else { "OFF" }),
        4 => format!("{}RESOLUTION:    < {} >", prefix, display.resolution.label()),
        5 => format!("{}FULLSCREEN:    < {} >", prefix, if display.fullscreen { "ON" } else { "OFF" }),
        6 => format!("{}EDIT TILES (Tilesheetinfo.tsi)", prefix),
        _ => format!("{}BACK TO MENU", prefix),
    }
}

fn spawn_options(
    mut commands: Commands,
    mut sel:      ResMut<OptionsSelection>,
    audio:        Res<AudioSettings>,
    display:      Res<DisplaySettings>,
    level:        Res<LevelData>,
) {
    sel.0 = 0;

    commands
        .spawn((
            OptionsRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap:         Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("OPTIONS"),
                TextFont { font_size: 52.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));

            for i in 0..OPTION_COUNT {
                parent.spawn((
                    OptionRowText(i),
                    Text::new(format_option_row(i, sel.0, &audio, &display, &level)),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(if i == sel.0 { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }),
                ));
            }

            parent.spawn((
                Text::new("Up/Down: Select | Left/Right: Adjust | Alt+Enter: Fullscreen | PgUp/PgDn: Music | ESC: Save & Back"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.7, 0.75, 0.85)),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn options_input(
    keys:        Res<ButtonInput<KeyCode>>,
    gamepads:    Query<&Gamepad>,
    mut sel:     ResMut<OptionsSelection>,
    mut audio:   ResMut<AudioSettings>,
    mut display: ResMut<DisplaySettings>,
    mut level:   ResMut<LevelData>,
    mut next:    ResMut<NextState<GameState>>,
    mut sfx:     EventWriter<SfxEvent>,
    mut text_q:  Query<(&OptionRowText, &mut Text, &mut TextColor)>,
) {
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let mut dirty = false;

    if keys.just_pressed(KeyCode::Escape) || input::any_gamepad_cancel(&gamepads) {
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::Menu);
        return;
    }

    if keys.just_pressed(KeyCode::ArrowDown) || input::any_gamepad_nav_down(&gamepads) {
        sel.0 = (sel.0 + 1) % OPTION_COUNT;
        dirty = true;
    } else if keys.just_pressed(KeyCode::ArrowUp) || input::any_gamepad_nav_up(&gamepads) {
        sel.0 = (sel.0 + OPTION_COUNT - 1) % OPTION_COUNT;
        dirty = true;
    }

    let left = keys.just_pressed(KeyCode::ArrowLeft) || input::any_gamepad_nav_left(&gamepads);
    let right = keys.just_pressed(KeyCode::ArrowRight) || input::any_gamepad_nav_right(&gamepads);
    let enter = (!alt && keys.just_pressed(KeyCode::Enter))
        || keys.just_pressed(KeyCode::Space)
        || input::any_gamepad_confirm(&gamepads);

    match sel.0 {
        0 => {
            if left {
                audio.sfx_volume = audio.sfx_volume.saturating_sub(10);
                dirty = true;
            } else if right {
                audio.sfx_volume = (audio.sfx_volume + 10).min(100);
                dirty = true;
            }
        }
        1 => {
            if left {
                audio.music_volume = audio.music_volume.saturating_sub(10);
                dirty = true;
            } else if right {
                audio.music_volume = (audio.music_volume + 10).min(100);
                dirty = true;
            }
        }
        2 => {
            let count = level.pack_count();
            if left {
                let idx = (level.pack_index + count - 1) % count;
                level.set_pack(idx);
                sfx.send(SfxEvent(SfxKind::Disk));
                dirty = true;
            } else if right || enter {
                let idx = (level.pack_index + 1) % count;
                level.set_pack(idx);
                sfx.send(SfxEvent(SfxKind::Disk));
                dirty = true;
            }
        }
        3 => {
            if left || right || enter {
                level.random_levels = !level.random_levels;
                if level.random_levels {
                    level.random_seed = level.random_seed.wrapping_add(1337);
                }
                dirty = true;
            }
        }
        4 => {
            if left {
                display.resolution = display.resolution.prev();
                dirty = true;
            } else if right || enter {
                display.resolution = display.resolution.next();
                dirty = true;
            }
        }
        5 => {
            if left || right || enter {
                display.fullscreen = !display.fullscreen;
                dirty = true;
            }
        }
        6 => {
            if enter {
                sfx.send(SfxEvent(SfxKind::Click));
                next.set(GameState::TileEditor);
                return;
            }
        }
        _ => {
            if enter {
                sfx.send(SfxEvent(SfxKind::Click));
                next.set(GameState::Menu);
                return;
            }
        }
    }

    if dirty || audio.is_changed() || display.is_changed() {
        if dirty {
            sfx.send(SfxEvent(SfxKind::Click));
        }
        for (row, mut text, mut color) in &mut text_q {
            **text = format_option_row(row.0, sel.0, &audio, &display, &level);
            color.0 = if row.0 == sel.0 {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::WHITE
            };
        }
    }
}

fn save_options_on_exit(
    paths:   Res<AssetPaths>,
    audio:   Res<AudioSettings>,
    display: Res<DisplaySettings>,
) {
    persist_options(&paths, &audio, &display);
}

fn despawn_options(mut commands: Commands, q: Query<Entity, With<OptionsRoot>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn saved_options_size_matches_options_oi() {
        assert_eq!(mem::size_of::<SavedOptions>(), 32);
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/Options.oi");
        let opts = load_saved_options(path.to_str().unwrap()).expect("Options.oi should load");
        assert!(opts.screen_width > 0 && opts.screen_height > 0);
    }

    #[test]
    fn save_and_load_options_round_trips_losslessly() {
        let tmp = std::env::temp_dir().join(format!(
            "openggs_options_test_{}.oi",
            std::process::id()
        ));
        let audio = AudioSettings { sfx_volume: 70, music_volume: 40 };
        let display = DisplaySettings {
            resolution: DisplayResolution::R1280x720,
            fullscreen: true,
            game_type: 2,
        };
        let expected = build_saved_options(&audio, &display);
        save_saved_options(tmp.to_str().unwrap(), &expected).expect("save should succeed");
        let loaded = load_saved_options(tmp.to_str().unwrap()).expect("load should succeed");
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(loaded, expected);
        assert_eq!(DisplayResolution::from_dims(loaded.screen_width, loaded.screen_height), DisplayResolution::R1280x720);
        assert_eq!(AudioSettings::from_saved(&loaded).sfx_volume, 70);
        assert_eq!(AudioSettings::from_saved(&loaded).music_volume, 40);
    }

    #[test]
    fn display_resolution_cycles_both_directions() {
        let r = DisplayResolution::R640x480;
        assert_eq!(r.next(), DisplayResolution::R800x600);
        assert_eq!(r.next().next(), DisplayResolution::R1280x720);
        assert_eq!(r.next().next().next(), DisplayResolution::R640x480);
        assert_eq!(r.prev(), DisplayResolution::R1280x720);
    }
}
