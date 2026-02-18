use bevy::prelude::*;
use crate::states::GameState;
use crate::hud::HudState;

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
    commands
        .spawn((
            GameOverRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GAME OVER"),
                TextFont { font_size: 80.0, ..default() },
                TextColor(Color::srgb(1.0, 0.1, 0.1)),
            ));
            
            parent.spawn((
                Text::new(format!("FINAL SCORE: {}", hud.score)),
                TextFont { font_size: 40.0, ..default() },
                TextColor(Color::WHITE),
            ));

            parent.spawn((
                Text::new("Press R to Restart or M for Menu"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut hud: ResMut<HudState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        // Reset HUD and restart game
        hud.score = 0;
        hud.lives = 3;
        next_state.set(GameState::InGame);
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
