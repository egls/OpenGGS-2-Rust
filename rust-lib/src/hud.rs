use bevy::prelude::*;

use crate::states::GameState;

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct HudState {
    pub score: u32,
    pub lives: u32,
    pub stage: u32,
}

// ── Marker components ─────────────────────────────────────────────────────────

#[derive(Component)] struct ScoreText;
#[derive(Component)] struct LivesText;
#[derive(Component)] struct StageText;
#[derive(Component)] struct HudRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HudState { score: 0, lives: 3, stage: 1 })
           .add_systems(OnEnter(GameState::InGame), spawn_hud)
           .add_systems(Update, update_hud.run_if(in_state(GameState::InGame)))
           .add_systems(OnExit(GameState::InGame), despawn_hud);
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Row,
                align_items:     AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                padding:         UiRect::all(Val::Px(8.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Left: PLAYER / score
            parent.spawn(Node { flex_direction: FlexDirection::Column, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("PLAYER"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((ScoreText, Text::new("0"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Center: LIVES
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("LIVES"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((LivesText, Text::new("3"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Right: STAGE
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("STAGE"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((StageText, Text::new("1"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });
        });
}

fn update_hud(
    hud:        Res<HudState>,
    mut score_q: Query<&mut Text, (With<ScoreText>, Without<LivesText>, Without<StageText>)>,
    mut lives_q: Query<&mut Text, (With<LivesText>, Without<ScoreText>, Without<StageText>)>,
    mut stage_q: Query<&mut Text, (With<StageText>, Without<ScoreText>, Without<LivesText>)>,
) {
    if let Ok(mut t) = score_q.get_single_mut() { **t = hud.score.to_string(); }
    if let Ok(mut t) = lives_q.get_single_mut() { **t = hud.lives.to_string(); }
    if let Ok(mut t) = stage_q.get_single_mut() { **t = hud.stage.to_string(); }
}

fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}
