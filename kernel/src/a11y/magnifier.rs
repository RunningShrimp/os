//! Screen Magnification System
//!
//! This module provides comprehensive screen magnification functionality including:
//! - Multiple zoom levels (2x to 16x)
//! - Magnification viewport management
//! - Mouse tracking and cursor enhancement
//! - Edge scrolling and panning
//! - Smooth zoom transitions

use crate::subsystems::sync::spinlock::SpinLock;
use core::sync::atomic::{AtomicBool, Ordering};

/// Screen magnifier configuration
#[derive(Debug, Clone)]
pub struct MagnifierConfig {
    /// Zoom level (1x = no magnification)
    pub zoom_level: u8,
    /// Minimum zoom level
    pub min_zoom: u8,
    /// Maximum zoom level
    pub max_zoom: u8,
    /// Enable smoothing
    pub smoothing: bool,
    /// Follow mouse cursor
    pub follow_mouse: bool,
    /// Follow keyboard focus
    pub follow_focus: bool,
    /// Invert colors
    pub invert_colors: bool,
    /// Enable high contrast mode
    pub high_contrast: bool,
    /// Mouse tracking mode
    pub tracking_mode: TrackingMode,
    /// Edge scrolling sensitivity (pixels from edge)
    pub edge_scroll_margin: u32,
    /// Lens shape
    pub lens_shape: LensShape,
}

impl Default for MagnifierConfig {
    fn default() -> Self {
        Self {
            zoom_level: 2,
            min_zoom: 2,
            max_zoom: 16,
            smoothing: true,
            follow_mouse: true,
            follow_focus: false,
            invert_colors: false,
            high_contrast: false,
            tracking_mode: TrackingMode::Centered,
            edge_scroll_margin: 50,
            lens_shape: LensShape::Rectangle,
        }
    }
}

/// Tracking mode for magnifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingMode {
    /// Magnifier stays centered on cursor/focus
    Centered,
    /// Magnifier moves to keep cursor at edge
    Edge,
    /// Magnifier stays in fixed position
    Fixed,
    /// Magnifier follows with offset
    Offset { x_offset: i32, y_offset: i32 },
}

/// Lens shape for magnified viewport
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LensShape {
    /// Rectangular viewport
    Rectangle,
    /// Elliptical viewport
    Ellipse,
    /// Full screen magnification
    FullScreen,
}

/// Screen position
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenPosition {
    pub x: u32,
    pub y: u32,
}

impl ScreenPosition {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// Screen rectangle
#[derive(Debug, Clone, Copy)]
pub struct ScreenRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, pos: ScreenPosition) -> bool {
        pos.x >= self.x &&
        pos.x < self.x + self.width &&
        pos.y >= self.y &&
        pos.y < self.y + self.height
    }

    pub fn center(&self) -> ScreenPosition {
        ScreenPosition {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Magnified viewport
#[derive(Debug, Clone)]
pub struct MagnifierViewport {
    /// Current position on screen (source)
    pub source_position: ScreenPosition,
    /// Size of source area
    pub source_size: ScreenSize,
    /// Position of magnified window on screen (destination)
    pub dest_position: ScreenPosition,
    /// Size of magnified window
    pub dest_size: ScreenSize,
    /// Current zoom level
    pub zoom_level: u8,
}

/// Screen size
#[derive(Debug, Clone, Copy)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

impl ScreenSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Magnification filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    /// Nearest neighbor (fast, pixelated)
    Nearest,
    /// Bilinear interpolation (smooth)
    Bilinear,
    /// Bicubic interpolation (smoother, slower)
    Bicubic,
    /// Lanczos resampling (best quality, slowest)
    Lanczos,
}

/// Mouse cursor enhancement
#[derive(Debug, Clone, Copy)]
pub struct CursorEnhancement {
    /// Enable cursor enhancement
    pub enabled: bool,
    /// Cursor size multiplier
    pub size_multiplier: u8,
    /// Invert cursor colors
    pub invert_colors: bool,
    /// Enable cursor highlighting
    pub highlight: bool,
    /// Highlight color (ARGB)
    pub highlight_color: u32,
}

impl Default for CursorEnhancement {
    fn default() -> Self {
        Self {
            enabled: true,
            size_multiplier: 2,
            invert_colors: false,
            highlight: true,
            highlight_color: 0xFFFF0000, // Red
        }
    }
}

/// Screen magnifier
pub struct ScreenMagnifier {
    config: SpinLock<MagnifierConfig>,
    viewport: SpinLock<MagnifierViewport>,
    screen_size: SpinLock<ScreenSize>,
    cursor_pos: SpinLock<ScreenPosition>,
    focus_pos: SpinLock<ScreenPosition>,
    enabled: AtomicBool,
    needs_update: AtomicBool,
    cursor_enhancement: SpinLock<CursorEnhancement>,
    filter_mode: SpinLock<FilterMode>,
}

impl ScreenMagnifier {
    /// Create a new screen magnifier
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        let screen_size = ScreenSize::new(screen_width, screen_height);
        let config = MagnifierConfig::default();

        let viewport = Self::calculate_viewport(&config, screen_size);

        Self {
            config: SpinLock::new(config),
            viewport: SpinLock::new(viewport),
            screen_size: SpinLock::new(screen_size),
            cursor_pos: SpinLock::new(ScreenPosition::default()),
            focus_pos: SpinLock::new(ScreenPosition::default()),
            enabled: AtomicBool::new(false),
            needs_update: AtomicBool::new(false),
            cursor_enhancement: SpinLock::new(CursorEnhancement::default()),
            filter_mode: SpinLock::new(FilterMode::Bilinear),
        }
    }

    /// Enable or disable magnifier
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        if enabled {
            self.update_viewport();
        }
    }

    /// Check if magnifier is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Set zoom level
    pub fn set_zoom_level(&self, level: u8) {
        let mut config = self.config.lock();
        if level < config.min_zoom || level > config.max_zoom {
            return;
        }
        config.zoom_level = level;

        drop(config);
        self.update_viewport();
    }

    /// Get current zoom level
    pub fn get_zoom_level(&self) -> u8 {
        self.config.lock().zoom_level
    }

    /// Increase zoom level
    pub fn zoom_in(&self) {
        let mut config = self.config.lock();
        if config.zoom_level < config.max_zoom {
            config.zoom_level += 1;
        }
        drop(config);
        self.update_viewport();
    }

    /// Decrease zoom level
    pub fn zoom_out(&self) {
        let mut config = self.config.lock();
        if config.zoom_level > config.min_zoom {
            config.zoom_level -= 1;
        }
        drop(config);
        self.update_viewport();
    }

    /// Update cursor position
    pub fn update_cursor(&self, pos: ScreenPosition) {
        *self.cursor_pos.lock() = pos;

        if self.is_enabled() && self.config.lock().follow_mouse {
            self.track_cursor(pos);
        }
    }

    /// Update focus position
    pub fn update_focus(&self, pos: ScreenPosition) {
        *self.focus_pos.lock() = pos;

        if self.is_enabled() && self.config.lock().follow_focus {
            self.track_focus(pos);
        }
    }

    /// Track cursor movement
    fn track_cursor(&self, pos: ScreenPosition) {
        let config = self.config.lock();
        let mut viewport = self.viewport.lock();
        let screen_size = self.screen_size.lock();

        match config.tracking_mode {
            TrackingMode::Centered => {
                // Center viewport on cursor
                viewport.source_position = Self::center_on(pos, viewport.source_size, *screen_size);
            }
            TrackingMode::Edge => {
                // Keep cursor at edge of viewport
                viewport.source_position = self.edge_track(pos, viewport.source_size, *screen_size);
            }
            TrackingMode::Fixed => {
                // Don't move viewport
            }
            TrackingMode::Offset { x_offset, y_offset } => {
                viewport.source_position = ScreenPosition {
                    x: (pos.x as i32 + x_offset).max(0) as u32,
                    y: (pos.y as i32 + y_offset).max(0) as u32,
                };
            }
        }
    }

    /// Track focus movement
    fn track_focus(&self, pos: ScreenPosition) {
        let _config = self.config.lock();
        let mut viewport = self.viewport.lock();
        let screen_size = self.screen_size.lock();

        viewport.source_position = Self::center_on(pos, viewport.source_size, *screen_size);
    }

    /// Center position within viewport
    fn center_on(pos: ScreenPosition, viewport_size: ScreenSize, screen_size: ScreenSize) -> ScreenPosition {
        let x = if viewport_size.width > screen_size.width {
            0
        } else {
            pos.x.saturating_sub(viewport_size.width / 2)
                .min(screen_size.width - viewport_size.width)
        };

        let y = if viewport_size.height > screen_size.height {
            0
        } else {
            pos.y.saturating_sub(viewport_size.height / 2)
                .min(screen_size.height - viewport_size.height)
        };

        ScreenPosition { x, y }
    }

    /// Edge tracking (keep cursor at edge)
    fn edge_track(&self, pos: ScreenPosition, viewport_size: ScreenSize, screen_size: ScreenSize) -> ScreenPosition {
        let margin = self.config.lock().edge_scroll_margin;

        let current_x = self.viewport.lock().source_position.x;
        let current_y = self.viewport.lock().source_position.y;

        let mut new_x = current_x;
        let mut new_y = current_y;

        // Edge scrolling horizontally
        if pos.x < margin {
            new_x = new_x.saturating_sub(margin - pos.x);
        } else if pos.x > viewport_size.width - margin {
            new_x = new_x.saturating_add(pos.x - (viewport_size.width - margin));
        }

        // Edge scrolling vertically
        if pos.y < margin {
            new_y = new_y.saturating_sub(margin - pos.y);
        } else if pos.y > viewport_size.height - margin {
            new_y = new_y.saturating_add(pos.y - (viewport_size.height - margin));
        }

        // Clamp to screen bounds
        new_x = new_x.min(screen_size.width.saturating_sub(viewport_size.width));
        new_y = new_y.min(screen_size.height.saturating_sub(viewport_size.height));

        ScreenPosition { x: new_x, y: new_y }
    }

    /// Update viewport configuration
    fn update_viewport(&self) {
        let config = self.config.lock();
        let screen_size = self.screen_size.lock();
        let viewport = Self::calculate_viewport(&config, *screen_size);
        *self.viewport.lock() = viewport;

        self.needs_update.store(true, Ordering::Release);
    }

    /// Calculate viewport from configuration
    fn calculate_viewport(config: &MagnifierConfig, screen_size: ScreenSize) -> MagnifierViewport {
        let zoom_factor = config.zoom_level as u32;

        let (source_size, dest_position, dest_size) = match config.lens_shape {
            LensShape::FullScreen => {
                // Full screen: source is fraction of screen, dest is full screen
                let source_size = ScreenSize {
                    width: screen_size.width / zoom_factor,
                    height: screen_size.height / zoom_factor,
                };
                let dest_size = screen_size;
                (source_size, ScreenPosition::new(0, 0), dest_size)
            }
            LensShape::Rectangle | LensShape::Ellipse => {
                // Lens: source is small portion, dest is window
                const LENS_SIZE: u32 = 400;
                let source_size = ScreenSize {
                    width: LENS_SIZE / zoom_factor,
                    height: LENS_SIZE / zoom_factor,
                };
                let dest_size = ScreenSize {
                    width: LENS_SIZE,
                    height: LENS_SIZE,
                };
                let dest_position = ScreenPosition::new(50, 50);
                (source_size, dest_position, dest_size)
            }
        };

        MagnifierViewport {
            source_position: ScreenPosition::default(),
            source_size,
            dest_position,
            dest_size,
            zoom_level: config.zoom_level,
        }
    }

    /// Get current viewport
    pub fn get_viewport(&self) -> MagnifierViewport {
        self.viewport.lock().clone()
    }

    /// Update magnifier configuration
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut MagnifierConfig),
    {
        let mut config = self.config.lock();
        f(&mut config);
        drop(config);
        self.update_viewport();
    }

    /// Get current configuration
    pub fn get_config(&self) -> MagnifierConfig {
        self.config.lock().clone()
    }

    /// Set tracking mode
    pub fn set_tracking_mode(&self, mode: TrackingMode) {
        self.config.lock().tracking_mode = mode;
    }

    /// Set lens shape
    pub fn set_lens_shape(&self, shape: LensShape) {
        let mut config = self.config.lock();
        config.lens_shape = shape;
        drop(config);
        self.update_viewport();
    }

    /// Set filter mode
    pub fn set_filter_mode(&self, mode: FilterMode) {
        *self.filter_mode.lock() = mode;
    }

    /// Get filter mode
    pub fn get_filter_mode(&self) -> FilterMode {
        *self.filter_mode.lock()
    }

    /// Enable/disable color inversion
    pub fn set_invert_colors(&self, invert: bool) {
        self.config.lock().invert_colors = invert;
    }

    /// Enable/disable high contrast mode
    pub fn set_high_contrast(&self, high_contrast: bool) {
        self.config.lock().high_contrast = high_contrast;
    }

    /// Get cursor enhancement settings
    pub fn get_cursor_enhancement(&self) -> CursorEnhancement {
        *self.cursor_enhancement.lock()
    }

    /// Set cursor enhancement settings
    pub fn set_cursor_enhancement(&self, enhancement: CursorEnhancement) {
        *self.cursor_enhancement.lock() = enhancement;
    }

    /// Update screen size (called when screen resolution changes)
    pub fn set_screen_size(&self, width: u32, height: u32) {
        *self.screen_size.lock() = ScreenSize::new(width, height);
        self.update_viewport();
    }

    /// Check if magnifier needs to be redrawn
    pub fn needs_update(&self) -> bool {
        self.needs_update.load(Ordering::Acquire)
    }

    /// Clear update flag
    pub fn clear_update_flag(&self) {
        self.needs_update.store(false, Ordering::Release);
    }

    /// Get magnified screen region (source coordinates)
    pub fn get_source_region(&self) -> ScreenRect {
        let viewport = self.viewport.lock();
        ScreenRect {
            x: viewport.source_position.x,
            y: viewport.source_position.y,
            width: viewport.source_size.width,
            height: viewport.source_size.height,
        }
    }

    /// Get magnified window position and size
    pub fn get_destination_rect(&self) -> ScreenRect {
        let viewport = self.viewport.lock();
        ScreenRect {
            x: viewport.dest_position.x,
            y: viewport.dest_position.y,
            width: viewport.dest_size.width,
            height: viewport.dest_size.height,
        }
    }

    /// Convert screen coordinate to magnified coordinate
    pub fn screen_to_magnified(&self, screen_pos: ScreenPosition) -> Option<ScreenPosition> {
        let viewport = self.viewport.lock();
        let zoom = viewport.zoom_level as u32;

        if screen_pos.x < viewport.source_position.x ||
           screen_pos.y < viewport.source_position.y {
            return None;
        }

        let relative_x = screen_pos.x - viewport.source_position.x;
        let relative_y = screen_pos.y - viewport.source_position.y;

        if relative_x >= viewport.source_size.width ||
           relative_y >= viewport.source_size.height {
            return None;
        }

        Some(ScreenPosition {
            x: viewport.dest_position.x + relative_x * zoom,
            y: viewport.dest_position.y + relative_y * zoom,
        })
    }

    /// Convert magnified coordinate to screen coordinate
    pub fn magnified_to_screen(&self, mag_pos: ScreenPosition) -> Option<ScreenPosition> {
        let viewport = self.viewport.lock();
        let zoom = viewport.zoom_level as u32;

        if mag_pos.x < viewport.dest_position.x ||
           mag_pos.y < viewport.dest_position.y {
            return None;
        }

        let relative_x = mag_pos.x - viewport.dest_position.x;
        let relative_y = mag_pos.y - viewport.dest_position.y;

        if relative_x >= viewport.dest_size.width ||
           relative_y >= viewport.dest_size.height {
            return None;
        }

        Some(ScreenPosition {
            x: viewport.source_position.x + relative_x / zoom,
            y: viewport.source_position.y + relative_y / zoom,
        })
    }
}

/// Color transformation utilities
pub struct ColorTransform;

impl ColorTransform {
    /// Invert color (for color blindness assistance)
    pub fn invert_color(argb: u32) -> u32 {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;

        (a << 24) | ((255 - r) << 16) | ((255 - g) << 8) | (255 - b)
    }

    /// Convert to grayscale
    pub fn to_grayscale(argb: u32) -> u32 {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;

        // Standard luminance formula
        let gray = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;

        (a << 24) | ((gray as u32) << 16) | ((gray as u32) << 8) | gray as u32
    }

    /// High contrast transformation
    pub fn high_contrast(argb: u32) -> u32 {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;

        // Threshold at 128
        let threshold = 128;

        let new_r = if r > threshold { 255 } else { 0 };
        let new_g = if g > threshold { 255 } else { 0 };
        let new_b = if b > threshold { 255 } else { 0 };

        (a << 24) | ((new_r as u32) << 16) | ((new_g as u32) << 8) | new_b as u32
    }

    /// Apply blue light filter
    pub fn blue_light_filter(argb: u32, intensity: u8) -> u32 {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = (argb & 0xFF) as u8;

        let reduction = (b as u32 * intensity as u32 / 255) as u8;
        let new_b = b.saturating_sub(reduction);

        (a << 24) | ((r as u32) << 16) | ((g as u32) << 8) | new_b as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magnifier_creation() {
        let magnifier = ScreenMagnifier::new(1920, 1080);

        assert!(!magnifier.is_enabled());
        assert_eq!(magnifier.get_zoom_level(), 2);

        magnifier.set_enabled(true);
        assert!(magnifier.is_enabled());
    }

    #[test]
    fn test_zoom_levels() {
        let magnifier = ScreenMagnifier::new(1920, 1080);

        magnifier.set_zoom_level(4);
        assert_eq!(magnifier.get_zoom_level(), 4);

        magnifier.zoom_in();
        assert_eq!(magnifier.get_zoom_level(), 5);

        magnifier.zoom_out();
        assert_eq!(magnifier.get_zoom_level(), 4);
    }

    #[test]
    fn test_viewport_calculation() {
        let config = MagnifierConfig::default();
        let screen_size = ScreenSize::new(1920, 1080);

        let viewport = ScreenMagnifier::calculate_viewport(&config, screen_size);

        assert_eq!(viewport.zoom_level, 2);
        assert!(viewport.source_size.width < screen_size.width);
    }

    #[test]
    fn test_screen_rect() {
        let rect = ScreenRect::new(10, 10, 100, 100);

        assert!(rect.contains(ScreenPosition::new(50, 50)));
        assert!(!rect.contains(ScreenPosition::new(5, 5)));
        assert!(!rect.contains(ScreenPosition::new(200, 200)));

        let center = rect.center();
        assert_eq!(center.x, 60);
        assert_eq!(center.y, 60);
    }

    #[test]
    fn test_color_transform() {
        let color = 0xFF808080;

        let inverted = ColorTransform::invert_color(color);
        assert_eq!(inverted, 0xFF7F7F7F);

        let gray = ColorTransform::to_grayscale(color);
        // Mid-gray should stay mid-gray
        assert!(gray & 0xFF == 128 || gray & 0xFF == 127);
    }

    #[test]
    fn test_coordinate_conversion() {
        let magnifier = ScreenMagnifier::new(1920, 1080);
        magnifier.set_enabled(true);

        let screen_pos = ScreenPosition::new(100, 100);
        magnifier.update_cursor(screen_pos);

        // Test coordinate conversions
        if let Some(mag_pos) = magnifier.screen_to_magnified(screen_pos) {
            assert!(mag_pos.x >= screen_pos.x); // Magnified should be larger
            assert!(mag_pos.y >= screen_pos.y);
        }
    }
}
