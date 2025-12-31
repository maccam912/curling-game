//! # Game Constants
//!
//! This module contains all the physical and game-related constants used throughout
//! the curling game. Constants are organized by category for easy reference.
//!
//! ## Categories
//! - **Sheet Dimensions**: Physical dimensions of the curling sheet
//! - **House Dimensions**: Radii of the scoring house rings
//! - **Stone Properties**: Physical properties of curling stones
//! - **Physics Constants**: Ice friction, curl behavior
//! - **Game Settings**: Shot limits, input rates, timing

// ============================================================================
// SHEET DIMENSIONS
// ============================================================================

/// Total length of the curling sheet in meters (150 ft).
pub const SHEET_LENGTH: f32 = 45.72;

/// Width of the curling sheet in meters (~15 ft 7 in).
pub const SHEET_WIDTH: f32 = 4.75;

/// Distance from sheet center to the tee line in meters (57 ft).
pub const TEE_FROM_CENTER: f32 = 17.375;

/// Distance from tee to back line in meters (6 ft).
pub const BACK_FROM_TEE: f32 = 1.829;

/// Distance from tee to hog line in meters (21 ft).
pub const HOG_FROM_TEE: f32 = 6.40;

/// Distance from back line to hack in meters (6 ft).
pub const HACK_FROM_BACK: f32 = 1.829;

/// Height/thickness of line markings on the ice.
pub const LINE_HEIGHT: f32 = 0.001;

/// Thickness of the ice sheet mesh for rendering.
pub const SHEET_THICKNESS: f32 = 0.1;

// ============================================================================
// HOUSE DIMENSIONS
// ============================================================================

/// Radius of the 12-foot ring (outer ring) in meters (6 ft radius).
pub const HOUSE_RADIUS_12: f32 = 1.829;

/// Radius of the 8-foot ring in meters (4 ft radius).
pub const HOUSE_RADIUS_8: f32 = 1.219;

/// Radius of the 4-foot ring in meters (2 ft radius).
pub const HOUSE_RADIUS_4: f32 = 0.610;

/// Radius of the button (center circle) in meters (~6 inches).
pub const HOUSE_RADIUS_BUTTON: f32 = 0.15;

// ============================================================================
// STONE PROPERTIES
// ============================================================================

/// Radius of a curling stone in meters (~5.7 inches).
pub const STONE_RADIUS: f32 = 0.145;

/// Height of a curling stone in meters (~4.5 inches).
pub const STONE_HEIGHT: f32 = 0.114;

/// Total number of shots per end (8 per team, 16 total).
pub const TOTAL_SHOTS: u8 = 16;

// ============================================================================
// PHYSICS CONSTANTS
// ============================================================================

/// Minimum throwing speed in m/s (light draw weight).
///
/// Based on real curling research: typical shots range 2.0-3.5 m/s.
pub const WEIGHT_MIN_SPEED: f32 = 2.0;

/// Maximum throwing speed in m/s (hard takeout weight).
pub const WEIGHT_MAX_SPEED: f32 = 3.5;

/// Ice friction deceleration in m/s².
///
/// Derived from friction coefficient μ ≈ 0.012: deceleration = μ × g ≈ 0.115 m/s².
pub const ICE_FRICTION_DECEL: f32 = 0.115;

/// Initial angular velocity for stone curl physics in rad/s.
///
/// This value affects how much lateral curl force is applied.
/// Stones typically rotate 2-3 times over their travel distance.
pub const CURL_ANGULAR_VELOCITY: f32 = 1.5;

/// Visual rotation speed for stones in rad/s.
///
/// In real curling, stones complete roughly 2-3 rotations over ~25 seconds of travel,
/// which is about 1 rotation every 8-10 seconds (~0.63-0.79 rad/s).
/// A full rotation = 2π rad, so 0.7 rad/s = ~9 seconds per rotation.
pub const VISUAL_ROTATION_SPEED: f32 = 0.7;

/// Damping factor for visual rotation when stone is at rest.
///
/// Higher values = faster damping. At 5.0, a stone will damp from full
/// rotation speed to near-zero in roughly 1 second (exponential decay).
pub const VISUAL_ROTATION_DAMPING: f32 = 5.0;

/// Coefficient for lateral curl force per rad/s of rotation.
///
/// Controls how much the stone curves based on its rotation speed.
pub const CURL_COEFFICIENT: f32 = 0.008;

/// Speed threshold below which a stone is considered stopped (m/s).
pub const STOP_SPEED: f32 = 0.02;

/// Time in seconds that stones must be still before resolving the shot.
pub const STOP_HOLD_SECS: f32 = 0.5;

// ============================================================================
// INPUT SETTINGS
// ============================================================================

/// Maximum angle deviation from center in degrees.
pub const ANGLE_LIMIT_DEG: f32 = 8.0;

/// Rate of angle change when adjusting aim in degrees per second.
pub const ANGLE_RATE_DEG: f32 = 16.0;

/// Minimum weight value on the throw scale (1-10).
pub const WEIGHT_MIN: f32 = 1.0;

/// Maximum weight value on the throw scale (1-10).
pub const WEIGHT_MAX: f32 = 10.0;

/// Rate of weight change when adjusting throw power per second.
pub const WEIGHT_RATE: f32 = 6.0;

// ============================================================================
// DERIVED CONSTANTS
// ============================================================================

/// Y-coordinate where stone delivery starts (near the hack).
pub const DELIVERY_START_Y: f32 = -TEE_FROM_CENTER - BACK_FROM_TEE;

// ============================================================================
// MODEL CONSTANTS
// ============================================================================

/// Scale factor for GLB stone models to match physics collider.
pub const MODEL_SCALE: f32 = 0.285;

/// Z offset for GLB stone models to align with ice surface.
pub const MODEL_Z_OFFSET: f32 = 0.10;

// ============================================================================
// CAMERA CONSTANTS
// ============================================================================

/// Height of camera during ThrowingView phase (lowered for immersion).
pub const THROWING_VIEW_HEIGHT: f32 = 1.0;

/// Distance behind delivery start for ThrowingView camera.
pub const THROWING_VIEW_BEHIND: f32 = 1.5;

/// Initial height of camera during FollowStone phase.
pub const FOLLOW_START_HEIGHT: f32 = 0.8;

/// Final height camera rises to during FollowStone phase.
pub const FOLLOW_RISE_HEIGHT: f32 = 1.5;

/// Rate at which camera rises during FollowStone (meters per second).
pub const CAMERA_RISE_RATE: f32 = 0.15;

/// Height of HouseOverhead camera view.
pub const HOUSE_OVERHEAD_HEIGHT: f32 = 8.0;

/// Radius of the orbit path for game over camera.
pub const ORBIT_RADIUS: f32 = 8.0;

/// Height of the orbiting camera during game over.
pub const ORBIT_HEIGHT: f32 = 5.0;

/// Orbit speed in radians per second.
pub const ORBIT_SPEED: f32 = 0.3;

// ============================================================================
// RESPONSIVE VIEWPORT CONSTANTS
// ============================================================================

/// Mobile breakpoint width in logical pixels.
///
/// Viewports narrower than this are considered mobile.
pub const MOBILE_BREAKPOINT: f32 = 768.0;

/// Tablet breakpoint width in logical pixels.
///
/// Viewports between MOBILE_BREAKPOINT and this are considered tablet.
pub const TABLET_BREAKPOINT: f32 = 1024.0;

/// Minimum UI scale factor.
///
/// UI elements will never be scaled smaller than this.
pub const MIN_UI_SCALE: f32 = 0.6;

/// Maximum UI scale factor.
///
/// UI elements will never be scaled larger than this.
pub const MAX_UI_SCALE: f32 = 1.2;

/// Camera height multiplier for portrait mode.
///
/// Increases camera height to maintain field of view on narrow screens.
pub const PORTRAIT_CAMERA_SCALE: f32 = 1.5;

// ============================================================================
// UI DIMENSIONS
// ============================================================================

/// Standard button width in pixels.
pub const UI_BUTTON_WIDTH: f32 = 180.0;

/// Standard button height in pixels.
pub const UI_BUTTON_HEIGHT: f32 = 55.0;

/// Curl button (IN/OUT) width in pixels.
pub const UI_CURL_BUTTON_WIDTH: f32 = 60.0;

/// Curl button (IN/OUT) height in pixels.
pub const UI_CURL_BUTTON_HEIGHT: f32 = 50.0;

/// Z-offset for broom visual above ice surface.
pub const BROOM_Z_OFFSET: f32 = 0.05;

// ============================================================================
// AI CONSTANTS
// ============================================================================

/// Minimum AI "thinking" delay in seconds.
pub const AI_THINK_TIME_MIN: f32 = 1.0;

/// Maximum AI "thinking" delay in seconds.
pub const AI_THINK_TIME_MAX: f32 = 2.0;

/// Probability (0.0-1.0) that AI makes a suboptimal shot choice.
pub const AI_MISTAKE_CHANCE: f32 = 0.05;

/// Weight values for AI broom search grid.
///
/// AI searches this grid to find optimal weight for each shot.
pub const AI_WEIGHT_SEARCH_STEPS: [f32; 13] = [
    1.0, 2.0, 3.0, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 8.0, 9.0, 10.0,
];

/// X-offset values for AI broom search grid in meters.
///
/// AI searches these offsets to compensate for curl.
pub const AI_X_OFFSET_RANGE: [f32; 11] = [
    -2.0, -1.5, -1.0, -0.5, -0.25, 0.0, 0.25, 0.5, 1.0, 1.5, 2.0,
];

// ============================================================================
// UI COLORS (as SRGBA values)
// ============================================================================

/// Curl button background when selected.
pub const COLOR_CURL_SELECTED: (f32, f32, f32, f32) = (0.3, 0.5, 0.3, 0.9);

/// Curl button background when not selected.
pub const COLOR_CURL_DESELECTED: (f32, f32, f32, f32) = (0.2, 0.2, 0.3, 0.8);
