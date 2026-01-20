//! # Physics Systems
//!
//! Systems that handle curling stone physics simulation.

use bevy::prelude::*;
use bevy::time::Fixed;
use bevy_rapier2d::prelude::*;
use tracing::{debug, trace};

use crate::components::{Stone, ThrowingStone};
use crate::constants::{CURL_COEFFICIENT, ICE_FRICTION_DECEL, STOP_SPEED, VISUAL_ROTATION_DAMPING};

/// Tracks the maximum Y position of the throwing stone.
///
/// This is used to check if the stone crossed the hog lines.
pub fn track_throwing_stone(mut stones: Query<(&Transform, &mut ThrowingStone)>) {
    for (transform, mut marker) in stones.iter_mut() {
        let new_max = marker.max_y.max(transform.translation.y);
        if new_max > marker.max_y {
            marker.max_y = new_max;
            trace!(max_y = new_max, "Throwing stone max Y updated");
        }
    }
}

/// Detects collisions between the throwing stone and other stones.
///
/// When a collision occurs, marks the throwing stone so the far hog line
/// rule doesn't apply (stone hitting another stone before the far hog line
/// is allowed to stay in play).
pub fn detect_stone_collision(
    mut collision_events: MessageReader<CollisionEvent>,
    mut throwing_stones: Query<&mut ThrowingStone>,
    stones: Query<Entity, With<Stone>>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Check if one entity is the throwing stone and the other is any stone
            let (throwing_entity, other_entity) = if throwing_stones.get(*e1).is_ok() {
                (*e1, *e2)
            } else if throwing_stones.get(*e2).is_ok() {
                (*e2, *e1)
            } else {
                continue;
            };

            // Check if the other entity is also a stone
            if stones.get(other_entity).is_ok() {
                if let Ok(mut marker) = throwing_stones.get_mut(throwing_entity) {
                    marker.hit_stone = true;
                    debug!("Throwing stone collided with another stone");
                }
            }
        }
    }
}

/// Applies ice friction and curl physics to all stones.
///
/// This system simulates realistic curling physics:
/// - **Constant friction**: Unlike damping (exponential decay), curling stones
///   experience nearly constant friction force on pebbled ice.
/// - **Curl effect**: Lateral force proportional to angular velocity, causing
///   the stone to curve. The curl effect increases as the stone slows down.
/// - **Angular velocity decay**: The rotation gradually decreases over time.
/// Result of applying ice friction for one physics tick.
#[derive(Debug, Clone, PartialEq)]
pub struct FrictionResult {
    /// New linear velocity after friction
    pub new_velocity: Vec2,
    /// New angular velocity after decay
    pub new_angular_velocity: f32,
    /// Whether the stone has stopped
    pub stopped: bool,
}

/// Applies ice friction physics for a single tick.
///
/// This is a pure function that can be easily unit tested.
///
/// # Arguments
/// * `velocity` - Current linear velocity
/// * `angular_velocity` - Current angular velocity (for curl)
/// * `dt` - Delta time for this tick
///
/// # Returns
/// A `FrictionResult` with the new velocity state
pub fn apply_ice_friction(velocity: Vec2, angular_velocity: f32, dt: f32) -> FrictionResult {
    let speed = velocity.length();

    if speed > STOP_SPEED {
        let decel_amount = ICE_FRICTION_DECEL * dt;
        let move_direction = velocity.normalize();
        let new_speed = (speed - decel_amount).max(0.0);

        // Calculate curl: lateral force proportional to angular velocity
        let perpendicular = Vec2::new(-move_direction.y, move_direction.x);
        let curl_factor = CURL_COEFFICIENT * angular_velocity * dt;
        let speed_factor = (2.0 / (speed + 0.5)).min(3.0);
        let curl_offset = perpendicular * curl_factor * speed_factor;

        let new_velocity = move_direction * new_speed + curl_offset;
        let new_angular_velocity = angular_velocity * 0.998;

        FrictionResult {
            new_velocity,
            new_angular_velocity,
            stopped: false,
        }
    } else {
        FrictionResult {
            new_velocity: Vec2::ZERO,
            new_angular_velocity: 0.0,
            stopped: true,
        }
    }
}

pub fn ice_friction_system(time: Res<Time<Fixed>>, mut stones: Query<(&mut Velocity, &mut Stone)>) {
    let dt = time.delta_secs();

    for (mut velocity, mut stone) in stones.iter_mut() {
        let result = apply_ice_friction(velocity.linvel, stone.angular_velocity, dt);

        velocity.linvel = result.new_velocity;
        stone.angular_velocity = result.new_angular_velocity;

        if result.stopped {
            // Also zero Rapier's angular velocity to prevent perpetual spinning
            // (collisions impart angular momentum that would otherwise persist)
            velocity.angvel = 0.0;
            trace!("Stone stopped");
        } else {
            trace!(
                speed = result.new_velocity.length(),
                angular_vel = result.new_angular_velocity,
                "Stone physics updated"
            );
        }
    }
}

/// Rotates the visual stone models to show spinning.
///
/// This system:
/// - Maintains constant rotation speed while the stone is moving
/// - Applies smooth exponential damping when the stone comes to rest
/// - Uses the `visual_rotation_speed` field on Stone to track current spin rate
pub fn update_stone_visual_rotation(
    time: Res<Time>,
    mut stones: Query<(&Velocity, &mut Stone, &Children)>,
    mut visuals: Query<&mut Transform, With<crate::components::StoneVisual>>,
) {
    let dt = time.delta_secs();

    for (velocity, mut stone, children) in stones.iter_mut() {
        let is_moving = velocity.linvel.length() > STOP_SPEED;

        if is_moving {
            // While moving, maintain constant rotation speed (already set at spawn)
            // No damping while in motion
        } else {
            // At rest: apply exponential damping to spin down smoothly
            // decay = e^(-damping * dt), but we use a linear approximation for small dt
            let decay = (-VISUAL_ROTATION_DAMPING * dt).exp();
            stone.visual_rotation_speed *= decay;

            // Snap to zero when very small to avoid floating point issues
            if stone.visual_rotation_speed.abs() < 0.01 {
                stone.visual_rotation_speed = 0.0;
            }
        }

        // Apply rotation to visual based on current rotation speed
        if stone.visual_rotation_speed.abs() > 0.001 {
            for child in children.iter() {
                if let Ok(mut transform) = visuals.get_mut(child) {
                    // Apply rotation around Y axis (local Y is world Z due to 90° X rotation)
                    let rotation_delta = Quat::from_rotation_y(stone.visual_rotation_speed * dt);
                    transform.rotation = transform.rotation * rotation_delta;
                }
            }
        }
    }
}

// ============================================================================
// PHYSICS UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::CURL_ANGULAR_VELOCITY;

    const FIXED_DT: f32 = 1.0 / 60.0; // 60 Hz physics tick

    /// Test that friction reduces speed over one tick
    #[test]
    fn friction_reduces_speed() {
        let initial_velocity = Vec2::new(0.0, 3.0); // 3 m/s straight up
        let result = apply_ice_friction(initial_velocity, 0.0, FIXED_DT);

        let expected_decel = ICE_FRICTION_DECEL * FIXED_DT;
        let expected_speed = 3.0 - expected_decel;

        assert!(!result.stopped);
        assert!(
            (result.new_velocity.length() - expected_speed).abs() < 0.001,
            "Speed should decrease by {} m/s per tick, got {} instead of {}",
            expected_decel,
            result.new_velocity.length(),
            expected_speed
        );
    }

    /// Test that stone stops when below threshold
    #[test]
    fn stone_stops_below_threshold() {
        let slow_velocity = Vec2::new(0.0, STOP_SPEED * 0.5);
        let result = apply_ice_friction(slow_velocity, 1.0, FIXED_DT);

        assert!(result.stopped);
        assert_eq!(result.new_velocity, Vec2::ZERO);
        assert_eq!(result.new_angular_velocity, 0.0);
    }

    /// Test that stone at exactly STOP_SPEED stops
    #[test]
    fn stone_at_threshold_stops() {
        let threshold_velocity = Vec2::new(0.0, STOP_SPEED);
        let result = apply_ice_friction(threshold_velocity, 1.0, FIXED_DT);

        assert!(result.stopped);
    }

    /// Test that angular velocity decays over time
    #[test]
    fn angular_velocity_decays() {
        let velocity = Vec2::new(0.0, 3.0);
        let initial_angular = 1.5;
        let result = apply_ice_friction(velocity, initial_angular, FIXED_DT);

        assert!(result.new_angular_velocity < initial_angular);
        assert!(
            (result.new_angular_velocity - initial_angular * 0.998).abs() < 0.0001,
            "Angular velocity should decay by 0.2% per tick"
        );
    }

    /// Test that deceleration is approximately constant (linear, not exponential)
    #[test]
    fn deceleration_is_constant() {
        let mut velocity = Vec2::new(0.0, 3.0);
        let mut prev_decel = 0.0;

        for i in 0..100 {
            let old_speed = velocity.length();
            let result = apply_ice_friction(velocity, 0.0, FIXED_DT);
            velocity = result.new_velocity;
            let new_speed = velocity.length();

            if result.stopped {
                break;
            }

            let decel = old_speed - new_speed;

            if i > 0 && prev_decel > 0.0 {
                // Deceleration should be nearly constant (within 1% due to curl effects)
                assert!(
                    (decel - prev_decel).abs() / prev_decel < 0.01,
                    "Deceleration should be constant, got {} vs {}",
                    decel,
                    prev_decel
                );
            }
            prev_decel = decel;
        }
    }

    /// Test realistic travel distance for a draw shot
    #[test]
    fn draw_shot_travel_distance() {
        // A draw shot at ~2.5 m/s should travel roughly 25-30 meters
        let mut velocity = Vec2::new(0.0, 2.5);
        let mut total_distance = 0.0;
        let mut ticks = 0;

        while !apply_ice_friction(velocity, 0.0, FIXED_DT).stopped && ticks < 10000 {
            let result = apply_ice_friction(velocity, 0.0, FIXED_DT);
            velocity = result.new_velocity;
            total_distance += velocity.length() * FIXED_DT;
            ticks += 1;
        }

        // Using physics: d = v² / (2a) = 2.5² / (2 * 0.115) ≈ 27.2 m
        let expected_distance = 2.5_f32.powi(2) / (2.0 * ICE_FRICTION_DECEL);

        assert!(
            (total_distance - expected_distance).abs() < 2.0,
            "Draw shot should travel ~{:.1}m, got {:.1}m",
            expected_distance,
            total_distance
        );
    }

    /// Test realistic travel time
    #[test]
    fn stone_travel_time() {
        // Time to stop: t = v / a
        let initial_speed = 3.0; // m/s
        let expected_time = initial_speed / ICE_FRICTION_DECEL; // ~26 seconds

        let mut velocity = Vec2::new(0.0, initial_speed);
        let mut ticks = 0;

        while !apply_ice_friction(velocity, 0.0, FIXED_DT).stopped && ticks < 10000 {
            let result = apply_ice_friction(velocity, 0.0, FIXED_DT);
            velocity = result.new_velocity;
            ticks += 1;
        }

        let actual_time = ticks as f32 * FIXED_DT;

        assert!(
            (actual_time - expected_time).abs() < 1.0,
            "Stone should stop in ~{:.1}s, got {:.1}s",
            expected_time,
            actual_time
        );
    }

    /// Test that curl deflects the stone perpendicular to motion
    #[test]
    fn curl_deflects_stone() {
        let velocity = Vec2::new(0.0, 2.0); // Moving straight up
        let angular_vel = CURL_ANGULAR_VELOCITY; // Positive = in-turn = curls left

        let result = apply_ice_friction(velocity, angular_vel, FIXED_DT);

        // With positive angular velocity, stone should curl left (negative X)
        assert!(
            result.new_velocity.x < 0.0,
            "Positive angular velocity should curl left, got x={}",
            result.new_velocity.x
        );
    }

    /// Test that opposite curl directions produce opposite deflection
    #[test]
    fn opposite_curl_directions() {
        let velocity = Vec2::new(0.0, 2.0);

        let in_turn = apply_ice_friction(velocity, CURL_ANGULAR_VELOCITY, FIXED_DT);
        let out_turn = apply_ice_friction(velocity, -CURL_ANGULAR_VELOCITY, FIXED_DT);

        assert!(in_turn.new_velocity.x < 0.0, "In-turn should curl left");
        assert!(out_turn.new_velocity.x > 0.0, "Out-turn should curl right");

        // Magnitudes should be equal
        assert!(
            (in_turn.new_velocity.x.abs() - out_turn.new_velocity.x.abs()).abs() < 0.0001,
            "Curl magnitudes should be equal"
        );
    }

    /// Test that curl increases as stone slows down
    #[test]
    fn curl_increases_when_slow() {
        let fast = Vec2::new(0.0, 3.0);
        let slow = Vec2::new(0.0, 1.0);
        let angular = CURL_ANGULAR_VELOCITY;

        let fast_result = apply_ice_friction(fast, angular, FIXED_DT);
        let slow_result = apply_ice_friction(slow, angular, FIXED_DT);

        // Normalize by speed to get curl per unit distance
        let fast_curl_ratio = fast_result.new_velocity.x.abs() / fast_result.new_velocity.y;
        let slow_curl_ratio = slow_result.new_velocity.x.abs() / slow_result.new_velocity.y;

        assert!(
            slow_curl_ratio > fast_curl_ratio,
            "Slower stones should curl more: fast={:.4}, slow={:.4}",
            fast_curl_ratio,
            slow_curl_ratio
        );
    }

    /// Test zero velocity doesn't panic
    #[test]
    fn zero_velocity_handled() {
        let result = apply_ice_friction(Vec2::ZERO, 1.0, FIXED_DT);
        assert!(result.stopped);
        assert_eq!(result.new_velocity, Vec2::ZERO);
    }

    /// Test direction is preserved during deceleration
    #[test]
    fn direction_preserved() {
        let velocity = Vec2::new(1.0, 2.0).normalize() * 3.0; // Diagonal, 3 m/s
        let result = apply_ice_friction(velocity, 0.0, FIXED_DT);

        let old_dir = velocity.normalize();
        let new_dir = result.new_velocity.normalize();

        // Direction should be nearly the same (within 1 degree)
        let dot = old_dir.dot(new_dir);
        assert!(
            dot > 0.9998,
            "Direction should be preserved, dot product was {}",
            dot
        );
    }
}
