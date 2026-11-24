use bevy::prelude::*;

use crate::assets::{LevelData, STAGE_PACKS};
use crate::audio::{SfxEvent, SfxKind};
use crate::hud::HudState;
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

const MENU_ITEM_COUNT: usize = 4;

fn format_menu_item(idx: usize, selected: usize, pack_name: &str) -> String {
    let prefix = if idx == selected { "> " } else { "  " };
    match idx {
        0 => format!("{}START GAME", prefix),
        1 => format!("{}STAGE PACK: < {} >", prefix, pack_name),
        2 => format!("{}OPTIONS", prefix),
        _ => format!("{}QUIT", prefix),
    }
}

fn spawn_menu(
    mut commands: Commands,
    mut sel:      ResMut<MenuSelection>,
    level:        Res<LevelData>,
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
                row_gap:         Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("OpenGGS"),
                TextFont { font_size: 64.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
            ));

            for i in 0..MENU_ITEM_COUNT {
                parent.spawn((
                    MenuItemText(i),
                    Text::new(format_menu_item(i, sel.0, level.pack_name())),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(if i == sel.0 { Color::srgb(1.0, 0.9, 0.3) } else { Color::WHITE }),
                ));
            }

            parent.spawn((
                Text::new("Arrows: Move | Space/Up: Jump | Ctrl/X: Shoot | P/ESC: Pause"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.72, 0.78, 0.9)),
            ));
        });
}

fn menu_input(
    keys:       Res<ButtonInput<KeyCode>>,
    mut sel:    ResMut<MenuSelection>,
    mut next:   ResMut<NextState<GameState>>,
    mut hud:    ResMut<HudState>,
    mut level:  ResMut<LevelData>,
    mut sfx:    EventWriter<SfxEvent>,
    mut exit:   EventWriter<AppExit>,
    mut text_q: Query<(&MenuItemText, &mut Text, &mut TextColor)>,
) {
    let mut dirty = false;

    if keys.just_pressed(KeyCode::ArrowDown) {
        sel.0 = (sel.0 + 1) % MENU_ITEM_COUNT;
        dirty = true;
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        sel.0 = (sel.0 + MENU_ITEM_COUNT - 1) % MENU_ITEM_COUNT;
        dirty = true;
    }

    if sel.0 == 1 {
        if keys.just_pressed(KeyCode::ArrowLeft) {
            let idx = (level.pack_index + STAGE_PACKS.len() - 1) % STAGE_PACKS.len();
            level.set_pack(idx);
            dirty = true;
        } else if keys.just_pressed(KeyCode::ArrowRight) {
            let idx = (level.pack_index + 1) % STAGE_PACKS.len();
            level.set_pack(idx);
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

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        sfx.send(SfxEvent(SfxKind::Click));
        match sel.0 {
            0 => {
                hud.reset();
                level.checkpoint = 0;
                level.sub_stage_entry_stage = None;
                level.sub_stage_counter = 0;
                next.set(GameState::StageTransition);
            }
            1 => {
                let idx = (level.pack_index + 1) % STAGE_PACKS.len();
                level.set_pack(idx);
                for (item, mut text, mut color) in &mut text_q {
                    **text = format_menu_item(item.0, sel.0, level.pack_name());
                    color.0 = if item.0 == sel.0 {
                        Color::srgb(1.0, 0.9, 0.3)
                    } else {
                        Color::WHITE
                    };
                }
            }
            2 => {
                next.set(GameState::Options);
            }
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
