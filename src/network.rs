//! # Network Module
//!
//! Matchbox WebRTC networking for online multiplayer.
//!
//! This module handles:
//! - Socket creation and connection via signaling server
//! - Peer connection/disconnection events
//! - Game message serialization and transmission

use bevy::prelude::*;
use bevy_matchbox::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::{CurlDirection, Team};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Matchbox signaling server for WebRTC peer discovery.
pub const SIGNALING_SERVER: &str = "wss://signaling.rackspace.koski.co";

// ============================================================================
// GAME MESSAGES
// ============================================================================

/// Messages sent between peers during a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    /// Sent when a player calls their shot (during CallingShot phase).
    ShotCalled {
        angle: f32,
        weight: f32,
        curl: CurlDirection,
    },
    /// Sent when a player throws their stone (during Aiming phase).
    ShotThrown {
        angle: f32,
        weight: f32,
        curl: CurlDirection,
    },
    /// Sent periodically while active player drags broom.
    BroomUpdate { x: f32, y: f32 },
    /// Sent after physics simulation completes, syncing final stone positions.
    ShotResolved { stones: Vec<StoneState> },
    /// Periodic position sync during stone movement (~1/sec).
    PositionSync { stones: Vec<StoneState> },
    /// Sent after scoring an end.
    EndScored { team1: u32, team2: u32 },
    /// Sent to confirm the game is ready to start.
    GameReady,
}

/// Serialized state of a stone for network sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoneState {
    pub team: Team,
    pub x: f32,
    pub y: f32,
}

// ============================================================================
// SOCKET MARKER
// ============================================================================

/// Marker component for the matchbox socket entity.
#[derive(Component)]
pub struct NetworkSocket;

// ============================================================================
// SOCKET CREATION
// ============================================================================

/// Creates a matchbox socket and spawns it as an entity.
///
/// The room URL format is: `wss://server/room_code?next=2`
/// - `next=2` means wait for exactly 2 players (1v1 curling)
pub fn create_socket(commands: &mut Commands, room_code: &str) {
    let room_url = format!("{}/{}?next=2", SIGNALING_SERVER, room_code);
    tracing::info!(url = %room_url, "Creating matchbox socket");

    let socket = MatchboxSocket::new_reliable(room_url);
    commands.spawn((socket, NetworkSocket));
}

/// Closes and despawns the matchbox socket.
pub fn close_socket(commands: &mut Commands, socket_entity: Entity) {
    tracing::info!("Closing matchbox socket");
    commands.entity(socket_entity).despawn();
}

// ============================================================================
// PEER EVENTS
// ============================================================================

/// Result of polling peer events.
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// A peer connected.
    Connected(PeerId),
    /// A peer disconnected.
    Disconnected(PeerId),
}

/// Polls the socket for peer connection/disconnection events.
/// Returns all events that occurred since last poll.
pub fn poll_peer_events(socket: &mut MatchboxSocket) -> Vec<PeerEvent> {
    let mut events = Vec::new();
    let peers = socket.update_peers();
    for (peer_id, state) in peers {
        match state {
            PeerState::Connected => {
                tracing::info!(peer = ?peer_id, "Peer connected!");
                events.push(PeerEvent::Connected(peer_id));
            }
            PeerState::Disconnected => {
                tracing::warn!(peer = ?peer_id, "Peer disconnected");
                events.push(PeerEvent::Disconnected(peer_id));
            }
        }
    }
    events
}

/// Polls the socket for peer connection events.
/// Returns Some(PeerId) if a peer connected, None otherwise.
///
/// Note: Consider using `poll_peer_events` for full event handling.
#[deprecated(note = "Use poll_peer_events for full connection/disconnection handling")]
pub fn poll_peer_connected(socket: &mut MatchboxSocket) -> Option<PeerId> {
    let peers = socket.update_peers();
    for (peer_id, state) in peers {
        match state {
            PeerState::Connected => {
                tracing::info!(peer = ?peer_id, "Peer connected!");
                return Some(peer_id);
            }
            PeerState::Disconnected => {
                tracing::warn!(peer = ?peer_id, "Peer disconnected");
            }
        }
    }
    None
}

// ============================================================================
// MESSAGE SENDING
// ============================================================================

/// Sends a game message to all connected peers.
pub fn send_message(socket: &mut MatchboxSocket, message: &GameMessage) {
    let data = match bincode::serialize(message) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "Failed to serialize message");
            return;
        }
    };

    let peers: Vec<_> = socket.connected_peers().collect();
    let channel = socket.channel_mut(0);
    for peer_id in peers {
        channel.send(data.clone().into(), peer_id);
    }
    tracing::debug!(message = ?message, "Sent message to peers");
}

/// Receives game messages from peers.
/// Returns a Vec of (PeerId, GameMessage) for all messages received.
pub fn receive_messages(socket: &mut MatchboxSocket) -> Vec<(PeerId, GameMessage)> {
    let mut messages = Vec::new();

    let channel = socket.channel_mut(0);
    for (peer_id, data) in channel.receive() {
        match bincode::deserialize::<GameMessage>(&data) {
            Ok(msg) => {
                tracing::debug!(peer = ?peer_id, message = ?msg, "Received message");
                messages.push((peer_id, msg));
            }
            Err(e) => {
                tracing::error!(peer = ?peer_id, error = ?e, "Failed to deserialize message");
            }
        }
    }

    messages
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_message_serializes() {
        let msg = GameMessage::ShotCalled {
            angle: 5.0,
            weight: 6.5,
            curl: CurlDirection::InTurn,
        };
        let data = bincode::serialize(&msg).unwrap();
        let decoded: GameMessage = bincode::deserialize(&data).unwrap();
        match decoded {
            GameMessage::ShotCalled {
                angle,
                weight,
                curl,
            } => {
                assert!((angle - 5.0).abs() < 0.01);
                assert!((weight - 6.5).abs() < 0.01);
                assert!(matches!(curl, CurlDirection::InTurn));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn stone_state_serializes() {
        let stone = StoneState {
            team: Team::One,
            x: 1.5,
            y: 10.0,
        };
        let data = bincode::serialize(&stone).unwrap();
        let decoded: StoneState = bincode::deserialize(&data).unwrap();
        assert_eq!(decoded.team, Team::One);
        assert!((decoded.x - 1.5).abs() < 0.01);
        assert!((decoded.y - 10.0).abs() < 0.01);
    }

    #[test]
    fn broom_update_serializes() {
        let msg = GameMessage::BroomUpdate { x: 1.5, y: 30.0 };
        let data = bincode::serialize(&msg).unwrap();
        let decoded: GameMessage = bincode::deserialize(&data).unwrap();
        match decoded {
            GameMessage::BroomUpdate { x, y } => {
                assert!((x - 1.5).abs() < 0.01);
                assert!((y - 30.0).abs() < 0.01);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
