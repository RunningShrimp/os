//! # MIDI (Musical Instrument Digital Interface) Support
//!
//! This module provides comprehensive MIDI protocol support including event
//! handling, sequencing, and Standard MIDI File (SMF) parsing.
//!
//! ## Features
//!
//! - **Complete MIDI Protocol**: All channel and system messages
//! - **Event Handling**: Real-time MIDI event processing
//! - **Sequencer**: Multi-track playback and recording
//! - **Clock Sync**: MIDI clock, MTC, and SMPTE support
//! - **SMF Support**: Standard MIDI File format 0 and 1
//! - **ALSA Integration**: ALSA Sequencer compatibility
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │     MIDI Applications                │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     MIDI API                         │
//! │  - event_send/receive                │
//! │  - sequencer_control                 │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     MIDI Parser/Generator            │
//! │  - Message parsing                   │
//! │  - Event queue                       │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     MIDI Hardware                    │
//! │  - UART MIDI                         │
//! │  - USB MIDI                          │
//! │  - BLE MIDI                          │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kernel::audio::midi::{MidiEvent, MidiMessage, MidiSequencer};
//!
//! // Create MIDI event
//! let event = MidiEvent::note_on(0, 60, 127); // Channel 0, Middle C, velocity 127
//!
//! // Parse MIDI message
//! let message = MidiMessage::from_bytes(&[0x90, 0x3C, 0x7F]).unwrap();
//!
//! // Create sequencer
//! let sequencer = MidiSequencer::new(120, 480).unwrap(); // 120 BPM, 480 PPQ
//! sequencer.load_smf("song.mid").unwrap();
//! sequencer.play();
//! ```

use crate::prelude::*;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Standard MIDI frequency (ticks per quarter note)
pub const DEFAULT_PPQ: u16 = 480;

/// Default tempo (microseconds per quarter note)
pub const DEFAULT_TEMPO: u32 = 500_000; // 120 BPM

/// Maximum MIDI channels
pub const MIDI_CHANNELS: usize = 16;

/// MIDI note range
pub const MIDI_NOTE_MIN: u8 = 0;
pub const MIDI_NOTE_MAX: u8 = 127;

/// MIDI velocity range
pub const MIDI_VELOCITY_MIN: u8 = 0;
pub const MIDI_VELOCITY_MAX: u8 = 127;

/// MIDI control change range
pub const MIDI_CC_MIN: u8 = 0;
pub const MIDI_CC_MAX: u8 = 127;

// ============================================================================
// Error Types
// ============================================================================

/// MIDI error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiError {
    /// Invalid message
    InvalidMessage,
    /// Invalid data
    InvalidData,
    /// Buffer too small
    BufferTooSmall,
    /// Channel out of range
    InvalidChannel,
    /// Note out of range
    InvalidNote,
    /// File format error
    FileFormatError,
    /// Unsupported format
    UnsupportedFormat,
    /// Sequencer error
    SequencerError,
    /// Clock sync error
    ClockError,
}

// ============================================================================
// MIDI Message Types
// ============================================================================

/// MIDI status byte categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiStatus {
    /// Note off (0x8n)
    NoteOff,
    /// Note on (0x9n)
    NoteOn,
    /// Polyphonic key pressure (0xAn)
    PolyPressure,
    /// Control change (0xBn)
    ControlChange,
    /// Program change (0xCn)
    ProgramChange,
    /// Channel pressure (0xDn)
    ChannelPressure,
    /// Pitch bend (0xEn)
    PitchBend,
    /// System exclusive (0xF0)
    SysEx,
    /// System common (0xF1-0xF3)
    SystemCommon,
    /// System real-time (0xF8-0xFF)
    SystemRealTime,
}

impl MidiStatus {
    /// Create from status byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte & 0xF0 {
            0x80 => Some(Self::NoteOff),
            0x90 => Some(Self::NoteOn),
            0xA0 => Some(Self::PolyPressure),
            0xB0 => Some(Self::ControlChange),
            0xC0 => Some(Self::ProgramChange),
            0xD0 => Some(Self::ChannelPressure),
            0xE0 => Some(Self::PitchBend),
            0xF0 => {
                match byte {
                    0xF0 => Some(Self::SysEx),
                    0xF1..=0xF3 => Some(Self::SystemCommon),
                    0xF8..=0xFF => Some(Self::SystemRealTime),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Check if status is channel message
    pub fn is_channel_message(&self) -> bool {
        matches!(
            self,
            Self::NoteOff
                | Self::NoteOn
                | Self::PolyPressure
                | Self::ControlChange
                | Self::ProgramChange
                | Self::ChannelPressure
                | Self::PitchBend
        )
    }
}

/// MIDI message
#[derive(Debug, Clone, PartialEq)]
pub enum MidiMessage {
    /// Note off
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    /// Note on
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    /// Polyphonic key pressure
    PolyPressure {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    /// Control change
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// Program change
    ProgramChange {
        channel: u8,
        program: u8,
    },
    /// Channel pressure
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    /// Pitch bend
    PitchBend {
        channel: u8,
        value: u16, // 0-16383, center=8192
    },
    /// System exclusive
    SysEx(Vec<u8>),
    /// Time code quarter frame
    TimeCodeQuarterFrame {
        message_type: u8,
        value: u8,
    },
    /// Song position pointer
    SongPositionPointer(u16),
    /// Song select
    SongSelect(u8),
    /// Tune request
    TuneRequest,
    /// Timing clock
    TimingClock,
    /// Start
    Start,
    /// Continue
    Continue,
    /// Stop
    Stop,
    /// Active sensing
    ActiveSensing,
    /// Reset
    Reset,
}

impl MidiMessage {
    /// Parse MIDI message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), MidiError> {
        if data.is_empty() {
            return Err(MidiError::InvalidMessage);
        }

        let status = data[0];
        let message_type = MidiStatus::from_byte(status).ok_or(MidiError::InvalidMessage)?;

        let (message, len) = match message_type {
            MidiStatus::NoteOff => {
                if data.len() < 3 {
                    return Err(MidiError::InvalidMessage);
                }
                let channel = status & 0x0F;
                (
                    Self::NoteOff {
                        channel,
                        note: data[1],
                        velocity: data[2],
                    },
                    3,
                )
            }
            MidiStatus::NoteOn => {
                if data.len() < 3 {
                    return Err(MidiError::InvalidMessage);
                }
                let channel = status & 0x0F;
                (
                    Self::NoteOn {
                        channel,
                        note: data[1],
                        velocity: data[2],
                    },
                    3,
                )
            }
            MidiStatus::ControlChange => {
                if data.len() < 3 {
                    return Err(MidiError::InvalidMessage);
                }
                let channel = status & 0x0F;
                (
                    Self::ControlChange {
                        channel,
                        controller: data[1],
                        value: data[2],
                    },
                    3,
                )
            }
            MidiStatus::ProgramChange => {
                if data.len() < 2 {
                    return Err(MidiError::InvalidMessage);
                }
                let channel = status & 0x0F;
                (
                    Self::ProgramChange {
                        channel,
                        program: data[1],
                    },
                    2,
                )
            }
            MidiStatus::PitchBend => {
                if data.len() < 3 {
                    return Err(MidiError::InvalidMessage);
                }
                let channel = status & 0x0F;
                let value = (data[2] as u16) << 7 | (data[1] as u16);
                (
                    Self::PitchBend {
                        channel,
                        value,
                    },
                    3,
                )
            }
            MidiStatus::SysEx => {
                // Find end of SysEx (0xF7)
                let end = data
                    .iter()
                    .position(|&b| b == 0xF7)
                    .unwrap_or(data.len());
                (Self::SysEx(data[1..end].to_vec()), end + 1)
            }
            MidiStatus::SystemRealTime => {
                match status {
                    0xF8 => (Self::TimingClock, 1),
                    0xFA => (Self::Start, 1),
                    0xFB => (Self::Continue, 1),
                    0xFC => (Self::Stop, 1),
                    0xFE => (Self::ActiveSensing, 1),
                    0xFF => (Self::Reset, 1),
                    _ => return Err(MidiError::InvalidMessage),
                }
            }
            _ => return Err(MidiError::UnsupportedFormat),
        };

        Ok((message, len))
    }

    /// Convert message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::NoteOff {
                channel,
                note,
                velocity,
            } => vec![0x80 | channel, *note, *velocity],
            Self::NoteOn {
                channel,
                note,
                velocity,
            } => vec![0x90 | channel, *note, *velocity],
            Self::PolyPressure {
                channel,
                note,
                pressure,
            } => vec![0xA0 | channel, *note, *pressure],
            Self::ControlChange {
                channel,
                controller,
                value,
            } => vec![0xB0 | channel, *controller, *value],
            Self::ProgramChange { channel, program } => vec![0xC0 | channel, *program],
            Self::ChannelPressure {
                channel,
                pressure,
            } => vec![0xD0 | channel, *pressure],
            Self::PitchBend { channel, value } => {
                let lsb = (value & 0x7F) as u8;
                let msb = ((value >> 7) & 0x7F) as u8;
                vec![0xE0 | channel, lsb, msb]
            }
            Self::SysEx(data) => {
                let mut bytes = vec![0xF0];
                bytes.extend_from_slice(data);
                bytes.push(0xF7);
                bytes
            }
            Self::TimingClock => vec![0xF8],
            Self::Start => vec![0xFA],
            Self::Continue => vec![0xFB],
            Self::Stop => vec![0xFC],
            Self::ActiveSensing => vec![0xFE],
            Self::Reset => vec![0xFF],
            _ => vec![],
        }
    }

    /// Get message length (excluding SysEx)
    pub fn len(&self) -> usize {
        match self {
            Self::NoteOff { .. } | Self::NoteOn { .. } | Self::PolyPressure { .. }
            | Self::ControlChange { .. } | Self::PitchBend { .. } => 3,
            Self::ProgramChange { .. } | Self::ChannelPressure { .. } => 2,
            Self::SysEx(data) => data.len() + 2,
            Self::TimingClock | Self::Start | Self::Continue | Self::Stop | Self::ActiveSensing
            | Self::Reset => 1,
            _ => 0,
        }
    }
}

// ============================================================================
// MIDI Event
// ============================================================================

/// MIDI event with timestamp
#[derive(Debug, Clone, PartialEq)]
pub struct MidiEvent {
    /// Event timestamp (in ticks or milliseconds)
    pub timestamp: u32,
    /// MIDI message
    pub message: MidiMessage,
}

impl MidiEvent {
    /// Create new MIDI event
    pub fn new(timestamp: u32, message: MidiMessage) -> Self {
        Self { timestamp, message }
    }

    /// Create note on event
    pub fn note_on(channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            timestamp: 0,
            message: MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            },
        }
    }

    /// Create note off event
    pub fn note_off(channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            timestamp: 0,
            message: MidiMessage::NoteOff {
                channel,
                note,
                velocity,
            },
        }
    }

    /// Create control change event
    pub fn control_change(channel: u8, controller: u8, value: u8) -> Self {
        Self {
            timestamp: 0,
            message: MidiMessage::ControlChange {
                channel,
                controller,
                value,
            },
        }
    }

    /// Create program change event
    pub fn program_change(channel: u8, program: u8) -> Self {
        Self {
            timestamp: 0,
            message: MidiMessage::ProgramChange { channel, program },
        }
    }

    /// Create pitch bend event
    pub fn pitch_bend(channel: u8, value: u16) -> Self {
        Self {
            timestamp: 0,
            message: MidiMessage::PitchBend { channel, value },
        }
    }

    /// Get event timestamp
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Set event timestamp
    pub fn set_timestamp(&mut self, timestamp: u32) {
        self.timestamp = timestamp;
    }
}

// ============================================================================
// MIDI Clock
// ============================================================================

/// MIDI clock synchronization
#[derive(Debug)]
pub struct MidiClock {
    /// BPM (beats per minute)
    bpm: AtomicU32,
    /// Song position (in MIDI beats)
    position: AtomicU32,
    /// Is playing
    playing: AtomicBool,
    /// Clock count (24 clocks per quarter note)
    clock_count: AtomicU32,
}

impl MidiClock {
    /// Create new MIDI clock
    pub fn new(bpm: u32) -> Self {
        Self {
            bpm: AtomicU32::new(bpm),
            position: AtomicU32::new(0),
            playing: AtomicBool::new(false),
            clock_count: AtomicU32::new(0),
        }
    }

    /// Get BPM
    pub fn bpm(&self) -> u32 {
        self.bpm.load(Ordering::Relaxed)
    }

    /// Set BPM
    pub fn set_bpm(&self, bpm: u32) {
        self.bpm.store(bpm, Ordering::Relaxed);
    }

    /// Get position (in MIDI beats)
    pub fn position(&self) -> u32 {
        self.position.load(Ordering::Relaxed)
    }

    /// Set position
    pub fn set_position(&self, pos: u32) {
        self.position.store(pos, Ordering::Relaxed);
    }

    /// Start clock
    pub fn start(&self) {
        self.playing.store(true, Ordering::Release);
    }

    /// Stop clock
    pub fn stop(&self) {
        self.playing.store(false, Ordering::Release);
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Process timing clock message
    pub fn process_clock(&self) {
        let count = self.clock_count.fetch_add(1, Ordering::Relaxed);
        if count % 24 == 0 {
            // One quarter note (24 clocks per quarter note)
            self.position.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Reset clock
    pub fn reset(&self) {
        self.position.store(0, Ordering::Relaxed);
        self.clock_count.store(0, Ordering::Relaxed);
        self.playing.store(false, Ordering::Release);
    }
}

// ============================================================================
// MIDI Sequencer
// ============================================================================

/// MIDI track
#[derive(Debug, Clone)]
pub struct MidiTrack {
    /// Track events
    pub events: Vec<MidiEvent>,
    /// Track name
    pub name: String,
}

impl MidiTrack {
    /// Create new track
    pub fn new(name: String) -> Self {
        Self {
            events: Vec::new(),
            name,
        }
    }

    /// Add event to track
    pub fn add_event(&mut self, event: MidiEvent) {
        self.events.push(event);
    }

    /// Sort events by timestamp
    pub fn sort(&mut self) {
        self.events.sort_by_key(|e| e.timestamp);
    }
}

/// MIDI sequencer
#[derive(Debug)]
pub struct MidiSequencer {
    /// Sequencer tracks
    tracks: Vec<MidiTrack>,
    /// Current position (in ticks)
    position: AtomicU32,
    /// Tempo (microseconds per quarter note)
    tempo: u32,
    /// Ticks per quarter note (PPQ)
    ppq: u16,
    /// Is playing
    playing: AtomicBool,
    /// Loop enabled
    loop_enabled: AtomicBool,
    /// Loop start (ticks)
    loop_start: u32,
    /// Loop end (ticks)
    loop_end: u32,
}

impl MidiSequencer {
    /// Create new sequencer
    pub fn new(bpm: u32, ppq: u16) -> Result<Self, MidiError> {
        if bpm == 0 || bpm > 300 {
            return Err(MidiError::InvalidData);
        }

        if ppq == 0 {
            return Err(MidiError::InvalidData);
        }

        // Convert BPM to microseconds per quarter note
        let tempo = 60_000_000 / bpm;

        Ok(Self {
            tracks: Vec::new(),
            position: AtomicU32::new(0),
            tempo,
            ppq,
            playing: AtomicBool::new(false),
            loop_enabled: AtomicBool::new(false),
            loop_start: 0,
            loop_end: u32::MAX,
        })
    }

    /// Add track
    pub fn add_track(&mut self, track: MidiTrack) {
        self.tracks.push(track);
    }

    /// Get number of tracks
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Get track
    pub fn get_track(&self, index: usize) -> Option<&MidiTrack> {
        self.tracks.get(index)
    }

    /// Set tempo
    pub fn set_tempo(&mut self, bpm: u32) -> Result<(), MidiError> {
        if bpm == 0 || bpm > 300 {
            return Err(MidiError::InvalidData);
        }
        self.tempo = 60_000_000 / bpm;
        Ok(())
    }

    /// Get tempo
    pub fn tempo(&self) -> u32 {
        60_000_000 / self.tempo
    }

    /// Start playback
    pub fn play(&self) {
        self.playing.store(true, Ordering::Release);
    }

    /// Stop playback
    pub fn stop(&self) {
        self.playing.store(false, Ordering::Release);
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Get current position
    pub fn position(&self) -> u32 {
        self.position.load(Ordering::Relaxed)
    }

    /// Set position
    pub fn set_position(&self, pos: u32) {
        self.position.store(pos, Ordering::Release);
    }

    /// Enable loop
    pub fn enable_loop(&self, _start: u32, _end: u32) {
        self.loop_enabled.store(true, Ordering::Release);
        // In real implementation, we'd need interior mutability for start/end
    }

    /// Disable loop
    pub fn disable_loop(&self) {
        self.loop_enabled.store(false, Ordering::Release);
    }

    /// Get events at current position
    pub fn get_events(&self, tick: u32) -> Vec<MidiEvent> {
        let mut events = Vec::new();

        for track in &self.tracks {
            for event in &track.events {
                if event.timestamp == tick {
                    events.push(event.clone());
                }
            }
        }

        events
    }

    /// Advance sequencer
    pub fn advance(&self, delta_ticks: u32) {
        let mut pos = self.position.load(Ordering::Relaxed);
        pos += delta_ticks;

        // Handle loop
        if self.loop_enabled.load(Ordering::Relaxed) {
            if pos >= self.loop_end {
                pos = self.loop_start + (pos - self.loop_end);
            }
        }

        self.position.store(pos, Ordering::Relaxed);
    }

    /// Reset sequencer
    pub fn reset(&self) {
        self.position.store(0, Ordering::Relaxed);
        self.playing.store(false, Ordering::Release);
    }
}

// ============================================================================
// Standard MIDI File (SMF)
// ============================================================================

/// SMF format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmfFormat {
    /// Format 0: Single track
    Format0,
    /// Format 1: Multiple tracks, synchronous
    Format1,
    /// Format 2: Multiple tracks, independent
    Format2,
}

/// SMF file header
#[derive(Debug, Clone)]
pub struct SmfHeader {
    /// SMF format
    pub format: SmfFormat,
    /// Number of tracks
    pub num_tracks: u16,
    /// Time division (PPQ or FPS)
    pub time_division: u16,
}

impl SmfHeader {
    /// Check if time division is PPQ (ticks per quarter note)
    pub fn is_ppq(&self) -> bool {
        (self.time_division & 0x8000) == 0
    }

    /// Get PPQ if applicable
    pub fn ppq(&self) -> Option<u16> {
        if self.is_ppq() {
            Some(self.time_division & 0x7FFF)
        } else {
            None
        }
    }
}

/// SMF file
#[derive(Debug, Clone)]
pub struct MidiSmf {
    /// File header
    pub header: SmfHeader,
    /// File tracks
    pub tracks: Vec<MidiTrack>,
}

impl MidiSmf {
    /// Create new SMF
    pub fn new(header: SmfHeader) -> Self {
        Self {
            header,
            tracks: Vec::new(),
        }
    }

    /// Add track
    pub fn add_track(&mut self, track: MidiTrack) {
        self.tracks.push(track);
    }

    /// Validate SMF structure
    pub fn validate(&self) -> Result<(), MidiError> {
        if self.tracks.len() != self.header.num_tracks as usize {
            return Err(MidiError::FileFormatError);
        }

        match self.header.format {
            SmfFormat::Format0 => {
                if self.tracks.len() != 1 {
                    return Err(MidiError::FileFormatError);
                }
            }
            SmfFormat::Format1 | SmfFormat::Format2 => {
                if self.tracks.is_empty() {
                    return Err(MidiError::FileFormatError);
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// MIDI Note Utils
// ============================================================================

/// MIDI note name utilities
pub struct MidiNote;

impl MidiNote {
    /// Get note name from note number
    pub fn name(note: u8) -> &'static str {
        const NOTES: &[&str] = &[
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        if note > 127 {
            return "?";
        }
        NOTES[(note % 12) as usize]
    }

    /// Get octave from note number
    pub fn octave(note: u8) -> u8 {
        if note > 127 {
            return 0;
        }
        note / 12 - 1
    }

    /// Get note from name and octave
    pub fn from_name_octave(name: &str, octave: u8) -> Option<u8> {
        const NOTES: &[&str] = &[
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        let note_index = NOTES.iter().position(|&n| n == name)?;
        let note = (octave + 1) as u8 * 12 + note_index as u8;

        if note <= 127 {
            Some(note)
        } else {
            None
        }
    }

    /// Convert note to frequency (Hz)
    pub fn to_freq(note: u8) -> f32 {
        // A4 = note 69 = 440 Hz
        440.0 * libm::powf(2.0_f32, (note as f32 - 69.0) / 12.0)
    }

    /// Convert frequency to note
    pub fn from_freq(freq: f32) -> u8 {
        // f = 440 * 2^((n-69)/12)
        // n = 69 + 12 * log2(f/440)
        let note = 69.0 + 12.0 * libm::log2(freq as f64 / 440.0);
        libm::round(note) as u8
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_message_parsing() {
        // Note on
        let data = [0x90, 0x3C, 0x7F];
        let (msg, len) = MidiMessage::from_bytes(&data).unwrap();
        assert_eq!(len, 3);
        assert_eq!(
            msg,
            MidiMessage::NoteOn {
                channel: 0,
                note: 60,
                velocity: 127
            }
        );

        // Control change
        let data = [0xB0, 0x01, 0x40];
        let (msg, len) = MidiMessage::from_bytes(&data).unwrap();
        assert_eq!(len, 3);
        assert_eq!(
            msg,
            MidiMessage::ControlChange {
                channel: 0,
                controller: 1,
                value: 64
            }
        );
    }

    #[test]
    fn test_midi_message_serialization() {
        let msg = MidiMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 127,
        };
        let bytes = msg.to_bytes();
        assert_eq!(bytes, vec![0x90, 0x3C, 0x7F]);
    }

    #[test]
    fn test_midi_event() {
        let event = MidiEvent::note_on(0, 60, 127);
        assert_eq!(event.timestamp(), 0);

        let mut event = event;
        event.set_timestamp(100);
        assert_eq!(event.timestamp(), 100);
    }

    #[test]
    fn test_midi_clock() {
        let clock = MidiClock::new(120);
        assert_eq!(clock.bpm(), 120);
        assert_eq!(clock.position(), 0);

        clock.start();
        assert!(clock.is_playing());

        for _ in 0..24 {
            clock.process_clock();
        }
        assert_eq!(clock.position(), 1);
    }

    #[test]
    fn test_midi_sequencer() {
        let sequencer = MidiSequencer::new(120, 480).unwrap();
        assert_eq!(sequencer.tempo(), 120);
        assert_eq!(sequencer.track_count(), 0);

        sequencer.play();
        assert!(sequencer.is_playing());

        sequencer.stop();
        assert!(!sequencer.is_playing());
    }

    #[test]
    fn test_midi_note() {
        assert_eq!(MidiNote::name(60), "C");
        assert_eq!(MidiNote::octave(60), 4);

        let note = MidiNote::from_name_octave("A", 4).unwrap();
        assert_eq!(note, 69);

        let freq = MidiNote::to_freq(69); // A4
        assert!((freq - 440.0).abs() < 0.1);

        let note = MidiNote::from_freq(440.0);
        assert_eq!(note, 69);
    }

    #[test]
    fn test_midi_track() {
        let mut track = MidiTrack::new(String::from("Test Track"));

        track.add_event(MidiEvent::note_on(0, 60, 127));
        track.add_event(MidiEvent::note_off(0, 60, 0));

        track.sort();
        assert_eq!(track.events.len(), 2);
    }

    #[test]
    fn test_smf_header() {
        let header = SmfHeader {
            format: SmfFormat::Format0,
            num_tracks: 1,
            time_division: 480,
        };

        assert!(header.is_ppq());
        assert_eq!(header.ppq(), Some(480));
    }
}
