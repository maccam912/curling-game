//! # Game Logic Systems
//!
//! Systems that enforce curling rules and manage game flow.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use tracing::{debug, info, warn};

use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::resources::*;

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
    time: Res<Time>,
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

    // Check hog line rules for thrown stone
    if let Some(thrown) = state.thrown_stone
        && let Ok(throw_marker) = thrown_query.get(thrown)
    {
        if !hog_line_reached(throw_marker.max_y) {
            commands.entity(thrown).despawn();
            warn!(
                max_y = throw_marker.max_y,
                hog_line = hog_line_near(),
                "Near hog line violation: stone removed (never released before hog)"
            );
        } else if !throw_marker.hit_stone && !far_hog_line_reached(throw_marker.max_y) {
            commands.entity(thrown).despawn();
            warn!(
                max_y = throw_marker.max_y,
                far_hog_line = hog_line_far() + STONE_RADIUS,
                "Far hog line violation: stone removed (didn't reach far hog line)"
            );
        }
    }

    // Check Free Guard Zone rule
    if let Some(snapshot) = state.snapshot.clone()
        && snapshot.fgz_active
    {
        let mut removed_guards = Vec::new();
        for snap in &snapshot.stones {
            if snap.in_fgz && stones.get(snap.entity).is_err() {
                removed_guards.push(snap.clone());
            }
        }

        if !removed_guards.is_empty() {
            // FGZ violation: remove the thrown stone and restore guards
            if let Some(thrown) = state.thrown_stone {
                commands.entity(thrown).despawn();
            }

            for guard in &removed_guards {
                spawn_restored_guard(&mut commands, &assets, guard.team, guard.position);
            }

            warn!(
                restored_count = removed_guards.len(),
                "Free Guard Zone violation: restored guard(s), removed thrown stone"
            );
        }
    }

    // Clean up throwing stone marker and advance to next shot
    if let Some(thrown) = state.thrown_stone {
        commands.entity(thrown).remove::<ThrowingStone>();
    }
    state.thrown_stone = None;
    state.snapshot = None;
    state.still_time = 0.0;
    state.shot_index = state.shot_index.saturating_add(1);

    if state.shot_index >= TOTAL_SHOTS {
        // End complete - calculate score
        let stone_positions: Vec<(Team, Vec2)> = stones
            .iter()
            .map(|(_, transform, stone)| {
                (
                    stone.team,
                    Vec2::new(transform.translation.x, transform.translation.y),
                )
            })
            .collect();

        let (red_points, blue_points) = score_end(&stone_positions);
        state.red_score += red_points;
        state.blue_score += blue_points;

        info!(
            end = state.current_end,
            red_points = red_points,
            blue_points = blue_points,
            red_total = state.red_score,
            blue_total = state.blue_score,
            "End scored"
        );

        // Determine who throws first next end (scoring team throws first = loses hammer)
        // If blank end (no score), hammer stays with same team
        if red_points > 0 {
            state.first_throw_team = Team::Red;
            debug!("Red scored, Blue gets hammer next end");
        } else if blue_points > 0 {
            state.first_throw_team = Team::Blue;
            debug!("Blue scored, Red gets hammer next end");
        } else {
            debug!(
                "Blank end, hammer stays with {:?}",
                state.first_throw_team.opponent()
            );
        }

        // Clear all stones from the ice
        for (entity, _, _) in stones.iter() {
            commands.entity(entity).despawn();
        }

        // Check if game is over
        state.current_end += 1;
        if state.current_end > state.total_ends {
            state.phase = Phase::Ended;
            let winner = if state.red_score > state.blue_score {
                "Red"
            } else if state.blue_score > state.red_score {
                "Blue"
            } else {
                "Tie"
            };
            info!(
                red_final = state.red_score,
                blue_final = state.blue_score,
                winner = winner,
                "Game complete!"
            );
        } else {
            // Set up for next end
            state.reset_for_new_end();
            info!(
                next_end = state.current_end,
                first_throw = state.first_throw_team.name(),
                hammer = state.first_throw_team.opponent().name(),
                "Starting new end"
            );
        }
    } else {
        state.phase = Phase::CallingShot;
        state.shot_type = ShotType::Draw;
        state.broom_position = Vec2::new(0.0, TEE_FROM_CENTER);
        state.called_angle_deg = 0.0;
        state.called_weight = ShotType::Draw.default_weight();
        state.aim_angle_deg = state.called_angle_deg;
        state.aim_weight = state.called_weight;

        info!(
            next_shot = state.shot_index + 1,
            team = state.current_team().name(),
            "Ready for next shot"
        );
    }
}
