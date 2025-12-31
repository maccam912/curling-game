//! # Game Logic Systems
//!
//! Systems that enforce curling rules and manage game flow.
//!
//! This module contains systems for:
//! - Out of bounds detection
//! - Shot end detection (when stones stop moving)
//! - Shot resolution (applying hog line and FGZ rules)
//! - Score confirmation and end/game transitions

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use tracing::{debug, info, warn};

use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::resources::*;
use crate::rules::{check_far_hog_violation, check_near_hog_violation};

/// Checks if stones are out of bounds and removes them.
///
/// Stones that cross the side walls or back lines are immediately despawned.
pub fn check_out_of_bounds(
    mut commands: Commands,
    stones: Query<(Entity, &Transform), With<Stone>>,
) {
    for (entity, transform) in stones.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if is_out_of_bounds(pos) {
            commands.entity(entity).despawn();
            debug!(
                entity = ?entity,
                position = ?pos,
                "Stone removed: out of bounds"
            );
        }
    }
}

/// Detects when all stones have stopped moving.
///
/// After stones are still for `STOP_HOLD_SECS`, transitions to Resolve phase.
pub fn detect_shot_end(
    time: Res<Time<bevy::time::Fixed>>,
    velocities: Query<&Velocity, With<Stone>>,
    mut state: ResMut<GameState>,
) {
    if state.phase != Phase::StoneMoving {
        return;
    }

    let mut all_still = true;
    for velocity in velocities.iter() {
        if velocity.linvel.length() > STOP_SPEED {
            all_still = false;
            break;
        }
    }

    if all_still {
        state.still_time += time.delta_secs();
        if state.still_time > STOP_HOLD_SECS {
            state.phase = Phase::Resolve;
            debug!(
                still_time = state.still_time,
                "All stones stopped, entering Resolve phase"
            );
        }
    } else {
        state.still_time = 0.0;
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR RESOLVE_SHOT
// ============================================================================

/// Applies hog line rules to the thrown stone.
///
/// Returns `true` if the stone was removed due to a violation.
fn apply_hog_line_rules(
    commands: &mut Commands,
    thrown: Entity,
    throw_marker: &ThrowingStone,
) -> bool {
    if check_near_hog_violation(throw_marker.max_y) {
        commands.entity(thrown).despawn();
        warn!(
            max_y = throw_marker.max_y,
            hog_line = hog_line_near(),
            "Near hog line violation: stone removed (never released before hog)"
        );
        return true;
    }

    if check_far_hog_violation(throw_marker.max_y, throw_marker.hit_stone) {
        commands.entity(thrown).despawn();
        warn!(
            max_y = throw_marker.max_y,
            far_hog_line = hog_line_far() + STONE_RADIUS,
            "Far hog line violation: stone removed (didn't reach far hog line)"
        );
        return true;
    }

    false
}

/// Applies Free Guard Zone rules after a shot.
///
/// If guards in the FGZ were removed, restores them and removes the thrown stone.
/// Returns `true` if an FGZ violation occurred.
fn apply_fgz_rules(
    commands: &mut Commands,
    snapshot: &ShotSnapshot,
    stones: &Query<(Entity, &Transform, &Stone)>,
    thrown: Option<Entity>,
    assets: &StoneAssets,
) -> bool {
    if !snapshot.fgz_active {
        return false;
    }

    // Find guards that were removed
    let removed_guards: Vec<_> = snapshot
        .stones
        .iter()
        .filter(|snap| snap.in_fgz && stones.get(snap.entity).is_err())
        .cloned()
        .collect();

    if removed_guards.is_empty() {
        return false;
    }

    // FGZ violation: remove the thrown stone and restore guards
    if let Some(thrown_entity) = thrown {
        commands.entity(thrown_entity).despawn();
    }

    for guard in &removed_guards {
        spawn_restored_guard(commands, assets, guard.team, guard.position);
    }

    warn!(
        restored_count = removed_guards.len(),
        "Free Guard Zone violation: restored guard(s), removed thrown stone"
    );

    true
}

/// Calculates end scoring and identifies scoring stone entities.
///
/// Returns (team1_points, team2_points, scoring_entities).
fn calculate_end_score(
    stones: &Query<(Entity, &Transform, &Stone)>,
) -> (u32, u32, Vec<Entity>) {
    let tee = Vec2::new(0.0, tee_line_far());

    // Collect all stone positions for scoring
    let stone_positions: Vec<(Team, Vec2)> = stones
        .iter()
        .map(|(_, transform, stone)| {
            (
                stone.team,
                Vec2::new(transform.translation.x, transform.translation.y),
            )
        })
        .collect();

    let (team1_points, team2_points) = score_end(&stone_positions);

    // Determine scoring team
    let scoring_team = crate::rules::determine_scoring_team(team1_points, team2_points);

    // Find entities of scoring stones
    let scoring_entities = match scoring_team {
        Some(team) => {
            let points = if team == Team::One {
                team1_points
            } else {
                team2_points
            };
            find_scoring_entities(stones, team, points, tee)
        }
        None => Vec::new(),
    };

    (team1_points, team2_points, scoring_entities)
}

/// Finds the entities of the scoring stones for a team.
///
/// Returns entities of the closest N stones to the tee that are scoring.
fn find_scoring_entities(
    stones: &Query<(Entity, &Transform, &Stone)>,
    team: Team,
    points: u32,
    tee: Vec2,
) -> Vec<Entity> {
    let mut team_stones: Vec<(Entity, f32)> = stones
        .iter()
        .filter(|(_, _, s)| s.team == team)
        .map(|(e, t, _)| {
            let pos = Vec2::new(t.translation.x, t.translation.y);
            (e, pos.distance(tee))
        })
        .filter(|(_, dist)| *dist <= HOUSE_RADIUS_12 + STONE_RADIUS)
        .collect();

    team_stones.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    team_stones
        .iter()
        .take(points as usize)
        .map(|(e, _)| *e)
        .collect()
}

/// Resets game state for the next shot.
fn reset_for_next_shot(state: &mut GameState) {
    state.phase = Phase::CallingShot;
    state.shot_type = ShotType::Draw;
    state.broom_position = Vec2::new(0.0, TEE_FROM_CENTER);
    state.called_angle_deg = 0.0;
    state.called_weight = ShotType::Draw.default_weight();
    state.aim_angle_deg = state.called_angle_deg;
    state.aim_weight = state.called_weight;
}

// ============================================================================
// MAIN RESOLVE SYSTEM
// ============================================================================

/// Resolves the shot by applying game rules.
///
/// Rules enforced:
/// - **Near Hog Line**: Stone must cross the near hog line during delivery
/// - **Far Hog Line**: Stone must reach the far hog line (unless it hit another stone)
/// - **Free Guard Zone**: During first 5 shots, opponent's guards in FGZ cannot be removed
///
/// After resolution, advances to the next shot or ends the end.
pub fn resolve_shot(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    stones: Query<(Entity, &Transform, &Stone)>,
    thrown_query: Query<&ThrowingStone>,
    assets: Res<StoneAssets>,
) {
    if state.phase != Phase::Resolve {
        return;
    }

    info!(shot_index = state.shot_index + 1, "Resolving shot");

    // Apply hog line rules
    if let Some(thrown) = state.thrown_stone {
        if let Ok(throw_marker) = thrown_query.get(thrown) {
            apply_hog_line_rules(&mut commands, thrown, throw_marker);
        }
    }

    // Apply FGZ rules
    if let Some(ref snapshot) = state.snapshot {
        apply_fgz_rules(
            &mut commands,
            snapshot,
            &stones,
            state.thrown_stone,
            &assets,
        );
    }

    // Clean up throwing stone marker
    if let Some(thrown) = state.thrown_stone {
        commands.entity(thrown).remove::<ThrowingStone>();
    }
    state.thrown_stone = None;
    state.snapshot = None;
    state.still_time = 0.0;
    state.shot_index = state.shot_index.saturating_add(1);

    // Check if end is complete
    if state.shot_index >= TOTAL_SHOTS {
        let (team1_points, team2_points, scoring_entities) = calculate_end_score(&stones);

        // Mark scoring stones
        for entity in &scoring_entities {
            commands.entity(*entity).insert(ScoringStone);
        }

        state.pending_end_score = Some((team1_points, team2_points));
        state.scoring_entities = scoring_entities;
        state.phase = Phase::ShowingScore;

        info!(
            end = state.current_end,
            team1_points = team1_points,
            team2_points = team2_points,
            "End complete, showing score"
        );
    } else {
        reset_for_next_shot(&mut state);

        info!(
            next_shot = state.shot_index + 1,
            team = state.current_team().name(),
            "Ready for next shot"
        );
    }
}

/// Handles score confirmation and transitions to next end.
///
/// Called when user confirms the end score. Adds score to totals,
/// clears stones, and sets up for next end.
pub fn handle_score_confirmation(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    stones: Query<Entity, With<Stone>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<ConfirmScoreButton>)>,
) {
    if state.phase != Phase::ShowingScore {
        return;
    }

    // Check if confirm button was pressed
    let mut confirmed = false;
    for interaction in button_query.iter() {
        if *interaction == Interaction::Pressed {
            confirmed = true;
            break;
        }
    }

    if !confirmed {
        return;
    }

    // Add pending score to totals
    if let Some((team1_points, team2_points)) = state.pending_end_score.take() {
        // Record this end's score in history
        state.end_scores.push((team1_points, team2_points));

        state.team1_score += team1_points;
        state.team2_score += team2_points;

        // Determine who throws first next end (scoring team throws first = loses hammer)
        if team1_points > 0 {
            state.first_throw_team = Team::One;
            debug!("Team 1 scored, Team 2 gets hammer next end");
        } else if team2_points > 0 {
            state.first_throw_team = Team::Two;
            debug!("Team 2 scored, Team 1 gets hammer next end");
        } else {
            debug!(
                "Blank end, hammer stays with {:?}",
                state.first_throw_team.opponent()
            );
        }

        info!(
            team1_total = state.team1_score,
            team2_total = state.team2_score,
            "Score confirmed"
        );
    }

    // Clear scoring entities list
    state.scoring_entities.clear();

    // Check if game is over
    state.current_end += 1;
    if state.current_end > state.total_ends {
        // Game is over - keep stones on ice for the cool effect
        // Remove ScoringStone markers but don't despawn
        for entity in stones.iter() {
            commands.entity(entity).remove::<ScoringStone>();
        }

        state.phase = Phase::Ended;
        let winner = if state.team1_score > state.team2_score {
            "Team 1"
        } else if state.team2_score > state.team1_score {
            "Team 2"
        } else {
            "Tie"
        };
        info!(
            team1_final = state.team1_score,
            team2_final = state.team2_score,
            winner = winner,
            "Game complete!"
        );
    } else {
        // Clear all stones from the ice for next end
        for entity in stones.iter() {
            commands.entity(entity).despawn();
        }

        // Set up for next end
        state.reset_for_new_end();
        info!(
            next_end = state.current_end,
            first_throw = state.first_throw_team.name(),
            hammer = state.first_throw_team.opponent().name(),
            "Starting new end"
        );
    }
}
