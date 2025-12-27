//! # Game Resources
//!
//! This module contains all Bevy ECS resources that hold global game state.
//! Resources are shared state accessible by any system.
//!
//! ## Resources
//! - `GameState`: Core game state (phase, shots, parameters)
//! - `CameraState`: Camera positioning and transitions
//! - `TouchState`: Touch/mouse drag tracking
//! - `StoneAssets`: Cached mesh and material handles

use bevy::prelude::*;

use crate::components::{CurlDirection, Phase, ShotType, Team};
use crate::constants::{BACK_FROM_TEE, DELIVERY_START_Y, TEE_FROM_CENTER, WEIGHT_MAX, WEIGHT_MIN};
use crate::helpers::{back_line_far, hog_line_far};

// ============================================================================
// GAME STATE
// ============================================================================

/// Core game state resource tracking the current phase, shot, and parameters.
#[derive(Resource)]
pub struct GameState {
    /// Current phase of the shot.
    pub phase: Phase,
    /// Current shot index (0-15).
    pub shot_index: u8,
    /// Type of shot being played.
    pub shot_type: ShotType,
    /// Broom target position on the ice.
    pub broom_position: Vec2,
    /// Called shot angle in degrees.
    pub called_angle_deg: f32,
    /// Called shot weight (1-10 scale).
    pub called_weight: f32,
    /// Current aim angle in degrees.
    pub aim_angle_deg: f32,
    /// Current aim weight (1-10 scale).
    pub aim_weight: f32,
    /// Entity of the currently thrown stone, if any.
    pub thrown_stone: Option<Entity>,
    /// Time all stones have been still (for shot end detection).
    pub still_time: f32,
    /// Snapshot of stones before the throw (for FGZ rule).
    pub snapshot: Option<ShotSnapshot>,
    /// Curl direction for the next throw.
    pub curl_direction: CurlDirection,
    /// Current end number (1-based).
    pub current_end: u8,
    /// Team 1's cumulative score.
    pub team1_score: u32,
    /// Team 2's cumulative score.
    pub team2_score: u32,
    /// Team that throws first this end (opponent has hammer).
    pub first_throw_team: Team,
    /// Total number of ends in the game.
    pub total_ends: u8,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            phase: Phase::CallingShot,
            shot_index: 0,
            shot_type: ShotType::Draw,
            broom_position: Vec2::new(0.0, TEE_FROM_CENTER),
            called_angle_deg: 0.0,
            called_weight: ShotType::Draw.default_weight(),
            aim_angle_deg: 0.0,
            aim_weight: ShotType::Draw.default_weight(),
            thrown_stone: None,
            still_time: 0.0,
            snapshot: None,
            curl_direction: CurlDirection::default(),
            current_end: 1,
            team1_score: 0,
            team2_score: 0,
            first_throw_team: Team::One, // Will be randomized at startup
            total_ends: 8,
        }
    }
}

impl GameState {
    /// Returns the team that should throw the current shot.
    ///
    /// Takes into account which team throws first this end (first_throw_team).
    pub fn current_team(&self) -> Team {
        // Even shots (0, 2, 4...) are thrown by first_throw_team
        // Odd shots (1, 3, 5...) are thrown by the opponent
        if self.shot_index % 2 == 0 {
            self.first_throw_team
        } else {
            self.first_throw_team.opponent()
        }
    }

    /// Resets state for a new end while preserving scores.
    ///
    /// Call this after scoring an end to set up for the next end.
    pub fn reset_for_new_end(&mut self) {
        self.phase = Phase::CallingShot;
        self.shot_index = 0;
        self.shot_type = ShotType::Draw;
        self.broom_position = Vec2::new(0.0, TEE_FROM_CENTER);
        self.called_angle_deg = 0.0;
        self.called_weight = ShotType::Draw.default_weight();
        self.aim_angle_deg = 0.0;
        self.aim_weight = ShotType::Draw.default_weight();
        self.thrown_stone = None;
        self.still_time = 0.0;
        self.snapshot = None;
        self.curl_direction = CurlDirection::default();
    }

    /// Calculate throw angle from broom position.
    ///
    /// The angle is computed from the delivery start point to the broom position.
    pub fn angle_from_broom(&self) -> f32 {
        let start = Vec2::new(0.0, DELIVERY_START_Y);
        let direction = self.broom_position - start;
        direction.x.atan2(direction.y).to_degrees()
    }

    /// Calculate weight (1-10 scale) from broom Y position.
    ///
    /// Forward (closer to back line) = higher weight (harder throw).
    /// Back (closer to hog line) = lower weight (softer throw).
    pub fn weight_from_broom(&self) -> f32 {
        let min_y = hog_line_far();
        let max_y = back_line_far();
        let range = max_y - min_y;
        let normalized = ((self.broom_position.y - min_y) / range).clamp(0.0, 1.0);
        WEIGHT_MIN + normalized * (WEIGHT_MAX - WEIGHT_MIN)
    }
}

// ============================================================================
// CAMERA STATE
// ============================================================================

/// Camera state resource managing view mode and transitions.
#[derive(Resource)]
pub struct CameraState {
    /// Current camera mode.
    pub mode: crate::components::CameraMode,
    /// Target position the camera is transitioning to.
    pub target_position: Vec3,
    /// Target look-at point for camera orientation.
    pub target_look_at: Vec3,
    /// Progress of current transition (0.0 = start, 1.0 = complete).
    pub transition_progress: f32,
    /// Duration of the current transition in seconds.
    pub transition_duration: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            mode: crate::components::CameraMode::SkipView,
            target_position: Vec3::new(0.0, TEE_FROM_CENTER + BACK_FROM_TEE + 2.0, 1.7),
            target_look_at: Vec3::new(0.0, TEE_FROM_CENTER, 0.0),
            transition_progress: 1.0,
            transition_duration: 0.5,
        }
    }
}

// ============================================================================
// TOUCH STATE
// ============================================================================

/// Touch/mouse input state for drag operations.
#[derive(Resource, Default)]
pub struct TouchState {
    /// Whether a drag is currently active.
    pub dragging: bool,
    /// Start position of current drag (screen coordinates).
    pub drag_start: Vec2,
    /// Current/last drag position (screen coordinates).
    pub drag_current: Vec2,
    /// Time when drag started.
    pub drag_start_time: f32,
}

// ============================================================================
// STONE ASSETS
// ============================================================================

/// Cached mesh and material handles for spawning stones.
#[derive(Resource)]
pub struct StoneAssets {
    /// Scene handle for Team 1 stones (loaded from GLB). Default: red.
    pub red_scene: Handle<Scene>,
    /// Scene handle for Team 2 stones (loaded from GLB). Default: yellow.
    pub yellow_scene: Handle<Scene>,
    /// Debug mesh (cylinder) showing physics collider bounds.
    #[cfg(feature = "debug_mode")]
    pub debug_mesh: Handle<Mesh>,
    /// Semi-transparent material for debug cylinder.
    #[cfg(feature = "debug_mode")]
    pub debug_material: Handle<StandardMaterial>,
}

// ============================================================================
// MODEL TUNING (DEBUG)
// ============================================================================

/// Debug resource for tuning the GLB model transform.
///
/// Allows live adjustment of scale and Z offset via UI sliders.
#[derive(Resource)]
pub struct ModelTuning {
    /// Uniform scale factor for the model.
    pub scale: f32,
    /// Z offset (height above physics body).
    pub z_offset: f32,
}

impl Default for ModelTuning {
    fn default() -> Self {
        Self {
            scale: 0.285,   // Tuned to match physics collider
            z_offset: 0.10, // Tuned to align with ice surface
        }
    }
}

// ============================================================================
// SNAPSHOT TYPES
// ============================================================================

/// Snapshot of a single stone's state before a throw.
///
/// Used to detect Free Guard Zone violations.
#[derive(Clone)]
pub struct StoneSnapshot {
    /// Entity ID of the stone.
    pub entity: Entity,
    /// Team that owns the stone.
    pub team: Team,
    /// Position of the stone.
    pub position: Vec2,
    /// Whether the stone was in the Free Guard Zone.
    pub in_fgz: bool,
}

/// Snapshot of all stones before a throw.
#[derive(Clone)]
pub struct ShotSnapshot {
    /// All stones on the ice before the throw.
    pub stones: Vec<StoneSnapshot>,
    /// Whether FGZ rules are active for this shot.
    pub fgz_active: bool,
}
