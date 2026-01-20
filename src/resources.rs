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

use crate::components::{
    AimSkill, CurlDirection, Phase, PlayerPosition, ShotType, Team, WeightSkill,
};
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
    /// Pending end score during ShowingScore phase: (team1_points, team2_points).
    pub pending_end_score: Option<(u32, u32)>,
    /// Entities of stones that count toward the score (for highlighting).
    pub scoring_entities: Vec<Entity>,
    /// History of scores for each end: Vec<(team1_points, team2_points)>.
    pub end_scores: Vec<(u32, u32)>,
    /// Which team is controlled by AI (None = human vs human).
    pub ai_team: Option<Team>,
    /// Timer for AI "thinking" delay before executing a shot.
    pub ai_think_timer: f32,
    /// If true, both teams are AI-controlled (spectator mode).
    pub ai_vs_ai: bool,
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
            pending_end_score: None,
            scoring_entities: Vec::new(),
            end_scores: Vec::new(),
            ai_team: None,
            ai_think_timer: 0.0,
            ai_vs_ai: false,
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
    /// Current height of follow camera (rises during FollowStone phase).
    pub follow_camera_height: f32,
    /// Whether the thrown stone has crossed the far hog line.
    pub stone_crossed_hog: bool,
    /// Current orbit angle for game over camera (radians).
    pub orbit_angle: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        use crate::constants::FOLLOW_START_HEIGHT;
        Self {
            mode: crate::components::CameraMode::SkipView,
            target_position: Vec3::new(0.0, TEE_FROM_CENTER + BACK_FROM_TEE + 2.0, 1.7),
            target_look_at: Vec3::new(0.0, TEE_FROM_CENTER, 0.0),
            transition_progress: 1.0,
            transition_duration: 0.5,
            follow_camera_height: FOLLOW_START_HEIGHT,
            stone_crossed_hog: false,
            orbit_angle: 0.0,
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

// ============================================================================
// ONLINE STATE
// ============================================================================

/// State for online multiplayer sessions.
#[derive(Resource)]
pub struct OnlineState {
    /// Room code for this session (e.g., "ABC1").
    pub room_code: String,
    /// Whether we are hosting (created the room) or joining.
    pub is_host: bool,
    /// Whether the opponent has connected.
    pub opponent_connected: bool,
    /// Whether the opponent has disconnected (connection lost during game).
    pub opponent_disconnected: bool,
    /// Room code being typed by the user when joining.
    pub input_room_code: String,
    /// Which team the local player is (Host = Team1, Guest = Team2).
    pub local_team: Option<crate::components::Team>,
    /// Pending shot from opponent (received via network, waiting to be applied).
    pub pending_shot: Option<PendingShot>,
    /// Pending stone positions from opponent (after their shot resolved).
    pub pending_positions: Option<Vec<(crate::components::Team, f32, f32)>>,
    /// Timer for periodic sync during stone movement (fires every 1.0s).
    pub sync_timer: bevy::time::Timer,
    /// Pending periodic sync positions (during opponent's shot).
    pub pending_periodic_sync: Option<Vec<(crate::components::Team, f32, f32)>>,
    /// Pending broom position from opponent (during their calling/aiming).
    pub pending_broom_position: Option<(f32, f32)>,
}

impl Default for OnlineState {
    fn default() -> Self {
        Self {
            room_code: String::new(),
            is_host: false,
            opponent_connected: false,
            opponent_disconnected: false,
            input_room_code: String::new(),
            local_team: None,
            pending_shot: None,
            pending_positions: None,
            sync_timer: bevy::time::Timer::from_seconds(1.0, bevy::time::TimerMode::Repeating),
            pending_periodic_sync: None,
            pending_broom_position: None,
        }
    }
}

/// A shot received from the opponent, waiting to be simulated.
#[derive(Clone)]
pub struct PendingShot {
    pub angle: f32,
    pub weight: f32,
    pub curl: crate::components::CurlDirection,
}

// ============================================================================
// PREDICTION STATE
// ============================================================================

/// State for ghost stone trajectory prediction.
///
/// This resource tracks the predicted final position of the current shot
/// and related confidence metrics.
#[derive(Resource, Default)]
pub struct PredictionState {
    /// Predicted final position of the current shot.
    pub predicted_position: Option<Vec2>,
    /// Confidence in the prediction (1.0 = high, lower = likely collision).
    /// Reduces when the predicted path intersects existing stones.
    pub confidence: f32,
    /// Whether the prediction is currently valid and should be displayed.
    pub is_valid: bool,
}

// ============================================================================
// LIGHT SETTINGS
// ============================================================================

/// Settings for the main directional light (runtime adjustable).
#[derive(Resource)]
pub struct LightSettings {
    /// Illuminance value for the directional light.
    pub illuminance: f32,
}

impl Default for LightSettings {
    fn default() -> Self {
        Self {
            illuminance: 50_000.0, // Bright default
        }
    }
}

// ============================================================================
// PLAYER PERSONALITIES
// ============================================================================

/// A single player's personality/skill profile.
#[derive(Clone, Debug)]
pub struct PlayerPersonality {
    /// Player's position on the team.
    pub position: PlayerPosition,
    /// Skill at controlling weight.
    pub weight_skill: WeightSkill,
    /// Skill at aiming.
    pub aim_skill: AimSkill,
}

impl PlayerPersonality {
    /// Creates a new player personality.
    pub fn new(position: PlayerPosition, weight_skill: WeightSkill, aim_skill: AimSkill) -> Self {
        Self {
            position,
            weight_skill,
            aim_skill,
        }
    }

    /// Returns the total skill score (higher = better player).
    pub fn total_score(&self) -> u8 {
        self.weight_skill.score() + self.aim_skill.score()
    }

    /// Returns a display string describing this player's skills.
    pub fn description(&self) -> String {
        format!(
            "{}: {}, {}",
            self.position.name(),
            self.weight_skill.name(),
            self.aim_skill.name()
        )
    }

    /// Applies weight variance based on skill and returns the adjusted weight.
    pub fn apply_weight_variance(&self, weight: f32, rng: &mut impl rand::Rng) -> f32 {
        use crate::constants::{WEIGHT_MAX, WEIGHT_MIN};

        let variance = match self.weight_skill {
            WeightSkill::Good => rng.random_range(-0.2..0.2),
            WeightSkill::Average => rng.random_range(-0.5..0.5),
            WeightSkill::Poor => rng.random_range(-1.0..1.0),
            WeightSkill::TendsHeavy => rng.random_range(0.3..1.0),
            WeightSkill::TendsLight => rng.random_range(-1.0..-0.3),
        };

        (weight + variance).clamp(WEIGHT_MIN, WEIGHT_MAX)
    }

    /// Applies aim variance based on skill and returns the adjusted angle.
    pub fn apply_aim_variance(&self, angle_deg: f32, rng: &mut impl rand::Rng) -> f32 {
        use crate::constants::ANGLE_LIMIT_DEG;

        let variance = match self.aim_skill {
            AimSkill::Good => rng.random_range(-0.5..0.5),
            AimSkill::Average => rng.random_range(-1.5..1.5),
            AimSkill::Poor => rng.random_range(-3.0..3.0),
            AimSkill::TendsWide => {
                // Wide = away from center, so bias in same direction as current angle
                let bias = rng.random_range(0.5..2.0);
                if angle_deg >= 0.0 { bias } else { -bias }
            }
            AimSkill::TendsNarrow => {
                // Narrow = toward center, so bias opposite to current angle
                let bias = rng.random_range(0.5..2.0);
                if angle_deg >= 0.0 { -bias } else { bias }
            }
        };

        (angle_deg + variance).clamp(-ANGLE_LIMIT_DEG, ANGLE_LIMIT_DEG)
    }
}

/// Resource holding all player personalities for both teams.
///
/// Each team has 4 players (Lead, Second, Third, Skip) with unique skill profiles.
/// Personalities are generated at game start and persist through the entire game.
#[derive(Resource)]
pub struct PlayerPersonalities {
    /// Team 1's players, ordered by position (Lead first, Skip last).
    pub team1: [PlayerPersonality; 4],
    /// Team 2's players, ordered by position (Lead first, Skip last).
    pub team2: [PlayerPersonality; 4],
}

impl Default for PlayerPersonalities {
    fn default() -> Self {
        Self {
            team1: [
                PlayerPersonality::new(
                    PlayerPosition::Lead,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(
                    PlayerPosition::Second,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(
                    PlayerPosition::Third,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(PlayerPosition::Skip, WeightSkill::Good, AimSkill::Good),
            ],
            team2: [
                PlayerPersonality::new(
                    PlayerPosition::Lead,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(
                    PlayerPosition::Second,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(
                    PlayerPosition::Third,
                    WeightSkill::Average,
                    AimSkill::Average,
                ),
                PlayerPersonality::new(PlayerPosition::Skip, WeightSkill::Good, AimSkill::Good),
            ],
        }
    }
}

impl PlayerPersonalities {
    /// Gets the personality of the player throwing the current shot.
    ///
    /// Uses shot_index (0-15) and first_throw_team to determine which player is throwing.
    pub fn current_thrower(&self, shot_index: u8, first_throw_team: Team) -> &PlayerPersonality {
        // Determine which team is throwing
        let throwing_team = if shot_index % 2 == 0 {
            first_throw_team
        } else {
            first_throw_team.opponent()
        };

        // Determine which stone number this is for the team (0-7)
        // Even shots: team1 throws 0,2,4,6,8,10,12,14 -> stone 0,1,2,3,4,5,6,7
        // So team_stone_index = shot_index / 2 for first_throw_team
        let team_stone_index = shot_index / 2;

        // Get position index (0=Lead throws stones 0-1, 1=Second throws 2-3, etc.)
        let position_index = (team_stone_index / 2) as usize;
        let position_index = position_index.min(3); // Clamp to valid range

        match throwing_team {
            Team::One => &self.team1[position_index],
            Team::Two => &self.team2[position_index],
        }
    }
}

// ============================================================================
// SIDE SHEET GAMES
// ============================================================================

/// State for a single AI-vs-AI game on a decorative side sheet.
#[derive(Clone)]
pub struct SideGameState {
    /// Which side sheet this game is on (-2, -1, 1, 2).
    pub sheet_id: i32,
    /// X offset for this sheet in world coordinates.
    pub x_offset: f32,
    /// Current phase of the game.
    pub phase: Phase,
    /// Current shot index (0-15).
    pub shot_index: u8,
    /// Team that throws first this end.
    pub first_throw_team: Team,
    /// Current end number.
    pub current_end: u8,
    /// Positions of all stones on the sheet (local coordinates relative to sheet center).
    pub stones: Vec<(Team, Vec2, CurlDirection, f32)>, // (team, pos, curl, angular_vel)
    /// AI thinking timer.
    pub ai_timer: bevy::time::Timer,
    /// Pending shot parameters (angle, weight, curl) when throwing.
    pub pending_throw: Option<(f32, f32, CurlDirection)>,
    /// Entity of the currently moving stone.
    pub thrown_stone_entity: Option<Entity>,
    /// Time stones have been still.
    pub still_time: f32,
}

impl SideGameState {
    /// Creates a new side game state for the given sheet ID.
    pub fn new(sheet_id: i32, x_offset: f32) -> Self {
        use crate::constants::{SIDE_SHEET_AI_THINK_MAX, SIDE_SHEET_AI_THINK_MIN};
        use rand::Rng;

        let mut rng = rand::rng();
        let initial_delay = rng.random_range(SIDE_SHEET_AI_THINK_MIN..SIDE_SHEET_AI_THINK_MAX);

        Self {
            sheet_id,
            x_offset,
            phase: Phase::CallingShot,
            shot_index: 0,
            first_throw_team: if rng.random_bool(0.5) {
                Team::One
            } else {
                Team::Two
            },
            current_end: 1,
            stones: Vec::new(),
            ai_timer: bevy::time::Timer::from_seconds(initial_delay, bevy::time::TimerMode::Once),
            pending_throw: None,
            thrown_stone_entity: None,
            still_time: 0.0,
        }
    }

    /// Returns the team that should throw the current shot.
    pub fn current_team(&self) -> Team {
        if self.shot_index % 2 == 0 {
            self.first_throw_team
        } else {
            self.first_throw_team.opponent()
        }
    }

    /// Resets for a new end.
    pub fn reset_for_new_end(&mut self) {
        self.phase = Phase::CallingShot;
        self.shot_index = 0;
        self.stones.clear();
        self.pending_throw = None;
        self.thrown_stone_entity = None;
        self.still_time = 0.0;
        self.current_end += 1;
        // Swap first throw team (scoring team throws last = has hammer)
        self.first_throw_team = self.first_throw_team.opponent();
    }
}

/// Resource holding all side sheet game states.
/// Each entry is (sheet_id, x_offset, game_state).
#[derive(Resource, Default)]
pub struct SideSheetGames {
    /// Game states for each side sheet: (sheet_id, x_offset, state).
    pub games: Vec<(i32, f32, GameState)>,
}
