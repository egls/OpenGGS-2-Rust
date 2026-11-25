use bevy::prelude::*;

use crate::audio::{SfxEvent, SfxKind};
use crate::input;
use crate::states::GameState;

pub struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Story), spawn_story)
           .add_systems(Update, back_to_menu_input.run_if(in_state(GameState::Story)))
           .add_systems(OnExit(GameState::Story), despawn_screen)
           .add_systems(OnEnter(GameState::Credits), spawn_credits)
           .add_systems(Update, back_to_menu_input.run_if(in_state(GameState::Credits)))
           .add_systems(OnExit(GameState::Credits), despawn_screen);
    }
}

#[derive(Component)]
struct ScreenRoot;

/// Original story lines from C++ `src/LOOP_Story.cpp`.
pub const STORY_LINES: &[(&str, bool)] = &[
    ("Backgroundstory of 'The Great Giana Sisters'", true),
    ("", false),
    ("What happened?", true),
    ("One night, when little Giana from Milano was", false),
    ("fast asleep, she has a strange dream.", false),
    ("Everybody dreams weird things at night, but", false),
    ("no one will have experienced situations like", false),
    ("little Giana is about to.", false),
    ("", false),
    ("Giana suddenly finds herself in a strange", false),
    ("mysterious world, where everything is completely", false),
    ("different. Gravity has almost disappeared -", false),
    ("sometimes one feels like flying away - and", false),
    ("everywhere there are unexplainable buildings", false),
    ("and structures. Old grottos and deserted castles", false),
    ("seem to hide lots of secrets, and frightening and", false),
    ("hideous creatures appear. This wouldn't be too bad,", false),
    ("except that Giana can't leave this world unless she", false),
    ("finds the magic, huge diamond. So she starts", false),
    ("searching for this wonderful jewel.", false),
    ("", false),
    ("However, she is not totally alone, for her little", true),
    ("sister Maria can dream, too.", true),
];

/// Original credits lines from C++ `src/LOOP_Credits.cpp` plus Rust/Bevy port note.
pub const CREDITS_LINES: &[(&str, bool)] = &[
    ("CREDITS", true),
    ("", false),
    ("The Great Giana Sisters (C64)", true),
    ("1987 Softgold/Rainbow Arts", false),
    ("Written:      Armin Gessert", false),
    ("Artwork:      Manfred Trenz", false),
    ("Music:        Chris Huelsbeck", false),
    ("", false),
    ("OpenGGS 2.0 - OpenGGS Maker", true),
    ("Written:      RomanH", false),
    ("Artwork:      RomanH", false),
    ("Soundeffects: RomanH, Kenney (opengameart.org)", false),
    ("Music:        Alejandro Altimari (puch666)", false),
    ("", false),
    ("Interface elements", true),
    ("Clipart:      openclipart.org (Tango-Collection)", false),
    ("", false),
    ("OpenGGS-2-Rust (Bevy Port)", true),
    ("Rust/Bevy rewrite with native .it tracker audio", false),
    ("Thanks to everybody involved in porting the game!", true),
];

fn spawn_text_screen(commands: &mut Commands, title: &str, lines: &[(&str, bool)]) {
    commands
        .spawn((
            ScreenRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding:         UiRect::all(Val::Px(24.0)),
                row_gap:         Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.14)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont { font_size: 36.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items:    AlignItems::FlexStart,
                    row_gap:        Val::Px(2.0),
                    ..default()
                })
                .with_children(|col| {
                    for &(line, highlight) in lines {
                        col.spawn((
                            Text::new(if line.is_empty() { " " } else { line }),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(if highlight {
                                Color::srgb(1.0, 0.88, 0.35)
                            } else {
                                Color::srgb(0.9, 0.93, 0.98)
                            }),
                        ));
                    }
                });

            parent.spawn((
                Text::new("Press ESC / Enter / Space to return to Menu"),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::srgb(0.68, 0.74, 0.86)),
                Node {
                    margin: UiRect::top(Val::Px(14.0)),
                    ..default()
                },
            ));
        });
}

fn spawn_story(mut commands: Commands) {
    spawn_text_screen(&mut commands, "STORY", STORY_LINES);
}

fn spawn_credits(mut commands: Commands) {
    spawn_text_screen(&mut commands, "CREDITS", CREDITS_LINES);
}

fn back_to_menu_input(
    keys:     Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut sfx:  EventWriter<SfxEvent>,
    mut next: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space)
        || input::any_gamepad_cancel(&gamepads)
        || input::any_gamepad_confirm(&gamepads)
    {
        sfx.send(SfxEvent(SfxKind::Click));
        next.set(GameState::Menu);
    }
}

fn despawn_screen(mut commands: Commands, q: Query<Entity, With<ScreenRoot>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_and_credits_content_present() {
        assert!(STORY_LINES.iter().any(|(l, _)| l.contains("Giana")));
        assert!(CREDITS_LINES.iter().any(|(l, _)| l.contains("Chris Huelsbeck")));
        assert!(CREDITS_LINES.iter().any(|(l, _)| l.contains("RomanH")));
    }
}
