//! # Online Game Systems
//!
//! Systems for synchronizing game state during online multiplayer games.
//!
//! ## Architecture
//! - Host = Team 1, Guest = Team 2
//! - Active player simulates physics, sends final positions
//! - Watching player receives messages and updates state

use bevy::prelude::*;
use bevy_matchbox::prelude::*;

use crate::app_state::{AppState, NetworkRole};
use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::network::{
    GameMessage, NetworkSocket, PeerEvent, StoneState, poll_peer_events, receive_messages,
    send_message,
};
use crate::resources::*;

// ============================================================================
// SETUP SYSTEMS
// ============================================================================

/// Initializes the online game state when entering OnlineGame.
///
/// Sets up teams: Host = Team 1, Guest = Team 2.
pub fn setup_online_game(
    mut online_state: ResMut<OnlineState>,
    network_role: Res<State<NetworkRole>>,
) {
    // Assign teams based on network role
    let local_team = match network_role.get() {
        NetworkRole::Host => Team::One,
        NetworkRole::Guest => Team::Two,
        NetworkRole::None => {
            tracing::warn!("Entered OnlineGame with NetworkRole::None");
            Team::One // Fallback
        }
    };

    online_state.local_team = Some(local_team);
    online_state.pending_shot = None;
    online_state.pending_positions = None;

    tracing::info!(
        local_team = local_team.name(),
        is_host = online_state.is_host,
        "Online game initialized"
    );
}

// ============================================================================
// SEND SYSTEMS
// ============================================================================

/// Sends the shot parameters to the opponent when the local player throws.
///
/// Triggered when transitioning from Aiming to StoneMoving phase.
pub fn send_shot_on_throw(
    state: Res<GameState>,
    online_state: Res<OnlineState>,
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
) {
    // Only send when we just started moving (still_time == 0 means just thrown)
    if state.phase != Phase::StoneMoving || state.still_time > 0.01 {
        return;
    }

    // Check if it's our turn
    let Some(local_team) = online_state.local_team else {
        return;
    };
    if state.current_team() != local_team {
        return; // Not our turn, don't send
    }

    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    let message = GameMessage::ShotThrown {
        angle: state.aim_angle_deg,
        weight: state.aim_weight,
        curl: state.curl_direction,
    };

    send_message(&mut socket, &message);
    tracing::info!(
        angle = state.aim_angle_deg,
        weight = state.aim_weight,
        curl = ?state.curl_direction,
        "Sent shot to opponent"
    );
}

/// Sends the final stone positions after a shot resolves.
///
/// Triggered when transitioning from StoneMoving to CallingShot or ShowingScore.
pub fn send_positions_on_resolve(
    state: Res<GameState>,
    online_state: Res<OnlineState>,
    stones: Query<(&Transform, &Stone)>,
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
    mut sent_for_shot: Local<Option<u8>>,
) {
    // Only send when we just finished resolving (phase changed away from Resolve)
    if state.phase == Phase::Resolve || state.phase == Phase::StoneMoving {
        return;
    }

    // Check if we already sent for this shot
    if *sent_for_shot == Some(state.shot_index) {
        return;
    }

    // Only the player who threw sends the resolution
    let Some(local_team) = online_state.local_team else {
        return;
    };

    // Determine who threw the previous shot
    // shot_index is now at the NEXT shot, so the thrower was for shot_index - 1
    let prev_shot = state.shot_index.saturating_sub(1);
    let throwing_team = if prev_shot % 2 == 0 {
        state.first_throw_team
    } else {
        state.first_throw_team.opponent()
    };

    if throwing_team != local_team {
        return; // Opponent threw, they send the resolution
    }

    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    // Collect stone positions
    let stone_states: Vec<StoneState> = stones
        .iter()
        .map(|(transform, stone)| StoneState {
            team: stone.team,
            x: transform.translation.x,
            y: transform.translation.y,
        })
        .collect();

    let message = GameMessage::ShotResolved {
        stones: stone_states.clone(),
    };

    send_message(&mut socket, &message);
    *sent_for_shot = Some(state.shot_index);
    tracing::info!(
        stone_count = stone_states.len(),
        "Sent stone positions to opponent"
    );
}

// ============================================================================
// RECEIVE SYSTEMS
// ============================================================================

/// Receives and processes messages from the opponent.
pub fn receive_network_messages(
    mut online_state: ResMut<OnlineState>,
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
) {
    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    let messages = receive_messages(&mut socket);

    for (_peer_id, message) in messages {
        match message {
            GameMessage::ShotThrown {
                angle,
                weight,
                curl,
            } => {
                tracing::info!(
                    angle = angle,
                    weight = weight,
                    curl = ?curl,
                    "Received opponent's shot"
                );
                online_state.pending_shot = Some(PendingShot {
                    angle,
                    weight,
                    curl,
                });
            }
            GameMessage::BroomUpdate { x, y } => {
                tracing::trace!(x = x, y = y, "Received broom position update");
                online_state.pending_broom_position = Some((x, y));
            }
            GameMessage::ShotResolved { stones } => {
                tracing::info!(stone_count = stones.len(), "Received stone positions");
                online_state.pending_positions =
                    Some(stones.iter().map(|s| (s.team, s.x, s.y)).collect());
            }
            GameMessage::PositionSync { stones } => {
                tracing::debug!(stone_count = stones.len(), "Received periodic sync");
                online_state.pending_periodic_sync =
                    Some(stones.iter().map(|s| (s.team, s.x, s.y)).collect());
            }
            GameMessage::ShotCalled { .. } => {
                // Optional: could display opponent's called shot
            }
            GameMessage::EndScored { team1, team2 } => {
                tracing::info!(team1 = team1, team2 = team2, "Received end score");
            }
            GameMessage::GameReady => {
                tracing::info!("Opponent is ready");
            }
        }
    }
}

// ============================================================================
// PERIODIC SYNC SYSTEMS
// ============================================================================

/// Sends periodic stone position syncs during stone movement.
///
/// The active player (who threw) sends position updates every ~1 second
/// to help the watching player stay synchronized during physics simulation.
pub fn send_periodic_sync(
    time: Res<Time>,
    state: Res<GameState>,
    mut online_state: ResMut<OnlineState>,
    stones: Query<(&Transform, &Stone)>,
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
) {
    // Only send during stone movement
    if state.phase != Phase::StoneMoving {
        // Reset timer when not in StoneMoving
        online_state.sync_timer.reset();
        return;
    }

    // Check if it's our turn (we're simulating physics)
    let Some(local_team) = online_state.local_team else {
        return;
    };
    if state.current_team() != local_team {
        return; // Opponent threw, they send syncs
    }

    // Tick the timer
    online_state.sync_timer.tick(time.delta());

    // Send on timer fire
    if online_state.sync_timer.just_finished() {
        let Ok(mut socket) = socket_query.single_mut() else {
            return;
        };

        let stone_states: Vec<StoneState> = stones
            .iter()
            .map(|(transform, stone)| StoneState {
                team: stone.team,
                x: transform.translation.x,
                y: transform.translation.y,
            })
            .collect();

        let message = GameMessage::PositionSync {
            stones: stone_states.clone(),
        };

        send_message(&mut socket, &message);
        tracing::debug!(
            stone_count = stone_states.len(),
            "Sent periodic position sync"
        );
    }
}

/// Applies periodic position syncs received from the opponent.
///
/// During the opponent's shot, this corrects our local stone positions
/// to match what they're seeing, preventing drift due to FPS differences.
pub fn apply_periodic_sync(
    mut online_state: ResMut<OnlineState>,
    state: Res<GameState>,
    mut stones: Query<(&mut Transform, &Stone)>,
) {
    // Only apply during stone movement
    if state.phase != Phase::StoneMoving {
        return;
    }

    // Check if it's opponent's turn (we're watching)
    let Some(local_team) = online_state.local_team else {
        return;
    };
    if state.current_team() == local_team {
        return; // Our turn, we're authoritative
    }

    // Get pending sync
    let Some(positions) = online_state.pending_periodic_sync.take() else {
        return;
    };

    // Apply positions to existing stones
    // Note: We can't add/remove stones here - that happens on shot resolution
    // We just update positions of stones that exist on both sides
    for (mut transform, stone) in stones.iter_mut() {
        // Find matching stone in sync data (by team, closest position)
        // This is imperfect but handles most cases
        if let Some((_, x, y)) = positions
            .iter()
            .filter(|(team, _, _)| *team == stone.team)
            .min_by(|(_, ax, ay), (_, bx, by)| {
                let dist_a =
                    (transform.translation.x - ax).powi(2) + (transform.translation.y - ay).powi(2);
                let dist_b =
                    (transform.translation.x - bx).powi(2) + (transform.translation.y - by).powi(2);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
        {
            // Blend toward synced position (smooth interpolation)
            let blend = 0.5; // 50% toward synced position
            transform.translation.x = transform.translation.x * (1.0 - blend) + x * blend;
            transform.translation.y = transform.translation.y * (1.0 - blend) + y * blend;
        }
    }

    tracing::debug!("Applied periodic position sync");
}

// ============================================================================
// BROOM SYNC SYSTEMS
// ============================================================================

/// Sends broom position updates to the opponent during CallingShot/Aiming.
///
/// The active player sends position updates when their broom moves significantly.
pub fn send_broom_updates(
    state: Res<GameState>,
    online_state: Res<OnlineState>,
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
    mut last_broom_pos: Local<Vec2>,
) {
    // Only send during calling/aiming when it's our turn
    if state.phase != Phase::CallingShot && state.phase != Phase::Aiming {
        return;
    }

    let Some(local_team) = online_state.local_team else {
        return;
    };
    if state.current_team() != local_team {
        return; // Not our turn
    }

    // Only send if position changed significantly (0.05 units)
    if state.broom_position.distance(*last_broom_pos) < 0.05 {
        return;
    }

    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    let message = GameMessage::BroomUpdate {
        x: state.broom_position.x,
        y: state.broom_position.y,
    };
    send_message(&mut socket, &message);
    *last_broom_pos = state.broom_position;
}

/// Applies broom position updates received from the opponent.
///
/// Updates the broom position on the passive player's screen so they can
/// see where the active player is aiming.
pub fn apply_broom_updates(mut state: ResMut<GameState>, mut online_state: ResMut<OnlineState>) {
    // Only apply if it's opponent's turn
    let Some(local_team) = online_state.local_team else {
        return;
    };
    if state.current_team() == local_team {
        return; // Our turn, we control the broom
    }

    if let Some((x, y)) = online_state.pending_broom_position.take() {
        state.broom_position = Vec2::new(x, y);
    }
}

// ============================================================================
// APPLY SYSTEMS
// ============================================================================

/// Applies a pending shot from the opponent.
///
/// When it's the opponent's turn and they've sent a shot, spawn their stone.
pub fn apply_pending_shot(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    mut online_state: ResMut<OnlineState>,
    assets: Res<StoneAssets>,
    stones: Query<(Entity, &Transform, &Stone)>,
) {
    // Only apply during CallingShot or Aiming when it's opponent's turn
    if state.phase != Phase::CallingShot && state.phase != Phase::Aiming {
        return;
    }

    let Some(local_team) = online_state.local_team else {
        return;
    };

    // Check if it's opponent's turn
    if state.current_team() == local_team {
        return; // Our turn, don't apply opponent shots
    }

    // Get pending shot
    let Some(pending) = online_state.pending_shot.take() else {
        return;
    };

    // Create snapshot for FGZ checking
    let snapshot = snapshot_stones(&stones, state.shot_index);
    state.snapshot = Some(snapshot);

    // Calculate spawn parameters
    let team = state.current_team();
    let weight_normalized = (pending.weight - 1.0) / 9.0;
    let speed = WEIGHT_MIN_SPEED + weight_normalized * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
    let angle_rad = pending.angle.to_radians();
    let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
    let start = Vec2::new(0.0, DELIVERY_START_Y);

    // Spawn the opponent's stone
    let stone_entity = spawn_stone(
        &mut commands,
        &assets,
        team,
        start,
        direction * speed,
        true,
        pending.curl,
    );

    state.thrown_stone = Some(stone_entity);
    state.still_time = 0.0;
    state.aim_angle_deg = pending.angle;
    state.aim_weight = pending.weight;
    state.curl_direction = pending.curl;
    state.phase = Phase::StoneMoving;

    tracing::info!(
        team = team.name(),
        angle = pending.angle,
        weight = pending.weight,
        "Applied opponent's shot"
    );
}

/// Syncs stone positions after receiving ShotResolved from opponent.
///
/// This ensures both players have identical stone positions after a shot.
pub fn sync_stone_positions(
    mut commands: Commands,
    mut online_state: ResMut<OnlineState>,
    state: Res<GameState>,
    stones: Query<(Entity, &Stone)>,
    assets: Res<StoneAssets>,
) {
    // Only sync when we're waiting for opponent's resolution
    if state.phase != Phase::CallingShot && state.phase != Phase::ShowingScore {
        return;
    }

    let Some(local_team) = online_state.local_team else {
        return;
    };

    // Only apply if it was the opponent's turn (we're receiving their resolution)
    let last_throwing_team = if state.shot_index == 0 {
        state.first_throw_team.opponent()
    } else {
        let prev_index = state.shot_index.saturating_sub(1);
        if prev_index % 2 == 0 {
            state.first_throw_team
        } else {
            state.first_throw_team.opponent()
        }
    };

    if last_throwing_team == local_team {
        return; // We threw, we already have the correct positions
    }

    let Some(positions) = online_state.pending_positions.take() else {
        return;
    };

    // Despawn all current stones
    for (entity, _) in stones.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn stones at received positions
    for (team, x, y) in positions {
        let pos = Vec2::new(x, y);
        spawn_stone(
            &mut commands,
            &assets,
            team,
            pos,
            Vec2::ZERO, // Stationary
            false,      // Not throwing
            CurlDirection::default(),
        );
    }

    tracing::info!("Synced stone positions from opponent");
}

// ============================================================================
// INPUT CONTROL
// ============================================================================

/// Run condition: returns true only if it's the local player's turn.
///
/// Use this to gate input systems in OnlineGame state.
pub fn run_if_local_turn(state: Res<GameState>, online_state: Res<OnlineState>) -> bool {
    if let Some(local_team) = online_state.local_team {
        state.current_team() == local_team
    } else {
        true // Fallback to allowing input
    }
}

/// Checks if it's the local player's turn.
pub fn is_local_turn(state: &GameState, online_state: &OnlineState) -> bool {
    if let Some(local_team) = online_state.local_team {
        state.current_team() == local_team
    } else {
        true // Fallback to allowing input
    }
}

// ============================================================================
// CAMERA CONTROL (ONLINE)
// ============================================================================

/// Controls camera behavior for online game.
///
/// - When waiting (opponent's turn during CallingShot/Aiming): show skip view to see broom
/// - When stone is moving: explicitly set FollowStone for both players
pub fn online_camera_control(
    _time: Res<Time>,
    state: Res<GameState>,
    online_state: Res<OnlineState>,
    mut camera_state: ResMut<CameraState>,
) {
    let Some(local_team) = online_state.local_team else {
        return;
    };

    let is_our_turn = state.current_team() == local_team;

    match state.phase {
        Phase::CallingShot | Phase::Aiming => {
            if !is_our_turn {
                // Opponent is aiming - use skip view to see their broom position
                // (broom position synced via BroomUpdate messages)
                camera_state.mode = CameraMode::SkipView;
            }
            // If it's our turn, let the normal camera system handle it
        }
        Phase::StoneMoving => {
            // Both players watch the shot - explicitly set FollowStone
            // Reset camera state if transitioning into StoneMoving
            if camera_state.mode != CameraMode::FollowStone
                && camera_state.mode != CameraMode::HouseOverhead
            {
                camera_state.mode = CameraMode::FollowStone;
                camera_state.stone_crossed_hog = false;
                camera_state.follow_camera_height = FOLLOW_START_HEIGHT;
            }
        }
        Phase::ShowingScore => {
            // Both see the house overhead
            camera_state.mode = CameraMode::HouseOverhead;
        }
        _ => {}
    }
}

// ============================================================================
// UI COMPONENTS
// ============================================================================

/// Marker component for the "You are Team X" indicator.
#[derive(Component)]
pub struct YourTeamIndicator;

/// Spawns the "You are Team X" indicator during online game setup.
pub fn spawn_your_team_indicator(mut commands: Commands, online_state: Res<OnlineState>) {
    let Some(local_team) = online_state.local_team else {
        return;
    };

    // Spawn in top-left corner
    commands.spawn((
        Text::new(format!("You are {}", local_team.name())),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(local_team.color()),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        YourTeamIndicator,
    ));
}

/// Cleans up the team indicator when leaving online game.
pub fn cleanup_your_team_indicator(
    mut commands: Commands,
    query: Query<Entity, With<YourTeamIndicator>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ============================================================================
// DISCONNECTION DETECTION
// ============================================================================

/// Detects when the opponent disconnects during a game.
///
/// Sets online_state.opponent_disconnected = true when this occurs.
pub fn detect_disconnection(
    mut socket_query: Query<&mut MatchboxSocket, With<NetworkSocket>>,
    mut online_state: ResMut<OnlineState>,
) {
    let Ok(mut socket) = socket_query.single_mut() else {
        return;
    };

    for event in poll_peer_events(&mut socket) {
        match event {
            PeerEvent::Connected(_peer_id) => {
                // Peer reconnected (unusual during game, but handle it)
                online_state.opponent_disconnected = false;
                tracing::info!("Peer reconnected during game");
            }
            PeerEvent::Disconnected(_peer_id) => {
                online_state.opponent_disconnected = true;
                tracing::warn!("Opponent disconnected during game!");
            }
        }
    }
}

// ============================================================================
// CONNECTION STATUS UI
// ============================================================================

const CONNECTION_INDICATOR_SIZE: f32 = 12.0;
const CONNECTED_COLOR: Color = Color::srgb(0.2, 0.8, 0.2); // Green
const DISCONNECTED_COLOR: Color = Color::srgb(0.9, 0.2, 0.2); // Red

/// Spawns the connection status indicator in the top-right corner.
pub fn spawn_connection_status_indicator(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            width: Val::Px(CONNECTION_INDICATOR_SIZE),
            height: Val::Px(CONNECTION_INDICATOR_SIZE),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderRadius::all(Val::Px(CONNECTION_INDICATOR_SIZE / 2.0)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        BackgroundColor(CONNECTED_COLOR),
        ConnectionStatusIndicator,
    ));
}

/// Updates the connection status indicator color based on connection state.
pub fn update_connection_status_ui(
    online_state: Res<OnlineState>,
    mut indicator_query: Query<&mut BackgroundColor, With<ConnectionStatusIndicator>>,
) {
    for mut bg in indicator_query.iter_mut() {
        bg.0 = if online_state.opponent_disconnected {
            DISCONNECTED_COLOR
        } else {
            CONNECTED_COLOR
        };
    }
}

/// Shows the disconnection overlay when opponent disconnects.
pub fn show_disconnection_overlay(
    mut commands: Commands,
    online_state: Res<OnlineState>,
    existing_overlay: Query<Entity, With<DisconnectionOverlay>>,
) {
    // Only show if disconnected and overlay doesn't already exist
    if !online_state.opponent_disconnected || !existing_overlay.is_empty() {
        return;
    }

    // Spawn fullscreen overlay
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            DisconnectionOverlay,
            // Ensure it's on top of other UI
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Warning icon
            parent.spawn((
                Text::new("⚠"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Title
            parent.spawn((
                Text::new("Connection Lost"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(15.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("Your opponent has disconnected"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Return to menu button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                    BorderRadius::all(Val::Px(10.0)),
                    BackgroundColor(Color::srgb(0.2, 0.4, 0.6)),
                    DisconnectionReturnButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Return to Main Menu"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });

    tracing::info!("Showed disconnection overlay");
}

/// Handles the return button on the disconnection overlay.
pub fn handle_disconnection_return_button(
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_network_role: ResMut<NextState<NetworkRole>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<DisconnectionReturnButton>)>,
    socket_query: Query<Entity, With<NetworkSocket>>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<DisconnectionReturnButton>),
    >,
) {
    // Handle click
    for interaction in button_query.iter() {
        if *interaction == Interaction::Pressed {
            tracing::info!("Returning to main menu after disconnection");

            // Close the socket
            for entity in socket_query.iter() {
                commands.entity(entity).despawn();
            }

            // Return to main menu
            next_network_role.set(NetworkRole::None);
            next_app_state.set(AppState::MainMenu);
        }
    }

    // Visual feedback
    for (interaction, mut bg) in button_colors.iter_mut() {
        match *interaction {
            Interaction::Hovered => bg.0 = Color::srgb(0.3, 0.5, 0.7),
            Interaction::Pressed => bg.0 = Color::srgb(0.15, 0.3, 0.45),
            Interaction::None => bg.0 = Color::srgb(0.2, 0.4, 0.6),
        }
    }
}

// ============================================================================
// CLEANUP
// ============================================================================

/// Cleans up online game state when exiting.
pub fn cleanup_online_game(
    mut commands: Commands,
    socket_query: Query<Entity, With<NetworkSocket>>,
    indicator_query: Query<Entity, With<ConnectionStatusIndicator>>,
    overlay_query: Query<Entity, With<DisconnectionOverlay>>,
) {
    // Despawn the socket
    for entity in socket_query.iter() {
        commands.entity(entity).despawn();
    }
    // Despawn connection indicator
    for entity in indicator_query.iter() {
        commands.entity(entity).despawn();
    }
    // Despawn disconnection overlay if present
    for entity in overlay_query.iter() {
        commands.entity(entity).despawn();
    }
    tracing::info!("Online game cleaned up");
}
