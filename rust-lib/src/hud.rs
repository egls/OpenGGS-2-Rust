use bevy::prelude::*;

use crate::assets::GameAssets;
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
#[derive(Component)] struct HudCoinIcon;
#[derive(Component)] struct HudRoot;

/// Small spinning HUD coin frames in `Tiles.png` (`GRAPHICS_Sprites_Draw.cpp:71-72`: `y=912`, 6 frames of `16x16`).
pub fn hud_coin_frame_rect(frame: usize) -> Rect {
    let x = ((frame % 6) * 16) as f32;
    Rect::new(x, 912.0, x + 16.0, 928.0)
}

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

fn spawn_hud(mut commands: Commands, hud: Res<HudState>, assets: Res<GameAssets>) {
    commands
        .spawn((
            HudRoot,
            Node {
                width:           Val::Percent(100.0),
                height:          Val::Px(48.0),
                flex_direction:  FlexDirection::Row,
                align_items:     AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding:         UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.65)),
        ))
        .with_children(|parent| {
            // Score
            parent.spawn(Node { flex_direction: FlexDirection::Column, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("SCORE"), TextFont { font_size: 13.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((ScoreText, Text::new(hud.score.to_string()), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Coins (with animated retro small-coin icon from Tiles.png)
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("BONUS"), TextFont { font_size: 13.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(5.0), ..default() })
                        .with_children(|row| {
                            row.spawn((
                                HudCoinIcon,
                                ImageNode {
                                    image: assets.tiles.clone(),
                                    rect: Some(hud_coin_frame_rect(0)),
                                    ..default()
                                },
                                Node {
                                    width:  Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    ..default()
                                },
                            ));
                            row.spawn((CoinsText, Text::new(hud.coins.to_string()), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                        });
                });

            // PowerUp
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("MODE"), TextFont { font_size: 13.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((PowerText, Text::new(hud.power_up_label()), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(0.4, 1.0, 0.7))));
                });

            // Lives
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("LIVES"), TextFont { font_size: 13.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((LivesText, Text::new(hud.lives.to_string()), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });

            // Stage
            parent.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..default() })
                .with_children(|p| {
                    p.spawn((Text::new("STAGE"), TextFont { font_size: 13.0, ..default() }, TextColor(Color::WHITE)));
                    p.spawn((StageText, Text::new(format!("{:02}", hud.stage)), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.2))));
                });
        });
}

fn update_hud(
    hud:         Res<HudState>,
    clock:       Res<crate::tilemap::TileAnimClock>,
    mut score_q: Query<&mut Text, (With<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<StageText>, Without<PowerText>)>,
    mut coins_q: Query<&mut Text, (With<CoinsText>, Without<ScoreText>, Without<LivesText>, Without<StageText>, Without<PowerText>)>,
    mut lives_q: Query<&mut Text, (With<LivesText>, Without<ScoreText>, Without<CoinsText>, Without<StageText>, Without<PowerText>)>,
    mut stage_q: Query<&mut Text, (With<StageText>, Without<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<PowerText>)>,
    mut power_q: Query<&mut Text, (With<PowerText>, Without<ScoreText>, Without<CoinsText>, Without<LivesText>, Without<StageText>)>,
    mut icon_q:  Query<&mut ImageNode, With<HudCoinIcon>>,
) {
    if let Ok(mut t) = score_q.get_single_mut() { **t = hud.score.to_string(); }
    if let Ok(mut t) = coins_q.get_single_mut() { **t = hud.coins.to_string(); }
    if let Ok(mut t) = lives_q.get_single_mut() { **t = hud.lives.to_string(); }
    if let Ok(mut t) = stage_q.get_single_mut() { **t = format!("{:02}", hud.stage); }
    if let Ok(mut t) = power_q.get_single_mut() { **t = hud.power_up_label().to_string(); }
    if let Ok(mut img) = icon_q.get_single_mut() {
        img.rect = Some(hud_coin_frame_rect(clock.frame_6 as usize));
    }
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

    #[test]
    fn hud_coin_frames_stay_within_tiles_png_bounds() {
        for f in 0..12 {
            let r = hud_coin_frame_rect(f);
            assert!(r.min.x >= 0.0 && r.max.x <= 96.0);
            assert_eq!(r.min.y, 912.0);
            assert_eq!(r.width(), 16.0);
            assert_eq!(r.height(), 16.0);
        }
    }
}
