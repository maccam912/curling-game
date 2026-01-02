//! # Helper Functions
//!
//! This module contains utility functions used throughout the game:
//! - Line position calculations (hog lines, tee lines, back lines)
//! - Game rule predicates (out of bounds, free guard zone)
//! - Entity spawning helpers (stones, lines, house rings)

use bevy::math::primitives::{Cuboid, Cylinder};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;

use crate::components::{CurlDirection, Stone, Team, ThrowingStone};
use crate::constants::*;
use crate::resources::{ShotSnapshot, StoneAssets, StoneSnapshot};

// ============================================================================
// LINE POSITION FUNCTIONS
// ============================================================================

/// Returns the Y-coordinate of the far back line.
#[inline]
pub fn back_line_far() -> f32 {
    TEE_FROM_CENTER + BACK_FROM_TEE
}

/// Returns the Y-coordinate of the near back line.
#[inline]
pub fn back_line_near() -> f32 {
    -(TEE_FROM_CENTER + BACK_FROM_TEE)
}

/// Returns the Y-coordinate of the far tee line.
#[inline]
pub fn tee_line_far() -> f32 {
    TEE_FROM_CENTER
}

/// Returns the Y-coordinate of the near tee line.
#[inline]
pub fn tee_line_near() -> f32 {
    -TEE_FROM_CENTER
}

/// Returns the Y-coordinate of the far hog line.
#[inline]
pub fn hog_line_far() -> f32 {
    TEE_FROM_CENTER - HOG_FROM_TEE
}

/// Returns the Y-coordinate of the near hog line.
#[inline]
pub fn hog_line_near() -> f32 {
    -hog_line_far()
}

// ============================================================================
// GAME RULE PREDICATES
// ============================================================================

/// Checks if a position is in the Free Guard Zone.
///
/// The FGZ is the area between the hog line and house (12-foot ring),
/// not including stones inside the house.
///
/// # Arguments
/// * `position` - The position to check
///
/// # Returns
/// `true` if the position is in the FGZ
pub fn is_in_free_guard_zone(position: Vec2) -> bool {
    let in_hog_to_tee = position.y > hog_line_far() && position.y < tee_line_far();
    let dist_to_house = position.distance(Vec2::new(0.0, tee_line_far()));
    in_hog_to_tee && dist_to_house > HOUSE_RADIUS_12
}

/// Checks if a position is out of bounds.
///
/// Stones are out if they cross the side walls or back lines.
///
/// # Arguments
/// * `position` - The position to check
///
/// # Returns
/// `true` if the position is out of bounds
pub fn is_out_of_bounds(position: Vec2) -> bool {
    let half_width = SHEET_WIDTH * 0.5 + STONE_RADIUS;
    let back = back_line_far() + STONE_RADIUS;
    let near = back_line_near() - STONE_RADIUS;
    position.x.abs() > half_width || position.y > back || position.y < near
}

/// Checks if the stone crossed the near hog line during delivery.
///
/// # Arguments
/// * `max_y` - Maximum Y position reached by the stone
///
/// # Returns
/// `true` if the stone crossed the near hog line
pub fn hog_line_reached(max_y: f32) -> bool {
    max_y >= hog_line_near()
}

/// Checks if the stone fully crossed the far hog line.
///
/// The trailing edge of the stone must be past the line.
///
/// # Arguments
/// * `max_y` - Maximum Y position reached by the stone
///
/// # Returns
/// `true` if the stone fully crossed the far hog line
pub fn far_hog_line_reached(max_y: f32) -> bool {
    max_y > hog_line_far() + STONE_RADIUS
}

// ============================================================================
// SPAWN HELPERS
// ============================================================================

/// Spawns a curling stone entity.
///
/// # Arguments
/// * `commands` - Bevy command buffer
/// * `assets` - Stone mesh and material assets
/// * `team` - Which team owns this stone
/// * `position` - Starting position on the ice
/// * `initial_velocity` - Initial velocity vector
/// * `mark_throwing` - If true, adds `ThrowingStone` marker
/// * `curl_direction` - Direction of curl for physics
///
/// # Returns
/// The spawned stone's `Entity`
pub fn spawn_stone(
    commands: &mut Commands,
    assets: &StoneAssets,
    team: Team,
    position: Vec2,
    initial_velocity: Vec2,
    mark_throwing: bool,
    curl_direction: CurlDirection,
) -> Entity {
    let initial_angular_vel = curl_direction.angular_velocity();
    // Visual rotation direction: positive for InTurn (clockwise), negative for OutTurn
    let initial_visual_rotation = if curl_direction == CurlDirection::InTurn {
        VISUAL_ROTATION_SPEED
    } else {
        -VISUAL_ROTATION_SPEED
    };

    let stone_entity = commands
        .spawn((
            Stone {
                team,
                curl_direction,
                angular_velocity: initial_angular_vel,
                visual_rotation_speed: initial_visual_rotation,
            },
            RigidBody::Dynamic,
            Collider::ball(STONE_RADIUS),
            Velocity {
                linvel: initial_velocity,
                angvel: 0.0,
            },
            // Zero damping - we apply ice friction manually in ice_friction_system
            Damping {
                linear_damping: 0.0,
                angular_damping: 0.0,
            },
            Friction::coefficient(0.3),
            Restitution::coefficient(0.8),
            Transform::from_translation(Vec3::new(position.x, position.y, 0.0)),
            GlobalTransform::default(),
        ))
        .id();

    if mark_throwing {
        commands.entity(stone_entity).insert(ThrowingStone {
            max_y: position.y,
            hit_stone: false,
        });
    }

    // Spawn visual representation based on team using appropriate GLB model
    let scene = match team {
        Team::One => assets.red_scene.clone(),
        Team::Two => assets.yellow_scene.clone(),
    };
    // Apply model scale and z-offset from constants
    let model_transform = Transform::from_translation(Vec3::new(0.0, 0.0, MODEL_Z_OFFSET))
        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(MODEL_SCALE));

    commands.entity(stone_entity).with_children(|parent| {
        // GLB model (main stone above ice)
        parent.spawn((
            crate::components::StoneVisual,
            SceneRoot(scene),
            model_transform,
            RenderLayers::from_layers(&[0, STONE_LAYER]),
        ));

        // Debug cylinder showing physics collider bounds
        #[cfg(feature = "debug_mode")]
        {
            let debug_offset = Transform::from_translation(Vec3::new(0.0, 0.0, STONE_HEIGHT * 0.5))
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
            parent.spawn((
                Mesh3d(assets.debug_mesh.clone()),
                MeshMaterial3d(assets.debug_material.clone()),
                debug_offset,
            ));
        }
    });

    stone_entity
}

/// Spawns a restored guard stone (for FGZ violations).
///
/// The stone is placed stationary at the given position.
pub fn spawn_restored_guard(
    commands: &mut Commands,
    assets: &StoneAssets,
    team: Team,
    position: Vec2,
) {
    spawn_stone(
        commands,
        assets,
        team,
        position,
        Vec2::ZERO,
        false,
        CurlDirection::default(),
    );
}

/// Spawns a line on the ice sheet.
///
/// # Arguments
/// * `commands` - Bevy command buffer
/// * `meshes` - Mesh asset storage
/// * `material` - Material handle for the line
/// * `center` - Center position of the line
/// * `length` - Length of the line
/// * `along_y` - If true, line extends along Y axis; otherwise X axis
/// * `thickness` - Width/thickness of the line
/// * `z_pos` - Z position (depth below ice surface, should be negative)
pub fn spawn_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    center: Vec2,
    length: f32,
    along_y: bool,
    thickness: f32,
    z_pos: f32,
) {
    let (width, height) = if along_y {
        (thickness, length)
    } else {
        (length, thickness)
    };
    let mesh = meshes.add(Cuboid::new(width, height, LINE_HEIGHT));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(center.x, center.y, z_pos)),
    ));
}

/// Spawns a house ring (circular colored area).
///
/// # Arguments
/// * `commands` - Bevy command buffer
/// * `meshes` - Mesh asset storage
/// * `material` - Material handle for the ring
/// * `radius` - Radius of the ring
/// * `y_pos` - Y position (tee line coordinate)
/// * `z_pos` - Z position (layering height)
pub fn spawn_house_ring(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    radius: f32,
    y_pos: f32,
    z_pos: f32,
) {
    let ring_mesh = meshes.add(Cylinder::new(radius, 0.005));
    commands.spawn((
        Mesh3d(ring_mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, y_pos, z_pos)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));
}

// ============================================================================
// SNAPSHOT FUNCTIONS
// ============================================================================

/// Creates a snapshot of all stones on the ice before a throw.
///
/// Used to detect Free Guard Zone violations after the shot.
///
/// # Arguments
/// * `stones` - Query of all stones on the ice
/// * `shot_index` - Current shot index (FGZ is active for first 5 shots)
///
/// # Returns
/// A `ShotSnapshot` containing all stone positions and FGZ status
pub fn snapshot_stones(
    stones: &Query<(Entity, &Transform, &Stone)>,
    shot_index: u8,
) -> ShotSnapshot {
    let mut snapshots = Vec::new();
    for (entity, transform, stone) in stones.iter() {
        let position = Vec2::new(transform.translation.x, transform.translation.y);
        let in_fgz = is_in_free_guard_zone(position);
        snapshots.push(StoneSnapshot {
            entity,
            team: stone.team,
            position,
            in_fgz,
        });
    }
    ShotSnapshot {
        stones: snapshots,
        fgz_active: shot_index < 5,
    }
}

// ============================================================================
// SCORING FUNCTIONS
// ============================================================================

/// Calculates the score for an end based on stone positions.
///
/// In curling, only one team can score per end. The team with the stone
/// closest to the tee (button) scores one point for each of their stones
/// that is closer to the tee than the opponent's closest stone.
///
/// # Arguments
/// * `stones` - Query of all stones on the ice
///
/// # Returns
/// A tuple of (red_score, blue_score) for this end. One will always be 0.
/// If the house is empty, both will be 0 (blank end).
pub fn score_end(stones: &[(Team, Vec2)]) -> (u32, u32) {
    // Calculate distance from tee for each stone
    let tee = Vec2::new(0.0, tee_line_far());

    // Collect stones that are "biting" (touching) the house
    let mut scoring_stones: Vec<(Team, f32)> = stones
        .iter()
        .filter_map(|(team, pos)| {
            let dist = pos.distance(tee);
            // Stone must be biting the house (within 12-foot + stone radius)
            if dist <= HOUSE_RADIUS_12 + STONE_RADIUS {
                Some((*team, dist))
            } else {
                None
            }
        })
        .collect();

    // Sort by distance (closest first)
    scoring_stones.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    if scoring_stones.is_empty() {
        return (0, 0); // Blank end
    }

    // The team with the closest stone scores
    let scoring_team = scoring_stones[0].0;

    // Find the closest opponent stone distance (or infinity if none)
    let opponent_closest = scoring_stones
        .iter()
        .find(|(team, _)| *team != scoring_team)
        .map(|(_, dist)| *dist)
        .unwrap_or(f32::INFINITY);

    // Count scoring team's stones closer than opponent's closest
    let points = scoring_stones
        .iter()
        .filter(|(team, dist)| *team == scoring_team && *dist < opponent_closest)
        .count() as u32;

    match scoring_team {
        Team::One => (points, 0),
        Team::Two => (0, points),
    }
}
