//! # Setup Systems
//!
//! Systems that run during startup to initialize the game world.

use bevy::gltf::GltfAssetLabel;
use bevy::math::primitives::{Cuboid, Cylinder};
use bevy::prelude::*;
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
    ));
    debug!(position = ?skip_view_pos, "Spawned main camera");

    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_xyz(0.0, -12.0, 60.0).looking_at(Vec3::new(0.0, 12.0, 0.0), Vec3::Z),
    ));

    // Ice Sheet
    let sheet_mesh = meshes.add(Cuboid::new(SHEET_WIDTH, SHEET_LENGTH, SHEET_THICKNESS));
    let sheet_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.96, 0.98),
        perceptual_roughness: 0.2,
        ..default()
    });
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

    // Line Materials
    let line_black = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        ..default()
    });
    let line_red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        unlit: true,
        ..default()
    });
    let line_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.8),
        unlit: true,
        ..default()
    });

    // Center Line (Back to Back)
    spawn_line(
        &mut commands,
        &mut meshes,
        line_blue.clone(),
        Vec2::ZERO,
        back_line_far() * 2.0,
        true,
        0.02,
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
        );
    }

    // Transverse Lines (back lines and tee lines)
    for &y in &[back_line_far(), back_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.02,
        );
    }
    for &y in &[tee_line_far(), tee_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_black.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.02,
        );
    }
    for &y in &[hog_line_far(), hog_line_near()] {
        spawn_line(
            &mut commands,
            &mut meshes,
            line_red.clone(),
            Vec2::new(0.0, y),
            SHEET_WIDTH,
            false,
            0.1,
        );
    }

    // House Materials
    let ring_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.2, 0.8),
        unlit: true,
        ..default()
    });
    let ring_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        unlit: true,
        ..default()
    });
    let ring_red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        unlit: true,
        ..default()
    });

    // Draw Houses (Near and Far)
    for &y in &[tee_line_far(), tee_line_near()] {
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_blue.clone(),
            HOUSE_RADIUS_12,
            y,
            0.003,
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_8,
            y,
            0.004,
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_red.clone(),
            HOUSE_RADIUS_4,
            y,
            0.005,
        );
        spawn_house_ring(
            &mut commands,
            &mut meshes,
            ring_white.clone(),
            HOUSE_RADIUS_BUTTON,
            y,
            0.006,
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
