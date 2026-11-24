use bevy::prelude::*;

use crate::assets::LevelData;
use crate::hud::HudState;
use crate::states::GameState;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::GameOver), spawn_game_over)
           .add_systems(Update, game_over_input.run_if(in_state(GameState::GameOver)))
           .add_systems(OnExit(GameState::GameOver), despawn_game_over);
    }
}

#[derive(Component)]
struct GameOverRoot;

fn spawn_game_over(mut commands: Commands, hud: Res<HudState>) {
    let (headline, headline_color) = if hud.victory {
        ("VICTORY!", Color::srgb(1.0, 0.85, 0.2))
    } else {
        ("GAME OVER", Color::srgb(1.0, 0.15, 0.15))
    };

    commands
        .spawn((
            GameOverRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap:         Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(headline),
                TextFont { font_size: 76.0, ..default() },
                TextColor(headline_color),
            ));

            parent.spawn((
                Text::new(format!("FINAL SCORE: {}  |  COINS: {}", hud.score, hud.coins)),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
            ));

            parent.spawn((
                Text::new(format!("STAGE REACHED: {}", hud.stage)),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.85, 1.0)),
            ));

            parent.spawn((
                Text::new("Press R to Restart or M for Menu"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn game_over_input(
    keys:           Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut hud:        ResMut<HudState>,
    mut level:      ResMut<LevelData>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        hud.reset();
        level.checkpoint = 0;
        next_state.set(GameState::StageTransition);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        next_state.set(GameState::Menu);
    }
}

fn despawn_game_over(mut commands: Commands, q: Query<Entity, With<GameOverRoot>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
