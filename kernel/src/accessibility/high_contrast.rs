//! High Contrast Mode
//!
//! This module implements high contrast accessibility:
//! - High contrast themes
//! - Color customization
//! - Text readability enhancement
//!
//! Features:
//! - High contrast color schemes
//! - Color invert options
//! - Grayscale mode
//! - Font scaling

use libm::*;
use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// High Contrast Constants
// ============================================================================

/// Maximum color schemes
pub const MAX_COLOR_SCHEMES: usize = 1 << 8;

// ============================================================================
// Color Types
// ============================================================================

/// Color theme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTheme {
    /// Standard theme
    Standard,
    
    /// High contrast black on white
    HighContrastBlackWhite,
    
    /// High contrast white on black
    HighContrastWhiteBlack,
    
    /// High contrast yellow on black
    HighContrastYellowBlack,
    
    /// High contrast blue on yellow
    HighContrastBlueYellow,
    
    /// Grayscale
    Grayscale,
    
    /// Sepia
    Sepia,
    
    /// Custom theme
    Custom(String),
}

/// Color type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorType {
    /// Background color
    Background,
    
    /// Foreground (text) color
    Foreground,
    
    /// Primary color (buttons, links)
    Primary,
    
    /// Secondary color
    Secondary,
    
    /// Accent color
    Accent,
    
    /// Border color
    Border,
    
    /// Selected color
    Selected,
    
    /// Disabled color
    Disabled,
    
    /// Error color
    Error,
    
    /// Warning color
    Warning,
    
    /// Success color
    Success,
}

// ============================================================================
// Color Values
// ============================================================================

/// RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    pub fn white() -> Self {
        Self { r: 255, g: 255, b: 255 }
    }

    pub fn to_hex(&self) -> String {
        alloc::string::String::from("#") + /* TODO: {::02X} */ &self.r.to_string() + /* TODO: {::02X} */ &self.g.to_string() + /* TODO: {::02X} */ &self.b.to_string()
    }

    pub fn to_grayscale(&self) -> Self {
        // Luminance formula: 0.299R + 0.587G + 0.114B
        let gray = (0.299 * self.r as f64 + 0.587 * self.g as f64 + 0.114 * self.b as f64) as u8;
        Self { r: gray, g: gray, b: gray }
    }

    pub fn invert(&self) -> Self {
        Self {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
        }
    }

    pub fn get_contrast_ratio(&self, other: &RgbColor) -> f64 {
        let lum1 = self.get_luminance();
        let lum2 = other.get_luminance();
        
        let lighter = lum1.max(lum2);
        let darker = lum1.min(lum2);
        
        if darker == 0.0 {
            return 21.0; // Maximum contrast ratio
        }
        
        (lighter + 0.05) / (darker + 0.05)
    }

    fn get_luminance(&self) -> f64 {
        // Convert sRGB to linear RGB
        let r_linear = Self::to_linear(self.r);
        let g_linear = Self::to_linear(self.g);
        let b_linear = Self::to_linear(self.b);
        
        // Calculate luminance
        0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear
    }

    fn to_linear(c: u8) -> f64 {
        let c_scaled = c as f64 / 255.0;
        if c_scaled <= 0.04045 {
            c_scaled / 12.92
        } else {
            libm::powf((c_scaled + 0.055) / 1.055, 2.4)
        }
    }
}

// ============================================================================
// Color Scheme
// ============================================================================

/// Color scheme
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub theme_type: ColorTheme,
    pub colors: BTreeMap<ColorType, RgbColor>,
    pub high_contrast_ratio: f64,
}

impl ColorScheme {
    pub fn new(name: String, theme_type: ColorTheme) -> Self {
        let mut colors = BTreeMap::new();
        
        // Apply default colors based on theme
        match theme_type {
            ColorTheme::Standard => {
                colors.insert(ColorType::Background, RgbColor::white());
                colors.insert(ColorType::Foreground, RgbColor::new(0, 0, 0));
                colors.insert(ColorType::Primary, RgbColor::new(0, 102, 204));
                colors.insert(ColorType::Secondary, RgbColor::new(51, 51, 51));
                colors.insert(ColorType::Accent, RgbColor::new(255, 204, 0));
                colors.insert(ColorType::Border, RgbColor::new(221, 221, 221));
                colors.insert(ColorType::Selected, RgbColor::new(0, 120, 215));
                colors.insert(ColorType::Disabled, RgbColor::new(153, 153, 153));
                colors.insert(ColorType::Error, RgbColor::new(220, 53, 69));
                colors.insert(ColorType::Warning, RgbColor::new(255, 179, 0));
                colors.insert(ColorType::Success, RgbColor::new(92, 184, 92));
            }
            ColorTheme::HighContrastBlackWhite => {
                colors.insert(ColorType::Background, RgbColor::white());
                colors.insert(ColorType::Foreground, RgbColor::black());
                colors.insert(ColorType::Primary, RgbColor::black());
                colors.insert(ColorType::Secondary, RgbColor::new(128, 128, 128));
                colors.insert(ColorType::Accent, RgbColor::new(0, 0, 0));
                colors.insert(ColorType::Border, RgbColor::black());
                colors.insert(ColorType::Selected, RgbColor::new(0, 0, 255));
                colors.insert(ColorType::Disabled, RgbColor::new(169, 169, 169));
                colors.insert(ColorType::Error, RgbColor::new(255, 0, 0));
                colors.insert(ColorType::Warning, RgbColor::new(0, 0, 255));
                colors.insert(ColorType::Success, RgbColor::new(0, 128, 0));
            }
            ColorTheme::HighContrastWhiteBlack => {
                colors.insert(ColorType::Background, RgbColor::black());
                colors.insert(ColorType::Foreground, RgbColor::white());
                colors.insert(ColorType::Primary, RgbColor::white());
                colors.insert(ColorType::Secondary, RgbColor::new(192, 192, 192));
                colors.insert(ColorType::Accent, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Border, RgbColor::white());
                colors.insert(ColorType::Selected, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Disabled, RgbColor::new(128, 128, 128));
                colors.insert(ColorType::Error, RgbColor::new(255, 0, 0));
                colors.insert(ColorType::Warning, RgbColor::new(255, 165, 0));
                colors.insert(ColorType::Success, RgbColor::new(0, 255, 0));
            }
            ColorTheme::HighContrastYellowBlack => {
                colors.insert(ColorType::Background, RgbColor::black());
                colors.insert(ColorType::Foreground, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Primary, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Secondary, RgbColor::white());
                colors.insert(ColorType::Accent, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Border, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Selected, RgbColor::white());
                colors.insert(ColorType::Disabled, RgbColor::new(200, 200, 0));
                colors.insert(ColorType::Error, RgbColor::white());
                colors.insert(ColorType::Warning, RgbColor::new(255, 128, 0));
                colors.insert(ColorType::Success, RgbColor::new(0, 255, 0));
            }
            ColorTheme::HighContrastBlueYellow => {
                colors.insert(ColorType::Background, RgbColor::new(0, 0, 255));
                colors.insert(ColorType::Foreground, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Primary, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Secondary, RgbColor::new(128, 128, 255));
                colors.insert(ColorType::Accent, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Border, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Selected, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Disabled, RgbColor::new(0, 0, 128));
                colors.insert(ColorType::Error, RgbColor::new(255, 0, 0));
                colors.insert(ColorType::Warning, RgbColor::new(255, 255, 0));
                colors.insert(ColorType::Success, RgbColor::new(0, 255, 0));
            }
            ColorTheme::Grayscale => {
                colors.insert(ColorType::Background, RgbColor::new(255, 255, 255));
                colors.insert(ColorType::Foreground, RgbColor::new(0, 0, 0));
                colors.insert(ColorType::Primary, RgbColor::new(128, 128, 128));
                colors.insert(ColorType::Secondary, RgbColor::new(192, 192, 192));
                colors.insert(ColorType::Accent, RgbColor::new(96, 96, 96));
                colors.insert(ColorType::Border, RgbColor::new(221, 221, 221));
                colors.insert(ColorType::Selected, RgbColor::new(64, 64, 64));
                colors.insert(ColorType::Disabled, RgbColor::new(204, 204, 204));
                colors.insert(ColorType::Error, RgbColor::new(128, 0, 0));
                colors.insert(ColorType::Warning, RgbColor::new(128, 64, 0));
                colors.insert(ColorType::Success, RgbColor::new(0, 96, 0));
            }
            ColorTheme::Sepia => {
                colors.insert(ColorType::Background, RgbColor::new(251, 240, 230));
                colors.insert(ColorType::Foreground, RgbColor::new(94, 38, 18));
                colors.insert(ColorType::Primary, RgbColor::new(143, 89, 2));
                colors.insert(ColorType::Secondary, RgbColor::new(127, 127, 127));
                colors.insert(ColorType::Accent, RgbColor::new(181, 101, 29));
                colors.insert(ColorType::Border, RgbColor::new(210, 180, 140));
                colors.insert(ColorType::Selected, RgbColor::new(181, 101, 29));
                colors.insert(ColorType::Disabled, RgbColor::new(190, 181, 165));
                colors.insert(ColorType::Error, RgbColor::new(194, 59, 34));
                colors.insert(ColorType::Warning, RgbColor::new(191, 145, 47));
                colors.insert(ColorType::Success, RgbColor::new(85, 107, 47));
            }
            ColorTheme::Custom(_) => {
                // Custom colors will be set by user
            }
        }

        Self {
            name,
            theme_type,
            colors,
            high_contrast_ratio: Self::calculate_contrast_ratio(&colors),
        }
    }

    fn calculate_contrast_ratio(colors: &BTreeMap<ColorType, RgbColor>) -> f64 {
        let background = colors.get(&ColorType::Background).unwrap_or(&RgbColor::white());
        let foreground = colors.get(&ColorType::Foreground).unwrap_or(&RgbColor::black());
        background.get_contrast_ratio(foreground)
    }

    pub fn get_color(&self, color_type: ColorType) -> Option<RgbColor> {
        self.colors.get(&color_type).copied()
    }

    pub fn set_color(&mut self, color_type: ColorType, color: RgbColor) {
        self.colors.insert(color_type, color);
        self.high_contrast_ratio = Self::calculate_contrast_ratio(&self.colors);
    }

    pub fn is_high_contrast(&self) -> bool {
        self.high_contrast_ratio >= 7.0 // WCAG AA standard
    }

    pub fn to_grayscale(&mut self) {
        for (color_type, color) in self.colors.iter_mut() {
            *color = color.to_grayscale();
        }
        crate::println!("[high_contrast] Converted theme {} to grayscale", self.name);
    }

    pub fn invert_colors(&mut self) {
        for (color_type, color) in self.colors.iter_mut() {
            *color = color.invert();
        }
        crate::println!("[high_contrast] Inverted colors for theme {}", self.name);
    }
}

// ============================================================================
// High Contrast Manager
// ============================================================================

/// High contrast manager
pub struct HighContrastManager {
    pub schemes: Mutex<Vec<Arc<ColorScheme>>>>,
    pub current_scheme: Arc<ColorScheme>,
    pub high_contrast_enabled: AtomicBool,
    pub invert_colors_enabled: AtomicBool,
    pub grayscale_enabled: AtomicBool,
    pub stats: Mutex<HighContrastStats>,
}

/// High contrast statistics
#[derive(Debug, Clone, Copy)]
pub struct HighContrastStats {
    pub total_schemes: usize,
    pub scheme_changes: u64,
    pub contrast_ratio: f64,
}

impl Default for HighContrastStats {
    fn default() -> Self {
        Self {
            total_schemes: 0,
            scheme_changes: 0,
            contrast_ratio: 0.0,
        }
    }
}

impl HighContrastManager {
    pub fn new() -> Self {
        let schemes = {
    let mut v = alloc::vec::Vec::new();
    v.push(Arc::new(ColorScheme::new("Standard".to_string(), ColorTheme::Standard)));
    v.push(Arc::new(ColorScheme::new("High Contrast Black/White".to_string(), ColorTheme::HighContrastBlackWhite)));
    v.push(Arc::new(ColorScheme::new("High Contrast White/Black".to_string(), ColorTheme::HighContrastWhiteBlack)));
    v.push(Arc::new(ColorScheme::new("High Contrast Yellow/Black".to_string(), ColorTheme::HighContrastYellowBlack)));
    v.push(Arc::new(ColorScheme::new("High Contrast Blue/Yellow".to_string(), ColorTheme::HighContrastBlueYellow)));
    v.push(Arc::new(ColorScheme::new("Grayscale".to_string(), ColorTheme::Grayscale)));
    v.push(Arc::new(ColorScheme::new("Sepia".to_string(), ColorTheme::Sepia)));
    v
};

        let default_scheme = schemes[0].clone();

        Self {
            schemes: Mutex::new(schemes),
            current_scheme: default_scheme,
            high_contrast_enabled: AtomicBool::new(false),
            invert_colors_enabled: AtomicBool::new(false),
            grayscale_enabled: AtomicBool::new(false),
            stats: Mutex::new(HighContrastStats::default()),
        }
    }

    pub fn register_scheme(&self, scheme: Arc<ColorScheme>) -> Result<(), String> {
        let mut schemes = self.schemes.lock();
        schemes.push(scheme);
        crate::println!("[high_contrast] Registered scheme: {}", scheme.name);
        Ok(())
    }

    pub fn set_scheme(&self, scheme_name: String) -> Result<(), String> {
        let schemes = self.schemes.lock();
        let scheme = schemes.iter()
            .find(|s| s.name == scheme_name)
            .ok_or(alloc::string::String::from("Scheme ") + &scheme_name.to_string() + alloc::string::String::from(" not found"))?
            .clone();

        self.current_scheme = scheme;
        
        self.stats.lock().scheme_changes.fetch_add(1, Ordering::Relaxed);
        crate::println!("[high_contrast] Set scheme to {}", scheme_name);
        
        Ok(())
    }

    pub fn enable_high_contrast(&self) {
        self.high_contrast_enabled.store(true, Ordering::Relaxed);
        
        // Auto-switch to a high-contrast scheme
        if !self.current_scheme.is_high_contrast() {
            let schemes = self.schemes.lock();
            if let Some(high_contrast_scheme) = schemes.iter().find(|s| s.is_high_contrast()) {
                self.current_scheme = high_contrast_scheme.clone();
            }
        }

        crate::println!("[high_contrast] High contrast enabled");
    }

    pub fn disable_high_contrast(&self) {
        self.high_contrast_enabled.store(false, Ordering::Relaxed);
        crate::println!("[high_contrast] High contrast disabled");
    }

    pub fn enable_invert_colors(&self) {
        self.invert_colors_enabled.store(true, Ordering::Relaxed);
        crate::println!("[high_contrast] Color inversion enabled");
    }

    pub fn disable_invert_colors(&self) {
        self.invert_colors_enabled.store(false, Ordering::Relaxed);
        crate::println!("[high_contrast] Color inversion disabled");
    }

    pub fn enable_grayscale(&self) {
        self.grayscale_enabled.store(true, Ordering::Relaxed);
        crate::println!("[high_contrast] Grayscale enabled");
    }

    pub fn disable_grayscale(&self) {
        self.grayscale_enabled.store(false, Ordering::Relaxed);
        crate::println!("[high_contrast] Grayscale disabled");
    }

    pub fn get_color(&self, color_type: ColorType) -> RgbColor {
        let mut color = self.current_scheme.get_color(color_type).unwrap_or(RgbColor::black());

        if self.invert_colors_enabled.load(Ordering::Relaxed) {
            color = color.invert();
        }

        if self.grayscale_enabled.load(Ordering::Relaxed) {
            color = color.to_grayscale();
        }

        color
    }

    pub fn get_current_scheme(&self) -> Arc<ColorScheme> {
        self.current_scheme.clone()
    }

    pub fn is_high_contrast(&self) -> bool {
        self.high_contrast_enabled.load(Ordering::Relaxed) && self.current_scheme.is_high_contrast()
    }

    pub fn get_stats(&self) -> HighContrastStats {
        let mut stats = self.stats.lock();
        stats.total_schemes = self.schemes.lock().len();
        stats.contrast_ratio = self.current_scheme.high_contrast_ratio;
        *stats
    }
}
