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

/// Initial angular velocity for stone curl in rad/s.
///
/// Stones typically rotate 2-3 times over their travel distance.
pub const CURL_ANGULAR_VELOCITY: f32 = 1.5;

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
