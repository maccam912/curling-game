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
use crate::resources::{GameState, StoneAssets};

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
            // Top HUD bar with multiple info panels
            parent
                .spawn((
                    HudPanel,
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                ))
                .with_children(|hud| {
                    // Left side: Score Panel and End Info
                    hud.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(15.0),
                        ..default()
                    })
                    .with_children(|left| {
                        // Score Panel
                        left.spawn((
                            ScorePanel,
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(20.0),
                                padding: UiRect::axes(Val::Px(15.0), Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|scores| {
                            // Team 1 Score
                            scores
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                })
                                .with_children(|team1| {
                                    // Team color indicator
                                    team1.spawn((
                                        Node {
                                            width: Val::Px(12.0),
                                            height: Val::Px(12.0),
                                            ..default()
                                        },
                                        BackgroundColor(Team::One.color()),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                    team1.spawn((
                                        Text::new("Team 1:"),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                    team1.spawn((
                                        Team1ScoreText,
                                        Text::new("0"),
                                        TextFont {
                                            font_size: 24.0,
                                            ..default()
                                        },
                                        TextColor(Team::One.color()),
                                    ));
                                });

                            // Separator
                            scores.spawn((
                                Text::new("|"),
                                TextFont {
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                            ));

                            // Team 2 Score
                            scores
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                })
                                .with_children(|team2| {
                                    // Team color indicator
                                    team2.spawn((
                                        Node {
                                            width: Val::Px(12.0),
                                            height: Val::Px(12.0),
                                            ..default()
                                        },
                                        BackgroundColor(Team::Two.color()),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                    team2.spawn((
                                        Text::new("Team 2:"),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                    team2.spawn((
                                        Team2ScoreText,
                                        Text::new("0"),
                                        TextFont {
                                            font_size: 24.0,
                                            ..default()
                                        },
                                        TextColor(Team::Two.color()),
                                    ));
                                });
                        });

                        // End Info Panel
                        left.spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|end_panel| {
                            end_panel.spawn((
                                EndInfoText,
                                Text::new("END 1/8"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });

                        // Hammer Indicator
                        left.spawn((
                            HammerIndicator,
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.2, 0.1, 0.8)),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|hammer| {
                            hammer.spawn((
                                Text::new("HAMMER"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.8, 0.4)),
                            ));
                        });
                    });

                    // Right side: Game info panel
                    hud.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            min_width: Val::Px(180.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|info| {
                        // Shot counter
                        info.spawn((
                            ShotInfoText,
                            Text::new("Shot: 1/16"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));

                        // Shots remaining
                        info.spawn((
                            ShotsRemainingText,
                            Text::new("Remaining: 16"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                        ));

                        // Team turn indicator
                        info.spawn((
                            TeamTurnIndicator,
                            Text::new("Team 1's Turn"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Team::One.color()),
                        ));

                        // Phase indicator
                        info.spawn((
                            PhaseIndicator,
                            Text::new("Calling Shot"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.6, 0.8, 0.6)),
                        ));
                    });
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

            // Debug tuning panel (top-right)
            parent
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(20.0),
                    top: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                })
                .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)))
                .insert(BorderRadius::all(Val::Px(8.0)))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("Model Tuning"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Scale control row
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Scale:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            // Minus button
                            row.spawn((
                                ScaleSlider,
                                Button,
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.9)),
                                BorderRadius::all(Val::Px(4.0)),
                                TuningAdjust::Decrease,
                            ))
                            .with_child((
                                Text::new("-"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            // Value display
                            row.spawn((
                                ScaleValueLabel,
                                Text::new("0.53"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 1.0, 0.5)),
                                Node {
                                    min_width: Val::Px(50.0),
                                    ..default()
                                },
                            ));
                            // Plus button
                            row.spawn((
                                ScaleSlider,
                                Button,
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.9)),
                                BorderRadius::all(Val::Px(4.0)),
                                TuningAdjust::Increase,
                            ))
                            .with_child((
                                Text::new("+"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Z Offset control row
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Z Off:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            // Minus button
                            row.spawn((
                                ZOffsetSlider,
                                Button,
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.9)),
                                BorderRadius::all(Val::Px(4.0)),
                                TuningAdjust::Decrease,
                            ))
                            .with_child((
                                Text::new("-"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            // Value display
                            row.spawn((
                                ZOffsetValueLabel,
                                Text::new("0.18"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 1.0, 0.5)),
                                Node {
                                    min_width: Val::Px(50.0),
                                    ..default()
                                },
                            ));
                            // Plus button
                            row.spawn((
                                ZOffsetSlider,
                                Button,
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.9)),
                                BorderRadius::all(Val::Px(4.0)),
                                TuningAdjust::Increase,
                            ))
                            .with_child((
                                Text::new("+"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });

            // Spacer to push buttons to bottom
            parent.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            // Bottom control area
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(15.0),
                    ..default()
                })
                .with_children(|bottom| {
                    // Action buttons row
                    bottom
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        })
                        .with_children(|row| {
                            // Camera toggle button
                            row.spawn((
                                CameraToggleButton,
                                Button,
                                Node {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("VIEW"),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });

                            // Curl IN button (selected by default)
                            row.spawn((
                                CurlButton(CurlDirection::InTurn),
                                Button,
                                Node {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
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
                            row.spawn((
                                CurlButton(CurlDirection::OutTurn),
                                Button,
                                Node {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
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

                            // Confirm/throw button
                            row.spawn((
                                ConfirmButton,
                                Button,
                                Node {
                                    width: Val::Px(200.0),
                                    height: Val::Px(60.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.2, 0.6, 0.3, 0.9)),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("Confirm Shot"),
                                    TextFont {
                                        font_size: 22.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });

                            // Debug quick-simulate button (only in debug_mode)
                            #[cfg(feature = "debug_mode")]
                            row.spawn((
                                DebugQuickSimButton,
                                Button,
                                Node {
                                    width: Val::Px(100.0),
                                    height: Val::Px(60.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.6, 0.3, 0.1, 0.9)),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("QUICK\nSIM"),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        });
                });
        });

    debug!("UI setup complete");
}
