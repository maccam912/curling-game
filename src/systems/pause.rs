use crate::app_state::AppState;
use bevy::prelude::*;
use bevy_rapier2d::plugin::RapierConfiguration;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Marker for the pause menu overlay root node.
#[derive(Component)]
pub struct PauseMenuRoot;

/// Marker for the "Resume" button.
#[derive(Component)]
pub struct ResumeButton;

/// Marker for the "Quit to Main Menu" button.
#[derive(Component)]
pub struct QuitToMenuButton;

// ============================================================================
// SYSTEMS
// ============================================================================

/// Toggles pause state when Escape is pressed.
pub fn toggle_pause(
    mut next_state: ResMut<NextState<AppState>>,
    current_state: Res<State<AppState>>,
    mut previous_state: ResMut<PreviousGameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            AppState::PassAndPlay | AppState::VsAI | AppState::OnlineGame => {
                // Save current state so we can resume to it
                previous_state.0 = Some(current_state.get().clone());
                next_state.set(AppState::Paused);
            }
            AppState::Paused => {
                // If paused, resume to previous state
                if let Some(state) = previous_state.0.clone() {
                    next_state.set(state);
                } else {
                    next_state.set(AppState::PassAndPlay);
                }
            }
            _ => {}
        }
    }
}

/// Resource to store the state before pausing.
#[derive(Resource, Default)]
pub struct PreviousGameState(pub Option<AppState>);

/// Sets up the pause menu UI.
pub fn setup_pause_menu(mut commands: Commands) {
    // We don't need a camera because the game camera should still be there (rendering game background).
    // Overlay UI.

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)), // Semi-transparent black
            PauseMenuRoot,
            // Ensure it's on top? Z-index is determined by tree order.
            ZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Resume button
            spawn_pause_button(parent, "Resume", ResumeButton, Color::srgb(0.2, 0.6, 0.2));

            // Quit to Menu button
            spawn_pause_button(
                parent,
                "Quit to Menu",
                QuitToMenuButton,
                Color::srgb(0.7, 0.2, 0.2),
            );
        });
}

fn spawn_pause_button<T: Component>(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    marker: T,
    color: Color,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(250.0),
                height: Val::Px(60.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
            BorderRadius::all(Val::Px(8.0)),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Handles interactions on the pause menu.
pub fn handle_pause_buttons(
    mut next_state: ResMut<NextState<AppState>>,
    resume_query: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitToMenuButton>)>,
    previous_state: Res<PreviousGameState>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    // Handle Resume
    for interaction in resume_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(state) = previous_state.0.clone() {
                next_state.set(state);
            } else {
                // Fallback if no previous state (shouldn't happen if logic is correct)
                next_state.set(AppState::PassAndPlay);
            }
        }
    }

    // Handle Quit
    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
        }
    }

    // Visual feedback
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

/// Cleans up the pause menu.
pub fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Pauses the Rapier physics engine.
pub fn pause_physics(mut rapier_config: Query<&mut RapierConfiguration>) {
    for mut config in rapier_config.iter_mut() {
        config.physics_pipeline_active = false;
    }
}

/// Resumes the Rapier physics engine.
pub fn resume_physics(mut rapier_config: Query<&mut RapierConfiguration>) {
    for mut config in rapier_config.iter_mut() {
        config.physics_pipeline_active = true;
    }
}
