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

pub mod components;
pub mod constants;
pub mod helpers;
pub mod resources;
pub mod systems;

// Re-export commonly used items
pub use components::*;
pub use constants::*;
pub use resources::*;

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
            // Physics plugin
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
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
            .insert_resource(resources::ModelTuning::default())
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            // Startup systems
            .add_systems(
                Startup,
                (
                    systems::setup_scene,
                    systems::configure_rapier,
                    systems::setup_ui,
                    systems::randomize_first_team,
                ),
            )
            // Physics systems run at fixed rate for consistency
            .add_systems(FixedUpdate, systems::ice_friction_system)
            // Update systems
            .add_systems(
                Update,
                (
                    systems::update_window_title,
                    systems::handle_calling_input,
                    systems::handle_aiming_input,
                    systems::handle_touch_input,
                    systems::handle_broom_drag,
                    systems::update_broom_visual,
                    systems::track_throwing_stone,
                    systems::detect_stone_collision,
                    systems::check_out_of_bounds,
                    systems::detect_shot_end,
                    systems::resolve_shot,
                    systems::camera_control_system,
                    systems::update_ui,
                    systems::handle_tuning_buttons,
                    systems::apply_model_tuning,
                ),
            );

        // Debug-only systems
        #[cfg(feature = "debug_mode")]
        app.add_systems(Update, systems::handle_debug_quick_sim);
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
        assert_eq!(Team::from_shot_index(0), Team::Red);
        assert_eq!(Team::from_shot_index(1), Team::Blue);
        assert_eq!(Team::from_shot_index(2), Team::Red);
    }

    #[test]
    fn team_alternates_all_16_shots() {
        for i in 0..16u8 {
            let expected = if i % 2 == 0 { Team::Red } else { Team::Blue };
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
        assert_ne!(Team::Red.color(), Team::Blue.color());
    }

    #[test]
    fn team_names_are_correct() {
        assert_eq!(Team::Red.name(), "Red");
        assert_eq!(Team::Blue.name(), "Blue");
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
        let snap = test_stone_snapshot(Team::Red, 0.0, helpers::hog_line_far() + 1.0);
        assert_eq!(snap.team, Team::Red);
        assert_eq!(snap.position.x, 0.0);
        assert!(snap.in_fgz);

        let snap_in_house = test_stone_snapshot(Team::Blue, 0.0, helpers::tee_line_far());
        assert_eq!(snap_in_house.team, Team::Blue);
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
}
