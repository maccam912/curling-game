//! # Viewport Systems
//!
//! Systems that detect and respond to viewport size changes.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tracing::debug;

use crate::viewport::ViewportConfig;

/// Detects viewport size changes and updates the ViewportConfig resource.
///
/// This system runs every frame and checks if the window size has changed.
/// If it has, it updates the ViewportConfig with the new dimensions and
/// recomputes the layout mode and UI scale.
pub fn viewport_detection_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewport: ResMut<ViewportConfig>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let width = window.width();
    let height = window.height();

    // Only update if dimensions actually changed
    if (viewport.width - width).abs() > 0.1 || (viewport.height - height).abs() > 0.1 {
        let old_mode = viewport.layout_mode;
        viewport.update(width, height);

        if old_mode != viewport.layout_mode {
            debug!(
                width = width,
                height = height,
                mode = ?viewport.layout_mode,
                ui_scale = viewport.ui_scale,
                "Viewport layout mode changed"
            );
        }
    }
}
