//! # Camera Systems
//!
//! Systems that control camera positioning and transitions.

use bevy::prelude::*;
use tracing::{debug, trace};

use crate::components::*;
use crate::constants::*;
use crate::resources::*;

/// Controls camera position and transitions based on game phase.
///
/// Camera modes:
/// - **SkipView**: First-person view from behind the far house
/// - **Overhead**: Top-down view of the far house
/// - **ThrowingView**: Behind the hack looking up the sheet
/// - **FollowStone**: Tracks the moving stone
///
/// Transitions use smooth interpolation with configurable duration.
pub fn camera_control_system(
    time: Res<Time>,
    state: Res<GameState>,
    mut camera_state: ResMut<CameraState>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    thrown_stone_query: Query<&Transform, (With<ThrowingStone>, Without<MainCamera>)>,
) {
    let dt = time.delta_secs();

    // Determine target camera mode based on game phase
    let desired_mode = match state.phase {
        Phase::CallingShot => {
            // During CallingShot, prefer SkipView but allow user toggle
            // If not in SkipView or Overhead (valid calling modes), reset to SkipView
            match camera_state.mode {
                CameraMode::SkipView | CameraMode::Overhead => camera_state.mode,
                _ => CameraMode::SkipView,
            }
        }
        Phase::Aiming => CameraMode::ThrowingView,
        Phase::StoneMoving => CameraMode::FollowStone,
        Phase::Resolve | Phase::Ended => CameraMode::SkipView,
    };

    // Only auto-switch if not in CallingShot (user can toggle in CallingShot)
    // OR if current mode is invalid for CallingShot phase
    let should_switch = camera_state.mode != desired_mode;
    if should_switch {
        let previous_mode = camera_state.mode;
        camera_state.mode = desired_mode;
        camera_state.transition_progress = 0.0;

        // Set duration based on transition type
        camera_state.transition_duration = match desired_mode {
            CameraMode::SkipView | CameraMode::Overhead => 0.5,
            CameraMode::ThrowingView => 1.0,
            CameraMode::FollowStone => 0.5,
        };

        debug!(
            from = ?previous_mode,
            to = ?desired_mode,
            duration = camera_state.transition_duration,
            "Camera mode changed"
        );
    }

    // Calculate target position and look-at based on mode
    match camera_state.mode {
        CameraMode::SkipView => {
            camera_state.target_position =
                Vec3::new(0.0, TEE_FROM_CENTER + BACK_FROM_TEE + 2.0, 1.7);
            camera_state.target_look_at = Vec3::new(0.0, TEE_FROM_CENTER, 0.0);
        }
        CameraMode::Overhead => {
            camera_state.target_position = Vec3::new(0.0, TEE_FROM_CENTER, 12.0);
            camera_state.target_look_at = Vec3::new(0.0, TEE_FROM_CENTER, 0.0);
        }
        CameraMode::ThrowingView => {
            camera_state.target_position = Vec3::new(0.0, DELIVERY_START_Y - 3.0, 2.0);
            camera_state.target_look_at = Vec3::new(0.0, 0.0, 0.0);
        }
        CameraMode::FollowStone => {
            if let Some(stone_transform) = thrown_stone_query.iter().next() {
                let stone_pos = stone_transform.translation;
                // Lower camera (1.5m) and further back (7m) so stone is visible above house buttons
                camera_state.target_position = Vec3::new(stone_pos.x, stone_pos.y - 7.0, 1.5);
                camera_state.target_look_at = Vec3::new(stone_pos.x, stone_pos.y + 10.0, 0.0);
            }
        }
    }

    // Smoothly interpolate camera position and rotation
    if let Some(mut camera_transform) = camera_query.iter_mut().next() {
        let target_rotation = Transform::from_translation(camera_state.target_position)
            .looking_at(camera_state.target_look_at, Vec3::Z)
            .rotation;

        if camera_state.mode == CameraMode::FollowStone {
            // Use faster lerp to track moving stone smoothly
            let lerp_factor = (dt * 5.0).min(1.0);
            camera_transform.translation = camera_transform
                .translation
                .lerp(camera_state.target_position, lerp_factor);
            camera_transform.rotation = camera_transform
                .rotation
                .slerp(target_rotation, lerp_factor);

            trace!(
                position = ?camera_transform.translation,
                "Camera following stone"
            );
        } else {
            // Time-based transition for other modes
            if camera_state.transition_progress < 1.0 {
                camera_state.transition_progress += dt / camera_state.transition_duration.max(0.01);
                camera_state.transition_progress = camera_state.transition_progress.min(1.0);
            }

            // Use smooth ease-in-out curve (smoothstep)
            let t = camera_state.transition_progress;
            let smooth_t = t * t * (3.0 - 2.0 * t);

            camera_transform.translation = camera_transform
                .translation
                .lerp(camera_state.target_position, smooth_t.min(1.0).max(0.0));
            camera_transform.rotation = camera_transform
                .rotation
                .slerp(target_rotation, smooth_t.min(1.0).max(0.0));
        }
    }
}
