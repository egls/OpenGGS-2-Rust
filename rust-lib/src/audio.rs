use bevy::prelude::*;

use crate::assets::GameAssets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxKind {
    Jump,
    Stomp,
    Died,
    Ding,
    Crack,
    Morph,
    Shoot,
    Portal,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct SfxEvent(pub SfxKind);

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SfxEvent>()
           .add_systems(Update, play_sfx);
    }
}

fn play_sfx(
    mut commands: Commands,
    assets_opt: Option<Res<GameAssets>>,
    mut events: EventReader<SfxEvent>,
) {
    let Some(assets) = assets_opt else { return };
    for SfxEvent(kind) in events.read() {
        let handle = match kind {
            SfxKind::Jump   => assets.jump_sfx.clone(),
            SfxKind::Stomp  => assets.stomp_sfx.clone(),
            SfxKind::Died   => assets.died_sfx.clone(),
            SfxKind::Ding   => assets.ding_sfx.clone(),
            SfxKind::Crack  => assets.crack_sfx.clone(),
            SfxKind::Morph  => assets.morph_sfx.clone(),
            SfxKind::Shoot  => assets.shoot_sfx.clone(),
            SfxKind::Portal => assets.portal_sfx.clone(),
        };
        commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}
