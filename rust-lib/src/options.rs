use std::fs::File;
use std::io::{self, Read};
use std::mem;

use bevy::prelude::*;

use crate::assets::LevelData;
use crate::audio::{SfxEvent, SfxKind};
use crate::paths::AssetPaths;
use crate::states::GameState;

/// Exact C++ `SavedOptions_Type` binary layout (`base/Options.oi`, 32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
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

// ── Resource ──────────────────────────────────────────────────────────────────

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
    pub fn from_options_file(path: &str) -> Self {
        match load_saved_options(path) {
            Ok(saved) => {
                let sfx = if (0..=100).contains(&saved.volume_sounds) && saved.volume_sounds > 0 {
                    saved.volume_sounds as u8
                } else {
                    80
                };
                let music = if (0..=100).contains(&saved.volume_music) && saved.volume_music > 0 {
                    saved.volume_music as u8
                } else {
                    60
                };
                Self {
                    sfx_volume: sfx,
                    music_volume: music,
                }
            }
            Err(_) => Self::default(),
        }
    }

    pub fn sfx_gain(&self) -> f32 {
        (self.sfx_volume as f32 / 100.0).clamp(0.0, 1.0)
    }

    pub fn music_gain(&self) -> f32 {
        (self.music_volume as f32 / 100.0).clamp(0.0, 1.0)
    }
}

// ── UI Components & State ─────────────────────────────────────────────────────

#[derive(Component)]
struct OptionsRoot;

#[derive(Component)]
struct OptionRowText(usize);

#[derive(Resource, Default)]
struct OptionsSelection(usize);

const OPTION_COUNT: usize = 4;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        let paths = AssetPaths::resolve();
        let initial = AudioSettings::from_options_file(paths.options_file().to_str().unwrap_or(""));
        app.insert_resource(initial)
           .init_resource::<OptionsSelection>()
           .add_systems(OnEnter(GameState::Options), spawn_options)
           .add_systems(Update, options_input.run_if(in_state(GameState::Options)))
           .add_systems(OnExit(GameState::Options), despawn_options);
    }
}

fn format_option_row(
    idx:      usize,
    selected: usize,
    audio:    &AudioSettings,
    level:    &LevelData,
) -> String {
    let prefix = if idx == selected { "> " } else { "  " };
    match idx {
        0 => format!("{}SFX VOLUME:   < {}% >", prefix, audio.sfx_volume),
        1 => format!("{}MUSIC VOLUME: < {}% >", prefix, audio.music_volume),
        2 => format!("{}STAGE PACK:   < {} >", prefix, level.pack_name()),
        _ => format!("{}BACK TO MENU", prefix),
    }
}

fn spawn_options(
    mut commands: Commands,
    mut sel:      ResMut<OptionsSelection>,
    audio:        Res<AudioSettings>,
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
                row_gap:         Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("OPTIONS"),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));

            for i in 0..OPTION_COUNT {
                parent.spawn((
                    OptionRowText(i),
                    Text::new(format_option_row(i, sel.0, &audio, &level)),
                    TextFont { font_size: 26.0, ..default() },
                    TextColor(if i == sel.0 { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }),
                ));
            }

            parent.spawn((
                Text::new("Up/Down: Select  |  Left/Right: Adjust  |  Enter/ESC: Back"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.7, 0.75, 0.85)),
            ));
        });
}

fn options_input(
    keys:       Res<ButtonInput<KeyCode>>,
    mut sel:    ResMut<OptionsSelection>,
    mut audio:  ResMut<AudioSettings>,
    mut level:  ResMut<LevelData>,
    mut next:   ResMut<NextState<GameState>>,
    mut sfx:    EventWriter<SfxEvent>,
    mut text_q: Query<(&OptionRowText, &mut Text, &mut TextColor)>,
) {
    let mut dirty = false;

    if keys.just_pressed(KeyCode::Escape) {
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::Menu);
        return;
    }

    if keys.just_pressed(KeyCode::ArrowDown) {
        sel.0 = (sel.0 + 1) % OPTION_COUNT;
        dirty = true;
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        sel.0 = (sel.0 + OPTION_COUNT - 1) % OPTION_COUNT;
        dirty = true;
    }

    let left = keys.just_pressed(KeyCode::ArrowLeft);
    let right = keys.just_pressed(KeyCode::ArrowRight);
    let enter = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);

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
        _ => {
            if enter {
                sfx.send(SfxEvent(SfxKind::Click));
                next.set(GameState::Menu);
                return;
            }
        }
    }

    if dirty {
        sfx.send(SfxEvent(SfxKind::Click));
        for (row, mut text, mut color) in &mut text_q {
            **text = format_option_row(row.0, sel.0, &audio, &level);
            color.0 = if row.0 == sel.0 {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::WHITE
            };
        }
    }
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/Options.oi");
        let opts = load_saved_options(path.to_str().unwrap()).expect("Options.oi should load");
        assert!(opts.screen_width > 0 && opts.screen_height > 0);
    }
}
