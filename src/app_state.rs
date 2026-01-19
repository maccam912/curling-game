//! # Application State
//!
//! Bevy state machine for app-level navigation between game modes.
//!
//! ## States
//! - `MainMenu`: Initial menu screen with game mode selection
//! - `PassAndPlay`: Local hot-seat multiplayer (current game behavior)
//! - `OnlineMenu`: Create/Join game selection for online play
//! - `OnlineLobby`: Waiting room while connecting to opponent
//! - `OnlineGame`: Active online match

use bevy::prelude::*;

/// Main application states for navigation between game modes.
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    /// Splash screen with logo/title.
    #[default]
    Splash,
    /// Main menu with game mode selection.
    MainMenu,
    /// Settings menu.
    Settings,
    /// Game is paused.
    Paused,
    /// Local pass-and-play mode (hot-seat multiplayer).
    PassAndPlay,
    /// Single player vs AI mode.
    VsAI,
    /// Online multiplayer menu (create/join game).
    OnlineMenu,
    /// Lobby waiting for opponent to connect.
    OnlineLobby,
    /// Active online multiplayer game.
    OnlineGame,
}

/// Role in an online multiplayer game.
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum NetworkRole {
    /// Not in an online game.
    #[default]
    None,
    /// Created the game (Team 1).
    Host,
    /// Joined an existing game (Team 2).
    Guest,
}
