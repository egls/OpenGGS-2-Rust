use bevy::prelude::*;

use crate::states::GameState;

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct HudState {
    pub score:    u32,
    pub coins:    u32,
    pub lives:    u32,
    pub stage:    usize,
    pub power_up: u8,
    pub victory:  bool,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            score: 0,
            coins: 0,
            lives: 3,
            stage: 1,
            power_up: 0,
            victory: false,
        }
    }
}

impl HudState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Collect one coin (+50 points; 100 coins wraps to 0 and awards +1 life).
    pub fn add_coin(&mut self) {
        self.coins += 1;
        self.score += 50;
        if self.coins >= 100 {
            self.coins = 0;
            self.lives += 1;
        }
    }

    pub fn power_up_label(&self) -> &'static str {
        match self.power_up {
            0 => "NORMAL",
            1 => "PUNK",
            2 => "FIREBALL",
            3 => "RICOCHET",
            4..=6 => "HOMING",
            _ => "FIRE SHIELD",
        }
    }
}

// ── Marker components ─────────────────────────────────────────────────────────

#[derive(Component)] struct ScoreText;
#[derive(Component)] struct CoinsText;
#[derive(Component)] struct LivesText;
#[derive(Component)] struct StageText;
#[derive(Component)] struct PowerText;
#[derive(Component)] struct HudRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
           .add_systems(OnEnter(GameState::InGame), spawn_hud)
           .add_systems(Update, update_hud.run_if(in_state(GameState::InGame)))
           .add_systems(OnExit(GameState::InGame), despawn_hud);
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn spawn_hud(mut commands: Commands, hud: Res<HudState>) {
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
            // Score
            parent.spawn(Node { flex_direction: FlexDirection::Column, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("SCORE"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((ScoreText, Text::new(hud.score.to_string()), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Coins
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("COINS"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((CoinsText, Text::new(hud.coins.to_string()), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // PowerUp
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("MODE"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((PowerText, Text::new(hud.power_up_label()), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(0.4, 1.0, 0.7))));
                });

            // Lives
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("LIVES"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((LivesText, Text::new(hud.lives.to_string()), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Stage
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("STAGE"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((StageText, Text::new(hud.stage.to_string()), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });
        });
}

fn update_hud(
    hud:         Res<HudState>,
    mut score_q: Query<&mut Text, (With<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<StageText>, Without<PowerText>)>,
    mut coins_q: Query<&mut Text, (With<CoinsText>, Without<ScoreText>, Without<LivesText>, Without<StageText>, Without<PowerText>)>,
    mut lives_q: Query<&mut Text, (With<LivesText>, Without<ScoreText>, Without<CoinsText>, Without<StageText>, Without<PowerText>)>,
    mut stage_q: Query<&mut Text, (With<StageText>, Without<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<PowerText>)>,
    mut power_q: Query<&mut Text, (With<PowerText>, Without<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<StageText>)>,
) {
    if let Ok(mut t) = score_q.get_single_mut() { **t = hud.score.to_string(); }
    if let Ok(mut t) = coins_q.get_single_mut() { **t = hud.coins.to_string(); }
    if let Ok(mut t) = lives_q.get_single_mut() { **t = hud.lives.to_string(); }
    if let Ok(mut t) = stage_q.get_single_mut() { **t = hud.stage.to_string(); }
    if let Ok(mut t) = power_q.get_single_mut() { **t = hud.power_up_label().to_string(); }
}

fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hundred_coins_grants_extra_life() {
        let mut hud = HudState::default();
        for _ in 0..99 {
            hud.add_coin();
        }
        assert_eq!(hud.coins, 99);
        assert_eq!(hud.lives, 3);
        hud.add_coin();
        assert_eq!(hud.coins, 0);
        assert_eq!(hud.lives, 4);
        assert_eq!(hud.score, 5_000);
    }
}
