use bevy::prelude::*;

/// Top-level game states driving the state machine.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    /// Assets are being loaded.
    #[default]
    Loading,
    /// Main menu is shown.
    Menu,
    /// Gameplay is active.
    InGame,
    /// Game over screen.
    GameOver,
}
