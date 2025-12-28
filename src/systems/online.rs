//! # Online Multiplayer Systems
//!
//! UI and logic for online multiplayer menu, lobby, and game flow.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy_matchbox::prelude::*;
use rand::Rng;

use crate::app_state::{AppState, NetworkRole};
use crate::network::{NetworkSocket, create_socket, poll_peer_connected};
use crate::resources::OnlineState;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Marker for the online menu root node.
#[derive(Component)]
pub struct OnlineMenuRoot;

/// Marker for the online menu camera.
#[derive(Component)]
pub struct OnlineMenuCamera;

/// Marker for the "Create Game" button.
#[derive(Component)]
pub struct CreateGameButton;

/// Marker for the "Join Game" button.
#[derive(Component)]
pub struct JoinGameButton;

/// Marker for the "Back" button.
#[derive(Component)]
pub struct BackToMainMenuButton;

/// Marker for the room code input field.
#[derive(Component)]
pub struct RoomCodeInput;

/// Marker for the room code display text.
#[derive(Component)]
pub struct RoomCodeDisplay;

/// Marker for the online lobby root node.
#[derive(Component)]
pub struct OnlineLobbyRoot;

/// Marker for the lobby status text.
#[derive(Component)]
pub struct LobbyStatusText;

/// Marker for the "Cancel" button in lobby.
#[derive(Component)]
pub struct CancelLobbyButton;

// ============================================================================
// CONSTANTS
// ============================================================================

const BUTTON_COLOR: Color = Color::srgb(0.2, 0.4, 0.6);
const BUTTON_HOVER: Color = Color::srgb(0.3, 0.5, 0.7);
const BUTTON_PRESSED: Color = Color::srgb(0.15, 0.3, 0.45);
const BACKGROUND_COLOR: Color = Color::srgb(0.08, 0.12, 0.18);

// ============================================================================
// ONLINE MENU SYSTEMS
// ============================================================================

/// Generates a random 4-character room code (uppercase letters + digits).
fn generate_room_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Avoid confusing chars
    let mut rng = rand::rng();
    (0..4)
        .map(|_| {
            let idx = rng.random_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

/// Sets up the online menu UI.
pub fn setup_online_menu(mut commands: Commands, mut online_state: ResMut<OnlineState>) {
    // Reset online state
    *online_state = OnlineState::default();

    // Spawn menu camera
    commands.spawn((Camera2d::default(), OnlineMenuCamera));

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
            BackgroundColor(BACKGROUND_COLOR),
            OnlineMenuRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Online Multiplayer"),
                TextFont {
                    font_size: 56.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("Play with a friend over the internet"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                Node {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                },
            ));

            // Create Game button
            spawn_online_button(parent, "Create Game", "Host a new game", CreateGameButton);

            // Join Game button
            spawn_online_button(parent, "Join Game", "Enter a room code", JoinGameButton);

            // Room code input section (initially hidden, shown after Join Game clicked)
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(20.0)),
                        display: Display::None, // Hidden by default
                        ..default()
                    },
                    RoomCodeInput,
                ))
                .with_children(|input_section| {
                    input_section.spawn((
                        Text::new("Enter Room Code:"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                        Node {
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                    ));

                    // Room code display/input
                    input_section.spawn((
                        Text::new("____"),
                        TextFont {
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.4, 0.8, 1.0)),
                        RoomCodeDisplay,
                    ));

                    input_section.spawn((
                        Text::new("Type 4 characters, then press Enter"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                });

            // Spacer
            parent.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });

            // Back button
            spawn_back_button(parent, "Back to Main Menu", BackToMainMenuButton);
        });
}

/// Helper to spawn a styled online menu button.
fn spawn_online_button<T: Component>(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    subtitle: &str,
    marker: T,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(320.0),
                height: Val::Px(80.0),
                margin: UiRect::all(Val::Px(10.0)),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
            BorderRadius::all(Val::Px(10.0)),
            BackgroundColor(BUTTON_COLOR),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(title),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            btn.spawn((
                Text::new(subtitle),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));
        });
}

/// Helper to spawn a smaller back button.
fn spawn_back_button<T: Component>(parent: &mut ChildSpawnerCommands, label: &str, marker: T) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(45.0),
                margin: UiRect::top(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
            BorderRadius::all(Val::Px(8.0)),
            BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.5)),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));
        });
}

/// Handles button interactions and keyboard input on the online menu.
pub fn handle_online_menu_buttons(
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_network_role: ResMut<NextState<NetworkRole>>,
    mut online_state: ResMut<OnlineState>,
    create_query: Query<&Interaction, (Changed<Interaction>, With<CreateGameButton>)>,
    join_query: Query<&Interaction, (Changed<Interaction>, With<JoinGameButton>)>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<BackToMainMenuButton>)>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut input_section: Query<&mut Node, With<RoomCodeInput>>,
    mut room_code_text: Query<&mut Text, With<RoomCodeDisplay>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut char_events: MessageReader<KeyboardInput>,
) {
    // Handle Create Game button
    for interaction in create_query.iter() {
        if *interaction == Interaction::Pressed {
            let code = generate_room_code();
            tracing::info!(room_code = %code, "Creating game");
            online_state.room_code = code;
            online_state.is_host = true;
            next_network_role.set(NetworkRole::Host);
            next_app_state.set(AppState::OnlineLobby);
        }
    }

    // Handle Join Game button - show input section
    for interaction in join_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Join Game clicked - showing room code input");
            for mut node in input_section.iter_mut() {
                node.display = Display::Flex;
            }
            online_state.input_room_code.clear();
        }
    }

    // Handle Back button
    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Returning to main menu");
            next_app_state.set(AppState::MainMenu);
        }
    }

    // Handle keyboard input for room code
    for event in char_events.read() {
        if event.state.is_pressed() {
            if let Some(ref key_code) = Some(event.key_code) {
                let c = match key_code {
                    KeyCode::KeyA => Some('A'),
                    KeyCode::KeyB => Some('B'),
                    KeyCode::KeyC => Some('C'),
                    KeyCode::KeyD => Some('D'),
                    KeyCode::KeyE => Some('E'),
                    KeyCode::KeyF => Some('F'),
                    KeyCode::KeyG => Some('G'),
                    KeyCode::KeyH => Some('H'),
                    KeyCode::KeyJ => Some('J'),
                    KeyCode::KeyK => Some('K'),
                    KeyCode::KeyL => Some('L'),
                    KeyCode::KeyM => Some('M'),
                    KeyCode::KeyN => Some('N'),
                    KeyCode::KeyP => Some('P'),
                    KeyCode::KeyQ => Some('Q'),
                    KeyCode::KeyR => Some('R'),
                    KeyCode::KeyS => Some('S'),
                    KeyCode::KeyT => Some('T'),
                    KeyCode::KeyU => Some('U'),
                    KeyCode::KeyV => Some('V'),
                    KeyCode::KeyW => Some('W'),
                    KeyCode::KeyX => Some('X'),
                    KeyCode::KeyY => Some('Y'),
                    KeyCode::KeyZ => Some('Z'),
                    KeyCode::Digit0 | KeyCode::Numpad0 => Some('0'),
                    KeyCode::Digit1 | KeyCode::Numpad1 => Some('1'),
                    KeyCode::Digit2 | KeyCode::Numpad2 => Some('2'),
                    KeyCode::Digit3 | KeyCode::Numpad3 => Some('3'),
                    KeyCode::Digit4 | KeyCode::Numpad4 => Some('4'),
                    KeyCode::Digit5 | KeyCode::Numpad5 => Some('5'),
                    KeyCode::Digit6 | KeyCode::Numpad6 => Some('6'),
                    KeyCode::Digit7 | KeyCode::Numpad7 => Some('7'),
                    KeyCode::Digit8 | KeyCode::Numpad8 => Some('8'),
                    KeyCode::Digit9 | KeyCode::Numpad9 => Some('9'),
                    KeyCode::Backspace => {
                        online_state.input_room_code.pop();
                        None
                    }
                    _ => None,
                };

                if let Some(ch) = c {
                    if online_state.input_room_code.len() < 4 {
                        online_state.input_room_code.push(ch);
                    }
                }
            }
        }
    }

    // Handle Enter to submit room code
    if keyboard.just_pressed(KeyCode::Enter) && online_state.input_room_code.len() == 4 {
        let code = online_state.input_room_code.clone().to_uppercase();
        tracing::info!(room_code = %code, "Joining game");
        online_state.room_code = code;
        online_state.is_host = false;
        next_network_role.set(NetworkRole::Guest);
        next_app_state.set(AppState::OnlineLobby);
    }

    // Update room code display
    for mut text in room_code_text.iter_mut() {
        let display: String = online_state
            .input_room_code
            .chars()
            .chain(std::iter::repeat('_'))
            .take(4)
            .collect();
        **text = display;
    }

    // Visual feedback for buttons
    for (interaction, mut bg_color) in button_colors.iter_mut() {
        match *interaction {
            Interaction::Hovered => bg_color.0 = BUTTON_HOVER,
            Interaction::Pressed => bg_color.0 = BUTTON_PRESSED,
            Interaction::None => bg_color.0 = BUTTON_COLOR,
        }
    }
}

/// Cleans up the online menu when exiting the state.
pub fn cleanup_online_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<OnlineMenuRoot>>,
    camera_query: Query<Entity, With<OnlineMenuCamera>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ============================================================================
// ONLINE LOBBY SYSTEMS
// ============================================================================

/// Sets up the online lobby UI (waiting for opponent) and creates the matchbox socket.
pub fn setup_online_lobby(mut commands: Commands, online_state: Res<OnlineState>) {
    // Create the matchbox socket for P2P connection
    create_socket(&mut commands, &online_state.room_code);

    // Spawn lobby camera
    commands.spawn((Camera2d::default(), OnlineMenuCamera));

    let status_text = if online_state.is_host {
        "Waiting for opponent to join..."
    } else {
        "Connecting to host..."
    };

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
            BackgroundColor(BACKGROUND_COLOR),
            OnlineLobbyRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new(if online_state.is_host {
                    "Game Created"
                } else {
                    "Joining Game"
                }),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Room code display
            parent.spawn((
                Text::new(format!("Room Code: {}", online_state.room_code)),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Instruction for host
            if online_state.is_host {
                parent.spawn((
                    Text::new("Share this code with your opponent"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                    Node {
                        margin: UiRect::bottom(Val::Px(40.0)),
                        ..default()
                    },
                ));
            }

            // Status text
            parent.spawn((
                Text::new(status_text),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                LobbyStatusText,
                Node {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                },
            ));

            // Cancel button
            spawn_back_button(parent, "Cancel", CancelLobbyButton);
        });
}

/// Polls for peer connection events and transitions to game when connected.
pub fn poll_peer_events(
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
    mut online_state: ResMut<OnlineState>,
    mut status_text: Query<&mut Text, With<LobbyStatusText>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    // Check for peer connection events
    if let Some(_peer_id) = poll_peer_connected(&mut socket) {
        online_state.opponent_connected = true;
        tracing::info!("Opponent connected! Starting game...");

        // Update status text
        for mut text in status_text.iter_mut() {
            **text = "Opponent connected! Starting...".to_string();
        }

        // Transition to the online game
        next_app_state.set(AppState::OnlineGame);
    }
}

/// Handles lobby UI interactions.
pub fn handle_lobby_buttons(
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_network_role: ResMut<NextState<NetworkRole>>,
    cancel_query: Query<&Interaction, (Changed<Interaction>, With<CancelLobbyButton>)>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    socket_query: Query<Entity, With<NetworkSocket>>,
) {
    // Handle Cancel button
    for interaction in cancel_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Cancelling lobby, returning to online menu");

            // Close the socket
            for entity in socket_query.iter() {
                commands.entity(entity).despawn();
            }

            next_network_role.set(NetworkRole::None);
            next_app_state.set(AppState::OnlineMenu);
        }
    }

    // Visual feedback for buttons
    for (interaction, mut bg_color) in button_colors.iter_mut() {
        match *interaction {
            Interaction::Hovered => bg_color.0 = bg_color.0.lighter(0.1),
            Interaction::Pressed => bg_color.0 = bg_color.0.darker(0.1),
            _ => {}
        }
    }
}

/// Cleans up the lobby when exiting the state.
pub fn cleanup_online_lobby(
    mut commands: Commands,
    lobby_query: Query<Entity, With<OnlineLobbyRoot>>,
    camera_query: Query<Entity, With<OnlineMenuCamera>>,
    socket_query: Query<Entity, With<NetworkSocket>>,
) {
    for entity in lobby_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
    // Note: We don't despawn the socket here if transitioning to OnlineGame
    // The socket will be despawned when leaving OnlineGame or cancelling
    // Actually, let's keep the socket - we only despawn on cancel
    let _ = socket_query; // Socket is preserved for the game
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_is_4_chars() {
        let code = generate_room_code();
        assert_eq!(code.len(), 4, "Room code should be 4 characters");
    }

    #[test]
    fn room_code_is_alphanumeric() {
        let code = generate_room_code();
        assert!(
            code.chars().all(|c| c.is_ascii_alphanumeric()),
            "Room code should be alphanumeric"
        );
    }

    #[test]
    fn room_codes_are_unique() {
        let codes: std::collections::HashSet<_> = (0..100).map(|_| generate_room_code()).collect();
        // With 4 chars from 32 options, collision is unlikely in 100 samples
        assert!(codes.len() > 90, "Room codes should be mostly unique");
    }
}
