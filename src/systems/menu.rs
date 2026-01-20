//! # Main Menu System
//!
//! UI and logic for the main menu screen.

use bevy::prelude::*;

use crate::app_state::AppState;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Marker for the main menu root node.
#[derive(Component)]
pub struct MainMenuRoot;

/// Marker for the menu camera.
#[derive(Component)]
pub struct MenuCamera;

/// Marker for the "Pass and Play" button.
#[derive(Component)]
pub struct PassAndPlayButton;

/// Marker for the "Online Multiplayer" button.
#[derive(Component)]
pub struct OnlineMultiplayerButton;

/// Marker for the "Vs AI" button.
#[derive(Component)]
pub struct VsAIButton;

/// Marker for the "Watch AI" button (AI vs AI spectator mode).
#[derive(Component)]
pub struct WatchAIButton;

/// Marker for the "Settings" button.
#[derive(Component)]
pub struct SettingsButton;

/// Marker for the "Quit" button.
#[derive(Component)]
pub struct QuitButton;

// ============================================================================
// SYSTEMS
// ============================================================================

/// Sets up the main menu UI.
pub fn setup_main_menu(mut commands: Commands) {
    // Spawn a camera for the menu (UI needs a camera to render)
    commands.spawn((Camera2d::default(), MenuCamera));

    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.12, 0.18)),
            MainMenuRoot,
        ))
        .with_children(|parent| {
            // Game title
            parent.spawn((
                Text::new("CURLING"),
                TextFont {
                    font_size: 80.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(60.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("Select Game Mode"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Pass and Play button
            spawn_menu_button(
                parent,
                "Pass and Play",
                "Local hot-seat multiplayer",
                PassAndPlayButton,
                Color::srgb(0.2, 0.6, 0.4),
            );

            // Vs AI button
            spawn_menu_button(
                parent,
                "Vs AI",
                "Play against the computer",
                VsAIButton,
                Color::srgb(0.6, 0.3, 0.5),
            );

            // Watch AI button (AI vs AI spectator mode)
            spawn_menu_button(
                parent,
                "Watch AI",
                "Watch two AIs play each other",
                WatchAIButton,
                Color::srgb(0.4, 0.5, 0.6),
            );

            // Online Multiplayer button
            spawn_menu_button(
                parent,
                "Online Multiplayer",
                "Play with a friend online",
                OnlineMultiplayerButton,
                Color::srgb(0.3, 0.4, 0.7),
            );

            // Settings button
            spawn_menu_button(
                parent,
                "Settings",
                "Adjust game options",
                SettingsButton,
                Color::srgb(0.5, 0.5, 0.5),
            );

            // Quit button
            #[cfg(not(target_arch = "wasm32"))]
            spawn_menu_button(
                parent,
                "Quit",
                "Exit the game",
                QuitButton,
                Color::srgb(0.7, 0.2, 0.2),
            );
        });
}

/// Helper to spawn a styled menu button.
fn spawn_menu_button<T: Component>(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    subtitle: &str,
    marker: T,
    base_color: Color,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(350.0),
                height: Val::Px(90.0),
                margin: UiRect::all(Val::Px(12.0)),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            BorderRadius::all(Val::Px(12.0)),
            BackgroundColor(base_color),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(title),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            btn.spawn((
                Text::new(subtitle),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));
        });
}

/// Handles button interactions on the main menu.
pub fn handle_menu_buttons(
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit_events: MessageWriter<AppExit>,
    pass_play_query: Query<&Interaction, (Changed<Interaction>, With<PassAndPlayButton>)>,
    vs_ai_query: Query<&Interaction, (Changed<Interaction>, With<VsAIButton>)>,
    watch_ai_query: Query<&Interaction, (Changed<Interaction>, With<WatchAIButton>)>,
    online_query: Query<&Interaction, (Changed<Interaction>, With<OnlineMultiplayerButton>)>,
    settings_query: Query<&Interaction, (Changed<Interaction>, With<SettingsButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    // Handle Pass and Play button
    for interaction in pass_play_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Pass and Play selected");
            next_state.set(AppState::PassAndPlay);
        }
    }

    // Handle Vs AI button
    for interaction in vs_ai_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Vs AI selected");
            next_state.set(AppState::VsAI);
        }
    }

    // Handle Watch AI button (AI vs AI spectator mode)
    for interaction in watch_ai_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Watch AI selected");
            next_state.set(AppState::AiVsAi);
        }
    }

    // Handle Online Multiplayer button
    for interaction in online_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Online Multiplayer selected");
            next_state.set(AppState::OnlineMenu);
        }
    }

    // Handle Settings button
    for interaction in settings_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Settings selected");
            next_state.set(AppState::Settings);
        }
    }

    // Handle Quit button
    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Quit selected");
            app_exit_events.write(AppExit::Success);
        }
    }

    // Visual feedback for all buttons
    for (interaction, mut bg_color) in button_colors.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                bg_color.0 = bg_color.0.lighter(0.1);
            }
            Interaction::None => {
                bg_color.0 = bg_color.0.darker(0.05);
            }
            _ => {}
        }
    }
}

/// Cleans up the main menu when exiting the state.
pub fn cleanup_main_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MainMenuRoot>>,
    camera_query: Query<Entity, With<MenuCamera>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// Regression test: Main menu must spawn a camera for UI to render.
    /// Without a camera, Bevy UI will not display anything (gray screen).
    #[test]
    fn setup_main_menu_spawns_camera() {
        let mut app = App::new();
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::default());

        // Run the setup system
        app.world_mut().run_system_once(setup_main_menu).unwrap();

        // Verify a camera with MenuCamera marker exists
        let mut query = app.world_mut().query_filtered::<Entity, With<MenuCamera>>();
        let camera_count = query.iter(app.world()).count();

        assert_eq!(
            camera_count, 1,
            "Main menu must spawn exactly one MenuCamera for UI rendering"
        );
    }

    /// Verify main menu spawns the root UI node.
    #[test]
    fn setup_main_menu_spawns_root_node() {
        let mut app = App::new();
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::default());

        app.world_mut().run_system_once(setup_main_menu).unwrap();

        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<MainMenuRoot>>();
        let root_count = query.iter(app.world()).count();

        assert_eq!(root_count, 1, "Main menu must spawn MainMenuRoot");
    }

    /// Verify main menu spawns both game mode buttons.
    #[test]
    fn setup_main_menu_spawns_buttons() {
        let mut app = App::new();
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::default());

        app.world_mut().run_system_once(setup_main_menu).unwrap();

        let mut pass_query = app
            .world_mut()
            .query_filtered::<Entity, With<PassAndPlayButton>>();
        let pass_play_count = pass_query.iter(app.world()).count();

        let mut online_query = app
            .world_mut()
            .query_filtered::<Entity, With<OnlineMultiplayerButton>>();
        let online_count = online_query.iter(app.world()).count();

        assert_eq!(pass_play_count, 1, "Must spawn PassAndPlayButton");
        assert_eq!(online_count, 1, "Must spawn OnlineMultiplayerButton");
    }
}
