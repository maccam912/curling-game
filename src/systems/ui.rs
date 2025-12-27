//! # UI Systems
//!
//! Systems that update the user interface elements.

use bevy::prelude::*;
use tracing::trace;

use crate::components::*;
use crate::constants::*;
use crate::resources::*;

/// Updates the window title to reflect current game state.
///
/// Shows end number, scores, shot number, team, and current phase.
pub fn update_window_title(mut windows: Query<&mut Window>, state: Res<GameState>) {
    if !state.is_changed() {
        return;
    }

    let phase_label = match state.phase {
        Phase::CallingShot => "Calling shot (Enter)",
        Phase::Aiming => "Aiming (Space to throw)",
        Phase::StoneMoving => "Stones moving",
        Phase::Resolve => "Resolving shot",
        Phase::Ended => "Game Over",
    };

    let title = if state.phase == Phase::Ended {
        format!(
            "Curling - FINAL: Team 1 {} - Team 2 {}",
            state.team1_score, state.team2_score
        )
    } else {
        format!(
            "Curling - End {}/{} | Team 1 {} - Team 2 {} | Shot {}/{} | {} | {}",
            state.current_end,
            state.total_ends,
            state.team1_score,
            state.team2_score,
            state.shot_index + 1,
            TOTAL_SHOTS,
            state.current_team().name(),
            phase_label
        )
    };

    for mut window in windows.iter_mut() {
        window.title = title.clone();
    }

    trace!("Window title updated");
}

/// Updates the broom visual to match the current broom position.
pub fn update_broom_visual(
    state: Res<GameState>,
    mut broom_query: Query<&mut Transform, With<Broom>>,
) {
    for mut transform in broom_query.iter_mut() {
        transform.translation.x = state.broom_position.x;
        transform.translation.y = state.broom_position.y;
        transform.translation.z = 0.05;
    }
}

/// Updates all UI elements based on current game state.
///
/// Updates:
/// - HUD panels (scores, end info, hammer, shot counter, team turn, phase)
/// - Legacy status text (for window title compatibility)
/// - Confirm button text (changes based on phase)
/// - Curl button highlighting (shows selected direction)
pub fn update_ui(
    state: Res<GameState>,
    mut status_query: Query<&mut Text, With<StatusText>>,
    confirm_query: Query<&Children, With<ConfirmButton>>,
    mut text_query: Query<&mut Text, Without<StatusText>>,
    mut curl_buttons: Query<(&CurlButton, &mut BackgroundColor), Without<ConfirmButton>>,
    // HUD element queries
    mut team1_score_query: Query<
        &mut Text,
        (
            With<Team1ScoreText>,
            Without<StatusText>,
            Without<Team2ScoreText>,
        ),
    >,
    mut team2_score_query: Query<
        &mut Text,
        (
            With<Team2ScoreText>,
            Without<StatusText>,
            Without<Team1ScoreText>,
        ),
    >,
    mut end_info_query: Query<
        &mut Text,
        (
            With<EndInfoText>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
        ),
    >,
    mut shot_info_query: Query<
        &mut Text,
        (
            With<ShotInfoText>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
        ),
    >,
    mut shots_remaining_query: Query<
        &mut Text,
        (
            With<ShotsRemainingText>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
        ),
    >,
    mut team_turn_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<TeamTurnIndicator>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
        ),
    >,
    mut phase_query: Query<
        &mut Text,
        (
            With<PhaseIndicator>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
        ),
    >,
    hammer_query: Query<&Children, With<HammerIndicator>>,
    mut hammer_text_query: Query<
        &mut Text,
        (
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
        ),
    >,
) {
    // --- Update HUD Elements ---

    // Update Team 1 score
    for mut text in team1_score_query.iter_mut() {
        **text = state.team1_score.to_string();
    }

    // Update Team 2 score
    for mut text in team2_score_query.iter_mut() {
        **text = state.team2_score.to_string();
    }

    // Update End Info
    for mut text in end_info_query.iter_mut() {
        if state.phase == Phase::Ended {
            **text = "FINAL".to_string();
        } else {
            **text = format!("END {}/{}", state.current_end, state.total_ends);
        }
    }

    // Update Shot Info
    for mut text in shot_info_query.iter_mut() {
        if state.phase == Phase::Ended {
            **text = "Game Over".to_string();
        } else {
            **text = format!("Shot: {}/{}", state.shot_index + 1, TOTAL_SHOTS);
        }
    }

    // Update Shots Remaining
    for mut text in shots_remaining_query.iter_mut() {
        let remaining = TOTAL_SHOTS.saturating_sub(state.shot_index);
        if state.phase == Phase::Ended {
            **text = "".to_string();
        } else {
            **text = format!("Remaining: {}", remaining);
        }
    }

    // Update Team Turn Indicator
    for (mut text, mut color) in team_turn_query.iter_mut() {
        let current_team = state.current_team();
        if state.phase == Phase::Ended {
            // Show winner
            if state.team1_score > state.team2_score {
                **text = "Team 1 Wins!".to_string();
                *color = TextColor(Team::One.color());
            } else if state.team2_score > state.team1_score {
                **text = "Team 2 Wins!".to_string();
                *color = TextColor(Team::Two.color());
            } else {
                **text = "Tie Game!".to_string();
                *color = TextColor(Color::WHITE);
            }
        } else {
            **text = format!("{}'s Turn", current_team.name());
            *color = TextColor(current_team.color());
        }
    }

    // Update Phase Indicator
    for mut text in phase_query.iter_mut() {
        let phase_str = match state.phase {
            Phase::CallingShot => "Calling Shot",
            Phase::Aiming => "Ready to Throw",
            Phase::StoneMoving => "Stone Moving...",
            Phase::Resolve => "Resolving...",
            Phase::Ended => "",
        };
        **text = phase_str.to_string();
    }

    // Update Hammer Indicator text
    let hammer_team = state.first_throw_team.opponent();
    for children in hammer_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = hammer_text_query.get_mut(child) {
                **text = format!("{} HAMMER", hammer_team.name().to_uppercase());
            }
        }
    }

    // --- Legacy Status Text (kept for window title) ---
    if let Some(mut status) = status_query.iter_mut().next() {
        let phase_str = match state.phase {
            Phase::CallingShot => "Drag broom to aim",
            Phase::Aiming => "Ready to Throw",
            Phase::StoneMoving => "Stone Moving",
            Phase::Resolve => "Resolving",
            Phase::Ended => "Game Over",
        };

        if state.phase == Phase::Ended {
            let winner = if state.team1_score > state.team2_score {
                "Team 1 Wins!"
            } else if state.team2_score > state.team1_score {
                "Team 2 Wins!"
            } else {
                "Tie Game!"
            };
            **status = format!(
                "FINAL: Team 1 {} - Team 2 {} | {}",
                state.team1_score, state.team2_score, winner
            );
        } else {
            **status = format!(
                "End {}/{} | Shot {}/{} {} | Weight: {:.1} | {}",
                state.current_end,
                state.total_ends,
                state.shot_index + 1,
                TOTAL_SHOTS,
                state.current_team().name(),
                state.called_weight,
                phase_str
            );
        }
    }

    // Update confirm button text
    for children in confirm_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                **text = match state.phase {
                    Phase::CallingShot => "Confirm Shot".to_string(),
                    Phase::Aiming => "THROW!".to_string(),
                    _ => "Wait...".to_string(),
                };
            }
        }
    }

    // Highlight selected curl direction
    for (curl_btn, mut bg_color) in curl_buttons.iter_mut() {
        let is_selected = state.curl_direction == curl_btn.0;
        *bg_color = if is_selected {
            BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.9))
        } else {
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8))
        };
    }
}

/// Handles tuning slider button interactions.
///
/// Adjusts scale and Z offset values based on button presses.
pub fn handle_tuning_buttons(
    mut tuning: ResMut<ModelTuning>,
    scale_buttons: Query<(&Interaction, &TuningAdjust), (Changed<Interaction>, With<ScaleSlider>)>,
    z_buttons: Query<
        (&Interaction, &TuningAdjust),
        (
            Changed<Interaction>,
            With<ZOffsetSlider>,
            Without<ScaleSlider>,
        ),
    >,
    mut scale_label: Query<&mut Text, (With<ScaleValueLabel>, Without<ZOffsetValueLabel>)>,
    mut z_label: Query<&mut Text, (With<ZOffsetValueLabel>, Without<ScaleValueLabel>)>,
) {
    const SCALE_STEP: f32 = 0.01;
    const Z_STEP: f32 = 0.01;

    // Handle scale buttons
    for (interaction, adjust) in scale_buttons.iter() {
        if *interaction == Interaction::Pressed {
            match adjust {
                TuningAdjust::Increase => tuning.scale = (tuning.scale + SCALE_STEP).min(2.0),
                TuningAdjust::Decrease => tuning.scale = (tuning.scale - SCALE_STEP).max(0.01),
            }
            // Update label
            for mut text in scale_label.iter_mut() {
                **text = format!("{:.2}", tuning.scale);
            }
        }
    }

    // Handle Z offset buttons
    for (interaction, adjust) in z_buttons.iter() {
        if *interaction == Interaction::Pressed {
            match adjust {
                TuningAdjust::Increase => tuning.z_offset = (tuning.z_offset + Z_STEP).min(1.0),
                TuningAdjust::Decrease => tuning.z_offset = (tuning.z_offset - Z_STEP).max(-1.0),
            }
            // Update label
            for mut text in z_label.iter_mut() {
                **text = format!("{:.2}", tuning.z_offset);
            }
        }
    }
}

/// Applies model tuning to all red stone visuals.
///
/// Updates transform (scale and Z position) based on ModelTuning resource.
pub fn apply_model_tuning(
    tuning: Res<ModelTuning>,
    mut visuals: Query<&mut Transform, With<StoneVisual>>,
) {
    if !tuning.is_changed() {
        return;
    }

    for mut transform in visuals.iter_mut() {
        transform.scale = Vec3::splat(tuning.scale);
        transform.translation.z = tuning.z_offset;
        // Keep the rotation
        transform.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    }
}

/// Handles the debug quick-simulate button.
///
/// Places 15 stones randomly in the house and sets up for the hammer throw.
#[cfg(feature = "debug_mode")]
pub fn handle_debug_quick_sim(
    mut commands: Commands,
    assets: Res<crate::resources::StoneAssets>,
    mut state: ResMut<GameState>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<DebugQuickSimButton>)>,
    existing_stones: Query<Entity, With<Stone>>,
) {
    use rand::Rng;

    for interaction in button_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Only work if we're in calling shot phase at the start of an end
        if state.phase != Phase::CallingShot {
            continue;
        }

        // Remove any existing stones
        for entity in existing_stones.iter() {
            commands.entity(entity).despawn();
        }

        let mut rng = rand::rng();

        // Place 15 stones (alternating teams) randomly in/around the house
        // Hammer team (who throws last) gets all odd shot indices
        // Shot 15 (index 15) is the hammer throw - we leave that for the player
        for shot_idx in 0u8..15 {
            let team = if shot_idx % 2 == 0 {
                state.first_throw_team
            } else {
                state.first_throw_team.opponent()
            };

            // Random position in/around the house (within 12-foot ring + some margin)
            let tee_y = TEE_FROM_CENTER;
            let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
            let radius: f32 = rng.random_range(0.0..(HOUSE_RADIUS_12 + STONE_RADIUS));
            let x = angle.cos() * radius;
            let y = tee_y + angle.sin() * radius;

            crate::helpers::spawn_stone(
                &mut commands,
                &assets,
                team,
                Vec2::new(x, y),
                Vec2::ZERO,
                false,
                CurlDirection::default(),
            );
        }

        // Set game state to shot 15 (the hammer throw)
        state.shot_index = 15;
        state.phase = Phase::CallingShot;

        tracing::info!(
            team = state.current_team().name(),
            "Quick sim: placed 15 stones, ready for hammer throw"
        );
    }
}
