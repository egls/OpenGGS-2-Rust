use std::fs;
use std::time::Duration;

use bevy::audio::{AddAudioSource, Decodable, Source, Volume};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use xmrs::prelude::Module;
use xmrsplayer::prelude::XmrsPlayer;

use crate::assets::{GameAssets, LevelData};
use crate::options::AudioSettings;
use crate::states::GameState;

const TRACKER_SAMPLE_RATE: u32 = 44_100;

// ── SFX Events ────────────────────────────────────────────────────────────────

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
    Click,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct SfxEvent(pub SfxKind);

// ── Custom Bevy Decodable Audio Source for Impulse Tracker (.it) Music ────────

#[derive(Asset, TypePath, Clone)]
pub struct TrackerAudio {
    module: &'static Module,
}

pub struct TrackerDecoder {
    module: &'static Module,
    player: XmrsPlayer<'static>,
}

impl TrackerDecoder {
    fn new(module: &'static Module) -> Self {
        Self {
            module,
            player: XmrsPlayer::new(module, TRACKER_SAMPLE_RATE, 0),
        }
    }
}

impl Iterator for TrackerDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.player.next() {
            return Some(sample as f32 / 32_768.0);
        }
        // Loop seamlessly back to song start
        self.player = XmrsPlayer::new(self.module, TRACKER_SAMPLE_RATE, 0);
        self.player.next().map(|sample| sample as f32 / 32_768.0)
    }
}

impl Source for TrackerDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        TRACKER_SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for TrackerAudio {
    type DecoderItem = f32;
    type Decoder = TrackerDecoder;

    fn decoder(&self) -> Self::Decoder {
        TrackerDecoder::new(self.module)
    }
}

pub fn load_tracker_module(path: &str) -> Result<Module, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {}", path, e))?;
    Module::load(&bytes).map_err(|e| format!("failed to parse {}: {:?}", path, e))
}

// ── Background Music State ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusicTrack {
    Title,
    Outdoors,
    Indoors,
    Highscore,
}

#[derive(Resource)]
struct MusicAssets {
    title:     Option<Handle<TrackerAudio>>,
    outdoors:  Option<Handle<TrackerAudio>>,
    indoors:   Option<Handle<TrackerAudio>>,
    highscore: Option<Handle<TrackerAudio>>,
    current:   Option<MusicTrack>,
}

#[derive(Component)]
struct BgmPlayer;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<TrackerAudio>()
           .add_event::<SfxEvent>()
           .add_systems(Startup, load_music_tracks)
           .add_systems(Update, (play_sfx, update_bgm_track, sync_bgm_volume));
    }
}

fn load_music_tracks(
    mut commands: Commands,
    mut tracker_assets: ResMut<Assets<TrackerAudio>>,
) {
    let mut load_one = |path: &str| -> Option<Handle<TrackerAudio>> {
        match load_tracker_module(path) {
            Ok(module) => {
                // Static lifetime for the 4 fixed game BGM tracks so XmrsPlayer<'static> streams safely.
                let leaked: &'static Module = Box::leak(Box::new(module));
                Some(tracker_assets.add(TrackerAudio { module: leaked }))
            }
            Err(err) => {
                eprintln!("Music warning: {}", err);
                None
            }
        }
    };

    commands.insert_resource(MusicAssets {
        title:     load_one("../base/music/title.it"),
        outdoors:  load_one("../base/music/outdoors.it"),
        indoors:   load_one("../base/music/indoors.it"),
        highscore: load_one("../base/music/highscore.it"),
        current:   None,
    });
}

fn play_sfx(
    mut commands: Commands,
    assets_opt:   Option<Res<GameAssets>>,
    settings:     Res<AudioSettings>,
    mut events:   EventReader<SfxEvent>,
) {
    let Some(assets) = assets_opt else { return };
    let gain = settings.sfx_gain();
    for SfxEvent(kind) in events.read() {
        if gain <= 0.0 {
            continue;
        }
        let handle = match kind {
            SfxKind::Jump   => assets.jump_sfx.clone(),
            SfxKind::Stomp  => assets.stomp_sfx.clone(),
            SfxKind::Died   => assets.died_sfx.clone(),
            SfxKind::Ding   => assets.ding_sfx.clone(),
            SfxKind::Crack  => assets.crack_sfx.clone(),
            SfxKind::Morph  => assets.morph_sfx.clone(),
            SfxKind::Shoot  => assets.shoot_sfx.clone(),
            SfxKind::Portal => assets.portal_sfx.clone(),
            SfxKind::Click  => assets.click_sfx.clone(),
        };
        commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN.with_volume(Volume::new(gain)),
        ));
    }
}

fn update_bgm_track(
    mut commands: Commands,
    state:        Res<State<GameState>>,
    level_opt:    Option<Res<LevelData>>,
    settings:     Res<AudioSettings>,
    mut music:    ResMut<MusicAssets>,
    bgm_q:        Query<Entity, With<BgmPlayer>>,
) {
    let desired = match state.get() {
        GameState::Loading => return,
        GameState::Menu | GameState::Options => MusicTrack::Title,
        GameState::StageTransition | GameState::InGame => {
            let bg = level_opt.map(|l| l.stage.background_colour).unwrap_or(0);
            if bg == 0 { MusicTrack::Outdoors } else { MusicTrack::Indoors }
        }
        GameState::GameOver => MusicTrack::Highscore,
    };

    if music.current == Some(desired) {
        return;
    }

    for ent in &bgm_q {
        commands.entity(ent).despawn();
    }

    let handle_opt = match desired {
        MusicTrack::Title     => music.title.clone(),
        MusicTrack::Outdoors  => music.outdoors.clone(),
        MusicTrack::Indoors   => music.indoors.clone(),
        MusicTrack::Highscore => music.highscore.clone(),
    };

    music.current = Some(desired);
    if let Some(handle) = handle_opt {
        commands.spawn((
            BgmPlayer,
            AudioPlayer(handle),
            PlaybackSettings::LOOP.with_volume(Volume::new(settings.music_gain())),
        ));
    }
}

fn sync_bgm_volume(
    settings: Res<AudioSettings>,
    bgm_q:    Query<&AudioSink, With<BgmPlayer>>,
) {
    if !settings.is_changed() {
        return;
    }
    let gain = settings.music_gain();
    for sink in &bgm_q {
        sink.set_volume(gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn all_four_it_music_tracks_parse_and_generate_pcm() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/music");
        for name in ["title.it", "outdoors.it", "indoors.it", "highscore.it"] {
            let path = dir.join(name);
            let module = load_tracker_module(path.to_str().unwrap())
                .unwrap_or_else(|e| panic!("failed to load {}: {}", name, e));
            let mut player = XmrsPlayer::new(&module, TRACKER_SAMPLE_RATE, 0);
            let samples: Vec<i16> = player.by_ref().take(256).collect();
            assert_eq!(samples.len(), 256, "{} did not generate PCM samples", name);
        }
    }
}
