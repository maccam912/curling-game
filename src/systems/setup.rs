//! # Setup Systems
//!
//! Systems that run during startup to initialize the game world.

use bevy::gltf::GltfAssetLabel;
use bevy::light::NotShadowCaster;
use bevy::math::primitives::{Cuboid, Cylinder};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::Rng;
use tracing::{debug, info};

use crate::components::*;
use crate::constants::*;
use crate::helpers::*;
use crate::resources::{GameState, PlayerPersonalities, PlayerPersonality, StoneAssets};

/// Randomizes which team throws first at game start.
///
/// This gives a fair 50/50 chance for either team to have hammer (throw last).
pub fn randomize_first_team(mut state: ResMut<GameState>) {
    let mut rng = rand::rng();
    state.first_throw_team = if rng.random_bool(0.5) {
        Team::One
    } else {
        Team::Two
    };
    info!(
        first_throw = state.first_throw_team.name(),
        hammer = state.first_throw_team.opponent().name(),
        "Randomized starting teams"
    );
}

/// Generates random player personalities for both teams.
///
/// Each team gets 4 players with random skill combinations.
/// Players are sorted so weaker players throw first (Lead) and
/// stronger players throw last (Skip).
pub fn generate_player_personalities(mut commands: Commands) {
    let mut rng = rand::rng();

    // All possible weight skills
    let weight_skills = [
        WeightSkill::Good,
        WeightSkill::Average,
        WeightSkill::Poor,
        WeightSkill::TendsHeavy,
        WeightSkill::TendsLight,
    ];

    // All possible aim skills
    let aim_skills = [
        AimSkill::Good,
        AimSkill::Average,
        AimSkill::Poor,
        AimSkill::TendsWide,
        AimSkill::TendsNarrow,
    ];

    // Generate random personalities for a team and sort by skill (worst first)
    let generate_team = |rng: &mut rand::prelude::ThreadRng| -> [PlayerPersonality; 4] {
        let positions = [
            PlayerPosition::Lead,
            PlayerPosition::Second,
            PlayerPosition::Third,
            PlayerPosition::Skip,
        ];

        // Generate 4 random skill combinations
        let mut players: Vec<(WeightSkill, AimSkill, u8)> = (0..4)
            .map(|_| {
                let w = weight_skills[rng.random_range(0..weight_skills.len())];
                let a = aim_skills[rng.random_range(0..aim_skills.len())];
                let score = w.score() + a.score();
                (w, a, score)
            })
            .collect();

        // Sort by score (ascending - worst first for Lead)
        players.sort_by_key(|p| p.2);

        // Assign positions
        [
            PlayerPersonality::new(positions[0], players[0].0, players[0].1),
            PlayerPersonality::new(positions[1], players[1].0, players[1].1),
            PlayerPersonality::new(positions[2], players[2].0, players[2].1),
            PlayerPersonality::new(positions[3], players[3].0, players[3].1),
        ]
    };

    let team1 = generate_team(&mut rng);
    let team2 = generate_team(&mut rng);

    // Log the generated personalities
    info!("Generated player personalities:");
    info!("Team 1:");
    for p in &team1 {
        info!("  {}", p.description());
    }
    info!("Team 2:");
    for p in &team2 {
        info!("  {}", p.description());
    }

    commands.insert_resource(PlayerPersonalities { team1, team2 });
}

/// Configures the Rapier physics engine.
///
/// Sets gravity to zero since curling is played on a horizontal surface.
pub fn configure_rapier(mut config: Query<&mut RapierConfiguration, With<DefaultRapierContext>>) {
    for mut config in &mut config {
        config.gravity = Vec2::ZERO;
        debug!("Configured Rapier physics with zero gravity");
    }
}

/// Sets up the entire game scene.
///
/// This system runs once at startup and creates:
/// - Camera with skip view positioning
/// - Directional lighting
/// - Ice sheet surface
/// - All line markings (hog lines, tee lines, back lines, center line)
/// - Both houses with colored rings
/// - Stone assets (mesh and materials)
/// - Broom target indicator
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ice_materials: ResMut<Assets<crate::systems::ice_material::IceMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    info!("Setting up game scene");

    // Camera - start with SkipView (at far end looking at house)
    let skip_view_pos = Vec3::new(0.0, TEE_FROM_CENTER + BACK_FROM_TEE + 2.0, 1.7);
    let skip_view_look = Vec3::new(0.0, TEE_FROM_CENTER, 0.0);
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Transform::from_translation(skip_view_pos).looking_at(skip_view_look, Vec3::Z),
        // Main camera sees Layer 0 (default)
    ));
    debug!(position = ?skip_view_pos, "Spawned main camera");

    // Reflection Camera setup
    let reflection_size = Extent3d {
        width: 1024,
        height: 1024,
        ..default()
    };

    // Create the image that will be rendered to
    let mut reflection_image = Image::new_fill(
        reflection_size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    // Needed for using as a render target
    reflection_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;

    let reflection_image_handle = images.add(reflection_image);

    // Spawn the reflection camera
    // It captures only the stone layer
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: -1, // Render before main camera
            target: bevy::render::camera::RenderTarget::Image(reflection_image_handle.clone()),
            clear_color: Color::NONE.into(), // Transparent background
            ..default()
        },
        Transform::from_translation(skip_view_pos).looking_at(skip_view_look, Vec3::Z), // Initial pos, updated by system
        RenderLayers::layer(STONE_LAYER),
        ReflectionCamera,
    ));

    // Main directional light (overhead, for primary shadows)
    // Positioned centrally above the house for even shadow casting
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, TEE_FROM_CENTER, 30.0)
            .looking_at(Vec3::new(0.0, TEE_FROM_CENTER, 0.0), Vec3::Y),
    ));

    // Linear arrays of lights along both sides of the sheet
    // Simulates the rows of fluorescent fixtures in a curling club
    let side_light_height = 8.0;
    let side_light_x = 5.0; // Just outside the sheet width
    let light_spacing = 7.0; // Spacing between fixtures
    let num_lights = 8; // Lights per side
    let start_y = -SHEET_LENGTH * 0.4; // Start near delivery end

    for side in [-1.0, 1.0] {
        for i in 0..num_lights {
            let y_pos = start_y + (i as f32) * light_spacing;
            let pos = Vec3::new(side * side_light_x, y_pos, side_light_height);
            commands.spawn((
                PointLight {
                    intensity: 600_000.0, // Lumens - fluorescent fixture brightness
                    range: 25.0,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_translation(pos),
            ));
        }
        debug!(
            side = if side > 0.0 { "right" } else { "left" },
            count = num_lights,
            "Spawned side light array"
        );
    }

    // Four corner point lights for additional fill around the house
    let corner_light_height = 10.0;
    let corner_spread_x = 6.0;
    let corner_spread_y = 10.0;
    let house_y = TEE_FROM_CENTER;

    let corner_lights = [
        (1.0, corner_spread_y),   // NE - skip side, right
        (1.0, -corner_spread_y),  // SE - delivery side, right
        (-1.0, corner_spread_y),  // NW - skip side, left
        (-1.0, -corner_spread_y), // SW - delivery side, left
    ];

    for (x_sign, y_offset) in corner_lights {
        let pos = Vec3::new(
            x_sign * corner_spread_x,
            house_y + y_offset,
            corner_light_height,
        );
        commands.spawn((
            PointLight {
                intensity: 500_000.0,
                range: 30.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }
    debug!("Spawned 4 corner fill lights around house");

    // Mirrored light (below ice for reflections, no shadows)
    // Shines upward to match the flipped normals from the negative Y scale
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, TEE_FROM_CENTER, -30.0)
            .looking_at(Vec3::new(0.0, TEE_FROM_CENTER, 0.0), Vec3::Y),
    ));

    // Ambient light for base illumination
    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        ..default()
    });

    // Ice Sheet with pebbling texture
    // Must generate tangents for normal maps to work!
    let mut sheet_mesh_data = Mesh::from(Cuboid::new(SHEET_WIDTH, SHEET_LENGTH, SHEET_THICKNESS));
    sheet_mesh_data
        .generate_tangents()
        .expect("Failed to generate tangents for ice sheet");
    let sheet_mesh = meshes.add(sheet_mesh_data);

    // Generate smooth pebbling using value noise with bilinear interpolation
    // This creates natural-looking bumps instead of harsh per-pixel noise
    let texture_size = 256u32; // Larger texture = less visible tiling
    let grid_size = 32u32; // Random values at grid points, interpolate between
    let mut rng = rand::rng();

    // Generate random heights at grid points (0.0 to 1.0)
    let grid: Vec<f32> = (0..((grid_size + 1) * (grid_size + 1)))
        .map(|_| rng.random::<f32>())
        .collect();

    // Helper to get grid value with wrapping for seamless tiling
    let get_grid = |gx: u32, gy: u32| -> f32 {
        let gx = gx % (grid_size + 1);
        let gy = gy % (grid_size + 1);
        grid[(gy * (grid_size + 1) + gx) as usize]
    };

    // Bilinear interpolation helper
    let lerp = |a: f32, b: f32, t: f32| a + t * (b - a);
    let smoothstep = |t: f32| t * t * (3.0 - 2.0 * t); // Smoother interpolation

    // Generate heightmap with smooth interpolation
    let mut heightmap: Vec<f32> = Vec::with_capacity((texture_size * texture_size) as usize);
    let cell_size = texture_size as f32 / grid_size as f32;

    for py in 0..texture_size {
        for px in 0..texture_size {
            // Find which grid cell we're in
            let fx = px as f32 / cell_size;
            let fy = py as f32 / cell_size;
            let gx = fx as u32;
            let gy = fy as u32;

            // Fractional position within cell (0-1)
            let tx = smoothstep(fx - gx as f32);
            let ty = smoothstep(fy - gy as f32);

            // Get four corner values
            let v00 = get_grid(gx, gy);
            let v10 = get_grid(gx + 1, gy);
            let v01 = get_grid(gx, gy + 1);
            let v11 = get_grid(gx + 1, gy + 1);

            // Bilinear interpolation
            let v0 = lerp(v00, v10, tx);
            let v1 = lerp(v01, v11, tx);
            let height = lerp(v0, v1, ty);

            heightmap.push(height);
        }
    }

    // Generate normal map from heightmap using gradient
    let bump_strength = 0.3; // How pronounced the bumps appear
    let mut normal_data = Vec::with_capacity((texture_size * texture_size * 4) as usize);

    for py in 0..texture_size {
        for px in 0..texture_size {
            // Sample neighboring heights for gradient (with wrapping)
            let left = heightmap[((py * texture_size + (px + texture_size - 1) % texture_size)) as usize];
            let right = heightmap[((py * texture_size + (px + 1) % texture_size)) as usize];
            let up = heightmap[(((py + texture_size - 1) % texture_size * texture_size + px)) as usize];
            let down = heightmap[(((py + 1) % texture_size * texture_size + px)) as usize];

            // Gradient (derivative of height)
            let dx = (right - left) * bump_strength;
            let dy = (down - up) * bump_strength;

            // Convert gradient to normal (pointing mostly up)
            // Normal = normalize(-dx, -dy, 1)
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let nx = -dx / len;
            let ny = -dy / len;
            let nz = 1.0 / len;

            // Convert from [-1,1] to [0,255] range
            normal_data.push(((nx * 0.5 + 0.5) * 255.0) as u8);
            normal_data.push(((ny * 0.5 + 0.5) * 255.0) as u8);
            normal_data.push(((nz * 0.5 + 0.5) * 255.0) as u8);
            normal_data.push(255);
        }
    }

    let mut normal_image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        normal_data,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    normal_image.sampler = bevy::image::ImageSampler::Descriptor(
        bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..default()
        }
    );
    let normal_texture = images.add(normal_image);

    // Generate depth map from heightmap for parallax effect
    // White = bottom (low), Black = top (high) - inverted from heightmap
    let mut depth_data = Vec::with_capacity((texture_size * texture_size * 4) as usize);
    for &h in &heightmap {
        let depth = ((1.0 - h) * 255.0) as u8; // Invert: high points = dark
        depth_data.push(depth);
        depth_data.push(depth);
        depth_data.push(depth);
        depth_data.push(255);
    }

    let mut depth_image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        depth_data,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    // Use Nearest filtering for depth map (better performance per Bevy docs)
    depth_image.sampler = bevy::image::ImageSampler::Descriptor(
        bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            mag_filter: bevy::image::ImageFilterMode::Nearest,
            min_filter: bevy::image::ImageFilterMode::Nearest,
            ..default()
        }
    );
    let depth_texture = images.add(depth_image);

    // Use custom IceMaterial for planar reflections
    let sheet_material = ice_materials.add(crate::systems::ice_material::IceMaterial {
        base_color: LinearRgba::new(0.92, 0.95, 0.98, 0.15),
        reflection_texture: reflection_image_handle.clone(),
    });

    // NOTE: We lost the PBR properties (normal map, etc) by switching to a simple custom material.
    // If we wanted to keep them, we'd need ExtendedMaterial or a more complex shader.
    // For now, let's assume the user prioritizes the reflection effect.

    commands.spawn((
        Mesh3d(sheet_mesh),
        MeshMaterial3d(sheet_material),
        Transform::from_translation(Vec3::new(0.0, 0.0, -SHEET_THICKNESS * 0.5)),
    ));
    debug!(
        width = SHEET_WIDTH,
        length = SHEET_LENGTH,
        "Created ice sheet"
    );

    // Line Materials (lit so they receive shadows)
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

    // Z-depths for lines (negative = below ice surface)
    // Center line is shallowest - it runs through the house and should be visible
    const CENTER_LINE_Z: f32 = -0.001;
    const TEE_LINE_Z: f32 = -0.002;
    const OTHER_LINE_Z: f32 = -0.003;

    // Center Line (Back to Back) - shallowest so it's visible through house
    spawn_line(
        &mut commands,
        &mut meshes,
        line_blue.clone(),
        Vec2::ZERO,
        back_line_far() * 2.0,
        true,
        0.02,
        CENTER_LINE_Z,
    );

    // Hacks
    for &y in &[
        back_line_far() + HACK_FROM_BACK,
        back_line_near() - HACK_FROM_BACK,
    ] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(0.0, y),
            0.5,
            false,
            0.05,
            OTHER_LINE_Z,
        );
    }

    // Transverse Lines (back lines)
    for &y in &[back_line_far(), back_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.02,
            OTHER_LINE_Z,
        );
    }
    // Tee lines - slightly above house rings so they're visible
    for &y in &[tee_line_far(), tee_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.02,
            TEE_LINE_Z,
        );
    }
    // Hog lines
    for &y in &[hog_line_far(), hog_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_red.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.1,
            OTHER_LINE_Z,
        );
    }

    // White base layer under all paint (like real curling ice)
    let base_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        ..default()
    });
    let base_mesh = meshes.add(Cuboid::new(SHEET_WIDTH, SHEET_LENGTH, 0.001));
    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(base_white),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.008)), // Deepest layer
    ));

    // House Materials (lit so they receive shadows)
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

    // Draw Houses (Near and Far)
    // Z positions are negative to place rings below the ice surface
    // Larger rings are deeper so smaller rings paint over them
    for &y in &[tee_line_far(), tee_line_near()] {
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_blue.clone(),
            HOUSE_RADIUS_12,
            y,
            -0.006, // Largest ring, deepest
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_8,
            y,
            -0.005,
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_red.clone(),
            HOUSE_RADIUS_4,
            y,
            -0.004,
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_BUTTON,
            y,
            -0.003, // Smallest ring, shallowest
        );
    }
    debug!("Created houses at near and far ends");

    // Stone Assets - load GLB models for each team
    let red_scene: Handle<Scene> =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("red.glb"));
    let yellow_scene: Handle<Scene> =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("yellow.glb"));

    // Create debug mesh and material when debug_mode is enabled
    #[cfg(feature = "debug_mode")]
    let debug_mesh = meshes.add(Cylinder::new(STONE_RADIUS, STONE_HEIGHT));
    #[cfg(feature = "debug_mode")]
    let debug_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 1.0, 0.0, 0.3), // Semi-transparent green
        alpha_mode: bevy::render::alpha::AlphaMode::Blend,
        ..default()
    });

    commands.insert_resource(StoneAssets {
        red_scene,
        yellow_scene,
        #[cfg(feature = "debug_mode")]
        debug_mesh,
        #[cfg(feature = "debug_mode")]
        debug_material,
    });

    // Broom target indicator
    let broom_mesh = meshes.add(Cylinder::new(0.15, 0.02));
    let broom_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.9, 0.0),
        emissive: bevy::color::LinearRgba::new(0.5, 0.45, 0.0, 1.0),
        ..default()
    });
    commands.spawn((
        Broom,
        Mesh3d(broom_mesh),
        MeshMaterial3d(broom_material),
        Transform::from_xyz(0.0, TEE_FROM_CENTER, 0.05)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));
    debug!("Created broom indicator");

    // Ghost Stone prediction indicator
    let ghost_mesh = meshes.add(Cylinder::new(STONE_RADIUS, STONE_HEIGHT));
    let ghost_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.8, 1.0, 0.4), // Semi-transparent blue
        alpha_mode: bevy::render::alpha::AlphaMode::Blend,
        emissive: bevy::color::LinearRgba::new(0.1, 0.2, 0.4, 1.0),
        ..default()
    });
    commands.spawn((
        GhostStone,
        Mesh3d(ghost_mesh),
        MeshMaterial3d(ghost_material),
        Transform::from_xyz(0.0, TEE_FROM_CENTER, 0.15)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        Visibility::Hidden, // Start hidden until prediction is running
    ));
    debug!("Created ghost stone prediction indicator");

    info!("Game scene setup complete");
}

/// Sets up the UI elements.
///
/// Creates the UI hierarchy with:
/// - Status text at the top
/// - Camera toggle button
/// - Curl direction buttons (IN/OUT)
/// - Confirm/throw button
pub fn setup_ui(mut commands: Commands) {
    info!("Setting up UI");

    // Root UI node - full screen flex container
    commands
        .spawn((
            UiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Top HUD bar - Two rows, centered with flexbox wrapping
            parent
                .spawn((
                    HudPanel,
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ))
                .with_children(|hud| {
                    // ===== ROW 1: Scores + End Info (centered, wrapping) =====
                    hud.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|row1| {
                        // Score Panel (Team 1 score + hammer icon | Team 2 score + hammer icon)
                        row1.spawn((
                            ScorePanel,
                            CompactOnMobile,
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(10.0),
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                ..default()
                            },
                        ))
                        .with_children(|scores| {
                            // Team 1: color dot + score + hammer icon
                            scores
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(4.0),
                                    ..default()
                                })
                                .with_children(|team1| {
                                    // Team color indicator
                                    team1.spawn((
                                        Node {
                                            width: Val::Px(14.0),
                                            height: Val::Px(14.0),
                                            ..default()
                                        },
                                        BackgroundColor(Team::One.color()),
                                        BorderRadius::all(Val::Px(7.0)),
                                    ));
                                    // Score
                                    team1.spawn((
                                        Team1ScoreText,
                                        Text::new("0"),
                                        TextFont {
                                            font_size: 22.0,
                                            ..default()
                                        },
                                        TextColor(Team::One.color()),
                                    ));
                                    // Hammer icon (hidden by default, shown via update_ui)
                                    team1.spawn((
                                        Team1HammerIcon,
                                        Text::new("(H)"),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.8, 0.4)),
                                        Visibility::Hidden,
                                    ));
                                });

                            // Separator
                            scores.spawn((
                                Text::new("-"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                            ));

                            // Team 2: color dot + score + hammer icon
                            scores
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(4.0),
                                    ..default()
                                })
                                .with_children(|team2| {
                                    // Team color indicator
                                    team2.spawn((
                                        Node {
                                            width: Val::Px(14.0),
                                            height: Val::Px(14.0),
                                            ..default()
                                        },
                                        BackgroundColor(Team::Two.color()),
                                        BorderRadius::all(Val::Px(7.0)),
                                    ));
                                    // Score
                                    team2.spawn((
                                        Team2ScoreText,
                                        Text::new("0"),
                                        TextFont {
                                            font_size: 22.0,
                                            ..default()
                                        },
                                        TextColor(Team::Two.color()),
                                    ));
                                    // Hammer icon (hidden by default, shown via update_ui)
                                    team2.spawn((
                                        Team2HammerIcon,
                                        Text::new("(H)"),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.8, 0.4)),
                                        Visibility::Hidden,
                                    ));
                                });
                        });

                        // Separator
                        row1.spawn((
                            Text::new("|"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
                        ));

                        // End Info
                        row1.spawn((
                            EndInfoText,
                            Text::new("END 1/8"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));
                    });

                    // ===== ROW 2: Metadata (shot, team turn, personality, phase) =====
                    hud.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.0),
                            row_gap: Val::Px(4.0),
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|row2| {
                        // Shot counter
                        row2.spawn((
                            ShotInfoText,
                            Text::new("Shot 1/16"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));

                        // Separator
                        row2.spawn((
                            Text::new("•"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
                        ));

                        // Team turn indicator
                        row2.spawn((
                            TeamTurnIndicator,
                            Text::new("Team 1's Turn"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Team::One.color()),
                        ));

                        // Separator
                        row2.spawn((
                            Text::new("•"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
                        ));

                        // Phase indicator
                        row2.spawn((
                            PhaseIndicator,
                            Text::new("Calling Shot"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.6, 0.8, 0.6)),
                        ));

                        // Thrower info (position and skills) - hidden when empty
                        row2.spawn((
                            ThrowerInfoText,
                            Text::new(""),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.6)),
                        ));
                    });

                    // Hidden legacy elements for compatibility
                    // HammerIndicator (hidden, used for backward compat queries)
                    hud.spawn((
                        HammerIndicator,
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ))
                    .with_children(|hammer| {
                        hammer.spawn((
                            HammerText,
                            Text::new(""),
                            TextFont {
                                font_size: 1.0,
                                ..default()
                            },
                            TextColor(Color::NONE),
                        ));
                    });

                    // Hidden ShotsRemainingText for backward compat
                    hud.spawn((
                        ShotsRemainingText,
                        Text::new(""),
                        TextFont {
                            font_size: 1.0,
                            ..default()
                        },
                        TextColor(Color::NONE),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ));
                });

            // Legacy status text (kept for compatibility but can be hidden)
            parent.spawn((
                StatusText,
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)), // Hidden
            ));

            // Spacer to push buttons to bottom
            parent.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            // Bottom control area
            parent
                .spawn((
                    BottomControlPanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(15.0),
                        ..default()
                    },
                ))
                .with_children(|bottom| {
                    // Controls container - wraps on narrow screens
                    // Layout: [IN] [OUT] above [Confirm Shot] on narrow, side-by-side on wide
                    bottom
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|controls| {
                            // Curl buttons row (IN / OUT)
                            controls
                                .spawn((
                                    CurlButtonsRow,
                                    Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(10.0),
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                ))
                                .with_children(|curl_row| {
                                    // Curl IN button (selected by default)
                                    curl_row
                                        .spawn((
                                            CurlButton(CurlDirection::InTurn),
                                            Button,
                                            Node {
                                                width: Val::Px(60.0),
                                                height: Val::Px(50.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.9)),
                                            BorderRadius::all(Val::Px(10.0)),
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("IN"),
                                                TextFont {
                                                    font_size: 18.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });

                                    // Curl OUT button
                                    curl_row
                                        .spawn((
                                            CurlButton(CurlDirection::OutTurn),
                                            Button,
                                            Node {
                                                width: Val::Px(60.0),
                                                height: Val::Px(50.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                                            BorderRadius::all(Val::Px(10.0)),
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("OUT"),
                                                TextFont {
                                                    font_size: 18.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                });

                            // Confirm/throw button - full width, acts as minimum width anchor
                            controls
                                .spawn((
                                    ConfirmButton,
                                    Button,
                                    Node {
                                        width: Val::Px(180.0),
                                        min_width: Val::Px(140.0),
                                        height: Val::Px(55.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.6, 0.3, 0.9)),
                                    BorderRadius::all(Val::Px(10.0)),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        ConfirmButtonText,
                                        Text::new("Confirm Shot"),
                                        TextFont {
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                            // Debug buttons row (only in debug_mode)
                            #[cfg(feature = "debug_mode")]
                            controls
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(10.0),
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                })
                                .with_children(|debug_row| {
                                    // Debug quick-simulate button
                                    debug_row
                                        .spawn((
                                            DebugQuickSimButton,
                                            Button,
                                            Node {
                                                width: Val::Px(80.0),
                                                height: Val::Px(45.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.6, 0.3, 0.1, 0.9)),
                                            BorderRadius::all(Val::Px(8.0)),
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("QUICK\nSIM"),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });

                                    // Debug skip-to-8th-end button
                                    debug_row
                                        .spawn((
                                            DebugSkipTo8thEndButton,
                                            Button,
                                            Node {
                                                width: Val::Px(60.0),
                                                height: Val::Px(45.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.5, 0.2, 0.5, 0.9)),
                                            BorderRadius::all(Val::Px(8.0)),
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("END\n8"),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                });
                        });
                });
        });

    // Score Summary Panel (centered overlay, hidden by default)
    commands
        .spawn((
            ScoreSummaryPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(30.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
            BorderRadius::all(Val::Px(15.0)),
            Visibility::Hidden,
            // Offset to center the panel
            Transform::from_translation(Vec3::new(-150.0, 0.0, 0.0)),
        ))
        .with_children(|panel| {
            // Title
            panel.spawn((
                Text::new("END COMPLETE"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Score summary text
            panel.spawn((
                ScoreSummaryText,
                Text::new(""),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
            ));

            // Confirm button
            panel
                .spawn((
                    ConfirmScoreButton,
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.3, 0.6, 0.3, 0.9)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("CONFIRM"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });

    // Game Over Panel (centered overlay, hidden by default)
    commands
        .spawn((
            GameOverPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(20.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(40.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            BorderRadius::all(Val::Px(20.0)),
            Visibility::Hidden,
            // Offset to center the panel (approximately -200px for centering)
            Transform::from_translation(Vec3::new(-200.0, 0.0, 0.0)),
        ))
        .with_children(|panel| {
            // Title
            panel.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Winner announcement
            panel.spawn((
                GameOverWinnerText,
                Text::new(""),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.2)),
            ));

            // Score breakdown table header
            panel.spawn((
                Text::new("End   Team 1   Team 2"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
            ));

            // Score breakdown content
            panel.spawn((
                GameOverScoreBreakdown,
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

    debug!("UI setup complete");
}

