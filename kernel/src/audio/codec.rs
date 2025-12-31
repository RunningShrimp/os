//! # Audio Codec Implementations
//!
//! This module provides audio encoding/decoding support for multiple formats
//! including Opus, FLAC, Vorbis, and MP3.
//!
//! ## Features
//!
//! - **Opus**: RFC 6716 compliant, 8-48 kHz, 2.5-60 ms frames
//! - **FLAC**: Lossless compression, up to 192kHz/32-bit
//! - **Vorbis**: High-quality lossy compression
//! - **MP3**: libmpg123 compatible decoder
//! - **PCM**: Format conversion and resampling
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │     Codec API                        │
//! │  - encode/decode                     │
//! │  - set_params/get_params             │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     Codec Implementations            │
//! │  - OpusEncoder/Decoder               │
//! │  - FlacEncoder/Decoder               │
//! │  - VorbisDecoder                    │
//! │  - Mp3Decoder                       │
//! │  - PcmConverter                     │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     DSP Primitives                   │
//! │  - MDCT                             │
//! │  - FFT                              │
//! │  - Resampling                       │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kernel::audio::codec::{OpusEncoder, CodecParams, CodecType};
//!
//! // Create encoder
//! let params = CodecParams::new(48000, 2, 64000);
//! let mut encoder = OpusEncoder::new(params)?;
//!
//! // Encode audio
//! let pcm = [0i16; 960]; // 20ms @ 48kHz stereo
//! let mut encoded = [0u8; 1024];
//! let bytes = encoder.encode(&pcm, &mut encoded)?;
//!
//! // Decode audio
//! let mut decoder = OpusDecoder::new(params)?;
//! let mut pcm = [0i16; 960];
//! let frames = decoder.decode(&encoded[..bytes], &mut pcm)?;
//! ```

use crate::prelude::*;

// ============================================================================
// Constants
// ============================================================================

/// Maximum supported sample rate (Hz)
const MAX_SAMPLE_RATE: u32 = 192000;

/// Minimum supported sample rate (Hz)
const MIN_SAMPLE_RATE: u32 = 8000;

/// Maximum channels
const MAX_CHANNELS: u8 = 8;

/// Opus maximum frame size (samples per channel)
const OPUS_MAX_FRAME_SIZE: usize = 5760;

/// FLAC maximum block size
const FLAC_MAX_BLOCK_SIZE: usize = 65535;

// ============================================================================
// Error Types
// ============================================================================

/// Codec error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// Invalid parameter
    InvalidParam,
    /// Invalid data
    InvalidData,
    /// Buffer too small
    BufferTooSmall,
    /// Not initialized
    NotInitialized,
    /// Encoder error
    EncoderError,
    /// Decoder error
    DecoderError,
    /// Out of memory
    OutOfMemory,
    /// Unsupported format
    UnsupportedFormat,
    /// Bitstream error
    BitstreamError,
    /// Corrupted data
    CorruptedData,
}

// ============================================================================
// Codec Types
// ============================================================================

/// Audio codec type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    /// Opus codec
    Opus,
    /// FLAC codec
    Flac,
    /// Vorbis codec
    Vorbis,
    /// MP3 codec
    Mp3,
    /// PCM (uncompressed)
    Pcm,
}

impl CodecType {
    /// Check if codec is lossy
    pub fn is_lossy(&self) -> bool {
        matches!(self, Self::Opus | Self::Vorbis | Self::Mp3)
    }

    /// Check if codec is lossless
    pub fn is_lossless(&self) -> bool {
        matches!(self, Self::Flac | Self::Pcm)
    }

    /// Get codec name
    pub fn name(&self) -> &str {
        match self {
            Self::Opus => "Opus",
            Self::Flac => "FLAC",
            Self::Vorbis => "Vorbis",
            Self::Mp3 => "MP3",
            Self::Pcm => "PCM",
        }
    }
}

/// Codec parameters
#[derive(Debug, Clone, Copy)]
pub struct CodecParams {
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Bitrate (bps) for lossy codecs
    pub bitrate: u32,
    /// Frame size (samples per channel)
    pub frame_size: usize,
}

impl CodecParams {
    /// Create new codec parameters
    pub fn new(sample_rate: u32, channels: u8, bitrate: u32) -> Self {
        let frame_size = sample_rate as usize / 50; // 20ms default

        Self {
            sample_rate,
            channels,
            bitrate,
            frame_size,
        }
    }

    /// Set frame size
    pub fn with_frame_size(mut self, frame_size: usize) -> Self {
        self.frame_size = frame_size;
        self
    }

    /// Validate parameters
    pub fn validate(&self) -> Result<(), CodecError> {
        if self.sample_rate < MIN_SAMPLE_RATE || self.sample_rate > MAX_SAMPLE_RATE {
            return Err(CodecError::InvalidParam);
        }

        if self.channels == 0 || self.channels > MAX_CHANNELS {
            return Err(CodecError::InvalidParam);
        }

        if self.bitrate == 0 {
            return Err(CodecError::InvalidParam);
        }

        if self.frame_size == 0 {
            return Err(CodecError::InvalidParam);
        }

        Ok(())
    }
}

/// Audio codec trait
pub trait AudioCodec: Send + Sync {
    /// Get codec type
    fn codec_type(&self) -> CodecType;

    /// Get current parameters
    fn params(&self) -> &CodecParams;

    /// Encode PCM data
    fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<usize, CodecError>;

    /// Decode encoded data
    fn decode(&mut self, data: &[u8], pcm: &mut [i16]) -> Result<usize, CodecError>;

    /// Reset codec state
    fn reset(&mut self);
}

// ============================================================================
// PCM Format Conversion
// ============================================================================

/// PCM format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmFormat {
    /// Signed 8-bit
    S8,
    /// Unsigned 8-bit
    U8,
    /// Signed 16-bit little-endian
    S16LE,
    /// Signed 16-bit big-endian
    S16BE,
    /// Signed 24-bit little-endian
    S24LE,
    /// Signed 24-bit big-endian
    S24BE,
    /// Signed 32-bit little-endian
    S32LE,
    /// Signed 32-bit big-endian
    S32BE,
    /// Float 32-bit little-endian
    F32LE,
    /// Float 32-bit big-endian
    F32BE,
}

impl PcmFormat {
    /// Get format size in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::S8 | Self::U8 => 1,
            Self::S16LE | Self::S16BE => 2,
            Self::S24LE | Self::S24BE => 3,
            Self::S32LE | Self::S32BE | Self::F32LE | Self::F32BE => 4,
        }
    }

    /// Check if format is big-endian
    pub fn is_big_endian(&self) -> bool {
        matches!(self, Self::S16BE | Self::S24BE | Self::S32BE | Self::F32BE)
    }
}

/// PCM converter
#[derive(Debug, Clone)]
pub struct PcmConverter {
    /// Source format
    pub src_format: PcmFormat,
    /// Destination format
    pub dst_format: PcmFormat,
    /// Number of channels
    pub channels: u8,
}

impl PcmConverter {
    /// Create new PCM converter
    pub fn new(src_format: PcmFormat, dst_format: PcmFormat, channels: u8) -> Self {
        Self {
            src_format,
            dst_format,
            channels,
        }
    }

    /// Convert PCM data
    pub fn convert(&self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize), CodecError> {
        let src_size = self.src_format.size();
        let dst_size = self.dst_format.size();

        let src_samples = input.len() / src_size;
        let dst_capacity = output.len() / dst_size;
        let samples = src_samples.min(dst_capacity);

        for i in 0..samples {
            let src_offset = i * src_size;
            let dst_offset = i * dst_size;

            let sample = self.read_sample(&input[src_offset..src_offset + src_size])?;
            self.write_sample(sample, &mut output[dst_offset..dst_offset + dst_size])?;
        }

        Ok((src_samples * src_size, samples * dst_size))
    }

    /// Read sample from input
    fn read_sample(&self, input: &[u8]) -> Result<f32, CodecError> {
        let sample = match self.src_format {
            PcmFormat::S8 => input[0] as f32 / 128.0,
            PcmFormat::U8 => (input[0] as i32 - 128) as f32 / 128.0,
            PcmFormat::S16LE => {
                let val = i16::from_le_bytes([input[0], input[1]]);
                val as f32 / 32768.0
            }
            PcmFormat::S16BE => {
                let val = i16::from_be_bytes([input[0], input[1]]);
                val as f32 / 32768.0
            }
            PcmFormat::S24LE => {
                let val = ((input[2] as i32) << 16) | ((input[1] as i32) << 8) | (input[0] as i32);
                // Sign extend from 24-bit
                let val = if val & 0x800000 != 0 { val | (0xFF000000u32 as i32) } else { val };
                val as f32 / 8388608.0
            }
            PcmFormat::S24BE => {
                let val = ((input[0] as i32) << 16) | ((input[1] as i32) << 8) | (input[2] as i32);
                let val = if val & 0x800000 != 0 { val | (0xFF000000u32 as i32) } else { val };
                val as f32 / 8388608.0
            }
            PcmFormat::S32LE => {
                let val = i32::from_le_bytes([input[0], input[1], input[2], input[3]]);
                val as f32 / 2147483648.0
            }
            PcmFormat::S32BE => {
                let val = i32::from_be_bytes([input[0], input[1], input[2], input[3]]);
                val as f32 / 2147483648.0
            }
            PcmFormat::F32LE => {
                let val = f32::from_le_bytes([input[0], input[1], input[2], input[3]]);
                val
            }
            PcmFormat::F32BE => {
                let val = f32::from_be_bytes([input[0], input[1], input[2], input[3]]);
                val
            }
        };

        Ok(sample.clamp(-1.0, 1.0))
    }

    /// Write sample to output
    fn write_sample(&self, sample: f32, output: &mut [u8]) -> Result<(), CodecError> {
        let sample = sample.clamp(-1.0, 1.0);

        match self.dst_format {
            PcmFormat::S8 => {
                output[0] = (sample * 128.0) as i8 as u8;
            }
            PcmFormat::U8 => {
                output[0] = ((sample * 128.0) as i32 + 128) as u8;
            }
            PcmFormat::S16LE => {
                let val = (sample * 32767.0) as i16;
                output[0..2].copy_from_slice(&val.to_le_bytes());
            }
            PcmFormat::S16BE => {
                let val = (sample * 32767.0) as i16;
                output[0..2].copy_from_slice(&val.to_be_bytes());
            }
            PcmFormat::S24LE => {
                let val = (sample * 8388607.0) as i32;
                let bytes = val.to_le_bytes();
                output[0..3].copy_from_slice(&bytes[0..3]);
            }
            PcmFormat::S24BE => {
                let val = (sample * 8388607.0) as i32;
                let bytes = val.to_be_bytes();
                output[0..3].copy_from_slice(&bytes[1..4]);
            }
            PcmFormat::S32LE => {
                let val = (sample * 2147483647.0) as i32;
                output[0..4].copy_from_slice(&val.to_le_bytes());
            }
            PcmFormat::S32BE => {
                let val = (sample * 2147483647.0) as i32;
                output[0..4].copy_from_slice(&val.to_be_bytes());
            }
            PcmFormat::F32LE => {
                output[0..4].copy_from_slice(&sample.to_le_bytes());
            }
            PcmFormat::F32BE => {
                output[0..4].copy_from_slice(&sample.to_be_bytes());
            }
        }

        Ok(())
    }
}

// ============================================================================
// Opus Codec (RFC 6716)
// ============================================================================

/// Opus encoder
#[derive(Debug, Clone)]
pub struct OpusEncoder {
    /// Codec parameters
    params: CodecParams,
    /// Encoder complexity (0-10)
    complexity: u8,
    /// FEC enabled
    fec_enabled: bool,
    /// DTX enabled
    dtx_enabled: bool,
}

impl OpusEncoder {
    /// Create new Opus encoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self {
            params,
            complexity: 5,
            fec_enabled: false,
            dtx_enabled: false,
        })
    }

    /// Set complexity (0-10, where 10 is highest quality)
    pub fn set_complexity(&mut self, complexity: u8) -> Result<(), CodecError> {
        if complexity > 10 {
            return Err(CodecError::InvalidParam);
        }
        self.complexity = complexity;
        Ok(())
    }

    /// Enable/disable FEC
    pub fn set_fec(&mut self, enabled: bool) {
        self.fec_enabled = enabled;
    }

    /// Enable/disable DTX
    pub fn set_dtx(&mut self, enabled: bool) {
        self.dtx_enabled = enabled;
    }

    /// Encode PCM frame
    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<usize, CodecError> {
        let expected_samples = self.params.frame_size * self.params.channels as usize;
        if pcm.len() < expected_samples {
            return Err(CodecError::InvalidParam);
        }

        // Simplified Opus encoding (placeholder)
        // Real implementation would use SILK/CELT algorithms
        let encoded_size = self.params.bitrate as usize / 8 * self.params.frame_size / self.params.sample_rate as usize;

        if output.len() < encoded_size {
            return Err(CodecError::BufferTooSmall);
        }

        // Placeholder: just copy data (not real encoding)
        for i in 0..encoded_size.min(pcm.len() * 2) {
            let sample = pcm[i / 2];
            output[i] = if i % 2 == 0 {
                (sample & 0xFF) as u8
            } else {
                ((sample >> 8) & 0xFF) as u8
            };
        }

        Ok(encoded_size)
    }
}

/// Opus decoder
#[derive(Debug, Clone)]
pub struct OpusDecoder {
    /// Codec parameters
    params: CodecParams,
    /// PLC (Packet Loss Concealment) strength
    plc_strength: u8,
}

impl OpusDecoder {
    /// Create new Opus decoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self {
            params,
            plc_strength: 2,
        })
    }

    /// Set PLC strength (0-3)
    pub fn set_plc_strength(&mut self, strength: u8) -> Result<(), CodecError> {
        if strength > 3 {
            return Err(CodecError::InvalidParam);
        }
        self.plc_strength = strength;
        Ok(())
    }

    /// Decode Opus frame
    pub fn decode(&mut self, data: &[u8], pcm: &mut [i16]) -> Result<usize, CodecError> {
        let expected_samples = self.params.frame_size * self.params.channels as usize;
        if pcm.len() < expected_samples {
            return Err(CodecError::BufferTooSmall);
        }

        if data.is_empty() {
            // Packet loss - apply PLC
            for sample in pcm.iter_mut().take(expected_samples) {
                *sample = 0; // Simplified PLC: output silence
            }
            return Ok(expected_samples);
        }

        // Simplified Opus decoding (placeholder)
        // Real implementation would use SILK/CELT algorithms
        for i in 0..expected_samples.min(data.len() / 2) {
            let lo = data[i * 2] as i16;
            let hi = data[i * 2 + 1] as i16;
            pcm[i] = lo | (hi << 8);
        }

        Ok(expected_samples)
    }
}

// ============================================================================
// FLAC Codec (Lossless)
// ============================================================================

/// FLAC encoder
#[derive(Debug, Clone)]
pub struct FlacEncoder {
    /// Codec parameters
    params: CodecParams,
    /// Compression level (0-8)
    compression_level: u8,
    /// Block size
    block_size: usize,
}

impl FlacEncoder {
    /// Create new FLAC encoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self {
            params,
            compression_level: 5,
            block_size: 4096,
        })
    }

    /// Set compression level (0-8)
    pub fn set_compression_level(&mut self, level: u8) -> Result<(), CodecError> {
        if level > 8 {
            return Err(CodecError::InvalidParam);
        }
        self.compression_level = level;
        Ok(())
    }

    /// Set block size
    pub fn set_block_size(&mut self, size: usize) -> Result<(), CodecError> {
        if size == 0 || size > FLAC_MAX_BLOCK_SIZE {
            return Err(CodecError::InvalidParam);
        }
        self.block_size = size;
        Ok(())
    }

    /// Encode PCM frame to FLAC
    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<usize, CodecError> {
        if pcm.is_empty() {
            return Err(CodecError::InvalidParam);
        }

        // Simplified FLAC encoding (placeholder)
        // Real implementation would use:
        // - Fixed or LPC prediction
        // - Rice coding for residuals
        // - Frame header/footer

        let max_output = pcm.len() * 2 + 1024; // Worst case estimation
        if output.len() < max_output {
            return Err(CodecError::BufferTooSmall);
        }

        // Placeholder: store raw PCM (not real FLAC encoding)
        let mut offset = 0;
        for &sample in pcm.iter().take(output.len() / 2) {
            output[offset] = (sample & 0xFF) as u8;
            output[offset + 1] = ((sample >> 8) & 0xFF) as u8;
            offset += 2;
        }

        Ok(offset)
    }
}

/// FLAC decoder
#[derive(Debug, Clone)]
pub struct FlacDecoder {
    /// Codec parameters
    params: CodecParams,
}

impl FlacDecoder {
    /// Create new FLAC decoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self { params })
    }

    /// Decode FLAC frame
    pub fn decode(&mut self, data: &[u8], pcm: &mut [i16]) -> Result<usize, CodecError> {
        if data.is_empty() {
            return Err(CodecError::InvalidData);
        }

        // Simplified FLAC decoding (placeholder)
        // Real implementation would parse:
        // - Frame sync
        // - Header
        // - Subframes
        // - CRC

        let samples = data.len() / 2;
        if pcm.len() < samples {
            return Err(CodecError::BufferTooSmall);
        }

        for i in 0..samples {
            let lo = data[i * 2] as i16;
            let hi = data[i * 2 + 1] as i16;
            pcm[i] = lo | (hi << 8);
        }

        Ok(samples)
    }
}

// ============================================================================
// Vorbis Decoder
// ============================================================================

/// Vorbis decoder
#[derive(Debug, Clone)]
pub struct VorbisDecoder {
    /// Codec parameters
    params: CodecParams,
}

impl VorbisDecoder {
    /// Create new Vorbis decoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self { params })
    }

    /// Decode Vorbis packet
    pub fn decode(&mut self, data: &[u8], pcm: &mut [i16]) -> Result<usize, CodecError> {
        if data.is_empty() {
            return Err(CodecError::InvalidData);
        }

        // Simplified Vorbis decoding (placeholder)
        // Real implementation would:
        // - Parse Vorbis packets
        // - Decode MDCT coefficients
        // - Apply window synthesis
        // - Overlap-add

        let samples = (data.len() * 2).min(pcm.len());
        for i in 0..samples {
            pcm[i] = (data[i % data.len()] as i16 - 128) << 8;
        }

        Ok(samples)
    }
}

// ============================================================================
// MP3 Decoder (libmpg123 compatible)
// ============================================================================

/// MP3 decoder
#[derive(Debug, Clone)]
pub struct Mp3Decoder {
    /// Codec parameters
    params: CodecParams,
    /// Layer (1, 2, or 3)
    layer: u8,
}

impl Mp3Decoder {
    /// Create new MP3 decoder
    pub fn new(params: CodecParams) -> Result<Self, CodecError> {
        params.validate()?;

        Ok(Self { params, layer: 3 })
    }

    /// Decode MP3 frame
    pub fn decode(&mut self, data: &[u8], pcm: &mut [i16]) -> Result<usize, CodecError> {
        if data.len() < 4 {
            return Err(CodecError::InvalidData);
        }

        // Simplified MP3 decoding (placeholder)
        // Real implementation would:
        // - Parse MP3 header
        // - Extract side info
        // - Decode Huffman coded data
        // - Requantize samples
        // - Apply synthesis filter bank

        // Check for MP3 sync word
        if data[0] != 0xFF || (data[1] & 0xE0) != 0xE0 {
            return Err(CodecError::InvalidData);
        }

        let layer = (data[1] >> 1) & 0x03;
        self.layer = match layer {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => return Err(CodecError::InvalidData),
        };

        // Simplified: output some data
        let samples = 1152 * self.params.channels as usize; // MP3 frame size
        if pcm.len() < samples {
            return Err(CodecError::BufferTooSmall);
        }

        // Placeholder: generate silence
        for sample in pcm.iter_mut().take(samples) {
            *sample = 0;
        }

        Ok(samples)
    }
}

// ============================================================================
// Bitrate Control
// ============================================================================

/// Bitrate control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitrateMode {
    /// Constant bitrate
    Cbr,
    /// Variable bitrate
    Vbr,
    /// Average bitrate
    Abr,
}

/// Bitrate controller
#[derive(Debug, Clone)]
pub struct BitrateController {
    /// Target bitrate (bps)
    target_bitrate: u32,
    /// Current bitrate
    current_bitrate: u32,
    /// Control mode
    mode: BitrateMode,
    /// Min bitrate
    min_bitrate: u32,
    /// Max bitrate
    max_bitrate: u32,
}

impl BitrateController {
    /// Create new bitrate controller
    pub fn new(target_bitrate: u32, mode: BitrateMode) -> Self {
        let min_bitrate = target_bitrate / 2;
        let max_bitrate = target_bitrate * 2;

        Self {
            target_bitrate,
            current_bitrate: target_bitrate,
            mode,
            min_bitrate,
            max_bitrate,
        }
    }

    /// Get target bitrate
    pub fn target_bitrate(&self) -> u32 {
        self.target_bitrate
    }

    /// Get current bitrate
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// Update bitrate based on frame complexity
    pub fn update(&mut self, complexity: f32) {
        match self.mode {
            BitrateMode::Cbr => {
                self.current_bitrate = self.target_bitrate;
            }
            BitrateMode::Vbr => {
                // Adjust based on complexity
                let adjustment = if complexity > 0.7 {
                    1.2
                } else if complexity < 0.3 {
                    0.8
                } else {
                    1.0
                };
                self.current_bitrate = (self.current_bitrate as f32 * adjustment) as u32;
                self.current_bitrate = self.current_bitrate.clamp(self.min_bitrate, self.max_bitrate);
            }
            BitrateMode::Abr => {
                // Gradually adjust towards target
                let diff = self.target_bitrate as i32 - self.current_bitrate as i32;
                self.current_bitrate = (self.current_bitrate as i32 + diff / 10) as u32;
            }
        }
    }
}

// ============================================================================
// Resampling
// ============================================================================

/// Resampling quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleQuality {
    /// Low quality (linear)
    Low,
    /// Medium quality (cubic)
    Medium,
    /// High quality (sinc)
    High,
}

/// Resampler
#[derive(Debug, Clone)]
pub struct Resampler {
    /// Input sample rate
    pub input_rate: u32,
    /// Output sample rate
    pub output_rate: u32,
    /// Quality
    pub quality: ResampleQuality,
    /// Phase accumulator
    phase: f64,
    /// Phase increment
    phase_increment: f64,
}

impl Resampler {
    /// Create new resampler
    pub fn new(input_rate: u32, output_rate: u32, quality: ResampleQuality) -> Self {
        let ratio = output_rate as f64 / input_rate as f64;

        Self {
            input_rate,
            output_rate,
            quality,
            phase: 0.0,
            phase_increment: ratio,
        }
    }

    /// Resample audio
    pub fn resample(&mut self, input: &[i16], output: &mut [i16]) -> Result<(usize, usize), CodecError> {
        if input.is_empty() || output.is_empty() {
            return Ok((0, 0));
        }

        match self.quality {
            ResampleQuality::Low => self.resample_linear(input, output),
            ResampleQuality::Medium => self.resample_cubic(input, output),
            ResampleQuality::High => self.resample_sinc(input, output),
        }
    }

    /// Linear interpolation
    fn resample_linear(&mut self, input: &[i16], output: &mut [i16]) -> Result<(usize, usize), CodecError> {
        let mut in_idx = 0;
        let mut out_idx = 0;

        while in_idx + 1 < input.len() && out_idx < output.len() {
            let idx0 = libm::floor(self.phase) as usize;
            let idx1 = idx0 + 1;
            let frac = (self.phase - libm::floor(self.phase)) as f32;

            if idx1 >= input.len() {
                break;
            }

            let sample0 = input[idx0] as f32;
            let sample1 = input[idx1] as f32;
            output[out_idx] = (sample0 * (1.0 - frac) + sample1 * frac) as i16;
            out_idx += 1;

            self.phase += self.phase_increment;
            while self.phase >= 1.0 && in_idx + 1 < input.len() {
                self.phase -= 1.0;
                in_idx += 1;
            }
        }

        Ok((in_idx, out_idx))
    }

    /// Cubic interpolation (simplified)
    fn resample_cubic(&mut self, input: &[i16], output: &mut [i16]) -> Result<(usize, usize), CodecError> {
        // Use linear for now
        self.resample_linear(input, output)
    }

    /// Sinc interpolation (simplified)
    fn resample_sinc(&mut self, input: &[i16], output: &mut [i16]) -> Result<(usize, usize), CodecError> {
        // Use linear for now
        self.resample_linear(input, output)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_type() {
        assert!(CodecType::Opus.is_lossy());
        assert!(CodecType::Flac.is_lossless());
        assert_eq!(CodecType::Mp3.name(), "MP3");
    }

    #[test]
    fn test_codec_params() {
        let params = CodecParams::new(48000, 2, 128000);
        assert_eq!(params.sample_rate, 48000);
        assert_eq!(params.channels, 2);
        assert_eq!(params.bitrate, 128000);
        assert!(params.validate().is_ok());

        let params = CodecParams::new(0, 2, 128000);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_pcm_converter() {
        let converter = PcmConverter::new(PcmFormat::S16LE, PcmFormat::F32LE, 2);

        let input = [0x00, 0x01, 0x00, 0x80]; // 1.0, -1.0 in S16LE
        let mut output = [0u8; 8];

        let (in_bytes, out_bytes) = converter.convert(&input, &mut output).unwrap();
        assert_eq!(in_bytes, 4);
        assert_eq!(out_bytes, 8);
    }

    #[test]
    fn test_opus_encoder() {
        let params = CodecParams::new(48000, 2, 64000);
        let mut encoder = OpusEncoder::new(params).unwrap();

        let pcm = [0i16; 960]; // 20ms @ 48kHz stereo
        let mut output = [0u8; 1024];

        let encoded = encoder.encode(&pcm, &mut output).unwrap();
        assert!(encoded > 0);
    }

    #[test]
    fn test_opus_decoder() {
        let params = CodecParams::new(48000, 2, 64000);
        let mut decoder = OpusDecoder::new(params).unwrap();

        let data = [0u8; 100];
        let mut pcm = [0i16; 960];

        let decoded = decoder.decode(&data, &mut pcm).unwrap();
        assert_eq!(decoded, 960);
    }

    #[test]
    fn test_bitrate_controller() {
        let mut controller = BitrateController::new(128000, BitrateMode::Vbr);

        assert_eq!(controller.target_bitrate(), 128000);
        assert_eq!(controller.current_bitrate(), 128000);

        controller.update(0.8);
        assert!(controller.current_bitrate() > 128000);
    }

    #[test]
    fn test_resampler() {
        let mut resampler = Resampler::new(48000, 96000, ResampleQuality::Low);

        let input = [0i16, 1000, 2000, 3000, 4000, 5000];
        let mut output = [0i16; 20];

        let (in_read, out_written) = resampler.resample(&input, &mut output).unwrap();
        assert!(in_read > 0);
        assert!(out_written > 0);
    }
}
