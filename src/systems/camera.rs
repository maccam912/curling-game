//! # Camera Systems
//!
//! Systems that control camera positioning and transitions.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use tracing::{debug, trace};

use crate::components::*;
use crate::constants::*;
use crate::helpers::{back_line_far, hog_line_far};
use crate::resources::*;

/// Controls camera position and transitions based on game phase.
///
/// Camera modes:
/// - **SkipView**: First-person view from behind the far house
/// - **Overhead**: Top-down view of the far house (user toggleable)
/// - **ThrowingView**: Behind the stone looking up the sheet (lower, immersive)
/// - **FollowStone**: Tracks the moving stone with rising height
/// - **HouseOverhead**: Overhead view after stone crosses far hog line
///
/// Transitions use smooth interpolation with configurable duration.
pub fn camera_control_system(
    time: Res<Time>,
    state: Res<GameState>,
    mut camera_state: ResMut<CameraState>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    stone_query: Query<(Entity, &Transform, &Velocity), (With<Stone>, Without<MainCamera>)>,
) {
    let dt = time.delta_secs();

    // Find the currently thrown stone or any moving stone behind the far hog line
    // Priority: 1) The explicitly thrown stone, 2) Any moving stone
    let tracked_stone_transform = if let Some(thrown_entity) = state.thrown_stone {
        stone_query
            .iter()
            .find(|(e, _, _)| *e == thrown_entity)
            .map(|(_, t, _)| t)
    } else {
        // Fallback: find any moving stone (velocity > threshold)
        stone_query
            .iter()
            .filter(|(_, _, v)| v.linvel.length() > 0.1)
            .map(|(_, t, _)| t)
            .next()
    };

    // Check if stone crossed far hog line during FollowStone phase
    if camera_state.mode == CameraMode::FollowStone {
        if let Some(stone_transform) = tracked_stone_transform {
            let stone_y = stone_transform.translation.y;
            if stone_y > hog_line_far() && !camera_state.stone_crossed_hog {
                camera_state.stone_crossed_hog = true;
                debug!(
                    stone_y = stone_y,
                    hog_line = hog_line_far(),
                    "Stone crossed far hog line, transitioning to HouseOverhead"
                );
            }
        }
    }

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
        Phase::StoneMoving => {
            // Transition to HouseOverhead once stone crosses far hog line
            if camera_state.stone_crossed_hog {
                CameraMode::HouseOverhead
            } else {
                CameraMode::FollowStone
            }
        }
        Phase::Resolve | Phase::ShowingScore => CameraMode::HouseOverhead,
        Phase::Ended => CameraMode::SkipView,
    };

    // Only auto-switch if mode differs
    let should_switch = camera_state.mode != desired_mode;
    if should_switch {
        let previous_mode = camera_state.mode;
        camera_state.mode = desired_mode;
        camera_state.transition_progress = 0.0;

        // Reset follow camera height when entering FollowStone
        if desired_mode == CameraMode::FollowStone {
            camera_state.follow_camera_height = FOLLOW_START_HEIGHT;
            camera_state.stone_crossed_hog = false;
        }

        // Reset stone_crossed_hog when entering ThrowingView (before throw)
        // This ensures subsequent throws start with fresh tracking state
        if desired_mode == CameraMode::ThrowingView {
            camera_state.stone_crossed_hog = false;
        }

        // Set duration based on transition type
        camera_state.transition_duration = match desired_mode {
            CameraMode::SkipView | CameraMode::Overhead => 0.5,
            CameraMode::ThrowingView => 1.0,
            CameraMode::FollowStone => 0.3,
            CameraMode::HouseOverhead => 1.5, // Smooth transition to overhead
        };

        debug!(
            from = ?previous_mode,
            to = ?desired_mode,
            duration = camera_state.transition_duration,
            "Camera mode changed"
        );
    }

    // Gradually increase follow camera height while following stone
    if camera_state.mode == CameraMode::FollowStone {
        camera_state.follow_camera_height =
            (camera_state.follow_camera_height + CAMERA_RISE_RATE * dt).min(FOLLOW_RISE_HEIGHT);
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
            // Lower camera positioned behind where stone spawns
            camera_state.target_position = Vec3::new(
                0.0,
                DELIVERY_START_Y - THROWING_VIEW_BEHIND,
                THROWING_VIEW_HEIGHT,
            );
            // Look toward the broom position (roughly up the sheet)
            camera_state.target_look_at =
                Vec3::new(state.broom_position.x, state.broom_position.y, 0.0);
        }
        CameraMode::FollowStone => {
            if let Some(stone_transform) = tracked_stone_transform {
                let stone_pos = stone_transform.translation;
                // Camera behind stone at dynamically rising height
                camera_state.target_position = Vec3::new(
                    stone_pos.x,
                    stone_pos.y - 5.0,
                    camera_state.follow_camera_height,
                );
                camera_state.target_look_at = Vec3::new(stone_pos.x, stone_pos.y + 10.0, 0.0);
            }
        }
        CameraMode::HouseOverhead => {
            // Overhead view centered between hog line and back line to show guards and house
            let view_center_y = (hog_line_far() + back_line_far()) * 0.5;
            // Increase height to zoom out and show more area
            camera_state.target_position =
                Vec3::new(0.0, view_center_y, HOUSE_OVERHEAD_HEIGHT + 4.0);
            camera_state.target_look_at = Vec3::new(0.0, view_center_y, 0.0);
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
                height = camera_state.follow_camera_height,
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
