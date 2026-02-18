use bevy::prelude::*;
use crate::enemy::StompEvent;
use crate::assets::GameAssets;
use crate::states::GameState;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (play_stomp_audio, play_jump_audio).run_if(in_state(GameState::InGame)));
    }
}

fn play_stomp_audio(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut stomp_events: EventReader<StompEvent>,
) {
    for _ in stomp_events.read() {
        commands.spawn(AudioPlayer::new(assets.stomp_sfx.clone()));
    }
}

fn play_jump_audio(
    mut commands: Commands,
    assets: Res<GameAssets>,
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&crate::player::OnGround, With<crate::player::Player>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        if let Ok(on_ground) = player_q.get_single() {
            if on_ground.0 {
                commands.spawn(AudioPlayer::new(assets.jump_sfx.clone()));
            }
        }
    }
}
