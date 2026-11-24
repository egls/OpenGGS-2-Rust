use bevy::prelude::*;

/// Top-level game states driving the state machine.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    /// Assets are being loaded.
    #[default]
    Loading,
    /// Main menu is shown (with level pack selector and Options).
    Menu,
    /// Options screen (SFX / Music volume, resolution, fullscreen & level pack).
    Options,
    /// Original background story screen.
    Story,
    /// Original credits screen.
    Credits,
    /// Reloads target stage data and shows a brief stage banner before gameplay.
    StageTransition,
    /// Gameplay is active.
    InGame,
    /// Game over (or victory) screen.
    GameOver,
}

/// Pauses active gameplay simulation without despawning the current stage.
#[derive(Resource, Default)]
pub struct GamePaused(pub bool);

pub fn not_paused(paused: Res<GamePaused>) -> bool {
    !paused.0
}
