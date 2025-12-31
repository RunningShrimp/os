//! # Digital Content Creation (Media BF)
//!
//! This module provides comprehensive multimedia processing capabilities for digital content creation,
//! including image processing, video codecs, 3D rendering, audio workstation, font rendering, and color management.
//!
//! ## 概述
//!
//! 媒体子系统提供专业级的多媒体处理能力，支持：
//! - **高性能图像处理**: 基础操作、滤镜、色彩空间转换
//! - **现代视频编解码**: H.265/HEVC, AV1, VP9 支持
//! - **3D 渲染管线**: OpenGL/Vulkan 兼容的渲染引擎
//! - **专业音频工作站**: 多轨混音、效果处理、MIDI 支持
//! - **高级字体渲染**: TrueType/OpenType、复杂文字支持
//! - **完整色彩管理**: ICC Profile、HDR、广色域支持
//!
//! ## 架构
//!
//! ### 核心模块
//!
//! - [`image`]: 图像处理引擎
//! - [`video`]: 视频编解码器
//! - [`render3d`]: 3D 渲染管线
//! - [`audio_studio`]: 音频工作站
//! - [`font`]: 字体渲染引擎
//! - [`color`]: 色彩管理系统
//!
//! ## 特性
//!
//! - **硬件加速**: GPU/DSP 加速支持
//! - **实时处理**: 低延迟处理管道
//! - **专业质量**: 业界标准算法实现
//! - **广泛格式**: PNG/JPEG/WebP/H.265/AV1 等
//! - **高性能**: SIMD 优化、零拷贝设计
//!
//! ## 使用示例
//!
//! ### 图像处理
//!
//! ```no_run
//! use kernel::media::image::{ImageProcessor, ImageFormat, FilterType};
//!
//! let processor = ImageProcessor::new();
//! let img = processor.load("image.png")?;
//! let filtered = processor.apply_filter(&img, FilterType::GaussianBlur(5.0))?;
//! processor.save(&filtered, "output.jpg", ImageFormat::JPEG)?;
//! # Ok::<(), kernel::error::MediaError>(())
//! ```
//!
//! ### 视频解码
//!
//! ```no_run
//! use kernel::media::video::{VideoDecoder, VideoCodec, DecodeOptions};
//!
//! let decoder = VideoDecoder::new(VideoCodec::H265)?;
//! let options = DecodeOptions::default();
//! let frame = decoder.decode_next_frame(&data, &options)?;
//! # Ok::<(), kernel::error::MediaError>(())
//! ```
//!
//! ### 3D 渲染
//!
//! ```no_run
//! use kernel::media::render3d::{RenderEngine, RenderMode};
//!
//! let engine = RenderEngine::new(RenderMode::OpenGL)?;
//! engine.initialize_scene()?;
//! engine.render_frame()?;
//! # Ok::<(), kernel::error::MediaError>(())
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

pub mod image;
pub mod video;
pub mod render3d;
pub mod audio_studio;
pub mod font;
pub mod color;

// Re-export commonly used types
pub use image::{ImageProcessor, Image, ImageFormat, FilterType, ColorSpace};
pub use video::{VideoDecoder, VideoEncoder, VideoCodec, DecodeOptions, EncodeOptions};
pub use render3d::{RenderEngine, RenderMode, ShaderType, Geometry};
pub use audio_studio::{AudioStudio, AudioEffect, MIDINote};
pub use font::{FontEngine, Font, TextLayout};
pub use color::{ColorManager, ColorSpace as ColorSpaceType, ICCProfile};

/// Media subsystem errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    /// Invalid format
    InvalidFormat,
    /// Unsupported codec
    UnsupportedCodec,
    /// Encoding error
    EncodingError,
    /// Decoding error
    DecodingError,
    /// Out of memory
    OutOfMemory,
    /// Hardware error
    HardwareError,
    /// Invalid parameter
    InvalidParameter,
    /// Not implemented
    NotImplemented,
}

impl core::fmt::Display for MediaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MediaError::InvalidFormat => write!(f, "Invalid format"),
            MediaError::UnsupportedCodec => write!(f, "Unsupported codec"),
            MediaError::EncodingError => write!(f, "Encoding error"),
            MediaError::DecodingError => write!(f, "Decoding error"),
            MediaError::OutOfMemory => write!(f, "Out of memory"),
            MediaError::HardwareError => write!(f, "Hardware error"),
            MediaError::InvalidParameter => write!(f, "Invalid parameter"),
            MediaError::NotImplemented => write!(f, "Not implemented"),
        }
    }
}

/// Media result type
pub type MediaResult<T> = core::result::Result<T, MediaError>;

/// Hardware acceleration capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAcceleration {
    /// No hardware acceleration
    None,
    /// GPU acceleration
    GPU,
    /// DSP acceleration
    DSP,
    /// Both GPU and DSP
    Both,
}

/// Processing quality level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// Low quality, fast processing
    Low,
    /// Medium quality
    Medium,
    /// High quality
    High,
    /// Maximum quality
    Maximum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_error_display() {
        let err = MediaError::InvalidFormat;
        assert_eq!(format!("{}", err), "Invalid format");
    }

    #[test]
    fn test_hardware_acceleration() {
        assert_eq!(HardwareAcceleration::None, HardwareAcceleration::None);
        assert_ne!(HardwareAcceleration::GPU, HardwareAcceleration::DSP);
    }

    #[test]
    fn test_quality_levels() {
        assert!(Quality::Low as i32 < Quality::Maximum as i32);
    }
}
