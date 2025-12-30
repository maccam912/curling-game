//! # Prediction Systems
//!
//! Systems that handle ghost stone trajectory prediction.
//!
//! The prediction system simulates the physics of a thrown stone to predict
//! where it will end up. This uses the same `apply_ice_friction` function
//! as the real physics, run in a tight loop until the stone stops.
//!
//! ## Limitations
//! - Does not simulate collisions with other stones (only detects path intersections)
//! - Confidence drops when predicted path crosses existing stones

use bevy::prelude::*;

use crate::components::{GhostStone, Phase, Stone};
use crate::constants::{
    DELIVERY_START_Y, STONE_RADIUS, WEIGHT_MAX, WEIGHT_MAX_SPEED, WEIGHT_MIN, WEIGHT_MIN_SPEED,
};
use crate::resources::{GameState, PredictionState};
use crate::systems::physics::apply_ice_friction;

// ============================================================================
// PREDICTION CONSTANTS
// ============================================================================

/// Physics simulation timestep (60 Hz).
const PREDICTION_DT: f32 = 1.0 / 60.0;

/// Maximum simulation ticks (~33 seconds at 60 Hz).
const MAX_PREDICTION_TICKS: usize = 2000;

/// Distance threshold for collision detection (stone centers within 2 radii).
const COLLISION_THRESHOLD: f32 = STONE_RADIUS * 2.0;

/// Confidence penalty when path crosses an existing stone.
const COLLISION_CONFIDENCE_PENALTY: f32 = 0.3;

// ============================================================================
// PREDICTION FUNCTIONS
// ============================================================================

/// Predicts where a stone will end up given initial parameters.
///
/// # Arguments
/// * `start_pos` - Starting position of the stone
/// * `initial_velocity` - Initial velocity vector
/// * `angular_velocity` - Initial angular velocity (for curl calculation)
/// * `existing_stones` - Positions of existing stones on the ice
///
/// # Returns
/// Tuple of (final_position, confidence)
/// - `final_position`: Where the stone is predicted to stop
/// - `confidence`: 1.0 = no collisions expected, lower = path crossed stones
pub fn predict_stone_trajectory(
    start_pos: Vec2,
    initial_velocity: Vec2,
    angular_velocity: f32,
    existing_stones: &[Vec2],
) -> (Vec2, f32) {
    let mut pos = start_pos;
    let mut vel = initial_velocity;
    let mut ang_vel = angular_velocity;
    let mut confidence = 1.0;
    let mut has_collided_with = vec![false; existing_stones.len()];

    for _ in 0..MAX_PREDICTION_TICKS {
        let result = apply_ice_friction(vel, ang_vel, PREDICTION_DT);

        if result.stopped {
            break;
        }

        // Update position based on velocity
        pos += result.new_velocity * PREDICTION_DT;
        vel = result.new_velocity;
        ang_vel = result.new_angular_velocity;

        // Check for path intersection with existing stones
        // Only penalize confidence once per stone
        for (i, stone_pos) in existing_stones.iter().enumerate() {
            if !has_collided_with[i] && pos.distance(*stone_pos) < COLLISION_THRESHOLD {
                confidence *= COLLISION_CONFIDENCE_PENALTY;
                has_collided_with[i] = true;
            }
        }
    }

    (pos, confidence)
}

/// Converts weight (1-10 scale) to speed in m/s.
fn weight_to_speed(weight: f32) -> f32 {
    let normalized = (weight - WEIGHT_MIN) / (WEIGHT_MAX - WEIGHT_MIN);
    WEIGHT_MIN_SPEED + normalized * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED)
}

// ============================================================================
// PREDICTION SYSTEMS
// ============================================================================

/// Updates the prediction state based on current shot parameters.
///
/// This system runs during CallingShot and Aiming phases to continuously
/// update where the ghost stone should be positioned.
pub fn update_prediction(
    state: Res<GameState>,
    mut prediction: ResMut<PredictionState>,
    stones: Query<&Transform, With<Stone>>,
) {
    // Only predict during calling and aiming phases
    if state.phase != Phase::CallingShot && state.phase != Phase::Aiming {
        prediction.is_valid = false;
        prediction.predicted_position = None;
        return;
    }

    // Get aim parameters (use called values during CallingShot, aim values during Aiming)
    let (angle_deg, weight) = match state.phase {
        Phase::CallingShot => (state.angle_from_broom(), state.weight_from_broom()),
        Phase::Aiming => (state.aim_angle_deg, state.aim_weight),
        _ => return,
    };

    // Calculate initial position and velocity
    let start_pos = Vec2::new(0.0, DELIVERY_START_Y);
    let angle_rad = angle_deg.to_radians();
    let speed = weight_to_speed(weight);
    let initial_velocity = Vec2::new(angle_rad.sin(), angle_rad.cos()) * speed;

    // Get angular velocity from curl direction
    let angular_velocity = state.curl_direction.angular_velocity();

    // Collect existing stone positions
    let existing_stones: Vec<Vec2> = stones
        .iter()
        .map(|t| Vec2::new(t.translation.x, t.translation.y))
        .collect();

    // Run prediction
    let (final_pos, confidence) = predict_stone_trajectory(
        start_pos,
        initial_velocity,
        angular_velocity,
        &existing_stones,
    );

    prediction.predicted_position = Some(final_pos);
    prediction.confidence = confidence;
    prediction.is_valid = true;
}

/// Updates the ghost stone visual based on prediction state.
///
/// Positions the ghost stone at the predicted final position and adjusts
/// its opacity based on confidence.
pub fn update_ghost_stone_visual(
    prediction: Res<PredictionState>,
    _state: Res<GameState>,
    mut ghost_query: Query<(&mut Transform, &mut Visibility), With<GhostStone>>,
) {
    for (mut transform, mut visibility) in ghost_query.iter_mut() {
        if prediction.is_valid {
            if let Some(pos) = prediction.predicted_position {
                // Position the ghost stone at predicted location
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
                // Keep Z at a fixed height (slightly above ice)
                transform.translation.z = 0.15;

                // Show the ghost stone
                *visibility = Visibility::Visible;
            }
        } else {
            // Hide ghost stone when not predicting
            *visibility = Visibility::Hidden;
        }
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_straight_shot_lands_forward() {
        let start = Vec2::new(0.0, -20.0);
        let velocity = Vec2::new(0.0, 2.5); // Straight forward at 2.5 m/s

        let (final_pos, confidence) = predict_stone_trajectory(start, velocity, 0.0, &[]);

        // Should land forward of start position
        assert!(final_pos.y > start.y, "Stone should move forward");
        // Should land roughly on center line (no curl with 0 angular velocity)
        assert!(
            final_pos.x.abs() < 0.5,
            "Stone should stay near center without curl"
        );
        // Full confidence with no obstacles
        assert_eq!(confidence, 1.0);
    }

    #[test]
    fn predict_curl_deflects_stone() {
        let start = Vec2::new(0.0, -20.0);
        let velocity = Vec2::new(0.0, 2.5);
        let angular_velocity = 1.5; // Positive = in-turn = curls left

        let (final_pos, _) = predict_stone_trajectory(start, velocity, angular_velocity, &[]);

        // Should curl left (negative X)
        assert!(
            final_pos.x < -0.1,
            "In-turn should curl left, got x={}",
            final_pos.x
        );
    }

    #[test]
    fn predict_opposite_curl_direction() {
        let start = Vec2::new(0.0, -20.0);
        let velocity = Vec2::new(0.0, 2.5);

        let (left_pos, _) = predict_stone_trajectory(start, velocity, 1.5, &[]);
        let (right_pos, _) = predict_stone_trajectory(start, velocity, -1.5, &[]);

        assert!(left_pos.x < 0.0, "Positive angular should curl left");
        assert!(right_pos.x > 0.0, "Negative angular should curl right");
    }

    #[test]
    fn predict_confidence_drops_on_collision() {
        let start = Vec2::new(0.0, -20.0);
        let velocity = Vec2::new(0.0, 2.5);

        // Place a stone in the path (stone starts at -20, travels ~27m, so anywhere in between)
        // At 2.5 m/s with friction 0.115, stone travels ~27m: -20 + 27 = 7
        // Let's place obstacle at y=0 which is well within the path
        let obstacle = Vec2::new(0.0, 0.0);

        let (_, confidence) = predict_stone_trajectory(start, velocity, 0.0, &[obstacle]);

        assert!(confidence < 1.0, "Confidence should drop with obstacle");
        assert!(
            (confidence - COLLISION_CONFIDENCE_PENALTY).abs() < 0.01,
            "Confidence should be ~{}, got {}",
            COLLISION_CONFIDENCE_PENALTY,
            confidence
        );
    }

    #[test]
    fn weight_to_speed_conversion() {
        let min_speed = weight_to_speed(WEIGHT_MIN);
        let max_speed = weight_to_speed(WEIGHT_MAX);
        let mid_speed = weight_to_speed((WEIGHT_MIN + WEIGHT_MAX) / 2.0);

        assert!(
            (min_speed - WEIGHT_MIN_SPEED).abs() < 0.01,
            "Min weight should give min speed"
        );
        assert!(
            (max_speed - WEIGHT_MAX_SPEED).abs() < 0.01,
            "Max weight should give max speed"
        );
        assert!(
            mid_speed > min_speed && mid_speed < max_speed,
            "Mid weight should give mid speed"
        );
    }
}
