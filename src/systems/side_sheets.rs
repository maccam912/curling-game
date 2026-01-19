//! # Side Sheets Systems
//!
//! Systems for spawning decorative side sheets as visual ambiance.
//! These are empty sheets showing only ice, house markings, and lines.
//! Reuses spawn helpers from helpers.rs.

use bevy::math::primitives::Cuboid;
use bevy::prelude::*;

use crate::components::*;
use crate::constants::*;
use crate::helpers::{
    hog_line_far, hog_line_near, spawn_house_ring_at_offset, spawn_line_at_offset, tee_line_far,
};

// ============================================================================
// SETUP SYSTEM
// ============================================================================

/// Sets up the decorative side sheets (empty - just visuals, no games).
pub fn setup_side_sheets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sheet IDs: -2, -1 for left, 1, 2 for right (0 is main sheet)
    let sheet_ids: [i32; 4] = [-2, -1, 1, 2];

    // Z-values matching main sheet (from setup.rs)
    const BASE_Z: f32 = 0.0;
    const RING_Z_12: f32 = 0.001;
    const RING_Z_8: f32 = 0.002;
    const RING_Z_4: f32 = 0.003;
    const RING_Z_BUTTON: f32 = 0.004;
    const LINE_Z: f32 = 0.005;
    const TEE_LINE_Z: f32 = 0.006;
    const CENTER_LINE_Z: f32 = 0.007;

    // Materials (same as main sheet)
    let line_black = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 0.0),
        ..default()
    });
    let line_red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        ..default()
    });
    let line_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.8),
        ..default()
    });
    let ring_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.2, 0.8),
        ..default()
    });
    let ring_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        ..default()
    });
    let ring_red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        ..default()
    });
    let ice_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.92, 0.95, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.0,
        ..default()
    });
    let base_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        ..default()
    });

    for &sheet_id in &sheet_ids {
        let x_offset = sheet_id as f32 * SIDE_SHEET_SPACING;
        let side_sheet = Some(sheet_id);

        // Ice layer
        let ice_mesh = meshes.add(Cuboid::new(SHEET_WIDTH, SHEET_LENGTH, SHEET_THICKNESS));
        commands.spawn((
            Mesh3d(ice_mesh),
            MeshMaterial3d(ice_material.clone()),
            Transform::from_xyz(x_offset, 0.0, -SHEET_THICKNESS * 0.5),
            SideSheetElement(sheet_id),
            GameSceneElement,
        ));

        // Base white layer
        let base_mesh = meshes.add(Cuboid::new(SHEET_WIDTH, SHEET_LENGTH, 0.001));
        commands.spawn((
            Mesh3d(base_mesh),
            MeshMaterial3d(base_white.clone()),
            Transform::from_xyz(x_offset, 0.0, BASE_Z),
            SideSheetElement(sheet_id),
            GameSceneElement,
        ));

        // Center Line
        spawn_line_at_offset(
            &mut commands,
            &mut meshes,
            line_blue.clone(),
            Vec2::ZERO,
            crate::helpers::back_line_far() * 2.0,
            true,
            0.02,
            CENTER_LINE_Z,
            x_offset,
            side_sheet,
        );

        // Hog lines
        for &y in &[hog_line_far(), hog_line_near()] {
            spawn_line_at_offset(
                &mut commands,
                &mut meshes,
                line_red.clone(),
                Vec2::new(0.0, y),
                SHEET_WIDTH,
                false,
                0.1,
                LINE_Z,
                x_offset,
                side_sheet,
            );
        }

        // Tee lines
        for &y in &[tee_line_far(), crate::helpers::tee_line_near()] {
            spawn_line_at_offset(
                &mut commands,
                &mut meshes,
                line_black.clone(),
                Vec2::new(0.0, y),
                SHEET_WIDTH,
                false,
                0.02,
                TEE_LINE_Z,
                x_offset,
                side_sheet,
            );
        }

        // Back lines
        for &y in &[
            crate::helpers::back_line_far(),
            crate::helpers::back_line_near(),
        ] {
            spawn_line_at_offset(
                &mut commands,
                &mut meshes,
                line_black.clone(),
                Vec2::new(0.0, y),
                SHEET_WIDTH,
                false,
                0.02,
                LINE_Z,
                x_offset,
                side_sheet,
            );
        }

        // Houses (far end only for side sheets - the visible end)
        let y = tee_line_far();
        spawn_house_ring_at_offset(
            &mut commands,
            &mut meshes,
            ring_blue.clone(),
            HOUSE_RADIUS_12,
            y,
            RING_Z_12,
            x_offset,
            side_sheet,
        );
        spawn_house_ring_at_offset(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_8,
            y,
            RING_Z_8,
            x_offset,
            side_sheet,
        );
        spawn_house_ring_at_offset(
            &mut commands,
            &mut meshes,
            ring_red.clone(),
            HOUSE_RADIUS_4,
            y,
            RING_Z_4,
            x_offset,
            side_sheet,
        );
        spawn_house_ring_at_offset(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_BUTTON,
            y,
            RING_Z_BUTTON,
            x_offset,
            side_sheet,
        );

        // Edge lines (black lines on the sides of the sheet to delineate it)
        const EDGE_LINE_Z: f32 = 0.008; // Above everything
        let half_width = SHEET_WIDTH * 0.5;
        // Left edge
        spawn_line_at_offset(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(-half_width, 0.0),
            SHEET_LENGTH,
            true,
            0.03,
            EDGE_LINE_Z,
            x_offset,
            side_sheet,
        );
        // Right edge
        spawn_line_at_offset(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(half_width, 0.0),
            SHEET_LENGTH,
            true,
            0.03,
            EDGE_LINE_Z,
            x_offset,
            side_sheet,
        );
    }

    tracing::info!("Set up {} decorative side sheets", sheet_ids.len());
}

// ============================================================================
// PLACEHOLDER SYSTEMS (no-ops since no AI games)
// ============================================================================

/// Placeholder - no AI games on side sheets.
pub fn update_side_sheet_games() {
    // No-op: side sheets are just empty visual ambiance
}

/// Placeholder - no physics on side sheets.
pub fn update_side_sheet_physics() {
    // No-op: no stones on side sheets
}

/// Placeholder velocity component (kept for API compatibility).
#[derive(Component)]
pub struct SideSheetStoneVelocity(pub Vec2);
