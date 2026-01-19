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
//! - `ai`: AI opponent for single-player mode
//! - `prediction`: Ghost stone trajectory prediction
//! - `reflection`: Planar reflection camera for ice

pub mod ai;
pub mod camera;
pub mod game_logic;
pub mod input;
pub mod menu;
pub mod online;
pub mod online_game;
pub mod pause;
pub mod physics;
pub mod prediction;
// pub mod reflection; // Removed
pub mod settings;
pub mod setup;
pub mod splash;
pub mod ui;
pub mod viewport;

// Re-export all systems for convenient access
pub use ai::*;
pub use camera::*;
pub use game_logic::*;
pub use input::*;
pub use menu::*;
pub use online::*;
pub use online_game::*;
pub use pause::*;
pub use physics::*;
pub use prediction::*;
// pub use reflection::*; // Removed
pub use settings::*;
pub use setup::*;
pub use splash::*;
pub use ui::*;
pub use viewport::*;
