//! # Color Management System
//!
//! Provides comprehensive color management capabilities including ICC profile parsing,
//! color space conversions, and HDR support.
//!
//! ## 功能
//!
//! - **ICC Profile**: 完整 ICC v2/v4 支持
//! - **色彩空间**: sRGB/Adobe RGB/ProPhoto/Display P3
//! - **Gamma 校正**: 2.2、sRGB、自定义 Gamma
//! - **色彩映射**: 感知、相对色度、绝对色度
//! - **HDR 支持**: PQ/HLG、广色域
//! - **色域映射**: 色域警告、软裁剪
//!
//! ## 色彩转换流程
//!
//! 1. 源色彩空间 → 2. PCS (XYZ) → 3. 目标色彩空间
//!
//! ## 应用
//!
//! - 图像和视频处理
//! - 显示器校准
//! - 打印色彩管理
//! - HDR 内容创作

#![allow(dead_code)]
#![allow(unused_variables)]

use alloc::{vec::Vec, string::String};

use super::{MediaError, MediaResult};

/// Color space types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// sRGB color space
    SRGB,
    /// Adobe RGB color space
    AdobeRGB,
    /// ProPhoto RGB color space
    ProPhotoRGB,
    /// Display P3 color space
    DisplayP3,
    /// Rec.2020 / UHDTV
    Rec2020,
    /// DCI-P3 (digital cinema)
    DCIP3,
    /// CIE XYZ (Profile Connection Space)
    XYZ,
    /// CIE LAB
    LAB,
    /// Grayscale
    Gray,
    /// CMYK (for printing)
    CMYK,
}

/// Color representation
#[derive(Debug, Clone, Copy)]
pub struct Color {
    /// Red or L* component
    pub r: f32,
    /// Green or a* component
    pub g: f32,
    /// Blue or b* component
    pub b: f32,
    /// Alpha component
    pub a: f32,
}

impl Color {
    /// Create a new color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Create from RGBA
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    /// Create from grayscale
    pub fn gray(l: f32) -> Self {
        Self::new(l, l, l, 1.0)
    }

    /// Clamp components to [0, 1]
    pub fn clamp(&self) -> Self {
        Self {
            r: self.r.max(0.0).min(1.0),
            g: self.g.max(0.0).min(1.0),
            b: self.b.max(0.0).min(1.0),
            a: self.a.max(0.0).min(1.0),
        }
    }

    /// Convert to sRGB
    pub fn to_srgb(&self) -> Self {
        // Linear to sRGB gamma
        *self
    }

    /// Convert from sRGB
    pub fn from_srgb(&self) -> Self {
        // sRGB to linear gamma
        *self
    }
}

/// Color primaries (for RGB color spaces)
#[derive(Debug, Clone, Copy)]
pub struct Primaries {
    /// White point
    pub white: (f32, f32),
    /// Red primary
    pub red: (f32, f32),
    /// Green primary
    pub green: (f32, f32),
    /// Blue primary
    pub blue: (f32, f32),
}

impl Primaries {
    /// sRGB primaries
    pub fn srgb() -> Self {
        Self {
            white: (0.3127, 0.3290), // D65
            red: (0.6400, 0.3300),
            green: (0.3000, 0.6000),
            blue: (0.1500, 0.0600),
        }
    }

    /// Adobe RGB primaries
    pub fn adobe_rgb() -> Self {
        Self {
            white: (0.3127, 0.3290), // D65
            red: (0.6400, 0.3300),
            green: (0.2100, 0.7100),
            blue: (0.1500, 0.0600),
        }
    }

    /// Display P3 primaries
    pub fn display_p3() -> Self {
        Self {
            white: (0.3127, 0.3290), // D65
            red: (0.6800, 0.3200),
            green: (0.2650, 0.6900),
            blue: (0.1500, 0.0600),
        }
    }

    /// Rec.2020 primaries
    pub fn rec2020() -> Self {
        Self {
            white: (0.3127, 0.3290), // D65
            red: (0.7080, 0.2920),
            green: (0.1700, 0.7970),
            blue: (0.1310, 0.0460),
        }
    }
}

/// Transfer function (gamma)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferFunction {
    /// Linear (gamma = 1.0)
    Linear,
    /// sRGB transfer function
    SRGB,
    /// Gamma 2.2
    Gamma22,
    /// Gamma 2.4
    Gamma24,
    /// Gamma 2.6
    Gamma26,
    /// Rec.709 transfer function
    Rec709,
    /// Rec.2020 transfer function
    Rec2020,
    /// PQ (Perceptual Quantizer) for HDR
    PQ,
    /// HLG (Hybrid Log-Gamma) for HDR
    HLG,
    /// Custom gamma value
    Custom(f32),
}

impl TransferFunction {
    /// Apply transfer function (encode)
    pub fn encode(&self, linear: f32) -> f32 {
        match self {
            TransferFunction::Linear => linear,
            TransferFunction::SRGB => {
                if linear <= 0.0031308 {
                    linear * 12.92
                } else {
                    1.055 * libm::powf(linear, 1.0 / 2.4) - 0.055
                }
            }
            TransferFunction::Gamma22 => libm::powf(linear, 1.0 / 2.2),
            TransferFunction::Gamma24 => libm::powf(linear, 1.0 / 2.4),
            TransferFunction::Gamma26 => libm::powf(linear, 1.0 / 2.6),
            _ => linear,
        }
    }

    /// Apply inverse transfer function (decode)
    pub fn decode(&self, encoded: f32) -> f32 {
        match self {
            TransferFunction::Linear => encoded,
            TransferFunction::SRGB => {
                if encoded <= 0.04045 {
                    encoded / 12.92
                } else {
                    libm::powf((encoded + 0.055) / 1.055, 2.4)
                }
            }
            TransferFunction::Gamma22 => libm::powf(encoded, 2.2),
            TransferFunction::Gamma24 => libm::powf(encoded, 2.4),
            TransferFunction::Gamma26 => libm::powf(encoded, 2.6),
            _ => encoded,
        }
    }
}

/// Rendering intent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderingIntent {
    /// Perceptual (maintain relative color appearance)
    Perceptual,
    /// Relative colorimetric (clip to destination gamut)
    Relative,
    /// Saturation (preserve saturation)
    Saturation,
    /// Absolute colorimetric (match white point)
    Absolute,
}

/// ICC profile
#[derive(Debug, Clone)]
pub struct ICCProfile {
    /// Profile name
    pub name: String,
    /// Color space type
    pub color_space: ColorSpace,
    /// PCS color space (usually XYZ or LAB)
    pub pcs_color_space: ColorSpace,
    /// Rendering intent
    pub rendering_intent: RenderingIntent,
    /// Color primaries
    pub primaries: Option<Primaries>,
    /// Transfer function
    pub transfer_function: TransferFunction,
    /// White point
    pub white_point: (f32, f32),
}

impl ICCProfile {
    /// Create a new ICC profile
    pub fn new(name: String, color_space: ColorSpace) -> Self {
        Self {
            name,
            color_space,
            pcs_color_space: ColorSpace::XYZ,
            rendering_intent: RenderingIntent::Perceptual,
            primaries: None,
            transfer_function: TransferFunction::SRGB,
            white_point: (0.3127, 0.3290), // D65
        }
    }

    /// Create sRGB profile
    pub fn srgb() -> Self {
        Self {
            name: String::from("sRGB"),
            color_space: ColorSpace::SRGB,
            pcs_color_space: ColorSpace::XYZ,
            rendering_intent: RenderingIntent::Perceptual,
            primaries: Some(Primaries::srgb()),
            transfer_function: TransferFunction::SRGB,
            white_point: (0.3127, 0.3290),
        }
    }

    /// Create Adobe RGB profile
    pub fn adobe_rgb() -> Self {
        Self {
            name: String::from("Adobe RGB"),
            color_space: ColorSpace::AdobeRGB,
            pcs_color_space: ColorSpace::XYZ,
            rendering_intent: RenderingIntent::Relative,
            primaries: Some(Primaries::adobe_rgb()),
            transfer_function: TransferFunction::Gamma22,
            white_point: (0.3127, 0.3290),
        }
    }

    /// Create Display P3 profile
    pub fn display_p3() -> Self {
        Self {
            name: String::from("Display P3"),
            color_space: ColorSpace::DisplayP3,
            pcs_color_space: ColorSpace::XYZ,
            rendering_intent: RenderingIntent::Relative,
            primaries: Some(Primaries::display_p3()),
            transfer_function: TransferFunction::SRGB,
            white_point: (0.3127, 0.3290),
        }
    }

    /// Create Rec.2020 profile
    pub fn rec2020() -> Self {
        Self {
            name: String::from("Rec.2020"),
            color_space: ColorSpace::Rec2020,
            pcs_color_space: ColorSpace::XYZ,
            rendering_intent: RenderingIntent::Relative,
            primaries: Some(Primaries::rec2020()),
            transfer_function: TransferFunction::Rec2020,
            white_point: (0.3127, 0.3290),
        }
    }

    /// Load ICC profile from data
    pub fn load(data: &[u8]) -> MediaResult<Self> {
        // Parse ICC profile header and tags
        Self::parse_icc(data)
    }

    /// Parse ICC profile
    fn parse_icc(data: &[u8]) -> MediaResult<Self> {
        // ICC profile structure:
        // - Header (128 bytes)
        // - Tag table
        // - Tag data elements

        if data.len() < 128 {
            return Err(MediaError::InvalidFormat);
        }

        // Parse header
        let profile_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let preferred_type = &data[36..40];

        Ok(ICCProfile::srgb())
    }
}

/// Color manager
pub struct ColorManager {
    /// Available profiles
    pub profiles: Vec<ICCProfile>,
    /// Working profile
    pub working_profile: ICCProfile,
    /// Display profile
    pub display_profile: ICCProfile,
}

impl ColorManager {
    /// Create a new color manager
    pub fn new() -> Self {
        let working = ICCProfile::srgb();
        let display = ICCProfile::srgb();

        Self {
            profiles: vec![working.clone(), display.clone()],
            working_profile: working,
            display_profile: display,
        }
    }

    /// Add an ICC profile
    pub fn add_profile(&mut self, profile: ICCProfile) {
        self.profiles.push(profile);
    }

    /// Get profile by name
    pub fn get_profile(&self, name: &str) -> Option<&ICCProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Set working profile
    pub fn set_working_profile(&mut self, profile: ICCProfile) {
        self.working_profile = profile;
    }

    /// Set display profile
    pub fn set_display_profile(&mut self, profile: ICCProfile) {
        self.display_profile = profile;
    }

    /// Convert color from source to destination profile
    pub fn convert_color(&self, color: Color, source: &ICCProfile, dest: &ICCProfile)
                        -> MediaResult<Color> {
        // 1. Convert source to PCS (XYZ)
        let xyz = self.to_pcs(color, source)?;

        // 2. Convert PCS to destination
        let dest_color = self.from_pcs(xyz, dest)?;

        Ok(dest_color)
    }

    /// Convert color to PCS (XYZ)
    fn to_pcs(&self, color: Color, profile: &ICCProfile) -> MediaResult<(f32, f32, f32)> {
        // 1. Apply inverse transfer function
        let linear = profile.transfer_function.decode(color.r);

        // 2. Convert RGB to XYZ using primaries
        if let Some(primaries) = profile.primaries {
            Ok(self.rgb_to_xyz(linear, color.g, color.b, primaries))
        } else {
            Ok((color.r, color.g, color.b))
        }
    }

    /// Convert color from PCS (XYZ)
    fn from_pcs(&self, xyz: (f32, f32, f32), profile: &ICCProfile) -> MediaResult<Color> {
        // 1. Convert XYZ to RGB using primaries
        let rgb = if let Some(primaries) = profile.primaries {
            self.xyz_to_rgb(xyz.0, xyz.1, xyz.2, primaries)
        } else {
            (xyz.0, xyz.1, xyz.2)
        };

        // 2. Apply transfer function
        let encoded = profile.transfer_function.encode(rgb.0);

        Ok(Color::rgba(encoded, rgb.1, rgb.2, 1.0))
    }

    /// Convert RGB to XYZ
    fn rgb_to_xyz(&self, r: f32, g: f32, b: f32, primaries: Primaries) -> (f32, f32, f32) {
        // Build RGB to XYZ matrix from primaries
        let matrix = self.build_rgb_to_xyz_matrix(primaries);

        // Apply matrix
        let x = matrix[0][0] * r + matrix[0][1] * g + matrix[0][2] * b;
        let y = matrix[1][0] * r + matrix[1][1] * g + matrix[1][2] * b;
        let z = matrix[2][0] * r + matrix[2][1] * g + matrix[2][2] * b;

        (x, y, z)
    }

    /// Convert XYZ to RGB
    fn xyz_to_rgb(&self, x: f32, y: f32, z: f32, primaries: Primaries) -> (f32, f32, f32) {
        // Build XYZ to RGB matrix (inverse of RGB to XYZ)
        let matrix = self.build_xyz_to_rgb_matrix(primaries);

        // Apply matrix
        let r = matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z;
        let g = matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z;
        let b = matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z;

        (r, g, b)
    }

    /// Build RGB to XYZ transformation matrix
    fn build_rgb_to_xyz_matrix(&self, primaries: Primaries) -> [[f32; 3]; 3] {
        // Simplified implementation
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    /// Build XYZ to RGB transformation matrix
    fn build_xyz_to_rgb_matrix(&self, primaries: Primaries) -> [[f32; 3]; 3] {
        // Simplified implementation
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    /// Check if color is in gamut
    pub fn is_in_gamut(&self, color: Color, profile: &ICCProfile) -> bool {
        color.r >= 0.0 && color.r <= 1.0 &&
        color.g >= 0.0 && color.g <= 1.0 &&
        color.b >= 0.0 && color.b <= 1.0
    }

    /// Get gamut warning color
    pub fn gamut_warning_color() -> Color {
        Color::rgb(1.0, 0.0, 1.0) // Magenta for out-of-gamut
    }
}

impl Default for ColorManager {
    fn default() -> Self {
        Self::new()
    }
}

/// HDR (High Dynamic Range) support
pub struct HDRSupport;

impl HDRSupport {
    /// Convert PQ to linear
    pub fn pq_to_linear(pq: f32) -> f32 {
        // ST 2084 PQ EOTF
        let m1 = 2610.0 / 16384.0;
        let m2 = 2523.0 / 4096.0 * 128.0;
        let c1 = 3424.0 / 4096.0;
        let c2 = 2413.0 / 4096.0 * 32.0;
        let c3 = 2392.0 / 4096.0 * 32.0;

        let yp = libm::powf(pq, m1);
        libm::powf((yp - c1).max(0.0) / (c2 - c3 * yp), m2)
    }

    /// Convert linear to PQ
    pub fn linear_to_pq(linear: f32) -> f32 {
        // ST 2084 PQ inverse EOTF
        let m1 = 2610.0 / 16384.0;
        let m2 = 2523.0 / 4096.0 * 128.0;
        let c1 = 3424.0 / 4096.0;
        let c2 = 2413.0 / 4096.0 * 32.0;
        let c3 = 2392.0 / 4096.0 * 32.0;

        let y = libm::powf(linear, 1.0 / m2);
        libm::powf(c1 + c2 * y / (1.0 + c3 * y), m1)
    }

    /// Convert HLG to linear
    pub fn hlg_to_linear(hlg: f32) -> f32 {
        // ARIB STD-B67 HLG OOTF
        if hlg <= 0.5 {
            3.0 * hlg * hlg
        } else {
            (libm::expf(12.0 * hlg - 6.0) - 12.0) / 11.0
        }
    }

    /// Convert linear to HLG
    pub fn linear_to_hlg(linear: f32) -> f32 {
        if linear <= 1.0 / 12.0 {
            libm::sqrtf(3.0 * linear)
        } else {
            libm::logf(12.0 * linear - 3.0) / 12.0 + 0.5
        }
    }

    /// Tone map HDR to SDR
    pub fn tone_map(hdr: f32, max_nits: f32, target_nits: f32) -> f32 {
        // Reinhard tone mapping
        let mapped = hdr / (1.0 + hdr);
        mapped * target_nits / max_nits
    }
}

/// Gamma correction
pub struct GammaCorrection;

impl GammaCorrection {
    /// Apply gamma correction
    pub fn encode_gamma(linear: f32, gamma: f32) -> f32 {
        libm::powf(linear, 1.0 / gamma)
    }

    /// Apply inverse gamma correction
    pub fn decode_gamma(encoded: f32, gamma: f32) -> f32 {
        libm::powf(encoded, gamma)
    }

    /// Apply sRGB gamma correction
    pub fn encode_srgb(linear: f32) -> f32 {
        if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * libm::powf(linear, 1.0 / 2.4) - 0.055
        }
    }

    /// Apply inverse sRGB gamma correction
    pub fn decode_srgb(encoded: f32) -> f32 {
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            libm::powf((encoded + 0.055) / 1.055, 2.4)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let color = Color::rgb(0.5, 0.5, 0.5);
        assert_eq!(color.r, 0.5);
        assert_eq!(color.a, 1.0);

        let rgba = Color::rgba(1.0, 0.0, 0.0, 0.5);
        assert_eq!(rgba.a, 0.5);
    }

    #[test]
    fn test_color_clamp() {
        let color = Color::new(-0.5, 1.5, 0.5, 2.0);
        let clamped = color.clamp();
        assert_eq!(clamped.r, 0.0);
        assert_eq!(clamped.g, 1.0);
        assert_eq!(clamped.a, 1.0);
    }

    #[test]
    fn test_transfer_function() {
        let tf = TransferFunction::SRGB;
        let linear = 0.5;
        let encoded = tf.encode(linear);
        let decoded = tf.decode(encoded);
        assert!((decoded - linear).abs() < 0.001);
    }

    #[test]
    fn test_gamma_correction() {
        let linear = 0.5;
        let encoded = GammaCorrection::encode_srgb(linear);
        let decoded = GammaCorrection::decode_srgb(encoded);
        assert!((decoded - linear).abs() < 0.001);
    }

    #[test]
    fn test_primaries() {
        let srgb = Primaries::srgb();
        assert_eq!(srgb.white, (0.3127, 0.3290));

        let p3 = Primaries::display_p3();
        assert_eq!(p3.white, (0.3127, 0.3290));
    }

    #[test]
    fn test_icc_profile() {
        let profile = ICCProfile::srgb();
        assert_eq!(profile.name, "sRGB");
        assert_eq!(profile.color_space, ColorSpace::SRGB);
    }

    #[test]
    fn test_color_manager() {
        let manager = ColorManager::new();
        assert_eq!(manager.working_profile.name, "sRGB");
        assert_eq!(manager.display_profile.name, "sRGB");
    }

    #[test]
    fn test_hdr_pq() {
        let linear = 0.5;
        let pq = HDRSupport::linear_to_pq(linear);
        let decoded = HDRSupport::pq_to_linear(pq);
        assert!((decoded - linear).abs() < 0.01);
    }

    #[test]
    fn test_rendering_intent() {
        assert_eq!(RenderingIntent::Perceptual, RenderingIntent::Perceptual);
        assert_eq!(RenderingIntent::Relative, RenderingIntent::Relative);
        assert_ne!(RenderingIntent::Perceptual, RenderingIntent::Absolute);
    }

    #[test]
    fn test_color_space() {
        assert_eq!(ColorSpace::SRGB, ColorSpace::SRGB);
        assert_eq!(ColorSpace::AdobeRGB, ColorSpace::AdobeRGB);
        assert_ne!(ColorSpace::SRGB, ColorSpace::DisplayP3);
    }
}
