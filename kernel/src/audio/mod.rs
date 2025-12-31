//! # Real-time Audio Subsystem for NOS Kernel
//!
//! This module provides a comprehensive audio subsystem supporting:
//! - ALSA (Advanced Linux Sound Architecture) compatibility
//! - JACK low-latency audio processing
//! - Real-time audio pipelines with lock-free synchronization
//! - Multiple audio codec support (Opus, FLAC, Vorbis, MP3)
//! - MIDI protocol support
//! - Real-time scheduling integration for audio tasks
//!
//! ## Architecture Overview
//!
//! The audio subsystem is organized into several layers:
//!
//! 1. **Hardware Abstraction (ALSA)**: Low-level hardware access
//! 2. **Low-Latency Processing (JACK)**: Real-time audio graph processing
//! 3. **Pipeline (pipeline)**: Real-time audio processing pipeline
//! 4. **Codecs (codec)**: Audio encoding/decoding
//! 5. **MIDI (midi)**: MIDI protocol and sequencer
//! 6. **Scheduler (sched)**: Real-time scheduling for audio tasks
//!
//! ## Features
//!
//! ### ALSA Support
//! - Full ALSA compatibility layer for Linux audio applications
//! - PCM playback and capture with mmap support
//! - Hardware parameter management
//! - Mixer controls for volume and routing
//! - Plugin architecture for extensibility
//!
//! ### JACK Integration
//! - Low-latency audio graph (< 5ms latency)
//! - Real-time client port connections
//! - Transport control for synchronization
//! - MIDI over JACK support
//! - D-Bus interface for control
//!
//! ### Real-time Pipeline
//! - Lock-free ring buffers for zero-copy audio transfer
//! - DSP chain processing with minimal latency
//! - Sample rate conversion with quality control
//! - Latency compensation across the pipeline
//! - Integration with real-time scheduler
//!
//! ### Codec Support
//! - **Opus**: RFC 6716 compliant encoder/decoder (8-48 kHz)
//! - **FLAC**: Lossless audio compression
//! - **Vorbis**: High-quality lossy compression
//! - **MP3**: libmpg123 compatible decoder
//! - PCM format conversion and resampling
//!
//! ### MIDI Support
//! - Complete MIDI protocol implementation
//! - Event handling and routing
//! - Sequencer with clock sync
//! - Standard MIDI File (SMF) support
//! - USB MIDI device support
//!
//! ## Performance
//!
//! - **Latency**: < 5ms end-to-end with JACK
//! - **Throughput**: Supports 192kHz/32-bit multi-channel audio
//! - **CPU**: Lock-free critical paths, no heap allocation
//! - **Memory**: Lock-free ring buffers with fixed capacity
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kernel::audio::{AlsaDevice, JackClient, AudioPipeline};
//!
//! // Open ALSA device
//! let device = AlsaDevice::open("hw:0", AlsaDirection::Playback)?;
//!
//! // Configure hardware parameters
//!
//! // Create JACK client
//! let client = JackClient::new("audio_app", JackOptions::default())?;
//! let port = client.register_port("output", JackPortType::AudioOut)?;
//!
//! // Create processing pipeline
//! let pipeline = AudioPipeline::new(48000, 2, 1024)?;
//! ```
//!
//! ## Real-time Guarantees
//!
//! The audio subsystem provides real-time guarantees through:
//! - Integration with SCHED_FIFO/SCHED_RR scheduling
//! - Lock-free data structures in critical paths
//! - CPU isolation for audio processing
//! - DMA integration for zero-copy I/O
//! - XRUN detection and recovery
//!
//! ## POSIX Compliance
//!
//! The audio subsystem follows:
//! - ALSA API (Linux sound subsystem)
//! - JACK API (JACK Audio Connection Kit)
//! - POSIX real-time scheduling (SCHED_FIFO)
//! - POSIX timers for audio timing

#![allow(dead_code)]

pub mod alsa;
pub mod jack;
pub mod pipeline;
pub mod codec;
pub mod midi;
pub mod sched;

// Re-exports for convenience

pub use alsa::{
    AlsaDevice,
    AlsaDirection,
    AlsaFormat,
    AlsaHwParams,
    AlsaSwParams,
    AlsaMixer,
    AlsaError,
};

pub use jack::{
    JackClient,
    JackPort,
    JackPortType,
    JackPortFlags,
    JackOptions,
    JackTransportState,
    JackError,
};

pub use pipeline::{
    AudioPipeline,
    AudioBuffer,
    DspNode,
    DspChain,
    RingBuffer,
    PipelineError,
};

pub use codec::{
    AudioCodec,
    CodecType,
    CodecParams,
    OpusEncoder,
    OpusDecoder,
    FlacEncoder,
    FlacDecoder,
    VorbisDecoder,
    Mp3Decoder,
    CodecError,
};

pub use midi::{
    MidiEvent,
    MidiMessage,
    MidiSequencer,
    MidiClock,
    MidiSmf,
    MidiError,
};

pub use sched::{
    AudioScheduler,
    AudioTask,
    XrunDetector,
    LatencyMonitor,
    SchedError,
};

/// Audio subsystem version
pub const AUDIO_VERSION: &str = "1.0.0";

/// Maximum number of audio channels
pub const MAX_CHANNELS: usize = 32;

/// Default sample rate (Hz)
pub const DEFAULT_SAMPLE_RATE: u32 = 48000;

/// Default buffer size (frames)
pub const DEFAULT_BUFFER_SIZE: usize = 1024;

/// Maximum sample rate (Hz)
pub const MAX_SAMPLE_RATE: u32 = 192000;

/// Minimum sample rate (Hz)
pub const MIN_SAMPLE_RATE: u32 = 8000;

/// Audio subsystem initialization state
static AUDIO_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Initialize the audio subsystem
pub fn init() -> crate::Result<()> {
    // Initialize audio scheduler
    if let Err(e) = sched::init() {
        crate::log_error!("Failed to initialize audio scheduler: {:?}", e);
    }

    crate::log_debug!("Audio subsystem v{} initialized", AUDIO_VERSION);
    crate::log_debug!("  - Max channels: {}", MAX_CHANNELS);
    crate::log_debug!("  - Sample rate range: {}-{} Hz", MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
    crate::log_debug!("  - Default buffer size: {} frames", DEFAULT_BUFFER_SIZE);

    AUDIO_INIT.store(true, core::sync::atomic::Ordering::Release);
    Ok(())
}

/// Check if audio subsystem is initialized
pub fn is_initialized() -> bool {
    AUDIO_INIT.load(core::sync::atomic::Ordering::Acquire)
}

/// Audio subsystem error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Subsystem not initialized
    NotInitialized,
    /// Invalid parameter
    InvalidParam,
    /// Device not found
    DeviceNotFound,
    /// Operation not supported
    NotSupported,
    /// Resource busy
    Busy,
    /// Out of memory
    OutOfMemory,
    /// Timeout
    Timeout,
    /// XRUN (underflow/overflow)
    Xrun,
    /// Codec-specific error
    Codec(CodecError),
    /// ALSA-specific error
    Alsa(AlsaError),
    /// JACK-specific error
    Jack(JackError),
    /// MIDI-specific error
    Midi(MidiError),
    /// Scheduler-specific error
    Sched(SchedError),
    /// Pipeline-specific error
    Pipeline(PipelineError),
}

impl From<CodecError> for Error {
    fn from(err: CodecError) -> Self {
        Self::Codec(err)
    }
}

impl From<AlsaError> for Error {
    fn from(err: AlsaError) -> Self {
        Self::Alsa(err)
    }
}

impl From<JackError> for Error {
    fn from(err: JackError) -> Self {
        Self::Jack(err)
    }
}

impl From<MidiError> for Error {
    fn from(err: MidiError) -> Self {
        Self::Midi(err)
    }
}

impl From<SchedError> for Error {
    fn from(err: SchedError) -> Self {
        Self::Sched(err)
    }
}

impl From<PipelineError> for Error {
    fn from(err: PipelineError) -> Self {
        Self::Pipeline(err)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Audio subsystem not initialized"),
            Self::InvalidParam => write!(f, "Invalid parameter"),
            Self::DeviceNotFound => write!(f, "Audio device not found"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::Busy => write!(f, "Audio resource busy"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::Timeout => write!(f, "Operation timeout"),
            Self::Xrun => write!(f, "XRUN (underflow/overflow) detected"),
            Self::Codec(e) => write!(f, "Codec error: {:?}", e),
            Self::Alsa(e) => write!(f, "ALSA error: {:?}", e),
            Self::Jack(e) => write!(f, "JACK error: {:?}", e),
            Self::Midi(e) => write!(f, "MIDI error: {:?}", e),
            Self::Sched(e) => write!(f, "Scheduler error: {:?}", e),
            Self::Pipeline(e) => write!(f, "Pipeline error: {:?}", e),
        }
    }
}

/// Result type for audio subsystem
pub type AudioResult<T> = Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_constants() {
        assert_eq!(MAX_CHANNELS, 32);
        assert_eq!(DEFAULT_SAMPLE_RATE, 48000);
        assert_eq!(DEFAULT_BUFFER_SIZE, 1024);
        assert_eq!(MAX_SAMPLE_RATE, 192000);
        assert_eq!(MIN_SAMPLE_RATE, 8000);
    }

    #[test]
    fn test_error_display() {
        let err = Error::NotInitialized;
        assert_eq!(format!("{}", err), "Audio subsystem not initialized");

        let err = Error::Xrun;
        assert_eq!(format!("{}", err), "XRUN (underflow/overflow) detected");
    }

    #[test]
    fn test_error_conversion() {
        let codec_err = CodecError::InvalidParam;
        let audio_err: Error = codec_err.into();
        assert!(matches!(audio_err, Error::Codec(_)));

        let alsa_err = AlsaError::DeviceNotFound;
        let audio_err: Error = alsa_err.into();
        assert!(matches!(audio_err, Error::Alsa(_)));
    }
}
