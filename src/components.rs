//! # Game Components
//!
//! This module contains all Bevy ECS components, enums, and marker types
//! used to identify and categorize entities in the curling game.
//!
//! ## Component Categories
//! - **Game Enums**: `Team`, `Phase`, `ShotType`, `CurlDirection`
//! - **Stone Components**: `Stone`, `ThrowingStone`
//! - **Scene Components**: `Broom`, `MainCamera`
//! - **UI Components**: Marker components for UI elements

use bevy::prelude::*;

use crate::constants::CURL_ANGULAR_VELOCITY;

// ============================================================================
// TEAM
// ============================================================================

/// Represents one of the two teams in curling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    /// The red team (throws first on even-numbered shots).
    Red,
    /// The blue team (throws on odd-numbered shots).
    Blue,
}

impl Team {
    /// Determines which team throws based on the shot index (0-15).
    ///
    /// Red throws on even shots (0, 2, 4, ...), Blue on odd (1, 3, 5, ...).
    ///
    /// # Example
    /// ```
    /// use curling_game::components::Team;
    /// assert_eq!(Team::from_shot_index(0), Team::Red);
    /// assert_eq!(Team::from_shot_index(1), Team::Blue);
    /// ```
    pub fn from_shot_index(index: u8) -> Self {
        if index.is_multiple_of(2) {
            Team::Red
        } else {
            Team::Blue
        }
    }

    /// Returns the team's display color.
    pub fn color(self) -> Color {
        match self {
            Team::Red => Color::srgb(0.85, 0.15, 0.15),
            Team::Blue => Color::srgb(0.15, 0.3, 0.85),
        }
    }

    /// Returns the team's display name.
    pub fn name(self) -> &'static str {
        match self {
            Team::Red => "Red",
            Team::Blue => "Blue",
        }
    }

    /// Returns the opposing team.
    pub fn opponent(self) -> Team {
        match self {
            Team::Red => Team::Blue,
            Team::Blue => Team::Red,
        }
    }
}

// ============================================================================
// GAME PHASE
// ============================================================================

/// Current phase of gameplay within a shot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Skip is calling the shot (selecting target and weight).
    CallingShot,
    /// Player is fine-tuning aim before throwing.
    Aiming,
    /// Stone has been released and is in motion.
    StoneMoving,
    /// All stones have stopped; applying rules.
    Resolve,
    /// The end (set of 16 shots) has completed.
    Ended,
}

// ============================================================================
// CAMERA MODE
// ============================================================================

/// Camera view modes available during gameplay.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CameraMode {
    /// First-person skip view at far end looking at house.
    #[default]
    SkipView,
    /// Top-down overhead view of the far house.
    Overhead,
    /// View from behind the stone during delivery.
    ThrowingView,
    /// Camera follows the moving stone.
    FollowStone,
}

// ============================================================================
// SHOT TYPE
// ============================================================================

/// Types of curling shots with their default weights.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ShotType {
    /// Soft shot to place stone in the house.
    #[default]
    Draw,
    /// Place stone in front of house for protection.
    Guard,
    /// Remove opponent's stone with force.
    Takeout,
    /// Stop touching another stone.
    Freeze,
    /// Hit and deflect to a new position.
    HitAndRoll,
}

impl ShotType {
    /// Returns the default weight (1-10 scale) for this shot type.
    ///
    /// Lighter weights for placement shots, heavier for takeouts.
    pub fn default_weight(self) -> f32 {
        match self {
            ShotType::Draw => 4.0,
            ShotType::Guard => 3.5,
            ShotType::Takeout => 8.5,
            ShotType::Freeze => 4.5,
            ShotType::HitAndRoll => 7.0,
        }
    }

    /// Returns the display name for this shot type.
    pub fn name(self) -> &'static str {
        match self {
            ShotType::Draw => "Draw",
            ShotType::Guard => "Guard",
            ShotType::Takeout => "Takeout",
            ShotType::Freeze => "Freeze",
            ShotType::HitAndRoll => "Hit & Roll",
        }
    }
}

// ============================================================================
// CURL DIRECTION
// ============================================================================

/// Curl direction for stone delivery (rotation direction).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CurlDirection {
    /// Clockwise rotation - stone curls left as it slows.
    #[default]
    InTurn,
    /// Counter-clockwise rotation - stone curls right as it slows.
    OutTurn,
}

impl CurlDirection {
    /// Returns the initial angular velocity for this curl direction.
    ///
    /// Positive values indicate clockwise rotation (InTurn),
    /// negative values indicate counter-clockwise (OutTurn).
    pub fn angular_velocity(self) -> f32 {
        match self {
            CurlDirection::InTurn => CURL_ANGULAR_VELOCITY,
            CurlDirection::OutTurn => -CURL_ANGULAR_VELOCITY,
        }
    }

    /// Returns the short display name for this direction.
    pub fn name(self) -> &'static str {
        match self {
            CurlDirection::InTurn => "IN",
            CurlDirection::OutTurn => "OUT",
        }
    }
}

// ============================================================================
// STONE COMPONENTS
// ============================================================================

/// Component attached to all curling stones on the ice.
#[derive(Component)]
pub struct Stone {
    /// Which team owns this stone.
    pub team: Team,
    /// The curl direction this stone was thrown with.
    pub curl_direction: CurlDirection,
    /// Current angular velocity (decays over time).
    pub angular_velocity: f32,
}

/// Marker component for the stone currently being thrown.
///
/// Tracks delivery progress for hog line rule enforcement.
#[derive(Component)]
pub struct ThrowingStone {
    /// Maximum Y position reached during delivery (for hog line check).
    pub max_y: f32,
    /// Whether this stone has collided with another stone during delivery.
    pub hit_stone: bool,
}

// ============================================================================
// SCENE COMPONENTS
// ============================================================================

/// Marker component for the broom target indicator.
///
/// The broom shows where the skip is calling the shot to land.
#[derive(Component)]
pub struct Broom;

/// Marker component for the main game camera.
#[derive(Component)]
pub struct MainCamera;

// ============================================================================
// UI COMPONENTS
// ============================================================================

/// Marker for the root UI node.
#[derive(Component)]
pub struct UiRoot;

/// Marker for the confirm/throw button.
#[derive(Component)]
pub struct ConfirmButton;

/// Marker for the camera toggle button.
#[derive(Component)]
pub struct CameraToggleButton;

/// Marker for curl direction buttons, storing which direction it represents.
#[derive(Component)]
pub struct CurlButton(pub CurlDirection);

/// Marker for the status text display.
#[derive(Component)]
pub struct StatusText;

/// Marker for the debug quick-simulate button (only in debug_mode).
#[cfg(feature = "debug_mode")]
#[derive(Component)]
pub struct DebugQuickSimButton;

/// Marker for the GLB model visual (for tuning).
#[derive(Component)]
pub struct StoneVisual;

/// Marker for the scale slider.
#[derive(Component)]
pub struct ScaleSlider;

/// Marker for the Z offset slider.
#[derive(Component)]
pub struct ZOffsetSlider;

/// Marker for the scale value label.
#[derive(Component)]
pub struct ScaleValueLabel;

/// Marker for the Z offset value label.
#[derive(Component)]
pub struct ZOffsetValueLabel;

/// Direction for tuning adjustments.
#[derive(Component, Clone, Copy)]
pub enum TuningAdjust {
    Increase,
    Decrease,
}
