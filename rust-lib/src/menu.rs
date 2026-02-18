use bevy::prelude::*;

use crate::states::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), spawn_menu)
           .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
           .add_systems(OnExit(GameState::Menu), despawn_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

fn spawn_menu(mut commands: Commands) {
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
            parent.spawn((
                Text::new("Press ENTER to Start"),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("Press ESC to Quit"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

fn menu_input(
    keys:     Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Enter) {
        next.set(GameState::InGame);
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}

fn despawn_menu(mut commands: Commands, q: Query<Entity, With<MenuRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}
