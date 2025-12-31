//! # AI Strategy
//!
//! Pure functions for AI decision-making in curling.
//!
//! This module contains testable, pure functions for:
//! - Board state analysis (finding closest stones, evaluating positions)
//! - Shot type selection (draw, guard, takeout, freeze)
//! - Target position calculation
//! - Curl direction selection
//!
//! These functions are extracted from the AI system to enable comprehensive
//! unit testing without Bevy ECS or timing dependencies.

use bevy::math::Vec2;
use rand::Rng;

use crate::components::{CurlDirection, ShotType, Team};
use crate::constants::*;
use crate::helpers::{hog_line_far, tee_line_far};

// ============================================================================
// BOARD ANALYSIS
// ============================================================================

/// Finds the closest stone to the tee (button) for a given team.
///
/// Only considers stones that are "biting" the house (within 12-foot ring
/// plus stone radius).
///
/// # Arguments
/// * `stones` - Slice of (team, position) pairs for all stones on ice
/// * `team` - The team to find the closest stone for
///
/// # Returns
/// `Some((position, distance))` for the closest stone, or `None` if no stones
/// of that team are in/biting the house
///
/// # Example
/// ```ignore
/// let stones = vec![(Team::One, Vec2::new(0.0, 17.375))];
/// let closest = find_closest_to_tee(&stones, Team::One);
/// assert!(closest.is_some());
/// ```
pub fn find_closest_to_tee(stones: &[(Team, Vec2)], team: Team) -> Option<(Vec2, f32)> {
    let tee = Vec2::new(0.0, tee_line_far());

    stones
        .iter()
        .filter(|(t, _)| *t == team)
        .map(|(_, pos)| (*pos, pos.distance(tee)))
        .filter(|(_, dist)| *dist <= HOUSE_RADIUS_12 + STONE_RADIUS)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}

/// Analyzes the current board state for AI decision making.
///
/// # Arguments
/// * `stones` - All stones on the ice
/// * `ai_team` - The team the AI is playing for
///
/// # Returns
/// A `BoardAnalysis` struct with closest stone info for each team
pub fn analyze_board(stones: &[(Team, Vec2)], ai_team: Team) -> BoardAnalysis {
    let our_closest = find_closest_to_tee(stones, ai_team);
    let their_closest = find_closest_to_tee(stones, ai_team.opponent());

    // Determine who has "shot" (closest stone to button)
    let we_have_shot = match (&our_closest, &their_closest) {
        (Some((_, our_dist)), Some((_, their_dist))) => our_dist < their_dist,
        (Some(_), None) => true,
        _ => false,
    };

    BoardAnalysis {
        our_closest,
        their_closest,
        we_have_shot,
        house_empty: our_closest.is_none() && their_closest.is_none(),
    }
}

/// Result of board analysis for AI decision making.
#[derive(Debug, Clone)]
pub struct BoardAnalysis {
    /// Our team's closest stone to the tee (position, distance)
    pub our_closest: Option<(Vec2, f32)>,
    /// Opponent's closest stone to the tee (position, distance)
    pub their_closest: Option<(Vec2, f32)>,
    /// Whether we currently have "shot" (closest to button)
    pub we_have_shot: bool,
    /// Whether the house is completely empty
    pub house_empty: bool,
}

// ============================================================================
// SHOT SELECTION
// ============================================================================

/// Selects the optimal shot type based on game situation.
///
/// This is a pure function that encapsulates the AI's strategic decision-making.
///
/// # Arguments
/// * `analysis` - Board analysis from `analyze_board`
/// * `score_diff` - Our score minus opponent's score (positive = ahead)
/// * `shot_index` - Current shot index (0-15)
/// * `make_mistake` - If true, intentionally make a suboptimal choice
/// * `rng` - Random number generator for variance
///
/// # Returns
/// The selected `ShotType`
pub fn select_shot_type(
    analysis: &BoardAnalysis,
    score_diff: i32,
    shot_index: u8,
    make_mistake: bool,
    rng: &mut impl Rng,
) -> ShotType {
    let early_game = shot_index < 5; // FGZ active

    // Empty house - draw to button
    if analysis.house_empty {
        return ShotType::Draw;
    }

    // They have shot, we don't
    if !analysis.we_have_shot {
        if let Some((_, their_dist)) = analysis.their_closest {
            // During FGZ period, if their stone is a guard (outside house), don't takeout
            if early_game && their_dist > HOUSE_RADIUS_12 && !make_mistake {
                return ShotType::Guard;
            }
            // Otherwise, try to take them out
            return ShotType::Takeout;
        }
    }

    // We have shot - protect or add more
    if analysis.we_have_shot {
        if make_mistake {
            // Mistake: risky draw instead of protecting
            return ShotType::Draw;
        }

        if score_diff >= 0 {
            // Ahead or tied - play conservatively
            if rng.random::<bool>() {
                return ShotType::Guard;
            } else {
                return ShotType::Freeze;
            }
        } else {
            // Behind - be aggressive, draw to button
            return ShotType::Draw;
        }
    }

    // Default fallback
    ShotType::Draw
}

/// Calculates the target position for a given shot type and board state.
///
/// # Arguments
/// * `shot_type` - The type of shot to make
/// * `analysis` - Board analysis
/// * `rng` - Random number generator for variance
///
/// # Returns
/// The target position where the AI wants the stone to end up
pub fn calculate_target_position(
    shot_type: ShotType,
    analysis: &BoardAnalysis,
    rng: &mut impl Rng,
) -> Vec2 {
    let tee = Vec2::new(0.0, tee_line_far());

    match shot_type {
        ShotType::Draw => {
            // Draw to button with slight variance
            tee + Vec2::new(rng.random_range(-0.3..0.3), rng.random_range(-0.2..0.2))
        }

        ShotType::Guard => {
            // Place guard in FGZ area (between hog line and house)
            let guard_y = (hog_line_far() + tee_line_far() - HOUSE_RADIUS_12) / 2.0;
            let guard_x = if let Some((our_pos, _)) = analysis.our_closest {
                // Guard in front of our stone
                our_pos.x + rng.random_range(-0.3..0.3)
            } else {
                // Center guard
                rng.random_range(-1.0..1.0)
            };
            Vec2::new(guard_x, guard_y)
        }

        ShotType::Takeout => {
            // Aim at opponent's closest stone
            if let Some((their_pos, _)) = analysis.their_closest {
                their_pos
            } else {
                // Fallback to tee if no opponent stone
                tee
            }
        }

        ShotType::Freeze => {
            // Freeze behind our closest stone
            if let Some((our_pos, _)) = analysis.our_closest {
                Vec2::new(
                    our_pos.x + rng.random_range(-0.2..0.2),
                    our_pos.y + STONE_RADIUS * 2.0 + rng.random_range(0.0..0.3),
                )
            } else {
                // Fallback to button
                tee
            }
        }

        ShotType::HitAndRoll => {
            // Similar to takeout, aim at opponent
            if let Some((their_pos, _)) = analysis.their_closest {
                their_pos
            } else {
                tee
            }
        }
    }
}

/// Clamps a target position to the valid playing area.
///
/// Ensures the target is within sheet width bounds and between
/// the far hog line and back line.
///
/// # Arguments
/// * `target` - The unclamped target position
///
/// # Returns
/// The clamped target position
pub fn clamp_target_to_playing_area(target: Vec2) -> Vec2 {
    let half_width = SHEET_WIDTH * 0.5 - STONE_RADIUS;
    let back_y = tee_line_far() + BACK_FROM_TEE - STONE_RADIUS;

    Vec2::new(
        target.x.clamp(-half_width, half_width),
        target.y.clamp(hog_line_far(), back_y),
    )
}

// ============================================================================
// CURL DIRECTION
// ============================================================================

/// Selects the curl direction that curves toward the center line.
///
/// To maximize accuracy, stones should curl toward the center of the sheet.
/// - Target on left (negative X): use OutTurn (curls right toward center)
/// - Target on right (positive X): use InTurn (curls left toward center)
///
/// # Arguments
/// * `target_x` - X coordinate of the target position
///
/// # Returns
/// The curl direction that will curve toward center
pub fn select_curl_toward_center(target_x: f32) -> CurlDirection {
    if target_x < 0.0 {
        CurlDirection::OutTurn // Curls right (toward center from left side)
    } else {
        CurlDirection::InTurn // Curls left (toward center from right side)
    }
}

// ============================================================================
// COMPLETE AI DECISION
// ============================================================================

/// Makes a complete AI shot decision given the current game state.
///
/// This is the main entry point for AI decision-making, combining:
/// 1. Board analysis
/// 2. Shot type selection
/// 3. Target calculation
/// 4. Curl direction
///
/// # Arguments
/// * `stones` - All stones currently on the ice
/// * `ai_team` - The team the AI is playing for
/// * `score_diff` - Our score minus opponent's score
/// * `shot_index` - Current shot index (0-15)
/// * `make_mistake` - Whether to intentionally make a suboptimal choice
/// * `rng` - Random number generator
///
/// # Returns
/// Tuple of (target_position, shot_type, curl_direction)
pub fn decide_shot(
    stones: &[(Team, Vec2)],
    ai_team: Team,
    score_diff: i32,
    shot_index: u8,
    make_mistake: bool,
    rng: &mut impl Rng,
) -> (Vec2, ShotType, CurlDirection) {
    // Analyze the board
    let analysis = analyze_board(stones, ai_team);

    // Select shot type
    let shot_type = select_shot_type(&analysis, score_diff, shot_index, make_mistake, rng);

    // Calculate target position
    let raw_target = calculate_target_position(shot_type, &analysis, rng);
    let target = clamp_target_to_playing_area(raw_target);

    // Select curl direction
    let curl = select_curl_toward_center(target.x);

    (target, shot_type, curl)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn tee() -> Vec2 {
        Vec2::new(0.0, tee_line_far())
    }

    fn make_rng() -> impl Rng {
        rand::rngs::StdRng::seed_from_u64(12345)
    }

    // ============ FIND CLOSEST TESTS ============

    #[test]
    fn find_closest_empty_house() {
        let stones: Vec<(Team, Vec2)> = vec![];
        assert!(find_closest_to_tee(&stones, Team::One).is_none());
    }

    #[test]
    fn find_closest_single_stone() {
        let stones = vec![(Team::One, tee())];
        let closest = find_closest_to_tee(&stones, Team::One);
        assert!(closest.is_some());
        let (pos, dist) = closest.unwrap();
        assert_eq!(pos, tee());
        assert!(dist < 0.01);
    }

    #[test]
    fn find_closest_ignores_other_team() {
        let stones = vec![(Team::Two, tee())];
        assert!(find_closest_to_tee(&stones, Team::One).is_none());
    }

    #[test]
    fn find_closest_selects_nearest() {
        let stones = vec![
            (Team::One, tee() + Vec2::new(0.5, 0.0)), // farther
            (Team::One, tee() + Vec2::new(0.2, 0.0)), // closer
        ];
        let (pos, _) = find_closest_to_tee(&stones, Team::One).unwrap();
        assert!((pos.x - 0.2).abs() < 0.01);
    }

    #[test]
    fn find_closest_ignores_outside_house() {
        let outside = tee() + Vec2::new(HOUSE_RADIUS_12 + STONE_RADIUS + 0.5, 0.0);
        let stones = vec![(Team::One, outside)];
        assert!(find_closest_to_tee(&stones, Team::One).is_none());
    }

    // ============ BOARD ANALYSIS TESTS ============

    #[test]
    fn analyze_empty_house() {
        let stones: Vec<(Team, Vec2)> = vec![];
        let analysis = analyze_board(&stones, Team::One);
        assert!(analysis.house_empty);
        assert!(!analysis.we_have_shot);
    }

    #[test]
    fn analyze_we_have_shot() {
        let stones = vec![
            (Team::One, tee() + Vec2::new(0.1, 0.0)), // closer
            (Team::Two, tee() + Vec2::new(0.5, 0.0)), // farther
        ];
        let analysis = analyze_board(&stones, Team::One);
        assert!(analysis.we_have_shot);
        assert!(!analysis.house_empty);
    }

    #[test]
    fn analyze_they_have_shot() {
        let stones = vec![
            (Team::One, tee() + Vec2::new(0.5, 0.0)), // farther
            (Team::Two, tee() + Vec2::new(0.1, 0.0)), // closer
        ];
        let analysis = analyze_board(&stones, Team::One);
        assert!(!analysis.we_have_shot);
    }

    // ============ SHOT SELECTION TESTS ============

    #[test]
    fn empty_house_draws_to_button() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: None,
            their_closest: None,
            we_have_shot: false,
            house_empty: true,
        };
        let shot = select_shot_type(&analysis, 0, 0, false, &mut rng);
        assert_eq!(shot, ShotType::Draw);
    }

    #[test]
    fn opponent_shot_triggers_takeout() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: Some((tee() + Vec2::new(0.5, 0.0), 0.5)),
            their_closest: Some((tee() + Vec2::new(0.1, 0.0), 0.1)), // they're closer
            we_have_shot: false,
            house_empty: false,
        };
        // After FGZ period (shot 5+), should takeout
        let shot = select_shot_type(&analysis, 0, 10, false, &mut rng);
        assert_eq!(shot, ShotType::Takeout);
    }

    #[test]
    fn early_game_prefers_guards_over_takeout() {
        let mut rng = make_rng();
        // Opponent has a guard (outside house but in FGZ)
        let guard_pos = tee() + Vec2::new(0.0, -HOUSE_RADIUS_12 - 0.5);
        let analysis = BoardAnalysis {
            our_closest: None,
            their_closest: Some((guard_pos, HOUSE_RADIUS_12 + 0.5)),
            we_have_shot: false,
            house_empty: false,
        };
        // During FGZ period (shot < 5), should guard instead of takeout
        let shot = select_shot_type(&analysis, 0, 2, false, &mut rng);
        assert_eq!(shot, ShotType::Guard);
    }

    #[test]
    fn behind_plays_aggressively() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: Some((tee() + Vec2::new(0.1, 0.0), 0.1)),
            their_closest: Some((tee() + Vec2::new(0.5, 0.0), 0.5)),
            we_have_shot: true,
            house_empty: false,
        };
        // Behind by points - should draw aggressively
        let shot = select_shot_type(&analysis, -3, 10, false, &mut rng);
        assert_eq!(shot, ShotType::Draw);
    }

    #[test]
    fn ahead_plays_conservatively() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: Some((tee() + Vec2::new(0.1, 0.0), 0.1)),
            their_closest: Some((tee() + Vec2::new(0.5, 0.0), 0.5)),
            we_have_shot: true,
            house_empty: false,
        };
        // Ahead by points - should play conservatively (guard or freeze)
        let shot = select_shot_type(&analysis, 3, 10, false, &mut rng);
        assert!(shot == ShotType::Guard || shot == ShotType::Freeze);
    }

    // ============ TARGET POSITION TESTS ============

    #[test]
    fn draw_target_near_button() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: None,
            their_closest: None,
            we_have_shot: false,
            house_empty: true,
        };
        let target = calculate_target_position(ShotType::Draw, &analysis, &mut rng);
        let dist_from_tee = target.distance(tee());
        assert!(dist_from_tee < 0.5, "Draw should target near button");
    }

    #[test]
    fn takeout_targets_opponent() {
        let mut rng = make_rng();
        let their_pos = tee() + Vec2::new(0.3, 0.2);
        let analysis = BoardAnalysis {
            our_closest: None,
            their_closest: Some((their_pos, 0.36)),
            we_have_shot: false,
            house_empty: false,
        };
        let target = calculate_target_position(ShotType::Takeout, &analysis, &mut rng);
        assert_eq!(target, their_pos, "Takeout should aim at opponent stone");
    }

    #[test]
    fn guard_target_in_fgz_area() {
        let mut rng = make_rng();
        let analysis = BoardAnalysis {
            our_closest: Some((tee(), 0.0)),
            their_closest: None,
            we_have_shot: true,
            house_empty: false,
        };
        let target = calculate_target_position(ShotType::Guard, &analysis, &mut rng);
        // Guard should be between hog line and house
        assert!(
            target.y > hog_line_far() && target.y < tee_line_far() - HOUSE_RADIUS_12,
            "Guard should be in FGZ area"
        );
    }

    #[test]
    fn target_clamped_to_valid_area() {
        let extreme = Vec2::new(100.0, 100.0);
        let clamped = clamp_target_to_playing_area(extreme);
        let half_width = SHEET_WIDTH * 0.5 - STONE_RADIUS;
        assert!(clamped.x <= half_width);
        assert!(clamped.y <= tee_line_far() + BACK_FROM_TEE - STONE_RADIUS);
    }

    // ============ CURL DIRECTION TESTS ============

    #[test]
    fn left_target_uses_outturn() {
        let curl = select_curl_toward_center(-1.0);
        assert_eq!(curl, CurlDirection::OutTurn);
    }

    #[test]
    fn right_target_uses_inturn() {
        let curl = select_curl_toward_center(1.0);
        assert_eq!(curl, CurlDirection::InTurn);
    }

    #[test]
    fn center_target_uses_inturn() {
        // When on center, defaults to InTurn
        let curl = select_curl_toward_center(0.0);
        assert_eq!(curl, CurlDirection::InTurn);
    }

    #[test]
    fn curl_directions_curl_toward_center() {
        // OutTurn has negative angular velocity (curls right)
        assert!(
            CurlDirection::OutTurn.angular_velocity() < 0.0,
            "OutTurn should curl right (toward center from left)"
        );
        // InTurn has positive angular velocity (curls left)
        assert!(
            CurlDirection::InTurn.angular_velocity() > 0.0,
            "InTurn should curl left (toward center from right)"
        );
    }

    // ============ COMPLETE DECISION TESTS ============

    #[test]
    fn decide_shot_returns_valid_decision() {
        let mut rng = make_rng();
        let stones: Vec<(Team, Vec2)> = vec![];
        let (target, shot_type, curl) = decide_shot(&stones, Team::One, 0, 0, false, &mut rng);

        // Should draw on empty house
        assert_eq!(shot_type, ShotType::Draw);

        // Target should be clamped
        let half_width = SHEET_WIDTH * 0.5 - STONE_RADIUS;
        assert!(target.x >= -half_width && target.x <= half_width);

        // Curl should be valid
        assert!(curl == CurlDirection::InTurn || curl == CurlDirection::OutTurn);
    }

    #[test]
    fn decide_shot_with_existing_stones() {
        let mut rng = make_rng();
        let stones = vec![
            (Team::Two, tee() + Vec2::new(0.1, 0.0)), // Opponent on button
        ];
        let (target, shot_type, _) = decide_shot(&stones, Team::One, 0, 10, false, &mut rng);

        // Should takeout opponent (after FGZ period)
        assert_eq!(shot_type, ShotType::Takeout);

        // Target should be at opponent's position
        let expected_target = tee() + Vec2::new(0.1, 0.0);
        assert!(target.distance(expected_target) < 0.01);
    }
}
