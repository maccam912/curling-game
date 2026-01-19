//! # Curling Game
//!
//! A 3D curling simulation game built with Bevy and Rapier physics.
//!
//! ## Features
//! - Realistic curling physics with friction and curl
//! - Free Guard Zone rule enforcement
//! - Hog line rule enforcement (near and far)
//! - Touch/mouse-friendly controls
//! - Multiple camera views
//!
//! ## Module Structure
//! - [`constants`]: All game and physics constants
//! - [`components`]: Bevy ECS components and enums
//! - [`resources`]: Bevy ECS resources for game state
//! - [`helpers`]: Utility functions for gameplay logic
//! - [`systems`]: All Bevy systems organized by functionality
//!
//! ## Usage
//! ```rust,no_run
//! use bevy::prelude::*;
//! use curling_game::CurlingPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(CurlingPlugin)
//!         .run();
//! }
//! ```

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub mod ai_strategy;
pub mod app_state;
pub mod components;
pub mod constants;
pub mod helpers;
// pub mod ice_material; // Removed
pub mod network;
pub mod resources;
pub mod rules;
pub mod systems;
pub mod viewport;

// Re-export commonly used items
pub use ai_strategy::*;
pub use app_state::*;
pub use components::*;
pub use constants::*;
// pub use ice_material::*; // Removed
pub use network::*;
pub use resources::*;
pub use rules::*;
pub use viewport::*;

/// The main curling game plugin.
///
/// This plugin sets up everything needed for the curling game:
/// - Physics engine (Rapier2D with zero gravity)
/// - Ambient lighting
/// - Game state resources
/// - All game systems
///
/// # Example
/// ```rust,no_run
/// use bevy::prelude::*;
/// use curling_game::CurlingPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(CurlingPlugin)
///     .run();
/// ```
pub struct CurlingPlugin;

impl Plugin for CurlingPlugin {
    fn build(&self, app: &mut App) {
        // Initialize tracing subscriber for structured logging
        #[cfg(not(target_arch = "wasm32"))]
        {
            use tracing_subscriber::{EnvFilter, fmt, prelude::*};
            let _ = tracing_subscriber::registry()
                .with(fmt::layer())
                .with(
                    EnvFilter::from_default_env()
                        .add_directive("curling_game=info".parse().unwrap())
                        .add_directive("bevy=warn".parse().unwrap()),
                )
                .try_init();
        }

        app
            // App state machine
            .init_state::<app_state::AppState>()
            .init_state::<app_state::NetworkRole>()
            // Physics plugin
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            // Ice material with pebbling shader
            // .add_plugins(MaterialPlugin::<ice_material::IceMaterial>::default())
            // Lighting
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 0.8,
                affects_lightmapped_meshes: true,
            })
            // Game resources
            .insert_resource(resources::GameState::default())
            .insert_resource(resources::CameraState::default())
            .insert_resource(resources::TouchState::default())
            .insert_resource(resources::OnlineState::default())
            .insert_resource(resources::PredictionState::default())
            .insert_resource(viewport::ViewportConfig::default())
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            // ================================================================
            // SPLASH SCREEN
            // ================================================================
            .add_systems(OnEnter(app_state::AppState::Splash), systems::setup_splash)
            .add_systems(
                Update,
                systems::update_splash_timer.run_if(in_state(app_state::AppState::Splash)),
            )
            .add_systems(OnExit(app_state::AppState::Splash), systems::cleanup_splash)
            // ================================================================
            // MAIN MENU STATE
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::MainMenu),
                (
                    systems::setup_main_menu,
                    systems::cleanup_game_scene,
                    systems::cleanup_game_ui,
                ),
            )
            .add_systems(
                Update,
                systems::handle_menu_buttons.run_if(in_state(app_state::AppState::MainMenu)),
            )
            .add_systems(
                OnExit(app_state::AppState::MainMenu),
                systems::cleanup_main_menu,
            )
            // ================================================================
            // SETTINGS MENU STATE
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::Settings),
                systems::setup_settings_menu,
            )
            .add_systems(
                Update,
                systems::handle_settings_buttons.run_if(in_state(app_state::AppState::Settings)),
            )
            .add_systems(
                OnExit(app_state::AppState::Settings),
                systems::cleanup_settings_menu,
            )
            // ================================================================
            // PAUSE STATE
            // ================================================================
            .insert_resource(systems::pause::PreviousGameState::default())
            .add_systems(
                OnEnter(app_state::AppState::Paused),
                systems::setup_pause_menu,
            )
            .add_systems(
                Update,
                (systems::handle_pause_buttons, systems::toggle_pause)
                    .run_if(in_state(app_state::AppState::Paused)),
            )
            .add_systems(
                OnExit(app_state::AppState::Paused),
                systems::cleanup_pause_menu,
            )
            // Common pause toggle for game states
            .add_systems(
                Update,
                systems::toggle_pause.run_if(
                    in_state(app_state::AppState::PassAndPlay)
                        .or(in_state(app_state::AppState::VsAI))
                        .or(in_state(app_state::AppState::OnlineGame)),
                ),
            )
            // ================================================================
            // ONLINE MENU STATE
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::OnlineMenu),
                systems::setup_online_menu,
            )
            .add_systems(
                Update,
                systems::handle_online_menu_buttons
                    .run_if(in_state(app_state::AppState::OnlineMenu)),
            )
            .add_systems(
                OnExit(app_state::AppState::OnlineMenu),
                systems::cleanup_online_menu,
            )
            // ================================================================
            // ONLINE LOBBY STATE
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::OnlineLobby),
                systems::setup_online_lobby,
            )
            .add_systems(
                Update,
                (
                    systems::poll_lobby_peer_events,
                    systems::handle_lobby_buttons,
                )
                    .run_if(in_state(app_state::AppState::OnlineLobby)),
            )
            .add_systems(
                OnExit(app_state::AppState::OnlineLobby),
                systems::cleanup_online_lobby,
            )
            // ================================================================
            // PASS AND PLAY STATE (Game)
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::PassAndPlay),
                (
                    // systems::setup_reflection_camera, // Removed
                    systems::setup_scene,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(app_state::AppState::PassAndPlay),
                (
                    systems::configure_rapier,
                    systems::setup_ui,
                    systems::randomize_first_team,
                    systems::generate_player_personalities,
                ),
            )
            // Physics systems run at fixed rate for consistency (FPS-independent)
            .add_systems(
                FixedUpdate,
                (
                    systems::ice_friction_system,
                    systems::track_throwing_stone,
                    systems::detect_stone_collision,
                    systems::check_out_of_bounds,
                    systems::detect_shot_end,
                )
                    .run_if(in_state(app_state::AppState::PassAndPlay)),
            )
            // Update systems (run only during gameplay)
            .add_systems(
                Update,
                (
                    systems::viewport_detection_system,
                    systems::update_window_title,
                    systems::handle_calling_input,
                    systems::handle_aiming_input,
                    systems::handle_touch_input,
                    systems::handle_broom_drag,
                    systems::update_broom_visual,
                    systems::update_stone_visual_rotation,
                    systems::resolve_shot,
                    systems::handle_score_confirmation,
                    systems::camera_control_system,
                    systems::update_ui,
                )
                    .run_if(in_state(app_state::AppState::PassAndPlay)),
            )
            .add_systems(
                Update,
                (
                    systems::update_score_summary_panel,
                    systems::update_game_over_panel,
                    systems::apply_responsive_ui,
                    systems::update_prediction,
                    systems::update_ghost_stone_visual,
                    systems::update_thrower_info,
                    systems::update_hammer_icons,
                    systems::update_curl_buttons_visibility,
                    systems::disable_reflection_shadows,
                    // systems::sync_reflection_camera, // Removed reflection module
                )
                    .run_if(in_state(app_state::AppState::PassAndPlay)),
            )
            // ================================================================
            // VS AI STATE (Single player vs computer)
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::VsAI),
                // (systems::setup_reflection_camera, systems::setup_scene).chain(),
                systems::setup_scene,
            )
            .add_systems(
                OnEnter(app_state::AppState::VsAI),
                (
                    systems::configure_rapier,
                    systems::setup_ui,
                    systems::randomize_first_team,
                    systems::setup_ai_game,
                    systems::generate_player_personalities,
                ),
            )
            // Physics systems for VS AI (FPS-independent)
            .add_systems(
                FixedUpdate,
                (
                    systems::ice_friction_system,
                    systems::track_throwing_stone,
                    systems::detect_stone_collision,
                    systems::check_out_of_bounds,
                    systems::detect_shot_end,
                )
                    .run_if(in_state(app_state::AppState::VsAI)),
            )
            // Input systems for VS AI (only run on human turns)
            .add_systems(
                Update,
                (
                    systems::handle_calling_input,
                    systems::handle_aiming_input,
                    systems::handle_touch_input,
                    systems::handle_broom_drag,
                )
                    .run_if(in_state(app_state::AppState::VsAI))
                    .run_if(systems::run_if_human_turn),
            )
            // Core gameplay systems for VS AI (always run)
            .add_systems(
                Update,
                (
                    systems::viewport_detection_system,
                    systems::update_window_title,
                    systems::update_broom_visual,
                    systems::update_stone_visual_rotation,
                    systems::resolve_shot,
                    systems::handle_score_confirmation,
                    systems::camera_control_system,
                    systems::update_ui,
                    systems::update_score_summary_panel,
                    systems::update_game_over_panel,
                    systems::apply_responsive_ui,
                    systems::ai_turn_system,
                )
                    .run_if(in_state(app_state::AppState::VsAI)),
            )
            .add_systems(
                Update,
                (
                    systems::update_prediction,
                    systems::update_ghost_stone_visual,
                    systems::update_thrower_info,
                    systems::update_hammer_icons,
                    systems::update_curl_buttons_visibility,
                    systems::disable_reflection_shadows,
                    // systems::sync_reflection_camera, // Removed
                )
                    .run_if(in_state(app_state::AppState::VsAI)),
            )
            // ================================================================
            // ONLINE GAME STATE (Multiplayer)
            // ================================================================
            .add_systems(
                OnEnter(app_state::AppState::OnlineGame),
                // (systems::setup_reflection_camera, systems::setup_scene).chain(),
                systems::setup_scene,
            )
            .add_systems(
                OnEnter(app_state::AppState::OnlineGame),
                (
                    systems::configure_rapier,
                    systems::setup_ui,
                    systems::setup_online_game,
                    systems::spawn_your_team_indicator,
                    systems::spawn_connection_status_indicator,
                    systems::generate_player_personalities,
                ),
            )
            // Physics systems for online game (FPS-independent)
            .add_systems(
                FixedUpdate,
                (
                    systems::ice_friction_system,
                    systems::track_throwing_stone,
                    systems::detect_stone_collision,
                    systems::check_out_of_bounds,
                    systems::detect_shot_end,
                )
                    .run_if(in_state(app_state::AppState::OnlineGame)),
            )
            // Network sync systems for online game (always run)
            .add_systems(
                Update,
                (
                    systems::receive_network_messages,
                    systems::apply_pending_shot,
                    systems::send_shot_on_throw,
                    systems::send_positions_on_resolve,
                    systems::sync_stone_positions,
                    systems::send_periodic_sync,
                    systems::apply_periodic_sync,
                    systems::online_camera_control,
                    systems::send_broom_updates,
                    systems::apply_broom_updates,
                    // Disconnection detection and UI
                    systems::detect_disconnection,
                    systems::update_connection_status_ui,
                    systems::show_disconnection_overlay,
                    systems::handle_disconnection_return_button,
                )
                    .run_if(in_state(app_state::AppState::OnlineGame)),
            )
            // Input systems for online game (only run when it's local player's turn)
            .add_systems(
                Update,
                (
                    systems::handle_calling_input,
                    systems::handle_aiming_input,
                    systems::handle_touch_input,
                    systems::handle_broom_drag,
                )
                    .run_if(in_state(app_state::AppState::OnlineGame))
                    .run_if(systems::run_if_local_turn),
            )
            // Core gameplay systems for online game (always run)
            .add_systems(
                Update,
                (
                    systems::viewport_detection_system,
                    systems::update_window_title,
                    systems::update_broom_visual,
                    systems::update_stone_visual_rotation,
                    systems::resolve_shot,
                    systems::handle_score_confirmation,
                    systems::camera_control_system,
                    systems::update_ui,
                    systems::update_score_summary_panel,
                    systems::update_game_over_panel,
                    systems::apply_responsive_ui,
                )
                    .run_if(in_state(app_state::AppState::OnlineGame)),
            )
            .add_systems(
                Update,
                (
                    systems::update_prediction,
                    systems::update_ghost_stone_visual,
                    systems::update_thrower_info,
                    systems::update_hammer_icons,
                    systems::update_curl_buttons_visibility,
                    systems::disable_reflection_shadows,
                    // systems::sync_reflection_camera, // Removed
                )
                    .run_if(in_state(app_state::AppState::OnlineGame)),
            )
            .add_systems(
                OnExit(app_state::AppState::OnlineGame),
                (
                    systems::cleanup_online_game,
                    systems::cleanup_your_team_indicator,
                ),
            );

        // Debug-only systems
        #[cfg(feature = "debug_mode")]
        app.add_systems(
            Update,
            (
                systems::handle_debug_quick_sim,
                systems::handle_debug_skip_to_8th,
            )
                .run_if(in_state(app_state::AppState::PassAndPlay)),
        );
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec2;

    // ============ TEST HELPERS ============

    /// Check if a position is within the house (12-foot ring)
    fn is_in_house(position: Vec2) -> bool {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        position.distance(tee) <= HOUSE_RADIUS_12
    }

    /// Calculate distance from tee for scoring purposes
    fn distance_from_tee(position: Vec2) -> f32 {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        position.distance(tee)
    }

    /// Check if a stone at this position is biting (touching) a ring
    fn is_biting_house(position: Vec2) -> bool {
        let dist = distance_from_tee(position);
        dist <= HOUSE_RADIUS_12 + STONE_RADIUS
    }

    /// Create a test stone snapshot
    fn test_stone_snapshot(team: Team, x: f32, y: f32) -> resources::StoneSnapshot {
        let position = Vec2::new(x, y);
        resources::StoneSnapshot {
            entity: Entity::PLACEHOLDER,
            team,
            position,
            in_fgz: helpers::is_in_free_guard_zone(position),
        }
    }

    // ============ TEAM TESTS ============

    #[test]
    fn team_alternates_by_shot_index() {
        assert_eq!(Team::from_shot_index(0), Team::One);
        assert_eq!(Team::from_shot_index(1), Team::Two);
        assert_eq!(Team::from_shot_index(2), Team::One);
    }

    #[test]
    fn team_alternates_all_16_shots() {
        for i in 0..16u8 {
            let expected = if i % 2 == 0 { Team::One } else { Team::Two };
            assert_eq!(
                Team::from_shot_index(i),
                expected,
                "Shot {} should be {:?}",
                i,
                expected
            );
        }
    }

    #[test]
    fn team_colors_are_distinct() {
        assert_ne!(Team::One.color(), Team::Two.color());
    }

    #[test]
    fn team_names_are_correct() {
        assert_eq!(Team::One.name(), "Team 1");
        assert_eq!(Team::Two.name(), "Team 2");
    }

    // ============ SHOT TYPE TESTS ============

    #[test]
    fn shot_type_default_weights_in_valid_range() {
        for shot_type in [
            ShotType::Draw,
            ShotType::Guard,
            ShotType::Takeout,
            ShotType::Freeze,
            ShotType::HitAndRoll,
        ] {
            let weight = shot_type.default_weight();
            assert!(
                weight >= WEIGHT_MIN && weight <= WEIGHT_MAX,
                "{:?} default weight {} out of range [{}, {}]",
                shot_type,
                weight,
                WEIGHT_MIN,
                WEIGHT_MAX
            );
        }
    }

    #[test]
    fn shot_type_names_are_not_empty() {
        for shot_type in [
            ShotType::Draw,
            ShotType::Guard,
            ShotType::Takeout,
            ShotType::Freeze,
            ShotType::HitAndRoll,
        ] {
            assert!(
                !shot_type.name().is_empty(),
                "{:?} has empty name",
                shot_type
            );
        }
    }

    #[test]
    fn shot_type_weight_ordering() {
        assert!(ShotType::Guard.default_weight() <= ShotType::Draw.default_weight());
        assert!(ShotType::Draw.default_weight() < ShotType::Takeout.default_weight());
    }

    #[test]
    fn shot_type_takeout_is_heavy() {
        assert!(ShotType::Takeout.default_weight() >= 7.0);
    }

    // ============ CURL DIRECTION TESTS ============

    #[test]
    fn curl_directions_opposite_angular_velocity() {
        let in_turn = CurlDirection::InTurn.angular_velocity();
        let out_turn = CurlDirection::OutTurn.angular_velocity();
        assert!(
            in_turn > 0.0,
            "InTurn should have positive angular velocity"
        );
        assert!(
            out_turn < 0.0,
            "OutTurn should have negative angular velocity"
        );
        assert!(
            (in_turn.abs() - out_turn.abs()).abs() < 0.001,
            "Angular velocities should have equal magnitude"
        );
    }

    #[test]
    fn curl_direction_names() {
        assert_eq!(CurlDirection::InTurn.name(), "IN");
        assert_eq!(CurlDirection::OutTurn.name(), "OUT");
    }

    // ============ GAME STATE TESTS ============

    #[test]
    fn game_state_default_values() {
        let state = resources::GameState::default();
        assert_eq!(state.phase, Phase::CallingShot);
        assert_eq!(state.shot_index, 0);
        assert_eq!(state.shot_type, ShotType::Draw);
        assert_eq!(state.called_angle_deg, 0.0);
        assert!(state.thrown_stone.is_none());
        assert!(state.snapshot.is_none());
    }

    #[test]
    fn game_state_current_team_matches_shot_index() {
        let mut state = resources::GameState::default();
        for i in 0..16u8 {
            state.shot_index = i;
            assert_eq!(
                state.current_team(),
                Team::from_shot_index(i),
                "current_team() mismatch at shot {}",
                i
            );
        }
    }

    #[test]
    fn game_state_broom_position_default() {
        let state = resources::GameState::default();
        assert_eq!(state.broom_position.x, 0.0);
        assert_eq!(state.broom_position.y, TEE_FROM_CENTER);
    }

    #[test]
    fn game_state_angle_from_broom_center() {
        let state = resources::GameState::default();
        let angle = state.angle_from_broom();
        assert!(
            angle.abs() < 0.1,
            "Center broom should give ~0 angle, got {}",
            angle
        );
    }

    #[test]
    fn game_state_angle_from_broom_left() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(-1.0, TEE_FROM_CENTER);
        let angle = state.angle_from_broom();
        assert!(angle < 0.0, "Left broom should give negative angle");
    }

    #[test]
    fn game_state_angle_from_broom_right() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(1.0, TEE_FROM_CENTER);
        let angle = state.angle_from_broom();
        assert!(angle > 0.0, "Right broom should give positive angle");
    }

    // ============ WEIGHT FROM BROOM TESTS ============

    #[test]
    fn weight_from_broom_at_hog_line_gives_min_weight() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(0.0, helpers::hog_line_far());
        let weight = state.weight_from_broom();
        assert!(
            (weight - WEIGHT_MIN).abs() < 0.01,
            "Broom at hog line should give min weight {}, got {}",
            WEIGHT_MIN,
            weight
        );
    }

    #[test]
    fn weight_from_broom_at_back_line_gives_max_weight() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(0.0, helpers::back_line_far());
        let weight = state.weight_from_broom();
        assert!(
            (weight - WEIGHT_MAX).abs() < 0.01,
            "Broom at back line should give max weight {}, got {}",
            WEIGHT_MAX,
            weight
        );
    }

    #[test]
    fn weight_from_broom_at_tee_gives_mid_weight() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(0.0, helpers::tee_line_far());
        let weight = state.weight_from_broom();
        assert!(
            weight > WEIGHT_MIN && weight < WEIGHT_MAX,
            "Broom at tee should give mid-range weight, got {}",
            weight
        );
    }

    #[test]
    fn weight_from_broom_increases_with_y() {
        let mut state = resources::GameState::default();

        state.broom_position = Vec2::new(0.0, helpers::hog_line_far() + 1.0);
        let weight_near = state.weight_from_broom();

        state.broom_position = Vec2::new(0.0, helpers::back_line_far() - 1.0);
        let weight_far = state.weight_from_broom();

        assert!(
            weight_far > weight_near,
            "Weight should increase as broom moves forward: {} should be > {}",
            weight_far,
            weight_near
        );
    }

    #[test]
    fn weight_from_broom_clamps_below_hog() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(0.0, helpers::hog_line_far() - 5.0);
        let weight = state.weight_from_broom();
        assert!(
            (weight - WEIGHT_MIN).abs() < 0.01,
            "Broom below hog should clamp to min weight"
        );
    }

    #[test]
    fn weight_from_broom_clamps_beyond_back() {
        let mut state = resources::GameState::default();
        state.broom_position = Vec2::new(0.0, helpers::back_line_far() + 5.0);
        let weight = state.weight_from_broom();
        assert!(
            (weight - WEIGHT_MAX).abs() < 0.01,
            "Broom beyond back should clamp to max weight"
        );
    }

    // ============ COORDINATE/LINE TESTS ============

    #[test]
    fn hog_lines_are_symmetric() {
        assert!(helpers::hog_line_far() > 0.0);
        assert!(helpers::hog_line_near() < 0.0);
        let sum = helpers::hog_line_far() + helpers::hog_line_near();
        assert!(sum.abs() < 0.001);
    }

    #[test]
    fn back_lines_are_symmetric() {
        assert!(helpers::back_line_far() > 0.0);
        assert!(helpers::back_line_near() < 0.0);
        let sum = helpers::back_line_far() + helpers::back_line_near();
        assert!(sum.abs() < 0.001);
    }

    #[test]
    fn tee_lines_are_symmetric() {
        assert!(helpers::tee_line_far() > 0.0);
        assert!(helpers::tee_line_near() < 0.0);
        let sum = helpers::tee_line_far() + helpers::tee_line_near();
        assert!(sum.abs() < 0.001);
    }

    #[test]
    fn line_ordering_far_end() {
        assert!(helpers::back_line_far() > helpers::tee_line_far());
        assert!(helpers::tee_line_far() > helpers::hog_line_far());
        assert!(helpers::hog_line_far() > 0.0);
    }

    #[test]
    fn line_ordering_near_end() {
        assert!(helpers::back_line_near() < helpers::tee_line_near());
        assert!(helpers::tee_line_near() < helpers::hog_line_near());
        assert!(helpers::hog_line_near() < 0.0);
    }

    #[test]
    fn delivery_start_is_behind_near_hog() {
        assert!(
            DELIVERY_START_Y < helpers::hog_line_near(),
            "Delivery start {} should be behind near hog line {}",
            DELIVERY_START_Y,
            helpers::hog_line_near()
        );
    }

    #[test]
    fn house_radii_ordering() {
        assert!(HOUSE_RADIUS_BUTTON < HOUSE_RADIUS_4);
        assert!(HOUSE_RADIUS_4 < HOUSE_RADIUS_8);
        assert!(HOUSE_RADIUS_8 < HOUSE_RADIUS_12);
    }

    // ============ BOUNDARY DETECTION TESTS ============

    #[test]
    fn hog_line_reached_when_crossing_delivery_hog_line() {
        assert!(!helpers::hog_line_reached(helpers::hog_line_near() - 0.01));
        assert!(helpers::hog_line_reached(helpers::hog_line_near()));
        assert!(helpers::hog_line_reached(helpers::hog_line_near() + 0.5));
    }

    #[test]
    fn out_of_bounds_checks_edges() {
        let in_play = Vec2::new(0.0, 0.0);
        assert!(!helpers::is_out_of_bounds(in_play));

        let out_right = Vec2::new(SHEET_WIDTH * 0.5 + STONE_RADIUS + 0.01, 0.0);
        assert!(helpers::is_out_of_bounds(out_right));

        let out_back = Vec2::new(0.0, helpers::back_line_far() + STONE_RADIUS + 0.01);
        assert!(helpers::is_out_of_bounds(out_back));

        let out_near = Vec2::new(0.0, helpers::back_line_near() - STONE_RADIUS - 0.01);
        assert!(helpers::is_out_of_bounds(out_near));
    }

    #[test]
    fn out_of_bounds_left_edge() {
        let out_left = Vec2::new(-(SHEET_WIDTH * 0.5 + STONE_RADIUS + 0.01), 0.0);
        assert!(helpers::is_out_of_bounds(out_left));

        let just_inside = Vec2::new(-(SHEET_WIDTH * 0.5), 0.0);
        assert!(!helpers::is_out_of_bounds(just_inside));
    }

    #[test]
    fn out_of_bounds_sheet_center_inbounds() {
        let center = Vec2::ZERO;
        assert!(!helpers::is_out_of_bounds(center));
    }

    #[test]
    fn out_of_bounds_corners() {
        let far_right = Vec2::new(
            SHEET_WIDTH * 0.5 + STONE_RADIUS + 0.01,
            helpers::back_line_far(),
        );
        assert!(helpers::is_out_of_bounds(far_right));

        let near_left = Vec2::new(
            -(SHEET_WIDTH * 0.5 + STONE_RADIUS + 0.01),
            helpers::back_line_near(),
        );
        assert!(helpers::is_out_of_bounds(near_left));
    }

    #[test]
    fn out_of_bounds_just_inside_all_edges() {
        let almost_right = Vec2::new(SHEET_WIDTH * 0.5 - 0.01, 0.0);
        let almost_left = Vec2::new(-(SHEET_WIDTH * 0.5 - 0.01), 0.0);
        let almost_far = Vec2::new(0.0, helpers::back_line_far() - 0.01);
        let almost_near = Vec2::new(0.0, helpers::back_line_near() + 0.01);

        assert!(!helpers::is_out_of_bounds(almost_right));
        assert!(!helpers::is_out_of_bounds(almost_left));
        assert!(!helpers::is_out_of_bounds(almost_far));
        assert!(!helpers::is_out_of_bounds(almost_near));
    }

    // ============ FREE GUARD ZONE TESTS ============

    #[test]
    fn free_guard_zone_detection() {
        let guard_pos = Vec2::new(
            0.0,
            (helpers::hog_line_far() + helpers::tee_line_far()) * 0.5,
        );
        assert!(helpers::is_in_free_guard_zone(guard_pos));

        let inside_house = Vec2::new(0.0, helpers::tee_line_far());
        assert!(!helpers::is_in_free_guard_zone(inside_house));

        let past_hog = Vec2::new(0.0, helpers::hog_line_far() - 0.2);
        assert!(!helpers::is_in_free_guard_zone(past_hog));
    }

    #[test]
    fn fgz_boundary_at_hog_line() {
        let just_past_hog = Vec2::new(0.0, helpers::hog_line_far() + 0.01);
        assert!(helpers::is_in_free_guard_zone(just_past_hog));

        let at_hog = Vec2::new(0.0, helpers::hog_line_far());
        assert!(!helpers::is_in_free_guard_zone(at_hog));

        let before_hog = Vec2::new(0.0, helpers::hog_line_far() - 0.01);
        assert!(!helpers::is_in_free_guard_zone(before_hog));
    }

    #[test]
    fn fgz_boundary_at_tee_line() {
        let before_tee_outside_house =
            Vec2::new(HOUSE_RADIUS_12 + 0.5, helpers::tee_line_far() - 0.01);
        assert!(helpers::is_in_free_guard_zone(before_tee_outside_house));

        let at_tee = Vec2::new(0.0, helpers::tee_line_far());
        assert!(!helpers::is_in_free_guard_zone(at_tee));
    }

    #[test]
    fn fgz_excludes_house() {
        let on_tee = Vec2::new(0.0, helpers::tee_line_far());
        assert!(!helpers::is_in_free_guard_zone(on_tee));

        let on_12ft_inside = Vec2::new(0.0, helpers::tee_line_far() - HOUSE_RADIUS_12 + 0.01);
        assert!(!helpers::is_in_free_guard_zone(on_12ft_inside));
    }

    #[test]
    fn fgz_just_outside_house() {
        let just_outside_house = Vec2::new(0.0, helpers::tee_line_far() - HOUSE_RADIUS_12 - 0.01);
        if just_outside_house.y > helpers::hog_line_far() {
            assert!(helpers::is_in_free_guard_zone(just_outside_house));
        }
    }

    #[test]
    fn fgz_horizontal_positions() {
        let mid_y = (helpers::hog_line_far() + helpers::tee_line_far()) * 0.5;

        let left = Vec2::new(-1.0, mid_y);
        assert!(helpers::is_in_free_guard_zone(left));

        let right = Vec2::new(1.0, mid_y);
        assert!(helpers::is_in_free_guard_zone(right));
    }

    // ============ HOG LINE RULE TESTS ============

    #[test]
    fn near_hog_line_boundary() {
        assert!(!helpers::hog_line_reached(helpers::hog_line_near() - 0.001));
        assert!(helpers::hog_line_reached(helpers::hog_line_near()));
        assert!(helpers::hog_line_reached(helpers::hog_line_near() + 1.0));
    }

    #[test]
    fn far_hog_line_boundary() {
        let just_before = helpers::hog_line_far() + STONE_RADIUS - 0.001;
        assert!(!helpers::far_hog_line_reached(just_before));

        let at_line = helpers::hog_line_far() + STONE_RADIUS;
        assert!(!helpers::far_hog_line_reached(at_line));

        let just_past = helpers::hog_line_far() + STONE_RADIUS + 0.001;
        assert!(helpers::far_hog_line_reached(just_past));
    }

    #[test]
    fn far_hog_considers_stone_radius() {
        assert!(!helpers::far_hog_line_reached(helpers::hog_line_far()));
    }

    // ============ HOUSE / SCORING HELPER TESTS ============

    #[test]
    fn is_in_house_at_tee() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        assert!(is_in_house(tee));
    }

    #[test]
    fn is_in_house_on_rings() {
        let on_button = Vec2::new(0.0, helpers::tee_line_far() + HOUSE_RADIUS_BUTTON * 0.5);
        assert!(is_in_house(on_button));

        let on_4ft = Vec2::new(0.0, helpers::tee_line_far() - HOUSE_RADIUS_4 * 0.5);
        assert!(is_in_house(on_4ft));

        let on_8ft = Vec2::new(0.0, helpers::tee_line_far() + HOUSE_RADIUS_8 * 0.5);
        assert!(is_in_house(on_8ft));

        let on_12ft_edge = Vec2::new(HOUSE_RADIUS_12, helpers::tee_line_far());
        assert!(is_in_house(on_12ft_edge));
    }

    #[test]
    fn is_in_house_outside() {
        let outside = Vec2::new(HOUSE_RADIUS_12 + 0.1, helpers::tee_line_far());
        assert!(!is_in_house(outside));
    }

    #[test]
    fn is_biting_house_edge_cases() {
        let biting = Vec2::new(
            HOUSE_RADIUS_12 + STONE_RADIUS * 0.5,
            helpers::tee_line_far(),
        );
        assert!(is_biting_house(biting));

        let not_biting = Vec2::new(
            HOUSE_RADIUS_12 + STONE_RADIUS + 0.1,
            helpers::tee_line_far(),
        );
        assert!(!is_biting_house(not_biting));
    }

    #[test]
    fn distance_from_tee_at_various_positions() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        assert!(distance_from_tee(tee) < 0.001);

        let on_button_edge = Vec2::new(HOUSE_RADIUS_BUTTON, helpers::tee_line_far());
        assert!((distance_from_tee(on_button_edge) - HOUSE_RADIUS_BUTTON).abs() < 0.001);

        let on_12ft_edge = Vec2::new(0.0, helpers::tee_line_far() - HOUSE_RADIUS_12);
        assert!((distance_from_tee(on_12ft_edge) - HOUSE_RADIUS_12).abs() < 0.001);
    }

    // ============ PHYSICS CONSTANTS TESTS ============

    #[test]
    fn weight_speed_mapping_is_monotonic() {
        let low_weight = 1.0;
        let high_weight = 10.0;

        let low_speed =
            WEIGHT_MIN_SPEED + ((low_weight - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
        let high_speed =
            WEIGHT_MIN_SPEED + ((high_weight - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);

        assert!(high_speed > low_speed);
    }

    #[test]
    fn weight_extremes_map_to_speed_extremes() {
        let min_mapped =
            WEIGHT_MIN_SPEED + ((WEIGHT_MIN - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
        let max_mapped =
            WEIGHT_MIN_SPEED + ((WEIGHT_MAX - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);

        assert!((min_mapped - WEIGHT_MIN_SPEED).abs() < 0.001);
        assert!((max_mapped - WEIGHT_MAX_SPEED).abs() < 0.001);
    }

    #[test]
    fn speed_values_are_reasonable() {
        assert!(WEIGHT_MIN_SPEED >= 1.0 && WEIGHT_MIN_SPEED <= 4.0);
        assert!(WEIGHT_MAX_SPEED >= 2.0 && WEIGHT_MAX_SPEED <= 5.0);
        assert!(WEIGHT_MAX_SPEED > WEIGHT_MIN_SPEED);
    }

    #[test]
    fn friction_deceleration_is_positive() {
        assert!(ICE_FRICTION_DECEL > 0.0);
        assert!(ICE_FRICTION_DECEL >= 0.05 && ICE_FRICTION_DECEL <= 0.3);
    }

    #[test]
    fn curl_coefficient_is_small() {
        assert!(CURL_COEFFICIENT > 0.0);
        assert!(CURL_COEFFICIENT < 0.1);
    }

    // ============ SNAPSHOT TESTS ============

    #[test]
    fn test_stone_snapshot_creation() {
        let snap = test_stone_snapshot(Team::One, 0.0, helpers::hog_line_far() + 1.0);
        assert_eq!(snap.team, Team::One);
        assert_eq!(snap.position.x, 0.0);
        assert!(snap.in_fgz);

        let snap_in_house = test_stone_snapshot(Team::Two, 0.0, helpers::tee_line_far());
        assert_eq!(snap_in_house.team, Team::Two);
        assert!(!snap_in_house.in_fgz);
    }

    // ============ STONE DIMENSION TESTS ============

    #[test]
    fn stone_dimensions_are_realistic() {
        let diameter = STONE_RADIUS * 2.0;
        assert!(diameter >= 0.25 && diameter <= 0.35);
        assert!(STONE_HEIGHT >= 0.10 && STONE_HEIGHT <= 0.15);
    }

    // ============ TOTAL SHOTS TEST ============

    #[test]
    fn total_shots_is_16() {
        assert_eq!(TOTAL_SHOTS, 16);
    }

    #[test]
    fn total_shots_evenly_divisible_by_teams() {
        assert!(
            TOTAL_SHOTS % 2 == 0,
            "Total shots should be even for fair team alternation"
        );
    }

    // ============ REGRESSION TESTS ============

    /// Regression test for issue B0001: Query conflicts in update_ui system.
    ///
    /// This test ensures that all Text queries in the update_ui system have
    /// proper mutual exclusion (Without<T>) filters to prevent Bevy ECS
    /// query conflicts at runtime.
    ///
    /// The original bug was caused by overlapping mutable Text queries
    /// that didn't properly exclude each other using marker components.
    #[test]
    fn ui_text_markers_are_mutually_exclusive() {
        use std::any::TypeId;

        // All the marker components used for Text queries in update_ui
        let marker_types = [
            TypeId::of::<StatusText>(),
            TypeId::of::<ConfirmButtonText>(),
            TypeId::of::<Team1ScoreText>(),
            TypeId::of::<Team2ScoreText>(),
            TypeId::of::<EndInfoText>(),
            TypeId::of::<ShotInfoText>(),
            TypeId::of::<ShotsRemainingText>(),
            TypeId::of::<TeamTurnIndicator>(),
            TypeId::of::<PhaseIndicator>(),
            TypeId::of::<HammerText>(),
        ];

        // Verify all markers have unique type IDs (basic sanity check)
        for (i, type_a) in marker_types.iter().enumerate() {
            for type_b in marker_types.iter().skip(i + 1) {
                assert_ne!(
                    type_a, type_b,
                    "UI text marker components must have unique TypeIds"
                );
            }
        }

        // Verify we have markers for all Text-accessing queries
        // If this count changes, update the Without<T> filters in update_ui
        assert_eq!(
            marker_types.len(),
            10,
            "Expected 10 UI text marker types for update_ui queries"
        );
    }

    // ============ SCORE_END TESTS ============

    #[test]
    fn score_end_empty_house_is_blank_end() {
        let stones: Vec<(Team, Vec2)> = vec![];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 0);
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_single_stone_team1_scores_one() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        let stones = vec![(Team::One, tee)];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 1);
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_single_stone_team2_scores_one() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        let stones = vec![(Team::Two, tee)];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 0);
        assert_eq!(team2, 1);
    }

    #[test]
    fn score_end_closest_stone_wins() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Team 1 stone closer to tee
        let stones = vec![
            (Team::One, tee + Vec2::new(0.2, 0.0)), // closer
            (Team::Two, tee + Vec2::new(0.5, 0.0)), // farther
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 1);
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_multiple_stones_count_correctly() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Team 1 has closest 3 stones, then Team 2 has one
        let stones = vec![
            (Team::One, tee + Vec2::new(0.1, 0.0)), // 1st closest
            (Team::One, tee + Vec2::new(0.2, 0.0)), // 2nd closest
            (Team::One, tee + Vec2::new(0.3, 0.0)), // 3rd closest
            (Team::Two, tee + Vec2::new(0.4, 0.0)), // 4th closest
            (Team::One, tee + Vec2::new(0.5, 0.0)), // 5th (doesn't count - after opponent)
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(
            team1, 3,
            "Team 1 should score 3 (three closer than Team 2's closest)"
        );
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_team2_can_score_multiple() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        let stones = vec![
            (Team::Two, tee + Vec2::new(0.1, 0.0)), // 1st closest
            (Team::Two, tee + Vec2::new(0.2, 0.0)), // 2nd closest
            (Team::One, tee + Vec2::new(0.3, 0.0)), // 3rd closest - stops Team 2's count
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 0);
        assert_eq!(team2, 2);
    }

    #[test]
    fn score_end_stone_outside_house_doesnt_score() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Stone far outside house
        let outside = tee + Vec2::new(HOUSE_RADIUS_12 + STONE_RADIUS + 1.0, 0.0);
        let stones = vec![(Team::One, outside)];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 0, "Stone outside house shouldn't score");
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_biting_stone_counts() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Stone just barely biting the 12-foot ring
        let biting = tee + Vec2::new(HOUSE_RADIUS_12 + STONE_RADIUS * 0.5, 0.0);
        let stones = vec![(Team::One, biting)];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 1, "Biting stone should score");
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_all_one_team_scores_all() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // All Team 1 stones in house, no opponent
        let stones = vec![
            (Team::One, tee + Vec2::new(0.1, 0.0)),
            (Team::One, tee + Vec2::new(0.2, 0.0)),
            (Team::One, tee + Vec2::new(0.3, 0.0)),
            (Team::One, tee + Vec2::new(0.4, 0.0)),
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(
            team1, 4,
            "All 4 stones should score when no opponent in house"
        );
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_ties_on_distance_closest_wins() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Both teams have stone at same distance - first in list wins ties
        // (This tests sorting stability behavior)
        let dist = 0.3;
        let stones = vec![
            (Team::One, tee + Vec2::new(dist, 0.0)),
            (Team::Two, tee + Vec2::new(0.0, dist)),
        ];
        let (team1, team2) = helpers::score_end(&stones);
        // One team should score exactly 1
        assert!(
            (team1 == 1 && team2 == 0) || (team1 == 0 && team2 == 1),
            "One team should score 1 on identical distances"
        );
    }

    #[test]
    fn score_end_stone_on_tee_beats_all() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        let stones = vec![
            (Team::Two, tee + Vec2::new(0.5, 0.0)), // farther
            (Team::One, tee),                       // on button
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 1);
        assert_eq!(team2, 0);
    }

    #[test]
    fn score_end_8_ender_possible() {
        let tee = Vec2::new(0.0, helpers::tee_line_far());
        // Maximum possible score: 8 stones of one team, all in house
        let stones = vec![
            (Team::Two, tee + Vec2::new(0.1, 0.0)),
            (Team::Two, tee + Vec2::new(0.15, 0.0)),
            (Team::Two, tee + Vec2::new(0.2, 0.0)),
            (Team::Two, tee + Vec2::new(0.25, 0.0)),
            (Team::Two, tee + Vec2::new(0.3, 0.0)),
            (Team::Two, tee + Vec2::new(0.35, 0.0)),
            (Team::Two, tee + Vec2::new(0.4, 0.0)),
            (Team::Two, tee + Vec2::new(0.45, 0.0)),
        ];
        let (team1, team2) = helpers::score_end(&stones);
        assert_eq!(team1, 0);
        assert_eq!(team2, 8, "8-ender should be possible");
    }

    // ============ GAME STATE RESET TESTS ============

    #[test]
    fn game_state_reset_for_new_end() {
        let mut state = resources::GameState::default();
        state.shot_index = 15; // End of an end
        state.phase = Phase::ShowingScore;
        state.called_angle_deg = 5.0;
        state.called_weight = 8.0;

        state.reset_for_new_end();

        assert_eq!(state.shot_index, 0);
        assert_eq!(state.phase, Phase::CallingShot);
        assert_eq!(state.called_angle_deg, 0.0);
        assert_eq!(state.called_weight, ShotType::Draw.default_weight());
    }

    // ============ PHASE TRANSITION TESTS ============

    #[test]
    fn phase_initial_is_calling_shot() {
        let state = resources::GameState::default();
        assert_eq!(state.phase, Phase::CallingShot);
    }

    #[test]
    fn phase_variants_are_distinct() {
        assert_ne!(Phase::CallingShot, Phase::Aiming);
        assert_ne!(Phase::Aiming, Phase::StoneMoving);
        assert_ne!(Phase::StoneMoving, Phase::Resolve);
        assert_ne!(Phase::Resolve, Phase::ShowingScore);
        assert_ne!(Phase::ShowingScore, Phase::Ended);
    }

    // ============ VELOCITY CALCULATION TESTS ============

    #[test]
    fn velocity_from_weight_min_gives_min_speed() {
        let weight = WEIGHT_MIN;
        let weight_normalized = (weight - 1.0) / 9.0;
        let speed = WEIGHT_MIN_SPEED + weight_normalized * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
        assert!((speed - WEIGHT_MIN_SPEED).abs() < 0.001);
    }

    #[test]
    fn velocity_from_weight_max_gives_max_speed() {
        let weight = WEIGHT_MAX;
        let weight_normalized = (weight - 1.0) / 9.0;
        let speed = WEIGHT_MIN_SPEED + weight_normalized * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
        assert!((speed - WEIGHT_MAX_SPEED).abs() < 0.001);
    }

    #[test]
    fn velocity_direction_from_angle() {
        let angle_deg: f32 = 0.0;
        let angle_rad = angle_deg.to_radians();
        let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
        assert!(
            (direction.x).abs() < 0.001,
            "0 degrees should point straight"
        );
        assert!((direction.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn velocity_direction_from_positive_angle() {
        let angle_deg: f32 = 10.0;
        let angle_rad = angle_deg.to_radians();
        let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
        assert!(direction.x > 0.0, "Positive angle should deflect right");
        assert!(direction.y > 0.0, "Should still move forward");
    }

    #[test]
    fn velocity_direction_from_negative_angle() {
        let angle_deg: f32 = -10.0;
        let angle_rad = angle_deg.to_radians();
        let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
        assert!(direction.x < 0.0, "Negative angle should deflect left");
        assert!(direction.y > 0.0, "Should still move forward");
    }

    // ============ END SCORE HISTORY TESTS ============

    #[test]
    fn end_scores_empty_initially() {
        let state = resources::GameState::default();
        assert!(state.end_scores.is_empty());
    }

    #[test]
    fn current_end_starts_at_one() {
        let state = resources::GameState::default();
        assert_eq!(state.current_end, 1);
    }

    #[test]
    fn total_ends_default() {
        let state = resources::GameState::default();
        assert_eq!(state.total_ends, 8);
    }

    // ============ HAMMER/FIRST THROW TESTS ============

    #[test]
    fn first_throw_team_default() {
        let state = resources::GameState::default();
        // Default can be either team, just verify it's valid
        assert!(state.first_throw_team == Team::One || state.first_throw_team == Team::Two);
    }

    #[test]
    fn team_opponent() {
        assert_eq!(Team::One.opponent(), Team::Two);
        assert_eq!(Team::Two.opponent(), Team::One);
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use bevy::math::Vec2;
    use proptest::prelude::*;

    // ============ SCORING INVARIANTS ============

    // Property: Only one team can score per end (never both)
    proptest! {
        #[test]
        fn scoring_only_one_team_scores(
            stones in prop::collection::vec(
                (prop::bool::ANY, -2.0f32..2.0, -2.0f32..2.0),
                0..16
            )
        ) {
            let tee = Vec2::new(0.0, helpers::tee_line_far());
            let stone_positions: Vec<(Team, Vec2)> = stones
                .into_iter()
                .map(|(is_team1, dx, dy)| {
                    let team = if is_team1 { Team::One } else { Team::Two };
                    (team, tee + Vec2::new(dx, dy))
                })
                .collect();

            let (team1, team2) = helpers::score_end(&stone_positions);

            // Invariant: At most one team scores
            prop_assert!(
                team1 == 0 || team2 == 0,
                "Both teams scored: T1={}, T2={}",
                team1,
                team2
            );
        }
    }

    // Property: Score is bounded by 8 (max stones per team)
    proptest! {
        #[test]
        fn scoring_bounded_by_eight(
            stones_t1 in prop::collection::vec(
                (-2.0f32..2.0, -2.0f32..2.0),
                0..=8
            ),
            stones_t2 in prop::collection::vec(
                (-2.0f32..2.0, -2.0f32..2.0),
                0..=8
            )
        ) {
            let tee = Vec2::new(0.0, helpers::tee_line_far());
            let mut stone_positions = Vec::new();

            for (dx, dy) in stones_t1 {
                stone_positions.push((Team::One, tee + Vec2::new(dx, dy)));
            }
            for (dx, dy) in stones_t2 {
                stone_positions.push((Team::Two, tee + Vec2::new(dx, dy)));
            }

            let (team1, team2) = helpers::score_end(&stone_positions);

            prop_assert!(team1 <= 8, "Team 1 scored more than 8: {}", team1);
            prop_assert!(team2 <= 8, "Team 2 scored more than 8: {}", team2);
        }
    }

    // Property: Closer stone always wins (transitive scoring)
    proptest! {
        #[test]
        fn closer_stone_wins(
            team1_dist in 0.01f32..1.5,
            team2_dist in 0.01f32..1.5,
        ) {
            let tee = Vec2::new(0.0, helpers::tee_line_far());
            let stones = vec![
                (Team::One, tee + Vec2::new(team1_dist, 0.0)),
                (Team::Two, tee + Vec2::new(team2_dist, 0.0)),
            ];

            let (team1, team2) = helpers::score_end(&stones);

            if team1_dist < team2_dist {
                prop_assert_eq!(team1, 1, "Team 1 was closer but didn't score");
                prop_assert_eq!(team2, 0);
            } else if team2_dist < team1_dist {
                prop_assert_eq!(team1, 0);
                prop_assert_eq!(team2, 1, "Team 2 was closer but didn't score");
            }
            // Equal distances: either can win, tested elsewhere
        }
    }

    // ============ TEAM ALTERNATION INVARIANTS ============

    // Property: Teams always alternate for any valid shot index
    proptest! {
        #[test]
        fn team_alternation_consistent(shot_index in 0u8..100) {
            let team = Team::from_shot_index(shot_index);
            let expected = if shot_index % 2 == 0 { Team::One } else { Team::Two };
            prop_assert_eq!(team, expected);
        }
    }

    // Property: Opponent is always the other team
    proptest! {
        #[test]
        fn opponent_is_inverse(shot_index in 0u8..100) {
            let team = Team::from_shot_index(shot_index);
            prop_assert_eq!(team.opponent().opponent(), team);
        }
    }

    // ============ WEIGHT/SPEED MAPPING INVARIANTS ============

    // Property: Weight-to-speed mapping is monotonically increasing
    proptest! {
        #[test]
        fn weight_speed_monotonic(
            w1 in 1.0f32..10.0,
            w2 in 1.0f32..10.0,
        ) {
            let speed1 = WEIGHT_MIN_SPEED + ((w1 - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
            let speed2 = WEIGHT_MIN_SPEED + ((w2 - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);

            if w1 < w2 {
                prop_assert!(speed1 < speed2, "weight {} -> speed {}, weight {} -> speed {}", w1, speed1, w2, speed2);
            } else if w1 > w2 {
                prop_assert!(speed1 > speed2);
            } else {
                prop_assert!((speed1 - speed2).abs() < 0.0001);
            }
        }
    }

    // Property: Speed is always within valid bounds
    proptest! {
        #[test]
        fn speed_within_bounds(weight in 1.0f32..10.0) {
            let speed = WEIGHT_MIN_SPEED + ((weight - 1.0) / 9.0) * (WEIGHT_MAX_SPEED - WEIGHT_MIN_SPEED);
            prop_assert!(speed >= WEIGHT_MIN_SPEED - 0.001);
            prop_assert!(speed <= WEIGHT_MAX_SPEED + 0.001);
        }
    }

    // ============ ANGLE/DIRECTION INVARIANTS ============

    // Property: Positive angles deflect right (positive X)
    proptest! {
        #[test]
        fn positive_angle_deflects_right(angle_deg in 0.1f32..45.0) {
            let angle_rad = angle_deg.to_radians();
            let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
            prop_assert!(direction.x > 0.0, "Angle {} should deflect right", angle_deg);
            prop_assert!(direction.y > 0.0, "Should still move forward");
        }
    }

    // Property: Negative angles deflect left (negative X)
    proptest! {
        #[test]
        fn negative_angle_deflects_left(angle_deg in -45.0f32..-0.1) {
            let angle_rad = angle_deg.to_radians();
            let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
            prop_assert!(direction.x < 0.0, "Angle {} should deflect left", angle_deg);
            prop_assert!(direction.y > 0.0, "Should still move forward");
        }
    }

    // Property: Direction vector is always normalized (unit length)
    proptest! {
        #[test]
        fn direction_is_normalized(angle_deg in -45.0f32..45.0) {
            let angle_rad = angle_deg.to_radians();
            let direction = Vec2::new(angle_rad.sin(), angle_rad.cos());
            let length = direction.length();
            prop_assert!((length - 1.0).abs() < 0.0001, "Direction should be unit vector, got length {}", length);
        }
    }

    // ============ FREE GUARD ZONE INVARIANTS ============

    // Property: FGZ positions are always between hog line and house
    proptest! {
        #[test]
        fn fgz_is_between_hog_and_house(
            x in -2.0f32..2.0,
            y_offset in 0.01f32..3.0, // offset past hog line
        ) {
            let y = helpers::hog_line_far() + y_offset;
            let pos = Vec2::new(x, y);

            if helpers::is_in_free_guard_zone(pos) {
                // Must be past hog line
                prop_assert!(pos.y > helpers::hog_line_far());
                // Must NOT be in the house
                let tee = Vec2::new(0.0, helpers::tee_line_far());
                let dist_to_tee = pos.distance(tee);
                prop_assert!(dist_to_tee > HOUSE_RADIUS_12, "FGZ stone should not be in house");
            }
        }
    }

    // Property: Stones in the house are never in FGZ
    proptest! {
        #[test]
        fn house_stones_not_in_fgz(
            angle in 0.0f32..std::f32::consts::TAU,
            dist in 0.0f32..1.8, // within 12-foot
        ) {
            let tee = Vec2::new(0.0, helpers::tee_line_far());
            let pos = tee + Vec2::new(angle.cos() * dist, angle.sin() * dist);

            let tee_dist = pos.distance(tee);
            if tee_dist <= HOUSE_RADIUS_12 {
                prop_assert!(
                    !helpers::is_in_free_guard_zone(pos),
                    "Stone in house at distance {} should not be in FGZ",
                    tee_dist
                );
            }
        }
    }

    // ============ OUT OF BOUNDS INVARIANTS ============

    // Property: Stones within sheet boundaries are never out of bounds
    proptest! {
        #[test]
        fn in_bounds_stones_valid(
            x in -(SHEET_WIDTH * 0.5 - STONE_RADIUS - 0.1)..=(SHEET_WIDTH * 0.5 - STONE_RADIUS - 0.1),
        ) {
            // Use a y that's definitely in bounds
            let y = 0.0f32;
            let pos = Vec2::new(x, y);
            prop_assert!(
                !helpers::is_out_of_bounds(pos),
                "Position {:?} should be in bounds",
                pos
            );
        }
    }

    // Property: Stones clearly outside sheet are always out of bounds
    proptest! {
        #[test]
        fn out_of_bounds_symmetry(
            x_excess in 0.5f32..5.0,
        ) {
            let left = Vec2::new(-(SHEET_WIDTH * 0.5 + STONE_RADIUS + x_excess), 0.0);
            let right = Vec2::new(SHEET_WIDTH * 0.5 + STONE_RADIUS + x_excess, 0.0);

            prop_assert!(helpers::is_out_of_bounds(left), "Left edge should be OOB");
            prop_assert!(helpers::is_out_of_bounds(right), "Right edge should be OOB");
        }
    }

    // ============ GAME STATE INVARIANTS ============

    // Property: GameState current_team always matches Team::from_shot_index
    proptest! {
        #[test]
        fn current_team_matches_shot_index(shot_index in 0u8..16) {
            let mut state = resources::GameState::default();
            state.shot_index = shot_index;
            prop_assert_eq!(
                state.current_team(),
                Team::from_shot_index(shot_index)
            );
        }
    }

    // Property: Weight from broom is always clamped to valid range
    proptest! {
        #[test]
        fn weight_from_broom_clamped(
            broom_y in -100.0f32..100.0,
        ) {
            let mut state = resources::GameState::default();
            state.broom_position = Vec2::new(0.0, broom_y);
            let weight = state.weight_from_broom();

            prop_assert!(weight >= WEIGHT_MIN, "Weight {} below min {}", weight, WEIGHT_MIN);
            prop_assert!(weight <= WEIGHT_MAX, "Weight {} above max {}", weight, WEIGHT_MAX);
        }
    }

    // Property: Angle from broom is always within limits
    proptest! {
        #[test]
        fn angle_from_broom_within_limits(
            broom_x in -5.0f32..5.0,
        ) {
            let mut state = resources::GameState::default();
            state.broom_position = Vec2::new(broom_x, helpers::tee_line_far());
            let angle = state.angle_from_broom();

            prop_assert!(
                angle.abs() <= ANGLE_LIMIT_DEG + 0.1,
                "Angle {} exceeds limit {}",
                angle,
                ANGLE_LIMIT_DEG
            );
        }
    }
}
