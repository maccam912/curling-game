//! # Game Systems
//!
//! This module contains all Bevy ECS systems organized by functionality.
//!
//! ## Submodules
//! - `setup`: Scene and UI initialization
//! - `input`: Keyboard, mouse, and touch handling
//! - `physics`: Ice friction and collision detection
//! - `camera`: Camera control and transitions
//! - `game_logic`: Shot resolution and rule enforcement
//! - `ui`: UI updates and display
//! - `viewport`: Responsive viewport detection
//! - `menu`: Main menu UI and navigation
//! - `online`: Online multiplayer menu and lobby
//! - `online_game`: Online game synchronization

pub mod camera;
pub mod game_logic;
pub mod input;
pub mod menu;
pub mod online;
pub mod online_game;
pub mod physics;
pub mod setup;
pub mod ui;
pub mod viewport;

// Re-export all systems for convenient access
pub use camera::*;
pub use game_logic::*;
pub use input::*;
pub use menu::*;
pub use online::*;
pub use online_game::*;
pub use physics::*;
pub use setup::*;
pub use ui::*;
pub use viewport::*;
