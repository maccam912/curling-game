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
/// Shows shot number, team, angle, weight, and current phase.
pub fn update_window_title(mut windows: Query<&mut Window>, state: Res<GameState>) {
    if !state.is_changed() {
        return;
    }

    let phase_label = match state.phase {
        Phase::CallingShot => "Calling shot (Enter)",
        Phase::Aiming => "Aiming (Space to throw)",
        Phase::StoneMoving => "Stones moving",
        Phase::Resolve => "Resolving shot",
        Phase::Ended => "End complete",
    };

    let title = if state.phase == Phase::Ended {
        "Curling - End Complete".to_string()
    } else {
        format!(
            "Curling - Shot {}/{} | Team {} | Call {:.1} deg / {:.1} | Aim {:.1} deg / {:.1} | {}",
            state.shot_index + 1,
            TOTAL_SHOTS,
            state.current_team().name(),
            state.called_angle_deg,
            state.called_weight,
            state.aim_angle_deg,
            state.aim_weight,
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
            Phase::Ended => "End Complete",
        };

        **status = format!(
            "Shot {}/{} - {} - Weight: {:.1} | Angle: {:.1}° | {}",
            state.shot_index + 1,
            TOTAL_SHOTS,
            state.current_team().name(),
            state.called_weight,
            state.called_angle_deg,
            phase_str
        );
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
