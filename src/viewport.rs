//! # Viewport Configuration
//!
//! This module provides responsive viewport detection and layout computation.
//! It tracks window dimensions and computes the appropriate layout mode
//! for desktop, tablet, and mobile screens.

use bevy::prelude::*;

use crate::constants::{MAX_UI_SCALE, MIN_UI_SCALE, MOBILE_BREAKPOINT, TABLET_BREAKPOINT};

// ============================================================================
// LAYOUT MODE
// ============================================================================

/// Layout mode based on viewport dimensions.
///
/// Used to determine UI layout and camera positioning.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LayoutMode {
    /// Desktop layout: width >= 1024px
    #[default]
    Desktop,
    /// Tablet layout: 768px <= width < 1024px
    Tablet,
    /// Mobile portrait: width < 768px and height > width
    MobilePortrait,
    /// Mobile landscape: width < 768px and width >= height
    MobileLandscape,
}

impl LayoutMode {
    /// Compute layout mode from viewport dimensions.
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        if width >= TABLET_BREAKPOINT {
            LayoutMode::Desktop
        } else if width >= MOBILE_BREAKPOINT {
            LayoutMode::Tablet
        } else if height > width {
            LayoutMode::MobilePortrait
        } else {
            LayoutMode::MobileLandscape
        }
    }

    /// Returns true if this is a mobile layout (portrait or landscape).
    pub fn is_mobile(self) -> bool {
        matches!(
            self,
            LayoutMode::MobilePortrait | LayoutMode::MobileLandscape
        )
    }

    /// Returns true if this is a portrait layout.
    pub fn is_portrait(self) -> bool {
        matches!(self, LayoutMode::MobilePortrait)
    }
}

// ============================================================================
// VIEWPORT CONFIG
// ============================================================================

/// Resource tracking current viewport size and computing layout parameters.
///
/// This resource is automatically updated by the `viewport_detection_system`.
#[derive(Resource)]
pub struct ViewportConfig {
    /// Current viewport width in logical pixels.
    pub width: f32,
    /// Current viewport height in logical pixels.
    pub height: f32,
    /// Current layout mode based on dimensions.
    pub layout_mode: LayoutMode,
    /// UI scale factor (1.0 = base size, adjusted for viewport).
    pub ui_scale: f32,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f32,
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            width: 1400.0,
            height: 900.0,
            layout_mode: LayoutMode::Desktop,
            ui_scale: 1.0,
            aspect_ratio: 1400.0 / 900.0,
        }
    }
}

impl ViewportConfig {
    /// Create a new ViewportConfig from dimensions.
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        let layout_mode = LayoutMode::from_dimensions(width, height);
        let ui_scale = Self::compute_ui_scale(width, height, layout_mode);
        let aspect_ratio = if height > 0.0 { width / height } else { 1.0 };

        Self {
            width,
            height,
            layout_mode,
            ui_scale,
            aspect_ratio,
        }
    }

    /// Update the config with new dimensions.
    pub fn update(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.layout_mode = LayoutMode::from_dimensions(width, height);
        self.ui_scale = Self::compute_ui_scale(width, height, self.layout_mode);
        self.aspect_ratio = if height > 0.0 { width / height } else { 1.0 };
    }

    /// Compute UI scale factor based on viewport and layout mode.
    ///
    /// Uses the smaller dimension to ensure UI fits on screen.
    fn compute_ui_scale(width: f32, height: f32, layout_mode: LayoutMode) -> f32 {
        // Base reference: 1400x900 desktop
        let base_width = 1400.0;
        let base_height = 900.0;

        let scale = match layout_mode {
            LayoutMode::Desktop => {
                // Scale based on width, clamped
                (width / base_width).clamp(MIN_UI_SCALE, MAX_UI_SCALE)
            }
            LayoutMode::Tablet => {
                // Scale down slightly for tablets
                (width / base_width).clamp(MIN_UI_SCALE, 1.0)
            }
            LayoutMode::MobilePortrait => {
                // Use width for portrait mobile (it's the limiting factor)
                (width / (base_width * 0.5)).clamp(MIN_UI_SCALE, 1.0)
            }
            LayoutMode::MobileLandscape => {
                // Use height for landscape mobile
                (height / (base_height * 0.6)).clamp(MIN_UI_SCALE, 1.0)
            }
        };

        scale
    }

    /// Returns true if viewport is considered mobile.
    pub fn is_mobile(&self) -> bool {
        self.layout_mode.is_mobile()
    }

    /// Returns true if viewport is in portrait orientation.
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Get responsive button size based on layout mode.
    pub fn button_size(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Desktop => 60.0,
            LayoutMode::Tablet => 55.0,
            LayoutMode::MobilePortrait => 50.0 * self.ui_scale,
            LayoutMode::MobileLandscape => 45.0 * self.ui_scale,
        }
    }

    /// Get responsive font size for primary text.
    pub fn primary_font_size(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Desktop => 18.0,
            LayoutMode::Tablet => 16.0,
            LayoutMode::MobilePortrait | LayoutMode::MobileLandscape => 14.0 * self.ui_scale,
        }
    }

    /// Get responsive font size for large/header text.
    pub fn header_font_size(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Desktop => 24.0,
            LayoutMode::Tablet => 22.0,
            LayoutMode::MobilePortrait | LayoutMode::MobileLandscape => 18.0 * self.ui_scale,
        }
    }

    /// Get responsive padding value.
    pub fn base_padding(&self) -> f32 {
        match self.layout_mode {
            LayoutMode::Desktop => 20.0,
            LayoutMode::Tablet => 15.0,
            LayoutMode::MobilePortrait | LayoutMode::MobileLandscape => 10.0,
        }
    }

    /// Get camera height multiplier for maintaining field of view.
    ///
    /// Portrait screens need higher camera to see the full sheet width.
    pub fn camera_height_multiplier(&self) -> f32 {
        if self.is_portrait() {
            // Increase height based on how narrow the aspect ratio is
            // 1:2 aspect (0.5) -> 1.5x height
            // 1:1.5 aspect (0.67) -> 1.25x height
            (1.0 / self.aspect_ratio).clamp(1.0, 2.0)
        } else {
            1.0
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_mode_desktop() {
        assert_eq!(
            LayoutMode::from_dimensions(1400.0, 900.0),
            LayoutMode::Desktop
        );
        assert_eq!(
            LayoutMode::from_dimensions(1920.0, 1080.0),
            LayoutMode::Desktop
        );
        assert_eq!(
            LayoutMode::from_dimensions(1024.0, 768.0),
            LayoutMode::Desktop
        );
    }

    #[test]
    fn layout_mode_tablet() {
        assert_eq!(
            LayoutMode::from_dimensions(900.0, 600.0),
            LayoutMode::Tablet
        );
        assert_eq!(
            LayoutMode::from_dimensions(768.0, 1024.0),
            LayoutMode::Tablet
        );
    }

    #[test]
    fn layout_mode_mobile_portrait() {
        assert_eq!(
            LayoutMode::from_dimensions(400.0, 800.0),
            LayoutMode::MobilePortrait
        );
        assert_eq!(
            LayoutMode::from_dimensions(375.0, 812.0),
            LayoutMode::MobilePortrait
        );
    }

    #[test]
    fn layout_mode_mobile_landscape() {
        assert_eq!(
            LayoutMode::from_dimensions(700.0, 400.0),
            LayoutMode::MobileLandscape
        );
        assert_eq!(
            LayoutMode::from_dimensions(600.0, 400.0),
            LayoutMode::MobileLandscape
        );
    }

    #[test]
    fn viewport_config_from_dimensions() {
        let config = ViewportConfig::from_dimensions(1400.0, 900.0);
        assert_eq!(config.layout_mode, LayoutMode::Desktop);
        assert!(!config.is_mobile());
        assert!(!config.is_portrait());
    }

    #[test]
    fn viewport_config_portrait() {
        let config = ViewportConfig::from_dimensions(400.0, 800.0);
        assert!(config.is_portrait());
        assert!(config.camera_height_multiplier() > 1.0);
    }

    #[test]
    fn ui_scale_bounds() {
        // Very small viewport
        let small = ViewportConfig::from_dimensions(200.0, 300.0);
        assert!(small.ui_scale >= MIN_UI_SCALE);

        // Very large viewport
        let large = ViewportConfig::from_dimensions(3000.0, 2000.0);
        assert!(large.ui_scale <= MAX_UI_SCALE);
    }
}
