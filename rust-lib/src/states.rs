use bevy::prelude::*;

/// Top-level game states driving the state machine.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    /// Assets are being loaded.
    #[default]
    Loading,
    /// Main menu is shown.
    Menu,
    /// Reloads target stage data and shows a brief stage banner before gameplay.
    StageTransition,
    /// Gameplay is active.
    InGame,
    /// Game over (or victory) screen.
    GameOver,
}
