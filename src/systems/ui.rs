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
            "Curling - FINAL: Red {} - Blue {}",
            state.red_score, state.blue_score
        )
    } else {
        format!(
            "Curling - End {}/{} | Red {} - Blue {} | Shot {}/{} | {} | {}",
            state.current_end,
            state.total_ends,
            state.red_score,
            state.blue_score,
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
/// - Status text (shot number, team, weight, angle, phase)
/// - Confirm button text (changes based on phase)
/// - Curl button highlighting (shows selected direction)
pub fn update_ui(
    state: Res<GameState>,
    mut status_query: Query<&mut Text, With<StatusText>>,
    confirm_query: Query<&Children, With<ConfirmButton>>,
    mut text_query: Query<&mut Text, Without<StatusText>>,
    mut curl_buttons: Query<(&CurlButton, &mut BackgroundColor), Without<ConfirmButton>>,
) {
    // Update status text
    if let Some(mut status) = status_query.iter_mut().next() {
        let phase_str = match state.phase {
            Phase::CallingShot => "Drag broom to aim",
            Phase::Aiming => "Ready to Throw",
            Phase::StoneMoving => "Stone Moving",
            Phase::Resolve => "Resolving",
            Phase::Ended => "Game Over",
        };

        let hammer_team = state.first_throw_team.opponent();

        if state.phase == Phase::Ended {
            let winner = if state.red_score > state.blue_score {
                "Red Wins!"
            } else if state.blue_score > state.red_score {
                "Blue Wins!"
            } else {
                "Tie Game!"
            };
            **status = format!(
                "FINAL: Red {} - Blue {} | {}",
                state.red_score, state.blue_score, winner
            );
        } else {
            **status = format!(
                "End {}/{} | Red {} - Blue {} | Shot {}/{} {} | Hammer: {} | {}",
                state.current_end,
                state.total_ends,
                state.red_score,
                state.blue_score,
                state.shot_index + 1,
                TOTAL_SHOTS,
                state.current_team().name(),
                hammer_team.name(),
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
