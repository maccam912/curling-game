//! # AI Systems
//!
//! AI opponent logic for single-player mode.
//! The AI makes strategic but imperfect shots for balanced gameplay.
//!
//! The strategic decision-making logic is in the `ai_strategy` module,
//! while this module handles the Bevy ECS integration and timing.

use bevy::prelude::*;
use rand::Rng;

use crate::ai_strategy;
use crate::components::{CurlDirection, Phase, ShotType, Stone, Team};
use crate::constants::*;
use crate::helpers::{back_line_far, hog_line_far, snapshot_stones, spawn_stone};
use crate::resources::{GameState, PlayerPersonalities, StoneAssets};
use crate::systems::prediction::predict_stone_trajectory;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Calculates the broom position needed to land a stone at the target position.
///
/// The broom position determines the throw angle and weight. Due to curl and
/// ice physics, the stone's final resting place differs from where the broom
/// is placed. This function uses trajectory prediction to find the optimal
/// broom position that results in the stone landing at the target.
///
/// # Arguments
/// * `target_pos` - Where we want the stone to end up
/// * `curl` - The curl direction for the shot
/// * `existing_stones` - Positions of stones already on the ice
///
/// # Returns
/// The broom position that should result in the stone landing near target_pos
fn calculate_broom_for_target(
    target_pos: Vec2,
    curl: CurlDirection,
    existing_stones: &[Vec2],
) -> Vec2 {
    let start_pos = Vec2::new(0.0, DELIVERY_START_Y);
    let angular_velocity = curl.angular_velocity();

    // Broom Y position maps to weight:
    // - min_y (hog line) = WEIGHT_MIN
    // - max_y (back line) = WEIGHT_MAX
    let min_y = hog_line_far();
    let max_y = back_line_far();

    let mut best_broom = target_pos;
    let mut best_distance = f32::MAX;

    // Search over the full weight range (not just small offsets around target)
    // This ensures we find the correct weight to reach the target distance
    let weight_steps = [
        1.0f32, 2.0, 3.0, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 8.0, 9.0, 10.0,
    ];

    // Search X offsets in the opposite direction of curl to compensate
    let x_offsets = [
        -2.0f32, -1.5, -1.0, -0.5, -0.25, 0.0, 0.25, 0.5, 1.0, 1.5, 2.0,
    ];

    for &weight in &weight_steps {
        // Convert weight to broom Y position
        let normalized = (weight - WEIGHT_MIN) / (WEIGHT_MAX - WEIGHT_MIN);
        let broom_y = min_y + normalized * (max_y - min_y);

        for &x_offset in &x_offsets {
            // Apply X offset in the opposite direction of curl to compensate
            let broom_x =
                target_pos.x + x_offset * (if angular_velocity > 0.0 { 1.0 } else { -1.0 });
            let test_broom = Vec2::new(broom_x, broom_y);

            // Calculate throw parameters from this broom position
            let direction = test_broom - start_pos;
            let angle_rad = direction.x.atan2(direction.y);

            // Convert weight to speed
            let speed = WEIGHT_MIN_SPEED
                + (weight - WEIGHT_MIN) / (WEIGHT_MAX - WEIGHT_MIN)
                    * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
            let velocity = Vec2::new(angle_rad.sin() * speed, angle_rad.cos() * speed);

            // Predict where this shot would land
            let (predicted_pos, _) =
                predict_stone_trajectory(start_pos, velocity, angular_velocity, existing_stones);

            // Check if this is closer to our target
            let distance = predicted_pos.distance(target_pos);
            if distance < best_distance {
                best_distance = distance;
                best_broom = test_broom;
            }
        }
    }

    best_broom
}

/// Sets up the game for AI mode by assigning Team Two to the AI.
pub fn setup_ai_game(mut state: ResMut<GameState>) {
    state.ai_team = Some(Team::Two);
    state.ai_think_timer = 0.0;
    tracing::info!("AI game setup: AI controls Team 2");
}

/// Run condition: returns true only if it's the human player's turn.
///
/// Use this to gate input systems in VsAI state so the human can't
/// interfere with the AI's turn.
pub fn run_if_human_turn(state: Res<GameState>) -> bool {
    if let Some(ai_team) = state.ai_team {
        state.current_team() != ai_team
    } else {
        true // No AI, allow input
    }
}

// ============================================================================
// AI TURN SYSTEM
// ============================================================================

/// Main AI system that detects when it's the AI's turn and handles the shot.
pub fn ai_turn_system(
    time: Res<Time>,
    mut state: ResMut<GameState>,
    mut commands: Commands,
    assets: Res<StoneAssets>,
    stones: Query<(Entity, &Transform, &Stone)>,
    personalities: Res<PlayerPersonalities>,
) {
    // Only act if AI is enabled and it's AI's turn
    let ai_team = match state.ai_team {
        Some(team) => team,
        None => return,
    };

    // Only act during calling or aiming phase
    if state.phase != Phase::CallingShot && state.phase != Phase::Aiming {
        return;
    }

    // Check if it's the AI's turn
    if state.current_team() != ai_team {
        // Reset timer when it's not AI's turn
        state.ai_think_timer = 0.0;
        return;
    }

    // AI "thinking" delay
    state.ai_think_timer += time.delta_secs();

    let think_time =
        AI_THINK_TIME_MIN + rand::rng().random::<f32>() * (AI_THINK_TIME_MAX - AI_THINK_TIME_MIN);

    if state.ai_think_timer < think_time {
        return;
    }

    // Time to make a decision!
    match state.phase {
        Phase::CallingShot => {
            // Calculate and set the shot
            let (target_pos, shot_type, curl) = calculate_ai_shot(&stones, ai_team, &state);

            state.broom_position = target_pos;
            state.shot_type = shot_type;
            state.curl_direction = curl;
            state.called_angle_deg = state.angle_from_broom();
            state.called_weight = state.weight_from_broom();

            // Set aim to called values (personality variance applied at throw time)
            state.aim_angle_deg = state.called_angle_deg;
            state.aim_weight = state.called_weight;

            tracing::info!(
                "AI calling shot: {:?} to ({:.2}, {:.2}), weight {:.1}",
                shot_type,
                target_pos.x,
                target_pos.y,
                state.called_weight
            );

            // Transition directly to aiming
            state.phase = Phase::Aiming;
            state.ai_think_timer = 0.0;
        }
        Phase::Aiming => {
            // Execute the throw
            execute_ai_throw(&mut state, &mut commands, &assets, &stones, &personalities);
        }
        _ => {}
    }
}

// ============================================================================
// AI DECISION LOGIC
// ============================================================================

/// Calculates the best shot for the AI based on current game state.
///
/// Uses the `ai_strategy` module for pure decision logic, then calculates
/// the broom position needed to achieve the target using trajectory prediction.
///
/// Returns (broom_position, shot_type, curl_direction).
fn calculate_ai_shot(
    stones: &Query<(Entity, &Transform, &Stone)>,
    ai_team: Team,
    state: &GameState,
) -> (Vec2, ShotType, CurlDirection) {
    let mut rng = rand::rng();

    // Collect stone positions for strategy calculation
    let stone_positions: Vec<(Team, Vec2)> = gather_stone_positions(stones);

    // Collect just Vec2 positions for trajectory prediction
    let existing_stone_positions: Vec<Vec2> = stone_positions.iter().map(|(_, pos)| *pos).collect();

    // Calculate score differential from AI's perspective
    let score_diff = calculate_score_diff(ai_team, state);

    // Random chance for suboptimal decision
    let make_mistake = rng.random::<f32>() < AI_MISTAKE_CHANCE;

    // Use ai_strategy module for pure decision logic
    let (target, shot_type, curl) = ai_strategy::decide_shot(
        &stone_positions,
        ai_team,
        score_diff,
        state.shot_index,
        make_mistake,
        &mut rng,
    );

    // Calculate the broom position that will result in the stone landing at target
    let broom_position = calculate_broom_for_target(target, curl, &existing_stone_positions);

    tracing::debug!(
        "AI target: ({:.2}, {:.2}) -> broom: ({:.2}, {:.2})",
        target.x,
        target.y,
        broom_position.x,
        broom_position.y
    );

    (broom_position, shot_type, curl)
}

/// Gathers stone positions from the ECS query.
fn gather_stone_positions(stones: &Query<(Entity, &Transform, &Stone)>) -> Vec<(Team, Vec2)> {
    stones
        .iter()
        .map(|(_, transform, stone)| {
            (
                stone.team,
                Vec2::new(transform.translation.x, transform.translation.y),
            )
        })
        .collect()
}

/// Calculates score differential from the AI team's perspective.
fn calculate_score_diff(ai_team: Team, state: &GameState) -> i32 {
    match ai_team {
        Team::One => state.team1_score as i32 - state.team2_score as i32,
        Team::Two => state.team2_score as i32 - state.team1_score as i32,
    }
}

// ============================================================================
// AI SHOT EXECUTION
// ============================================================================

/// Executes the AI's throw, spawning the stone and transitioning to StoneMoving.
fn execute_ai_throw(
    state: &mut ResMut<GameState>,
    commands: &mut Commands,
    assets: &StoneAssets,
    stones: &Query<(Entity, &Transform, &Stone)>,
    personalities: &PlayerPersonalities,
) {
    let team = state.current_team();
    let curl = state.curl_direction;

    // Get the current thrower's personality and apply variance
    let personality = personalities.current_thrower(state.shot_index, state.first_throw_team);
    let mut rng = rand::rng();

    // Apply personality variance to aim and weight
    let actual_angle = personality.apply_aim_variance(state.aim_angle_deg, &mut rng);
    let actual_weight = personality.apply_weight_variance(state.aim_weight, &mut rng);

    // Calculate throw velocity with personality-adjusted values
    let angle_rad = actual_angle.to_radians();
    let normalized_weight = (actual_weight - WEIGHT_MIN) / (WEIGHT_MAX - WEIGHT_MIN);
    let speed = WEIGHT_MIN_SPEED + normalized_weight * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);

    let velocity = Vec2::new(angle_rad.sin() * speed, angle_rad.cos() * speed);
    let start_pos = Vec2::new(0.0, DELIVERY_START_Y);

    // Create snapshot before throw
    state.snapshot = Some(snapshot_stones(stones, state.shot_index));

    // Spawn the stone
    let entity = spawn_stone(commands, assets, team, start_pos, velocity, true, curl);

    state.thrown_stone = Some(entity);
    state.phase = Phase::StoneMoving;
    state.still_time = 0.0;
    state.ai_think_timer = 0.0;

    tracing::info!(
        "AI {} threw stone: intended angle {:.1}°, actual {:.1}°, intended weight {:.1}, actual {:.1}, curl {:?} ({}, {})",
        personality.position.name(),
        state.aim_angle_deg,
        actual_angle,
        state.aim_weight,
        actual_weight,
        curl,
        personality.weight_skill.name(),
        personality.aim_skill.name()
    );
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get curl direction for a target position.
    /// Mimics the logic in calculate_ai_shot.
    fn curl_for_target_x(target_x: f32) -> CurlDirection {
        if target_x < 0.0 {
            CurlDirection::OutTurn
        } else {
            CurlDirection::InTurn
        }
    }

    #[test]
    fn ai_curl_toward_center_when_target_left() {
        // Target on left side of sheet (negative X)
        let curl = curl_for_target_x(-1.0);
        // OutTurn curls right (positive X direction), toward center
        assert_eq!(curl, CurlDirection::OutTurn);
        assert!(
            curl.angular_velocity() < 0.0,
            "OutTurn should have negative angular velocity (curls right)"
        );
    }

    #[test]
    fn ai_curl_toward_center_when_target_right() {
        // Target on right side of sheet (positive X)
        let curl = curl_for_target_x(1.0);
        // InTurn curls left (negative X direction), toward center
        assert_eq!(curl, CurlDirection::InTurn);
        assert!(
            curl.angular_velocity() > 0.0,
            "InTurn should have positive angular velocity (curls left)"
        );
    }

    #[test]
    fn ai_curl_toward_center_when_target_on_centerline() {
        // Target on center line - defaults to InTurn
        let curl = curl_for_target_x(0.0);
        assert_eq!(curl, CurlDirection::InTurn);
    }

    #[test]
    fn ai_curl_directions_consistent_with_physics() {
        // Verify that our curl choices actually curl toward center
        // by checking the angular velocity signs match expected behavior
        //
        // Physics: positive angular velocity = curls left (negative X)
        //          negative angular velocity = curls right (positive X)

        // Target left: we want to curl right (toward center at X=0)
        let left_curl = curl_for_target_x(-2.0);
        assert!(
            left_curl.angular_velocity() < 0.0,
            "Left target should use curl that goes right (negative angular vel)"
        );

        // Target right: we want to curl left (toward center at X=0)
        let right_curl = curl_for_target_x(2.0);
        assert!(
            right_curl.angular_velocity() > 0.0,
            "Right target should use curl that goes left (positive angular vel)"
        );
    }
}
