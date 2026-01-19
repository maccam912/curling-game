use crate::app_state::AppState;
use bevy::prelude::*;

/// Marker component for splash screen UI elements.
#[derive(Component)]
pub struct SplashUi;

/// Resource to track splash screen duration.
#[derive(Resource)]
pub struct SplashTimer(pub Timer);

/// Sets up the splash screen UI.
pub fn setup_splash(mut commands: Commands, _asset_server: Res<AssetServer>) {
    // Spawn a 2D camera if one doesn't exist?
    // Usually we want one persistent camera, but for now we follow the pattern likely used in menu.
    // If we assume a camera is spawned at startup of app or in menu.
    // Let's spawn a camera specifically for UI if needed, or assume existing.
    // Safest is to spawn a camera if we are the first state.

    commands.spawn((Camera2d, SplashUi));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            SplashUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("CURLING GAME"),
                TextFont {
                    font_size: 80.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

    commands.insert_resource(SplashTimer(Timer::from_seconds(2.0, TimerMode::Once)));
}

/// Updates the splash timer and transitions to menu.
pub fn update_splash_timer(
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if timer.0.tick(time.delta()).is_finished() {
        next_state.set(AppState::MainMenu);
    }
}

/// Cleans up splash screen UI.
pub fn cleanup_splash(mut commands: Commands, query: Query<Entity, With<SplashUi>>) {
    // Despawn UI entities
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    // Remove timer
    commands.remove_resource::<SplashTimer>();
}
