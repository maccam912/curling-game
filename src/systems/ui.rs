//! # UI Systems
//!
//! Systems that update the user interface elements.
//!
//! Uses `SystemParam` to bundle related UI queries, reducing the complexity
//! of individual system signatures.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use tracing::trace;

use crate::components::*;
use crate::constants::*;
use crate::resources::*;
use crate::viewport::ViewportConfig;

// ============================================================================
// SYSTEMPARAM FOR UI QUERIES
// ============================================================================

/// Bundled queries for HUD text elements.
///
/// Uses `SystemParam` to organize the many mutually-exclusive UI text queries
/// that need to be updated together. Each query excludes all other text markers
/// to satisfy Bevy's borrow checking rules.
#[derive(SystemParam)]
pub struct HudTextQueries<'w, 's> {
    pub team1_score: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<Team1ScoreText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub team2_score: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<Team2ScoreText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub end_info: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<EndInfoText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub shot_info: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ShotInfoText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub shots_remaining: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ShotsRemainingText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub team_turn: Query<
        'w,
        's,
        (&'static mut Text, &'static mut TextColor),
        (
            With<TeamTurnIndicator>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
    pub phase: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<PhaseIndicator>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<HammerText>,
        ),
    >,
    pub hammer_text: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<HammerText>,
            Without<StatusText>,
            Without<ConfirmButtonText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
        ),
    >,
    pub status: Query<'w, 's, &'static mut Text, With<StatusText>>,
    pub confirm_text: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ConfirmButtonText>,
            Without<StatusText>,
            Without<Team1ScoreText>,
            Without<Team2ScoreText>,
            Without<EndInfoText>,
            Without<ShotInfoText>,
            Without<ShotsRemainingText>,
            Without<TeamTurnIndicator>,
            Without<PhaseIndicator>,
            Without<HammerText>,
        ),
    >,
}

// ============================================================================
// UI UPDATE HELPER FUNCTIONS
// ============================================================================

/// Updates score display texts.
fn update_score_displays(hud: &mut HudTextQueries, state: &GameState) {
    for mut text in hud.team1_score.iter_mut() {
        **text = state.team1_score.to_string();
    }
    for mut text in hud.team2_score.iter_mut() {
        **text = state.team2_score.to_string();
    }
}

/// Updates end information text.
fn update_end_info(hud: &mut HudTextQueries, state: &GameState) {
    for mut text in hud.end_info.iter_mut() {
        **text = if state.phase == Phase::Ended {
            "FINAL".to_string()
        } else {
            format!("END {}/{}", state.current_end, state.total_ends)
        };
    }
}

/// Updates shot counter texts.
fn update_shot_info(hud: &mut HudTextQueries, state: &GameState) {
    for mut text in hud.shot_info.iter_mut() {
        **text = if state.phase == Phase::Ended {
            "Game Over".to_string()
        } else {
            format!("Shot: {}/{}", state.shot_index + 1, TOTAL_SHOTS)
        };
    }

    for mut text in hud.shots_remaining.iter_mut() {
        **text = if state.phase == Phase::Ended {
            "".to_string()
        } else {
            let remaining = TOTAL_SHOTS.saturating_sub(state.shot_index);
            format!("Remaining: {}", remaining)
        };
    }
}

/// Updates team turn indicator with appropriate color.
fn update_team_turn(hud: &mut HudTextQueries, state: &GameState) {
    for (mut text, mut color) in hud.team_turn.iter_mut() {
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
            let current_team = state.current_team();
            **text = format!("{}'s Turn", current_team.name());
            *color = TextColor(current_team.color());
        }
    }
}

/// Updates phase indicator text.
fn update_phase_display(hud: &mut HudTextQueries, state: &GameState) {
    for mut text in hud.phase.iter_mut() {
        **text = match state.phase {
            Phase::CallingShot => "Calling Shot",
            Phase::Aiming => "Ready to Throw",
            Phase::StoneMoving => "Stone Moving...",
            Phase::Resolve => "Resolving...",
            Phase::ShowingScore => "End Score",
            Phase::Ended => "",
        }
        .to_string();
    }
}

/// Updates hammer indicator text.
fn update_hammer_display(hud: &mut HudTextQueries, state: &GameState) {
    let hammer_team = state.first_throw_team.opponent();
    for mut text in hud.hammer_text.iter_mut() {
        **text = format!("{} HAMMER", hammer_team.name().to_uppercase());
    }
}

/// Updates legacy status text for window title compatibility.
fn update_status_text(hud: &mut HudTextQueries, state: &GameState) {
    if let Some(mut status) = hud.status.iter_mut().next() {
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
            let phase_str = match state.phase {
                Phase::CallingShot => "Drag broom to aim",
                Phase::Aiming => "Ready to Throw",
                Phase::StoneMoving => "Stone Moving",
                Phase::Resolve => "Resolving",
                Phase::ShowingScore => "End Score",
                Phase::Ended => "Game Over",
            };
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
}

/// Updates confirm button text based on phase.
fn update_confirm_button(hud: &mut HudTextQueries, state: &GameState) {
    for mut text in hud.confirm_text.iter_mut() {
        **text = match state.phase {
            Phase::CallingShot => "Confirm Shot".to_string(),
            Phase::Aiming => "THROW!".to_string(),
            _ => "Wait...".to_string(),
        };
    }
}

/// Updates curl button highlighting based on selected direction.
fn update_curl_buttons(
    curl_buttons: &mut Query<(&CurlButton, &mut BackgroundColor), Without<ConfirmButton>>,
    state: &GameState,
) {
    for (curl_btn, mut bg_color) in curl_buttons.iter_mut() {
        let is_selected = state.curl_direction == curl_btn.0;
        *bg_color = if is_selected {
            BackgroundColor(Color::srgba(
                COLOR_CURL_SELECTED.0,
                COLOR_CURL_SELECTED.1,
                COLOR_CURL_SELECTED.2,
                COLOR_CURL_SELECTED.3,
            ))
        } else {
            BackgroundColor(Color::srgba(
                COLOR_CURL_DESELECTED.0,
                COLOR_CURL_DESELECTED.1,
                COLOR_CURL_DESELECTED.2,
                COLOR_CURL_DESELECTED.3,
            ))
        };
    }
}

// ============================================================================
// UI SYSTEMS
// ============================================================================

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
        Phase::ShowingScore => "End Score",
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
/// Uses `HudTextQueries` SystemParam to bundle all the HUD text queries.
///
/// Updates:
/// - HUD panels (scores, end info, hammer, shot counter, team turn, phase)
/// - Legacy status text (for window title compatibility)
/// - Confirm button text (changes based on phase)
/// - Curl button highlighting (shows selected direction)
pub fn update_ui(
    state: Res<GameState>,
    mut hud: HudTextQueries,
    mut curl_buttons: Query<(&CurlButton, &mut BackgroundColor), Without<ConfirmButton>>,
) {
    update_score_displays(&mut hud, &state);
    update_end_info(&mut hud, &state);
    update_shot_info(&mut hud, &state);
    update_team_turn(&mut hud, &state);
    update_phase_display(&mut hud, &state);
    update_hammer_display(&mut hud, &state);
    update_status_text(&mut hud, &state);
    update_confirm_button(&mut hud, &state);
    update_curl_buttons(&mut curl_buttons, &state);
}

/// Updates the thrower info text to show current player's position and skills.
///
/// Shows the current thrower's position (Lead/Second/Third/Skip) and their
/// skill levels for weight and aim. Only visible during CallingShot and Aiming phases.
pub fn update_thrower_info(
    state: Res<GameState>,
    personalities: Res<PlayerPersonalities>,
    mut query: Query<(&mut Text, &mut Visibility), With<ThrowerInfoText>>,
) {
    for (mut text, mut visibility) in query.iter_mut() {
        // Only show during calling and aiming phases
        if state.phase == Phase::CallingShot || state.phase == Phase::Aiming {
            let personality =
                personalities.current_thrower(state.shot_index, state.first_throw_team);
            **text = format!(
                "{}: {}, {}",
                personality.position.name(),
                personality.weight_skill.name(),
                personality.aim_skill.name()
            );
            *visibility = Visibility::Visible;
        } else {
            **text = "".to_string();
            *visibility = Visibility::Hidden;
        }
    }
}

/// Updates the score summary panel visibility and content.
///
/// Shows the panel during ShowingScore phase with the pending end score.
pub fn update_score_summary_panel(
    state: Res<GameState>,
    mut panel_query: Query<&mut Visibility, With<ScoreSummaryPanel>>,
    mut text_query: Query<&mut Text, With<ScoreSummaryText>>,
) {
    // Show/hide panel based on phase
    for mut visibility in panel_query.iter_mut() {
        *visibility = if state.phase == Phase::ShowingScore {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update score text during ShowingScore phase
    if state.phase == Phase::ShowingScore {
        if let Some((team1_pts, team2_pts)) = state.pending_end_score {
            let score_text = if team1_pts > 0 {
                format!(
                    "Team 1 scores {} point{}!",
                    team1_pts,
                    if team1_pts == 1 { "" } else { "s" }
                )
            } else if team2_pts > 0 {
                format!(
                    "Team 2 scores {} point{}!",
                    team2_pts,
                    if team2_pts == 1 { "" } else { "s" }
                )
            } else {
                "Blank end - no score".to_string()
            };

            for mut text in text_query.iter_mut() {
                **text = score_text.clone();
            }
        }
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

/// Handles the debug skip-to-8th-end button.
///
/// Sets the game to end 8 with placeholder scores for previous ends.
#[cfg(feature = "debug_mode")]
pub fn handle_debug_skip_to_8th(
    mut state: ResMut<GameState>,
    button_query: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<crate::components::DebugSkipTo8thEndButton>,
        ),
    >,
) {
    for interaction in button_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Only work if we're in calling shot phase
        if state.phase != Phase::CallingShot {
            continue;
        }

        // Set to end 8
        state.current_end = 8;

        // Create placeholder end scores for ends 1-7
        // Simulate a close game with alternating scoring
        state.end_scores.clear();
        state.team1_score = 0;
        state.team2_score = 0;

        // Add realistic-looking scores for ends 1-7
        let scores = [
            (2, 0), // End 1: Team 1 scores 2
            (0, 1), // End 2: Team 2 scores 1
            (0, 3), // End 3: Team 2 scores 3
            (1, 0), // End 4: Team 1 scores 1
            (0, 2), // End 5: Team 2 scores 2
            (3, 0), // End 6: Team 1 scores 3
            (0, 1), // End 7: Team 2 scores 1
        ];

        for (t1, t2) in scores {
            state.end_scores.push((t1, t2));
            state.team1_score += t1;
            state.team2_score += t2;
        }

        tracing::info!(
            current_end = state.current_end,
            team1_score = state.team1_score,
            team2_score = state.team2_score,
            "Debug: Skipped to 8th end (Team 1: 6, Team 2: 7)"
        );
    }
}

/// Updates the game over panel visibility and content.
///
/// Shows the panel during Phase::Ended with score breakdown and winner.
pub fn update_game_over_panel(
    state: Res<GameState>,
    mut panel_query: Query<&mut Visibility, With<GameOverPanel>>,
    mut winner_query: Query<&mut Text, (With<GameOverWinnerText>, Without<GameOverScoreBreakdown>)>,
    mut breakdown_query: Query<
        &mut Text,
        (With<GameOverScoreBreakdown>, Without<GameOverWinnerText>),
    >,
) {
    // Show/hide panel based on phase
    for mut visibility in panel_query.iter_mut() {
        *visibility = if state.phase == Phase::Ended {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update content during Ended phase
    if state.phase == Phase::Ended {
        // Update winner text
        for mut text in winner_query.iter_mut() {
            if state.team1_score > state.team2_score {
                **text = format!("Team 1 Wins {} - {}!", state.team1_score, state.team2_score);
            } else if state.team2_score > state.team1_score {
                **text = format!("Team 2 Wins {} - {}!", state.team2_score, state.team1_score);
            } else {
                **text = format!("Tie Game {} - {}!", state.team1_score, state.team2_score);
            }
        }

        // Update score breakdown table
        for mut text in breakdown_query.iter_mut() {
            let mut breakdown = String::new();
            let mut team1_total = 0u32;
            let mut team2_total = 0u32;

            for (end_num, (t1, t2)) in state.end_scores.iter().enumerate() {
                breakdown.push_str(&format!(
                    " {:>2}      {:>3}      {:>3}\n",
                    end_num + 1,
                    t1,
                    t2
                ));
                team1_total += t1;
                team2_total += t2;
            }

            // Add total row
            breakdown.push_str(&format!("────────────────────\n",));
            breakdown.push_str(&format!(
                "TOT     {:>3}      {:>3}",
                team1_total, team2_total
            ));

            **text = breakdown;
        }
    }
}

/// Applies responsive styling to UI elements based on viewport configuration.
///
/// Updates:
/// - Element sizes for ResponsiveSize components
/// - Font sizes for ResponsiveText components
/// - Visibility for HideOnMobile components
/// - Padding/margins for CompactOnMobile components
pub fn apply_responsive_ui(
    viewport: Res<ViewportConfig>,
    mut size_query: Query<(&ResponsiveSize, &mut Node)>,
    mut text_query: Query<(&ResponsiveText, &mut TextFont)>,
    mut hide_query: Query<&mut Visibility, With<HideOnMobile>>,
    mut compact_query: Query<&mut Node, (With<CompactOnMobile>, Without<ResponsiveSize>)>,
    mut root_query: Query<
        &mut Node,
        (
            With<UiRoot>,
            Without<ResponsiveSize>,
            Without<CompactOnMobile>,
        ),
    >,
    mut bottom_panel_query: Query<
        &mut Node,
        (
            With<BottomControlPanel>,
            Without<UiRoot>,
            Without<ResponsiveSize>,
            Without<CompactOnMobile>,
        ),
    >,
) {
    // Only update when viewport changes
    if !viewport.is_changed() {
        return;
    }

    let ui_scale = viewport.ui_scale;
    let is_mobile = viewport.is_mobile();

    // Update responsive sizes
    for (resp_size, mut node) in size_query.iter_mut() {
        node.width = Val::Px(resp_size.base_width * ui_scale);
        node.height = Val::Px(resp_size.base_height * ui_scale);
    }

    // Update responsive text sizes
    for (resp_text, mut font) in text_query.iter_mut() {
        font.font_size = resp_text.base_size * ui_scale;
    }

    // Hide elements on mobile
    for mut visibility in hide_query.iter_mut() {
        *visibility = if is_mobile {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }

    // Apply compact styling on mobile
    let compact_padding = if is_mobile {
        Val::Px(5.0)
    } else {
        Val::Px(12.0)
    };
    for mut node in compact_query.iter_mut() {
        node.padding = UiRect::all(compact_padding);
    }

    // Update root padding based on viewport
    let base_padding = viewport.base_padding();
    for mut node in root_query.iter_mut() {
        node.padding = UiRect::all(Val::Px(base_padding));
    }

    // Adjust bottom panel positioning on mobile portrait
    if viewport.layout_mode == crate::viewport::LayoutMode::MobilePortrait {
        for mut node in bottom_panel_query.iter_mut() {
            // Move controls closer to bottom for thumb reach
            node.margin = UiRect::bottom(Val::Px(20.0));
        }
    }

    trace!(
        ui_scale = ui_scale,
        is_mobile = is_mobile,
        mode = ?viewport.layout_mode,
        "Applied responsive UI styling"
    );
}

/// Updates hammer icon visibility based on which team has hammer.
///
/// Shows the hammer icon next to the team that has hammer (throws last).
pub fn update_hammer_icons(
    state: Res<GameState>,
    mut team1_hammer: Query<&mut Visibility, (With<Team1HammerIcon>, Without<Team2HammerIcon>)>,
    mut team2_hammer: Query<&mut Visibility, (With<Team2HammerIcon>, Without<Team1HammerIcon>)>,
) {
    // Hammer team is the one that throws second (opponent of first throw team)
    let hammer_team = state.first_throw_team.opponent();

    // Update Team 1 hammer icon visibility
    for mut visibility in team1_hammer.iter_mut() {
        *visibility = if hammer_team == Team::One {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update Team 2 hammer icon visibility
    for mut visibility in team2_hammer.iter_mut() {
        *visibility = if hammer_team == Team::Two {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Updates curl buttons row visibility based on game phase.
///
/// Hides the IN/OUT buttons during Aiming phase (after confirming shot, before throwing).
/// Shows them during CallingShot phase for selection.
pub fn update_curl_buttons_visibility(
    state: Res<GameState>,
    mut curl_row_query: Query<&mut Visibility, With<CurlButtonsRow>>,
) {
    for mut visibility in curl_row_query.iter_mut() {
        *visibility = match state.phase {
            Phase::CallingShot => Visibility::Visible,
            Phase::Aiming => Visibility::Hidden,
            _ => Visibility::Hidden, // Hide during stone moving, resolve, etc.
        };
    }
}
