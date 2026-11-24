use bevy::prelude::*;

use crate::assets::{GameAssets, LevelData};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
use crate::input;
use crate::states::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuSelection>()
           .add_systems(OnEnter(GameState::Menu), spawn_menu)
           .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
           .add_systems(OnExit(GameState::Menu), despawn_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct MenuItemText(usize);

#[derive(Resource, Default)]
struct MenuSelection(usize);

const MENU_ITEM_COUNT: usize = 6;

fn format_menu_item(idx: usize, selected: usize, pack_name: &str) -> String {
    let prefix = if idx == selected { "> " } else { "  " };
    match idx {
        0 => format!("{}START GAME", prefix),
        1 => format!("{}STAGE PACK: < {} >", prefix, pack_name),
        2 => format!("{}OPTIONS", prefix),
        3 => format!("{}STORY", prefix),
        4 => format!("{}CREDITS", prefix),
        _ => format!("{}QUIT", prefix),
    }
}

/// Source rect for the original 320x200 title art embedded in `Tiles.png` (`LOOP_Menu.cpp:279-282`).
pub const TITLE_BANNER_RECT: Rect = Rect {
    min: Vec2::new(0.0, 1000.0),
    max: Vec2::new(320.0, 1200.0),
};

/// 32x32 menu icon rects from `Interface.png` (`LOOP_Menu.cpp:148-266`).
fn menu_icon_rect(idx: usize) -> Rect {
    let (x, y) = match idx {
        0 => (256.0, 240.0), // Start / Resume
        1 => (256.0, 208.0), // Load Stagefile
        2 => (288.0, 208.0), // Options
        3 => (256.0, 272.0), // Story
        4 => (320.0, 208.0), // Credits
        _ => (288.0, 240.0), // Quit
    };
    Rect::new(x, y, x + 32.0, y + 32.0)
}

fn spawn_menu(
    mut commands: Commands,
    mut sel:      ResMut<MenuSelection>,
    level:        Res<LevelData>,
    assets:       Res<GameAssets>,
) {
    sel.0 = 0;

    commands
        .spawn((
            MenuRoot,
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
            // Original C64 title screen banner from Tiles.png (320x200)
            parent.spawn((
                ImageNode {
                    image: assets.tiles.clone(),
                    rect: Some(TITLE_BANNER_RECT),
                    ..default()
                },
                Node {
                    width:  Val::Px(320.0),
                    height: Val::Px(200.0),
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
            ));

            for i in 0..MENU_ITEM_COUNT {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items:    AlignItems::Center,
                        column_gap:     Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            ImageNode {
                                image: assets.interface.clone(),
                                rect: Some(menu_icon_rect(i)),
                                ..default()
                            },
                            Node {
                                width:  Val::Px(24.0),
                                height: Val::Px(24.0),
                                ..default()
                            },
                        ));
                        row.spawn((
                            MenuItemText(i),
                            Text::new(format_menu_item(i, sel.0, level.pack_name())),
                            TextFont { font_size: 26.0, ..default() },
                            TextColor(if i == sel.0 { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }),
                        ));
                    });
            }

            parent.spawn((
                Text::new("Arrows: Move | Space/Up: Jump | Down: Fast-Fall | Ctrl/X: Shoot | P/ESC: Pause"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.72, 0.78, 0.9)),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn menu_input(
    keys:       Res<ButtonInput<KeyCode>>,
    gamepads:   Query<&Gamepad>,
    mut sel:    ResMut<MenuSelection>,
    mut next:   ResMut<NextState<GameState>>,
    mut hud:    ResMut<HudState>,
    mut level:  ResMut<LevelData>,
    mut sfx:    EventWriter<SfxEvent>,
    mut exit:   EventWriter<AppExit>,
    mut text_q: Query<(&MenuItemText, &mut Text, &mut TextColor)>,
) {
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let mut dirty = false;

    if keys.just_pressed(KeyCode::ArrowDown) || input::any_gamepad_nav_down(&gamepads) {
        sel.0 = (sel.0 + 1) % MENU_ITEM_COUNT;
        dirty = true;
    } else if keys.just_pressed(KeyCode::ArrowUp) || input::any_gamepad_nav_up(&gamepads) {
        sel.0 = (sel.0 + MENU_ITEM_COUNT - 1) % MENU_ITEM_COUNT;
        dirty = true;
    }

    if sel.0 == 1 {
        let count = level.pack_count();
        if keys.just_pressed(KeyCode::ArrowLeft) || input::any_gamepad_nav_left(&gamepads) {
            let idx = (level.pack_index + count - 1) % count;
            level.set_pack(idx);
            sfx.send(SfxEvent(SfxKind::Disk));
            dirty = true;
        } else if keys.just_pressed(KeyCode::ArrowRight) || input::any_gamepad_nav_right(&gamepads) {
            let idx = (level.pack_index + 1) % count;
            level.set_pack(idx);
            sfx.send(SfxEvent(SfxKind::Disk));
            dirty = true;
        }
    }

    if dirty {
        sfx.send(SfxEvent(SfxKind::Click));
        for (item, mut text, mut color) in &mut text_q {
            **text = format_menu_item(item.0, sel.0, level.pack_name());
            color.0 = if item.0 == sel.0 {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::WHITE
            };
        }
    }

    let confirm = (!alt && keys.just_pressed(KeyCode::Enter))
        || keys.just_pressed(KeyCode::Space)
        || input::any_gamepad_confirm(&gamepads);

    if confirm {
        sfx.send(SfxEvent(SfxKind::Click));
        match sel.0 {
            0 => {
                hud.reset();
                level.checkpoint = 0;
                level.sub_stage_entry_stage = None;
                level.sub_stage_counter = 0;
                level.is_warping = false;
                next.set(GameState::StageTransition);
            }
            1 => {
                let count = level.pack_count();
                let idx = (level.pack_index + 1) % count;
                level.set_pack(idx);
                sfx.send(SfxEvent(SfxKind::Disk));
                for (item, mut text, mut color) in &mut text_q {
                    **text = format_menu_item(item.0, sel.0, level.pack_name());
                    color.0 = if item.0 == sel.0 {
                        Color::srgb(1.0, 0.9, 0.3)
                    } else {
                        Color::WHITE
                    };
                }
            }
            2 => next.set(GameState::Options),
            3 => next.set(GameState::Story),
            4 => next.set(GameState::Credits),
            _ => {
                exit.send(AppExit::Success);
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}

fn despawn_menu(mut commands: Commands, q: Query<Entity, With<MenuRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}
