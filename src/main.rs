//! # Curling Game Entry Point
//!
//! This is the main entry point for the curling game application.
//! All game logic is contained in the `curling_game` library crate.

use bevy::prelude::*;
use curling_game::CurlingPlugin;

/// Application entry point.
///
/// Creates a Bevy app with default plugins and the curling game plugin.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Curling - Pass and Play".to_string(),
                resolution: (1400, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CurlingPlugin)
        .run();
}
