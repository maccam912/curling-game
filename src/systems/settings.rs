use crate::app_state::AppState;
use bevy::prelude::*;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Marker for the settings menu root node.
#[derive(Component)]
pub struct SettingsMenuRoot;

/// Marker for the settings camera.
#[derive(Component)]
pub struct SettingsCamera;

/// Marker for the "Back" button.
#[derive(Component)]
pub struct BackButton;

// ============================================================================
// SYSTEMS
// ============================================================================

/// Sets up the settings menu UI.
pub fn setup_settings_menu(mut commands: Commands) {
    commands.spawn((Camera2d::default(), SettingsCamera));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
            SettingsMenuRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("SETTINGS"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                },
            ));

            // Placeholder content
            parent.spawn((
                Text::new("Settings coming soon..."),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Back button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                    BorderRadius::all(Val::Px(8.0)),
                    BackButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Back"),
                        TextFont {
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

/// Handles interactions on the settings menu.
pub fn handle_settings_buttons(
    mut next_state: ResMut<NextState<AppState>>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut button_colors: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    // Handle Back button
    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
        }
    }

    // Visual feedback
    for (interaction, mut bg_color) in button_colors.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                bg_color.0 = bg_color.0.lighter(0.1);
            }
            Interaction::None => {
                bg_color.0 = bg_color.0.darker(0.05);
            }
            _ => {}
        }
    }
}

/// Cleans up the settings menu.
pub fn cleanup_settings_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<SettingsMenuRoot>>,
    camera_query: Query<Entity, With<SettingsCamera>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
}
