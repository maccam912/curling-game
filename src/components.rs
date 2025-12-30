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
use serde::{Deserialize, Serialize};

use crate::constants::CURL_ANGULAR_VELOCITY;

// ============================================================================
// TEAM
// ============================================================================

/// Represents one of the two teams in curling.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Team {
    /// Team 1 (throws first on even-numbered shots). Default color: red.
    One,
    /// Team 2 (throws on odd-numbered shots). Default color: yellow.
    Two,
}

impl Team {
    /// Determines which team throws based on the shot index (0-15).
    ///
    /// Team 1 throws on even shots (0, 2, 4, ...), Team 2 on odd (1, 3, 5, ...).
    ///
    /// # Example
    /// ```
    /// use curling_game::components::Team;
    /// assert_eq!(Team::from_shot_index(0), Team::One);
    /// assert_eq!(Team::from_shot_index(1), Team::Two);
    /// ```
    pub fn from_shot_index(index: u8) -> Self {
        if index.is_multiple_of(2) {
            Team::One
        } else {
            Team::Two
        }
    }

    /// Returns the team's display color.
    pub fn color(self) -> Color {
        match self {
            Team::One => Color::srgb(0.85, 0.15, 0.15), // Red
            Team::Two => Color::srgb(0.95, 0.85, 0.15), // Yellow
        }
    }

    /// Returns the team's display name.
    pub fn name(self) -> &'static str {
        match self {
            Team::One => "Team 1",
            Team::Two => "Team 2",
        }
    }

    /// Returns the opposing team.
    pub fn opponent(self) -> Team {
        match self {
            Team::One => Team::Two,
            Team::Two => Team::One,
        }
    }
}

// ============================================================================
// GAME PHASE
// ============================================================================

/// Current phase of gameplay within a shot.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    /// Skip is calling the shot (selecting target and weight).
    #[default]
    CallingShot,
    /// Player is fine-tuning aim before throwing.
    Aiming,
    /// Stone has been released and is in motion.
    StoneMoving,
    /// All stones have stopped; applying rules.
    Resolve,
    /// Showing end score and highlighting scoring stones.
    ShowingScore,
    /// The game has completed (all ends played).
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
    /// Top-down overhead view of the far house (user-toggled).
    Overhead,
    /// View from behind the stone during delivery.
    ThrowingView,
    /// Camera follows the moving stone.
    FollowStone,
    /// Overhead view of house for watching shot result.
    HouseOverhead,
    /// Orbiting camera for game over screen.
    GameOverOrbit,
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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
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
    /// Current angular velocity for physics curl (decays over time).
    pub angular_velocity: f32,
    /// Current visual rotation speed in rad/s.
    /// Spins at constant rate while moving, then damps to zero when at rest.
    pub visual_rotation_speed: f32,
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

/// Marker for the debug skip-to-8th-end button (only in debug_mode).
#[cfg(feature = "debug_mode")]
#[derive(Component)]
pub struct DebugSkipTo8thEndButton;

/// Marker for the confirm button's text child.
#[derive(Component)]
pub struct ConfirmButtonText;

/// Marker for the hammer indicator's text child.
#[derive(Component)]
pub struct HammerText;

/// Marker for the GLB model visual.
#[derive(Component)]
pub struct StoneVisual;

/// Marker component for the ghost stone prediction visual.
///
/// The ghost stone shows where the current shot is predicted to land
/// based on trajectory simulation.
#[derive(Component)]
pub struct GhostStone;

// ============================================================================
// HUD COMPONENTS
// ============================================================================

/// Marker for the main HUD panel container.
#[derive(Component)]
pub struct HudPanel;

/// Marker for the score display panel.
#[derive(Component)]
pub struct ScorePanel;

/// Marker for Team 1's score text.
#[derive(Component)]
pub struct Team1ScoreText;

/// Marker for Team 2's score text.
#[derive(Component)]
pub struct Team2ScoreText;

/// Marker for the current end indicator text.
#[derive(Component)]
pub struct EndInfoText;

/// Marker for the shot counter text.
#[derive(Component)]
pub struct ShotInfoText;

/// Marker for the shots remaining text.
#[derive(Component)]
pub struct ShotsRemainingText;

/// Marker for the hammer indicator.
#[derive(Component)]
pub struct HammerIndicator;

/// Marker for the team turn indicator.
#[derive(Component)]
pub struct TeamTurnIndicator;

/// Marker for the game phase indicator.
#[derive(Component)]
pub struct PhaseIndicator;

/// Marker for stones that count toward the score (applied during ShowingScore phase).
#[derive(Component)]
pub struct ScoringStone;

/// Marker for the score summary panel (shown during ShowingScore phase).
#[derive(Component)]
pub struct ScoreSummaryPanel;

/// Marker for the score summary text.
#[derive(Component)]
pub struct ScoreSummaryText;

/// Marker for the confirm score button.
#[derive(Component)]
pub struct ConfirmScoreButton;

/// Marker for the game over panel.
#[derive(Component)]
pub struct GameOverPanel;

/// Marker for the game over score breakdown text.
#[derive(Component)]
pub struct GameOverScoreBreakdown;

/// Marker for the game over winner text.
#[derive(Component)]
pub struct GameOverWinnerText;

// ============================================================================
// RESPONSIVE UI COMPONENTS
// ============================================================================

/// Marker for elements that should scale their size based on viewport.
///
/// The actual scaling is handled by the `apply_responsive_ui` system.
#[derive(Component)]
pub struct ResponsiveSize {
    /// Base width in pixels at desktop resolution.
    pub base_width: f32,
    /// Base height in pixels at desktop resolution.
    pub base_height: f32,
}

/// Marker for text elements that should scale font size based on viewport.
#[derive(Component)]
pub struct ResponsiveText {
    /// Base font size in pixels at desktop resolution.
    pub base_size: f32,
}

/// Marker for elements that should be hidden on mobile viewports.
#[derive(Component)]
pub struct HideOnMobile;

/// Marker for elements that should use compact layout on mobile.
///
/// This typically reduces padding and margins.
#[derive(Component)]
pub struct CompactOnMobile;

/// Marker for the bottom control panel (buttons row).
///
/// This panel may need to be repositioned on mobile.
#[derive(Component)]
pub struct BottomControlPanel;

// ============================================================================
// ONLINE GAME UI COMPONENTS
// ============================================================================

/// Marker for the connection status indicator in online games.
///
/// Shows green when connected, red when disconnected.
#[derive(Component)]
pub struct ConnectionStatusIndicator;

/// Marker for the disconnection overlay UI.
///
/// Shown when the opponent disconnects during a game.
#[derive(Component)]
pub struct DisconnectionOverlay;

/// Marker for the "Return to Menu" button on disconnection overlay.
#[derive(Component)]
pub struct DisconnectionReturnButton;
