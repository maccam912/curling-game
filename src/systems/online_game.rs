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

use crate::app_state::NetworkRole;
use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::network::{GameMessage, NetworkSocket, StoneState, receive_messages, send_message};
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
            GameMessage::ShotResolved { stones } => {
                tracing::info!(stone_count = stones.len(), "Received stone positions");
                online_state.pending_positions =
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
/// - When waiting (opponent's turn during CallingShot/Aiming): orbit camera around house
/// - When stone is moving: follow shot as usual
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
                // Opponent is aiming - show orbit view of house
                camera_state.mode = CameraMode::GameOverOrbit;
            }
            // If it's our turn, let the normal camera system handle it
        }
        Phase::StoneMoving => {
            // Both players watch the shot - follow mode
            // This is handled by the normal camera system via FollowStone mode
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
// CLEANUP
// ============================================================================

/// Cleans up online game state when exiting.
pub fn cleanup_online_game(
    mut commands: Commands,
    socket_query: Query<Entity, With<NetworkSocket>>,
) {
    // Despawn the socket
    for entity in socket_query.iter() {
        commands.entity(entity).despawn();
    }
    tracing::info!("Online game cleaned up");
}
