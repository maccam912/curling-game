//! # Input Systems
//!
//! Systems that handle all user input: keyboard, mouse, and touch.

use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tracing::{debug, info, trace};

use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::resources::*;

/// Handles keyboard input during the shot calling phase.
///
/// Controls:
/// - Arrow keys / WASD: Adjust angle and weight
/// - C: Toggle camera between SkipView and Overhead
/// - Enter: Confirm the called shot and move to aiming
pub fn handle_calling_input(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut camera_state: ResMut<CameraState>,
) {
    if state.phase != Phase::CallingShot {
        return;
    }

    // Toggle camera between SkipView and Overhead with 'C' key
    if input.just_pressed(KeyCode::KeyC) {
        let new_mode = match camera_state.mode {
            CameraMode::SkipView => CameraMode::Overhead,
            CameraMode::Overhead => CameraMode::SkipView,
            _ => CameraMode::SkipView,
        };
        camera_state.mode = new_mode;
        debug!(mode = ?new_mode, "Camera mode toggled via keyboard");
    }

    let dt = time.delta_secs();

    // Angle adjustment
    if input.pressed(KeyCode::ArrowLeft) || input.pressed(KeyCode::KeyA) {
        state.called_angle_deg -= ANGLE_RATE_DEG * dt;
        trace!(angle = state.called_angle_deg, "Adjusting angle left");
    }
    if input.pressed(KeyCode::ArrowRight) || input.pressed(KeyCode::KeyD) {
        state.called_angle_deg += ANGLE_RATE_DEG * dt;
        trace!(angle = state.called_angle_deg, "Adjusting angle right");
    }

    // Weight adjustment
    if input.pressed(KeyCode::ArrowUp) || input.pressed(KeyCode::KeyW) {
        state.called_weight += WEIGHT_RATE * dt;
        trace!(weight = state.called_weight, "Increasing weight");
    }
    if input.pressed(KeyCode::ArrowDown) || input.pressed(KeyCode::KeyS) {
        state.called_weight -= WEIGHT_RATE * dt;
        trace!(weight = state.called_weight, "Decreasing weight");
    }

    // Clamp values
    state.called_angle_deg = state
        .called_angle_deg
        .clamp(-ANGLE_LIMIT_DEG, ANGLE_LIMIT_DEG);
    state.called_weight = state.called_weight.clamp(WEIGHT_MIN, WEIGHT_MAX);

    // Confirm shot
    if input.just_pressed(KeyCode::Enter) {
        state.aim_angle_deg = state.called_angle_deg;
        state.aim_weight = state.called_weight;
        state.phase = Phase::Aiming;
        info!(
            shot_index = state.shot_index + 1,
            team = state.current_team().name(),
            angle = state.called_angle_deg,
            weight = state.called_weight,
            "Shot called"
        );
    }
}

/// Handles keyboard input during the aiming phase.
///
/// Controls:
/// - Arrow keys / WASD: Fine-tune aim
/// - R: Reset aim to called shot values
/// - Space: Throw the stone
pub fn handle_aiming_input(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut commands: Commands,
    assets: Res<StoneAssets>,
    stones: Query<(Entity, &Transform, &Stone)>,
) {
    if state.phase != Phase::Aiming {
        return;
    }

    let dt = time.delta_secs();

    // Angle adjustment
    if input.pressed(KeyCode::ArrowLeft) || input.pressed(KeyCode::KeyA) {
        state.aim_angle_deg -= ANGLE_RATE_DEG * dt;
    }
    if input.pressed(KeyCode::ArrowRight) || input.pressed(KeyCode::KeyD) {
        state.aim_angle_deg += ANGLE_RATE_DEG * dt;
    }

    // Weight adjustment
    if input.pressed(KeyCode::ArrowUp) || input.pressed(KeyCode::KeyW) {
        state.aim_weight += WEIGHT_RATE * dt;
    }
    if input.pressed(KeyCode::ArrowDown) || input.pressed(KeyCode::KeyS) {
        state.aim_weight -= WEIGHT_RATE * dt;
    }

    // Clamp values
    state.aim_angle_deg = state.aim_angle_deg.clamp(-ANGLE_LIMIT_DEG, ANGLE_LIMIT_DEG);
    state.aim_weight = state.aim_weight.clamp(WEIGHT_MIN, WEIGHT_MAX);

    // Reset aim to called values
    if input.just_pressed(KeyCode::KeyR) {
        state.aim_angle_deg = state.called_angle_deg;
        state.aim_weight = state.called_weight;
        debug!("Aim reset to called values");
    }

    // Throw the stone
    if input.just_pressed(KeyCode::Space) {
        throw_stone(&mut state, &mut commands, &assets, &stones);
    }
}

/// Handle mouse/touch drag to position the broom on the ice.
///
/// During the CallingShot phase, allows the player to drag the broom
/// target to set both angle and weight.
pub fn handle_broom_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut state: ResMut<GameState>,
    mut touch_state: ResMut<TouchState>,
    ui_interactions: Query<&Interaction, With<Button>>,
) {
    // Only allow broom dragging during CallingShot phase
    if state.phase != Phase::CallingShot {
        touch_state.dragging = false;
        return;
    }

    // Don't start a drag if clicking on a UI button
    let any_ui_pressed = ui_interactions
        .iter()
        .any(|i| *i == Interaction::Pressed || *i == Interaction::Hovered);

    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Check for drag start - only if not clicking UI
    if mouse_button.just_pressed(MouseButton::Left) && !any_ui_pressed {
        touch_state.dragging = true;
        touch_state.drag_start = cursor_pos;
        trace!("Broom drag started at {:?}", cursor_pos);
    }

    // Update broom position during drag
    if mouse_button.pressed(MouseButton::Left) && touch_state.dragging {
        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
            // Intersect ray with Z=0 plane (ice surface)
            debug!(
                ray_origin = ?ray.origin,
                ray_direction = ?ray.direction,
                "Broom drag raycast"
            );
            if ray.direction.z.abs() > 0.0001 {
                let t = -ray.origin.z / ray.direction.z;
                debug!(t = t, "Raycast t parameter");
                if t > 0.0 {
                    let world_pos = ray.origin + ray.direction * t;
                    debug!(world_pos = ?world_pos, "Raycast world position");

                    // Clamp to playable area
                    let half_width = SHEET_WIDTH * 0.5 - 0.2;
                    let min_y = hog_line_far();
                    let max_y = back_line_far();

                    state.broom_position = Vec2::new(
                        world_pos.x.clamp(-half_width, half_width),
                        world_pos.y.clamp(min_y, max_y),
                    );

                    // Update called values based on broom position
                    state.called_angle_deg = state.angle_from_broom();
                    state.called_weight = state.weight_from_broom();

                    debug!(
                        broom_x = state.broom_position.x,
                        broom_y = state.broom_position.y,
                        weight = state.called_weight,
                        "Broom position updated"
                    );
                }
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        touch_state.dragging = false;
        trace!("Broom drag ended");
    }
}

/// Handle touch and mouse input for UI buttons.
///
/// Handles:
/// - Camera toggle button
/// - Curl direction buttons (IN/OUT)
/// - Confirm/throw button
pub fn handle_touch_input(
    mut state: ResMut<GameState>,
    mut camera_state: ResMut<CameraState>,
    mut confirm_button: Query<
        (&Interaction, &mut BackgroundColor),
        (
            With<ConfirmButton>,
            Without<CameraToggleButton>,
            Without<CurlButton>,
            Changed<Interaction>,
        ),
    >,
    mut camera_button: Query<
        (&Interaction, &mut BackgroundColor),
        (
            With<CameraToggleButton>,
            Without<ConfirmButton>,
            Without<CurlButton>,
            Changed<Interaction>,
        ),
    >,
    mut curl_buttons: Query<
        (&Interaction, &CurlButton, &mut BackgroundColor),
        (
            Without<ConfirmButton>,
            Without<CameraToggleButton>,
            Changed<Interaction>,
        ),
    >,
    mut commands: Commands,
    assets: Res<StoneAssets>,
    stones: Query<(Entity, &Transform, &Stone)>,
) {
    // Handle camera toggle button
    for (interaction, mut bg_color) in camera_button.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if state.phase == Phase::CallingShot {
                    let new_mode = match camera_state.mode {
                        CameraMode::SkipView => CameraMode::Overhead,
                        CameraMode::Overhead => CameraMode::SkipView,
                        _ => CameraMode::SkipView,
                    };
                    if new_mode != camera_state.mode {
                        camera_state.mode = new_mode;
                        camera_state.transition_progress = 0.0;
                        camera_state.transition_duration = 0.5;
                        debug!(mode = ?new_mode, "Camera mode toggled via button");
                    }
                }
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.9));
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.9));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8));
            }
        }
    }

    // Handle curl direction button clicks
    for (interaction, curl_btn, mut bg_color) in curl_buttons.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if state.phase == Phase::CallingShot || state.phase == Phase::Aiming {
                    state.curl_direction = curl_btn.0;
                    *bg_color = BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.9));
                    debug!(direction = ?curl_btn.0, "Curl direction changed");
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.9));
            }
            Interaction::None => {
                let is_selected = state.curl_direction == curl_btn.0;
                *bg_color = if is_selected {
                    BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.9))
                } else {
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8))
                };
            }
        }
    }

    // Handle confirm/throw button
    for (interaction, mut bg_color) in confirm_button.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                match state.phase {
                    Phase::CallingShot => {
                        // Confirm the shot call
                        debug!(
                            before_broom_y = state.broom_position.y,
                            before_called_weight = state.called_weight,
                            "Before confirm"
                        );
                        state.aim_angle_deg = state.angle_from_broom();
                        state.aim_weight = state.called_weight;
                        state.phase = Phase::Aiming;
                        info!(
                            shot_index = state.shot_index + 1,
                            team = state.current_team().name(),
                            angle = state.aim_angle_deg,
                            weight = state.aim_weight,
                            "Shot confirmed via button"
                        );
                    }
                    Phase::Aiming => {
                        throw_stone(&mut state, &mut commands, &assets, &stones);
                    }
                    _ => {}
                }
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.7, 0.4, 1.0));
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.25, 0.65, 0.35, 0.95));
            }
            Interaction::None => {
                let color = match state.phase {
                    Phase::CallingShot => Color::srgba(0.2, 0.6, 0.3, 0.9),
                    Phase::Aiming => Color::srgba(0.6, 0.3, 0.2, 0.9),
                    _ => Color::srgba(0.3, 0.3, 0.3, 0.5),
                };
                *bg_color = BackgroundColor(color);
            }
        }
    }
}

/// Helper function to throw a stone.
///
/// Creates the stone entity, sets up the snapshot for FGZ checking,
/// and transitions to StoneMoving phase.
fn throw_stone(
    state: &mut ResMut<GameState>,
    commands: &mut Commands,
    assets: &StoneAssets,
    stones: &Query<(Entity, &Transform, &Stone)>,
) {
    let snapshot = snapshot_stones(stones, state.shot_index);
    state.snapshot = Some(snapshot);

    let team = state.current_team();
    let weight_normalized = (state.aim_weight - 1.0) / 9.0;
    let speed = WEIGHT_MIN_SPEED + weight_normalized * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
    let angle_rad = state.aim_angle_deg.to_radians();
    let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
    let start = Vec2::new(0.0, DELIVERY_START_Y);

    let stone_entity = spawn_stone(
        commands,
        assets,
        team,
        start,
        direction * speed,
        true,
        state.curl_direction,
    );

    state.thrown_stone = Some(stone_entity);
    state.still_time = 0.0;
    state.phase = Phase::StoneMoving;

    info!(
        shot_index = state.shot_index + 1,
        team = team.name(),
        angle = state.aim_angle_deg,
        weight = state.aim_weight,
        speed = speed,
        curl = ?state.curl_direction,
        "Stone thrown"
    );
}
