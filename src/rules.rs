//! # Game Rules
//!
//! Pure functions for curling game rule enforcement.
//!
//! This module contains testable, pure functions for:
//! - Hog line violations (near and far)
//! - Free Guard Zone (FGZ) rule enforcement
//! - Scoring calculations
//! - Hammer (last rock advantage) determination
//!
//! These functions are extracted from the game logic systems to enable
//! comprehensive unit testing without Bevy ECS dependencies.

use bevy::prelude::*;

use crate::components::Team;
use crate::helpers::{far_hog_line_reached, hog_line_reached};
use crate::resources::StoneSnapshot;

// ============================================================================
// HOG LINE RULES
// ============================================================================

/// Checks if a stone violated the near hog line rule.
///
/// A stone must cross the near hog line during delivery. If the stone
/// never reaches the hog line (e.g., released too early), it's a violation.
///
/// # Arguments
/// * `max_y` - Maximum Y position reached by the stone during delivery
///
/// # Returns
/// `true` if the stone violated the rule (did NOT cross the near hog line)
///
/// # Example
/// ```ignore
/// let max_y = -10.5; // Stone stopped before hog line
/// assert!(check_near_hog_violation(max_y));
/// ```
#[inline]
pub fn check_near_hog_violation(max_y: f32) -> bool {
    !hog_line_reached(max_y)
}

/// Checks if a stone violated the far hog line rule.
///
/// A stone that doesn't hit any other stone must fully cross the far hog line.
/// If it stops before the far hog line without hitting anything, it's removed.
///
/// # Arguments
/// * `max_y` - Maximum Y position reached by the stone
/// * `hit_stone` - Whether the stone hit another stone during its path
///
/// # Returns
/// `true` if the stone violated the rule (didn't reach far hog and didn't hit)
///
/// # Example
/// ```ignore
/// // Stone stopped short without hitting anything
/// assert!(check_far_hog_violation(10.0, false));
///
/// // Stone stopped short but hit another stone - allowed
/// assert!(!check_far_hog_violation(10.0, true));
/// ```
#[inline]
pub fn check_far_hog_violation(max_y: f32, hit_stone: bool) -> bool {
    !hit_stone && !far_hog_line_reached(max_y)
}

// ============================================================================
// FREE GUARD ZONE RULES
// ============================================================================

/// Detects Free Guard Zone violations by comparing snapshots.
///
/// During the first 5 shots, guards in the FGZ (between hog line and house)
/// cannot be removed by the opponent. This function identifies which guards
/// were removed by comparing the pre-shot snapshot with current stone entities.
///
/// # Arguments
/// * `snapshot` - Pre-shot snapshot containing stone positions and FGZ status
/// * `current_entities` - Entities of stones currently on the ice
///
/// # Returns
/// Vector of `StoneSnapshot` for guards that were in the FGZ but are now missing
///
/// # Example
/// ```ignore
/// let removed_guards = detect_fgz_violations(&snapshot, &current_stone_entities);
/// if !removed_guards.is_empty() {
///     // FGZ violation occurred - restore guards, remove thrown stone
/// }
/// ```
pub fn detect_fgz_violations(
    snapshot: &crate::resources::ShotSnapshot,
    current_entities: &[Entity],
) -> Vec<StoneSnapshot> {
    if !snapshot.fgz_active {
        return Vec::new();
    }

    snapshot
        .stones
        .iter()
        .filter(|snap| {
            // Stone was in FGZ before the shot
            snap.in_fgz
                // Stone is no longer present (was removed)
                && !current_entities.contains(&snap.entity)
        })
        .cloned()
        .collect()
}

/// Checks if FGZ rule is active for the current shot.
///
/// The Free Guard Zone rule only applies during the first 5 shots of an end.
///
/// # Arguments
/// * `shot_index` - Current shot index (0-15)
///
/// # Returns
/// `true` if FGZ rule is active (shot_index < 5)
#[inline]
pub fn is_fgz_active(shot_index: u8) -> bool {
    shot_index < 5
}

// ============================================================================
// SCORING RULES
// ============================================================================

/// Determines which team scored based on end-of-end points.
///
/// In curling, only one team can score per end. The team with points > 0
/// is the scoring team. If both teams have 0 points, it's a blank end.
///
/// # Arguments
/// * `team1_points` - Points scored by Team 1
/// * `team2_points` - Points scored by Team 2
///
/// # Returns
/// `Some(Team)` if a team scored, `None` for a blank end
///
/// # Panics
/// Debug builds will panic if both teams have points > 0 (invalid state)
///
/// # Example
/// ```ignore
/// assert_eq!(determine_scoring_team(2, 0), Some(Team::One));
/// assert_eq!(determine_scoring_team(0, 3), Some(Team::Two));
/// assert_eq!(determine_scoring_team(0, 0), None); // Blank end
/// ```
pub fn determine_scoring_team(team1_points: u32, team2_points: u32) -> Option<Team> {
    debug_assert!(
        team1_points == 0 || team2_points == 0,
        "Both teams cannot score in the same end: T1={}, T2={}",
        team1_points,
        team2_points
    );

    match (team1_points > 0, team2_points > 0) {
        (true, false) => Some(Team::One),
        (false, true) => Some(Team::Two),
        _ => None, // Blank end (or invalid state in release builds)
    }
}

/// Determines which team has hammer (last rock) for the next end.
///
/// In curling, the team that scores loses hammer for the next end
/// (they throw first, opponent throws last). If it's a blank end,
/// hammer stays with the current holder.
///
/// # Arguments
/// * `team1_points` - Points scored by Team 1 this end
/// * `team2_points` - Points scored by Team 2 this end
/// * `current_first_throw` - Team that threw first this end
///
/// # Returns
/// The team that should throw first next end (opponent has hammer)
///
/// # Example
/// ```ignore
/// // Team 1 scored, so they throw first next end (lose hammer)
/// let next_first = determine_next_first_throw(2, 0, Team::Two);
/// assert_eq!(next_first, Team::One);
///
/// // Blank end - first throw team stays the same
/// let next_first = determine_next_first_throw(0, 0, Team::One);
/// assert_eq!(next_first, Team::One);
/// ```
pub fn determine_next_first_throw(
    team1_points: u32,
    team2_points: u32,
    current_first_throw: Team,
) -> Team {
    if team1_points > 0 {
        Team::One // Team 1 scored, they throw first next (lose hammer)
    } else if team2_points > 0 {
        Team::Two // Team 2 scored, they throw first next (lose hammer)
    } else {
        current_first_throw // Blank end, no change
    }
}

/// Checks if the game has ended.
///
/// # Arguments
/// * `current_end` - The current end number (1-indexed)
/// * `total_ends` - Total number of ends in the game
///
/// # Returns
/// `true` if the game has ended (current_end > total_ends)
#[inline]
pub fn is_game_over(current_end: u8, total_ends: u8) -> bool {
    current_end > total_ends
}

/// Determines the winner of the game.
///
/// # Arguments
/// * `team1_score` - Total score for Team 1
/// * `team2_score` - Total score for Team 2
///
/// # Returns
/// `Some(Team::One)` if Team 1 wins, `Some(Team::Two)` if Team 2 wins,
/// `None` for a tie
pub fn determine_winner(team1_score: u32, team2_score: u32) -> Option<Team> {
    match team1_score.cmp(&team2_score) {
        std::cmp::Ordering::Greater => Some(Team::One),
        std::cmp::Ordering::Less => Some(Team::Two),
        std::cmp::Ordering::Equal => None,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{hog_line_far, hog_line_near};
    use crate::resources::ShotSnapshot;

    // ============ HOG LINE TESTS ============

    #[test]
    fn near_hog_violation_when_before_line() {
        // Stone stopped before reaching near hog line
        let max_y = hog_line_near() - 0.1;
        assert!(check_near_hog_violation(max_y));
    }

    #[test]
    fn near_hog_no_violation_when_past_line() {
        // Stone crossed near hog line
        let max_y = hog_line_near() + 0.1;
        assert!(!check_near_hog_violation(max_y));
    }

    #[test]
    fn near_hog_no_violation_at_exact_line() {
        // Stone exactly at near hog line
        let max_y = hog_line_near();
        assert!(!check_near_hog_violation(max_y));
    }

    #[test]
    fn far_hog_violation_when_short_no_hit() {
        // Stone stopped short without hitting anything
        let max_y = hog_line_far();
        assert!(check_far_hog_violation(max_y, false));
    }

    #[test]
    fn far_hog_no_violation_when_hit_stone() {
        // Stone stopped short but hit another stone - allowed
        let max_y = hog_line_far();
        assert!(!check_far_hog_violation(max_y, true));
    }

    #[test]
    fn far_hog_no_violation_when_past_line() {
        // Stone crossed far hog line
        let max_y = hog_line_far() + 1.0;
        assert!(!check_far_hog_violation(max_y, false));
    }

    // ============ FGZ TESTS ============

    #[test]
    fn fgz_active_for_first_five_shots() {
        assert!(is_fgz_active(0));
        assert!(is_fgz_active(4));
        assert!(!is_fgz_active(5));
        assert!(!is_fgz_active(15));
    }

    #[test]
    fn fgz_violation_detects_removed_guard() {
        let guard_entity = Entity::from_bits(42);
        let other_entity = Entity::from_bits(43);

        let snapshot = ShotSnapshot {
            stones: vec![StoneSnapshot {
                entity: guard_entity,
                team: Team::One,
                position: bevy::math::Vec2::new(0.0, 10.0),
                in_fgz: true,
            }],
            fgz_active: true,
        };

        // Guard is no longer in current entities
        let current = vec![other_entity];
        let violations = detect_fgz_violations(&snapshot, &current);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].entity, guard_entity);
    }

    #[test]
    fn fgz_no_violation_when_guard_still_present() {
        let guard_entity = Entity::from_bits(42);

        let snapshot = ShotSnapshot {
            stones: vec![StoneSnapshot {
                entity: guard_entity,
                team: Team::One,
                position: bevy::math::Vec2::new(0.0, 10.0),
                in_fgz: true,
            }],
            fgz_active: true,
        };

        // Guard is still present
        let current = vec![guard_entity];
        let violations = detect_fgz_violations(&snapshot, &current);

        assert!(violations.is_empty());
    }

    #[test]
    fn fgz_no_violation_when_not_active() {
        let guard_entity = Entity::from_bits(42);

        let snapshot = ShotSnapshot {
            stones: vec![StoneSnapshot {
                entity: guard_entity,
                team: Team::One,
                position: bevy::math::Vec2::new(0.0, 10.0),
                in_fgz: true,
            }],
            fgz_active: false, // FGZ not active (shot 5+)
        };

        // Guard was removed, but FGZ is not active
        let current = vec![];
        let violations = detect_fgz_violations(&snapshot, &current);

        assert!(violations.is_empty());
    }

    #[test]
    fn fgz_no_violation_when_stone_not_in_fgz() {
        let stone_entity = Entity::from_bits(42);

        let snapshot = ShotSnapshot {
            stones: vec![StoneSnapshot {
                entity: stone_entity,
                team: Team::One,
                position: bevy::math::Vec2::new(0.0, 20.0),
                in_fgz: false, // Stone was in house, not FGZ
            }],
            fgz_active: true,
        };

        // Stone was removed, but it wasn't in FGZ
        let current = vec![];
        let violations = detect_fgz_violations(&snapshot, &current);

        assert!(violations.is_empty());
    }

    // ============ SCORING TESTS ============

    #[test]
    fn scoring_team_team1_scores() {
        assert_eq!(determine_scoring_team(2, 0), Some(Team::One));
        assert_eq!(determine_scoring_team(8, 0), Some(Team::One));
    }

    #[test]
    fn scoring_team_team2_scores() {
        assert_eq!(determine_scoring_team(0, 3), Some(Team::Two));
        assert_eq!(determine_scoring_team(0, 1), Some(Team::Two));
    }

    #[test]
    fn scoring_team_blank_end() {
        assert_eq!(determine_scoring_team(0, 0), None);
    }

    // ============ HAMMER TESTS ============

    #[test]
    fn next_first_throw_team1_scores() {
        // Team 1 scored, they throw first next (lose hammer)
        let next = determine_next_first_throw(2, 0, Team::Two);
        assert_eq!(next, Team::One);
    }

    #[test]
    fn next_first_throw_team2_scores() {
        // Team 2 scored, they throw first next (lose hammer)
        let next = determine_next_first_throw(0, 3, Team::One);
        assert_eq!(next, Team::Two);
    }

    #[test]
    fn next_first_throw_blank_end_no_change() {
        // Blank end - first throw stays the same
        let next = determine_next_first_throw(0, 0, Team::One);
        assert_eq!(next, Team::One);

        let next = determine_next_first_throw(0, 0, Team::Two);
        assert_eq!(next, Team::Two);
    }

    // ============ GAME OVER TESTS ============

    #[test]
    fn game_over_after_total_ends() {
        assert!(!is_game_over(1, 8));
        assert!(!is_game_over(8, 8));
        assert!(is_game_over(9, 8));
    }

    #[test]
    fn determine_winner_team1_wins() {
        assert_eq!(determine_winner(5, 3), Some(Team::One));
    }

    #[test]
    fn determine_winner_team2_wins() {
        assert_eq!(determine_winner(3, 7), Some(Team::Two));
    }

    #[test]
    fn determine_winner_tie() {
        assert_eq!(determine_winner(4, 4), None);
    }
}
