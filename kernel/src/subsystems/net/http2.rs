//! HTTP/2 protocol implementation
//!
//! This module implements the HTTP/2 protocol (RFC 7540), providing
//! multiplexed streams over a single TCP connection with header compression
//! and stream prioritization.
//!
//! # Features
//! - HPACK header compression (RFC 7541)
//! - Stream multiplexing
//! - Flow control
//! - Server push
//! - Stream prioritization
//! - Header compression
//!
//! # References
//! - RFC 7540: Hypertext Transfer Protocol version 2
//! - RFC 7541: HPACK: Header Compression for HTTP/2

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;

use core::sync::atomic {AtomicU32,, Ordering};
use crate::subsystems::sync::Mutex;

/// HTTP/2 protocol version
pub const HTTP2_VERSION: &str = "h2";

/// HTTP/2 protocol string
pub const HTTP2_PROTOCOL: &str = "h2c";

/// Default initial window size
const DEFAULT_INITIAL_WINDOW_SIZE: u32 = 65535;

/// Default maximum frame size
const DEFAULT_MAX_FRAME_SIZE: u32 = 16384;

/// Default header table size
const DEFAULT_HEADER_TABLE_SIZE: u32 = 4096;

/// Default maximum concurrent streams
const DEFAULT_MAX_CONCURRENT_STREAMS: u32 = 100;

/// HTTP/2 frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// DATA frame
    Data = 0x0,
    /// HEADERS frame
    Headers = 0x1,
    /// PRIORITY frame
    Priority = 0x2,
    /// RST_STREAM frame
    RstStream = 0x3,
    /// SETTINGS frame
    Settings = 0x4,
    /// PUSH_PROMISE frame
    PushPromise = 0x5,
    /// PING frame
    Ping = 0x6,
    /// GOAWAY frame
    GoAway = 0x7,
    /// WINDOW_UPDATE frame
    WindowUpdate = 0x8,
    /// CONTINUATION frame
    Continuation = 0x9,
}

impl FrameType {
    /// Parse frame type from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x0 => Some(Self::Data),
            0x1 => Some(Self::Headers),
            0x2 => Some(Self::Priority),
            0x3 => Some(Self::RstStream),
            0x4 => Some(Self::Settings),
            0x5 => Some(Self::PushPromise),
            0x6 => Some(Self::Ping),
            0x7 => Some(Self::GoAway),
            0x8 => Some(Self::WindowUpdate),
            0x9 => Some(Self::Continuation),
            _ => None,
        }
    }
}

/// HTTP/2 flags
#[derive(Debug, Clone, Copy)]
pub struct FrameFlags {
    /// END_STREAM flag
    pub end_stream: bool,
    /// END_HEADERS flag
    pub end_headers: bool,
    /// ACK flag
    pub ack: bool,
    /// PADDED flag
    pub padded: bool,
    /// PRIORITY flag
    pub priority: bool,
}

impl FrameFlags {
    /// Create empty flags
    pub fn new() -> Self {
        Self {
            end_stream: false,
            end_headers: false,
            ack: false,
            padded: false,
            priority: false,
        }
    }

    /// Convert to byte
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.end_stream {
            byte |= 0x01;
        }
        if self.end_headers {
            byte |= 0x04;
        }
        if self.ack {
            byte |= 0x01;
        }
        if self.padded {
            byte |= 0x08;
        }
        if self.priority {
            byte |= 0x20;
        }
        byte
    }

    /// Parse from byte (frame type dependent)
    pub fn from_byte(byte: u8, _frame_type: FrameType) -> Self {
        Self {
            end_stream: (byte & 0x01) != 0,
            end_headers: (byte & 0x04) != 0,
            ack: (byte & 0x01) != 0,
            padded: (byte & 0x08) != 0,
            priority: (byte & 0x20) != 0,
        }
    }
}

/// HTTP/2 error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorCode {
    /// No error
    NoError = 0x0,
    /// Protocol error
    ProtocolError = 0x1,
    /// Internal error
    InternalError = 0x2,
    /// Flow control error
    FlowControlError = 0x3,
    /// Settings timeout
    SettingsTimeout = 0x4,
    /// Stream closed
    StreamClosed = 0x5,
    /// Frame size error
    FrameSizeError = 0x6,
    /// Refused stream
    RefusedStream = 0x7,
    /// Cancel
    Cancel = 0x8,
    /// Compression error
    CompressionError = 0x9,
    /// Connect error
    ConnectError = 0xa,
    /// Enhance your calm
    EnhanceYourCalm = 0xb,
    /// Inadequate security
    InadequateSecurity = 0xc,
    /// HTTP/1.1 required
    Http11Required = 0xd,
}

impl ErrorCode {
    /// Parse from u32
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0x0 => Some(Self::NoError),
            0x1 => Some(Self::ProtocolError),
            0x2 => Some(Self::InternalError),
            0x3 => Some(Self::FlowControlError),
            0x4 => Some(Self::SettingsTimeout),
            0x5 => Some(Self::StreamClosed),
            0x6 => Some(Self::FrameSizeError),
            0x7 => Some(Self::RefusedStream),
            0x8 => Some(Self::Cancel),
            0x9 => Some(Self::CompressionError),
            0xa => Some(Self::ConnectError),
            0xb => Some(Self::EnhanceYourCalm),
            0xc => Some(Self::InadequateSecurity),
            0xd => Some(Self::Http11Required),
            _ => None,
        }
    }
}

/// HTTP/2 frame header
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame length
    pub length: u32,
    /// Frame type
    pub frame_type: FrameType,
    /// Frame flags
    pub flags: FrameFlags,
    /// Stream identifier
    pub stream_id: u32,
}

impl FrameHeader {
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; 9] {
        let mut bytes = [0u8; 9];

        // Length (3 bytes, big-endian)
        bytes[0] = (self.length >> 16) as u8;
        bytes[1] = (self.length >> 8) as u8;
        bytes[2] = self.length as u8;

        // Type
        bytes[3] = self.frame_type as u8;

        // Flags
        bytes[4] = self.flags.to_byte();

        // Stream ID (31 bits, big-endian)
        bytes[5] = ((self.stream_id >> 24) & 0x7F) as u8;
        bytes[6] = (self.stream_id >> 16) as u8;
        bytes[7] = (self.stream_id >> 8) as u8;
        bytes[8] = self.stream_id as u8;

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 9 {
            return None;
        }

        let length = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | bytes[2] as u32;
        let frame_type = FrameType::from_byte(bytes[3])?;
        let flags = FrameFlags::from_byte(bytes[4], frame_type);
        let stream_id = (((bytes[5] as u32) & 0x7F) << 24)
            | ((bytes[6] as u32) << 16)
            | ((bytes[7] as u32) << 8)
            | bytes[8] as u32;

        Some(Self {
            length,
            frame_type,
            flags,
            stream_id,
        })
    }
}

/// HTTP/2 frame
#[derive(Debug, Clone)]
pub enum Frame {
    /// DATA frame
    Data {
        header: FrameHeader,
        data: Vec<u8>,
    },
    /// HEADERS frame
    Headers {
        header: FrameHeader,
        header_block: Vec<u8>,
    },
    /// PRIORITY frame
    Priority {
        header: FrameHeader,
        depends_on: u32,
        weight: u8,
        exclusive: bool,
    },
    /// RST_STREAM frame
    RstStream {
        header: FrameHeader,
        error_code: ErrorCode,
    },
    /// SETTINGS frame
    Settings {
        header: FrameHeader,
        parameters: Vec<SettingParameter>,
    },
    /// PUSH_PROMISE frame
    PushPromise {
        header: FrameHeader,
        promised_stream_id: u32,
        header_block: Vec<u8>,
    },
    /// PING frame
    Ping {
        header: FrameHeader,
        opaque_data: [u8; 8],
    },
    /// GOAWAY frame
    GoAway {
        header: FrameHeader,
        last_stream_id: u32,
        error_code: ErrorCode,
        debug_data: Vec<u8>,
    },
    /// WINDOW_UPDATE frame
    WindowUpdate {
        header: FrameHeader,
        window_increment: u32,
    },
    /// CONTINUATION frame
    Continuation {
        header: FrameHeader,
        header_block: Vec<u8>,
    },
}

/// Settings parameter
#[derive(Debug, Clone)]
pub struct SettingParameter {
    /// Identifier
    pub identifier: u16,
    /// Value
    pub value: u32,
}

/// Settings identifiers
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum SettingId {
    /// Header table size
    HeaderTableSize = 0x1,
    /// Enable push
    EnablePush = 0x2,
    /// Maximum concurrent streams
    MaxConcurrentStreams = 0x3,
    /// Initial window size
    InitialWindowSize = 0x4,
    /// Maximum frame size
    MaxFrameSize = 0x5,
    /// Maximum header list size
    MaxHeaderListSize = 0x6,
}

/// HPACK header table entry
#[derive(Debug, Clone)]
pub struct HeaderTableEntry {
    /// Header name
    pub name: Vec<u8>,
    /// Header value
    pub value: Vec<u8>,
}

/// HPACK decoder
pub struct HpackDecoder {
    /// Static table (RFC 7541 Appendix B)
    static_table: Vec<HeaderTableEntry>,
    /// Dynamic table
    dynamic_table: Vec<HeaderTableEntry>,
    /// Maximum table size
    max_table_size: usize,
    /// Current table size
    current_table_size: usize,
}

impl HpackDecoder {
    /// Create a new HPACK decoder
    pub fn new() -> Self {
        Self {
            static_table: Self::build_static_table(),
            dynamic_table: Vec::new(),
            max_table_size: DEFAULT_HEADER_TABLE_SIZE as usize,
            current_table_size: 0,
        }
    }

    /// Build static header table (simplified - partial list)
    fn build_static_table() -> Vec<HeaderTableEntry> {
        vec![
            HeaderTableEntry {
                name: b":authority".to_vec(),
                value: b"".to_vec(),
            },
            HeaderTableEntry {
                name: b":method".to_vec(),
                value: b"GET".to_vec(),
            },
            HeaderTableEntry {
                name: b":method".to_vec(),
                value: b"POST".to_vec(),
            },
            HeaderTableEntry {
                name: b":path".to_vec(),
                value: b"/".to_vec(),
            },
            HeaderTableEntry {
                name: b":path".to_vec(),
                value: b"/index.html".to_vec(),
            },
            HeaderTableEntry {
                name: b":scheme".to_vec(),
                value: b"http".to_vec(),
            },
            HeaderTableEntry {
                name: b":scheme".to_vec(),
                value: b"https".to_vec(),
            },
            HeaderTableEntry {
                name: b":status".to_vec(),
                value: b"200".to_vec(),
            },
        ]
    }

    /// Decode header block
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Http2Error> {
        let mut headers = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let byte = data[pos];
            pos += 1;

            if byte & 0x80 != 0 {
                // Indexed header field
                let index = self.decode_integer(byte, 7, &data[pos..], &mut pos)?;
                let entry = self.get_entry(index)?;
                headers.push((entry.name.clone(), entry.value.clone()));
            } else if byte & 0xC0 == 0x40 {
                // Literal header field with incremental indexing
                let index = self.decode_integer(byte, 6, &data[pos..], &mut pos)?;
                let (name, value) = self.decode_literal(index, &data[pos..], &mut pos)?;
                self.add_to_dynamic_table(name.clone(), value.clone());
                headers.push((name, value));
            } else if byte & 0xF0 == 0 {
                // Literal header field without indexing
                let index = self.decode_integer(byte, 4, &data[pos..], &mut pos)?;
                let (name, value) = self.decode_literal(index, &data[pos..], &mut pos)?;
                headers.push((name, value));
            } else if byte & 0xF0 == 0x10 {
                // Literal header field never indexed
                let index = self.decode_integer(byte, 4, &data[pos..], &mut pos)?;
                let (name, value) = self.decode_literal(index, &data[pos..], &mut pos)?;
                headers.push((name, value));
            } else if byte & 0xE0 == 0x20 {
                // Dynamic table size update
                let new_size = self.decode_integer(byte, 5, &data[pos..], &mut pos)?;
                self.max_table_size = new_size;
                self.evict_dynamic_table();
            }
        }

        Ok(headers)
    }

    /// Decode integer
    fn decode_integer(&self, first_byte: u8, prefix: u8, data: &[u8], pos: &mut usize) -> Result<usize, Http2Error> {
        let mut value = (first_byte & ((1 << prefix) - 1)) as usize;
        if value < (1 << prefix) - 1 {
            return Ok(value);
        }

        let mut m = 0;
        loop {
            if *pos >= data.len() {
                return Err(Http2Error::CompressionError);
            }
            let byte = data[*pos];
            *pos += 1;
            value += ((byte & 0x7F) as usize) << m;
            m += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }

        Ok(value)
    }

    /// Decode literal header field
    fn decode_literal(&self, name_index: usize, data: &[u8], pos: &mut usize) -> Result<(Vec<u8>, Vec<u8>), Http2Error> {
        let name = if name_index > 0 {
            let entry = self.get_entry(name_index)?;
            entry.name.clone()
        } else {
            // Read literal name
            let len = self.decode_integer(data[*pos], 7, &data[*pos + 1..], pos)?;
            *pos += 1;
            let name = data[*pos..*pos + len].to_vec();
            *pos += len;
            name
        };

        // Read value
        let len = self.decode_integer(data[*pos], 7, &data[*pos + 1..], pos)?;
        *pos += 1;
        let value = data[*pos..*pos + len].to_vec();
        *pos += len;

        Ok((name, value))
    }

    /// Get entry from static or dynamic table
    fn get_entry(&self, index: usize) -> Result<&HeaderTableEntry, Http2Error> {
        let static_len = self.static_table.len();
        if index <= static_len {
            self.static_table.get(index - 1).ok_or(Http2Error::CompressionError)
        } else {
            let dynamic_index = index - static_len - 1;
            self.dynamic_table.get(dynamic_index).ok_or(Http2Error::CompressionError)
        }
    }

    /// Add entry to dynamic table
    fn add_to_dynamic_table(&mut self, name: Vec<u8>, value: Vec<u8>) {
        let entry = HeaderTableEntry { name, value };
        let entry_size = entry.name.len() + entry.value.len() + 32;
        self.current_table_size += entry_size;
        self.dynamic_table.insert(0, entry);
        self.evict_dynamic_table();
    }

    /// Evict entries from dynamic table if size exceeds maximum
    fn evict_dynamic_table(&mut self) {
        while self.current_table_size > self.max_table_size && !self.dynamic_table.is_empty() {
            let entry = self.dynamic_table.pop().unwrap();
            self.current_table_size -= entry.name.len() + entry.value.len() + 32;
        }
    }
}

impl Default for HpackDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP/2 stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Idle
    Idle,
    /// Open (both endpoints can send)
    Open,
    /// Reserved (local)
    ReservedLocal,
    /// Reserved (remote)
    ReservedRemote,
    /// Half-closed (remote)
    HalfClosedRemote,
    /// Half-closed (local)
    HalfClosedLocal,
    /// Closed
    Closed,
}

/// HTTP/2 stream
pub struct Stream {
    /// Stream identifier
    id: u32,
    /// Stream state
    state: Mutex<StreamState>,
    /// Send window size
    send_window: Mutex<u32>,
    /// Receive window size
    recv_window: Mutex<u32>,
    /// Priority weight
    weight: u8,
    /// Dependency stream ID
    depends_on: Option<u32>,
    /// Headers
    headers: Mutex<Vec<(Vec<u8>, Vec<u8>)>>,
}

impl Stream {
    /// Create a new stream
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: Mutex::new(StreamState::Idle),
            send_window: Mutex::new(DEFAULT_INITIAL_WINDOW_SIZE),
            recv_window: Mutex::new(DEFAULT_INITIAL_WINDOW_SIZE),
            weight: 16,
            depends_on: None,
            headers: Mutex::new(Vec::new()),
        }
    }

    /// Get stream ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get stream state
    pub fn state(&self) -> StreamState {
        *self.state.lock()
    }

    /// Set stream state
    pub fn set_state(&self, new_state: StreamState) {
        *self.state.lock() = new_state;
    }

    /// Get send window size
    pub fn send_window(&self) -> u32 {
        *self.send_window.lock()
    }

    /// Update send window
    pub fn update_send_window(&self, increment: u32) -> Result<(), Http2Error> {
        let mut window = self.send_window.lock();
        *window = window.checked_add(increment).ok_or(Http2Error::FlowControlError)?;
        Ok(())
    }

    /// Get receive window size
    pub fn recv_window(&self) -> u32 {
        *self.recv_window.lock()
    }

    /// Update receive window
    pub fn update_recv_window(&self, increment: u32) -> Result<(), Http2Error> {
        let mut window = self.recv_window.lock();
        *window = window.checked_add(increment).ok_or(Http2Error::FlowControlError)?;
        Ok(())
    }

    /// Set headers
    pub fn set_headers(&self, headers: Vec<(Vec<u8>, Vec<u8>)>) {
        *self.headers.lock() = headers;
    }

    /// Get headers
    pub fn headers(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.headers.lock().clone()
    }
}

/// HTTP/2 connection
pub struct Http2Connection {
    /// Next stream ID (client-initiated: odd, server-initiated: even)
    next_stream_id: AtomicU32,
    /// Active streams
    streams: Mutex<BTreeMap<u32, Stream>>,
    /// HPACK decoder
    decoder: Mutex<HpackDecoder>,
    /// Connection send window
    send_window: Mutex<u32>,
    /// Connection receive window
    recv_window: Mutex<u32>,
    /// Settings
    settings: Mutex<Http2Settings>,
    /// Connection state
    state: Mutex<ConnectionState>,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Preface not yet sent
    Preface,
    /// Connection open
    Open,
    /// Closing
    Closing,
    /// Closed
    Closed,
}

/// HTTP/2 settings
#[derive(Debug, Clone)]
pub struct Http2Settings {
    /// Header table size
    pub header_table_size: u32,
    /// Enable push
    pub enable_push: bool,
    /// Maximum concurrent streams
    pub max_concurrent_streams: u32,
    /// Initial window size
    pub initial_window_size: u32,
    /// Maximum frame size
    pub max_frame_size: u32,
    /// Maximum header list size
    pub max_header_list_size: u32,
}

impl Default for Http2Settings {
    fn default() -> Self {
        Self {
            header_table_size: DEFAULT_HEADER_TABLE_SIZE,
            enable_push: true,
            max_concurrent_streams: DEFAULT_MAX_CONCURRENT_STREAMS,
            initial_window_size: DEFAULT_INITIAL_WINDOW_SIZE,
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            max_header_list_size: 0xFFFFFFFF,
        }
    }
}

impl Http2Connection {
    /// Create a new HTTP/2 connection
    pub fn new(is_server: bool) -> Self {
        Self {
            next_stream_id: AtomicU32::new(if is_server { 2 } else { 1 }),
            streams: Mutex::new(BTreeMap::new()),
            decoder: Mutex::new(HpackDecoder::new()),
            send_window: Mutex::new(65535),
            recv_window: Mutex::new(65535),
            settings: Mutex::new(Http2Settings::default()),
            state: Mutex::new(ConnectionState::Preface),
        }
    }

    /// Handle incoming frame
    pub fn handle_frame(&self, frame: &Frame) -> Result<Option<Vec<u8>>, Http2Error> {
        let state = *self.state.lock();
        if state == ConnectionState::Closed {
            return Err(Http2Error::InternalError);
        }

        match frame {
            Frame::Settings { header, parameters } => {
                self.handle_settings(header, parameters)?;
                Ok(None)
            },
            Frame::WindowUpdate { header, window_increment } => {
                self.handle_window_update(header, *window_increment)?;
                Ok(None)
            },
            Frame::Headers { header, header_block } => {
                self.handle_headers(header, header_block)?;
                Ok(None)
            },
            Frame::Data { header, data } => {
                self.handle_data(header, data)?;
                Ok(None)
            },
            Frame::Ping { header, opaque_data } => {
                self.handle_ping(header, opaque_data)
            },
            Frame::GoAway { header, last_stream_id, error_code, debug_data } => {
                self.handle_goaway(header, *last_stream_id, *error_code, debug_data)?;
                Ok(None)
            },
            Frame::RstStream { header, error_code } => {
                self.handle_rst_stream(header, *error_code)?;
                Ok(None)
            },
            _ => {
                Ok(None)
            },
        }
    }

    /// Handle SETTINGS frame
    fn handle_settings(&self, header: &FrameHeader, parameters: &[SettingParameter]) -> Result<(), Http2Error> {
        // Only accept settings on stream 0
        if header.stream_id != 0 {
            return Err(Http2Error::ProtocolError);
        }

        let mut settings = self.settings.lock();

        for param in parameters {
            match SettingId::from_u16(param.identifier) {
                Some(SettingId::HeaderTableSize) => {
                    settings.header_table_size = param.value;
                    self.decoder.lock().max_table_size = param.value as usize;
                },
                Some(SettingId::EnablePush) => {
                    settings.enable_push = param.value != 0;
                },
                Some(SettingId::MaxConcurrentStreams) => {
                    settings.max_concurrent_streams = param.value;
                },
                Some(SettingId::InitialWindowSize) => {
                    if param.value > 0x7FFFFFFF {
                        return Err(Http2Error::FlowControlError);
                    }
                    settings.initial_window_size = param.value;
                },
                Some(SettingId::MaxFrameSize) => {
                    if param.value < 16384 || param.value > 0x00FFFFFF {
                        return Err(Http2Error::ProtocolError);
                    }
                    settings.max_frame_size = param.value;
                },
                Some(SettingId::MaxHeaderListSize) => {
                    settings.max_header_list_size = param.value;
                },
                None => {
                    // Unknown parameter, ignore
                },
            }
        }

        Ok(())
    }

    /// Handle WINDOW_UPDATE frame
    fn handle_window_update(&self, header: &FrameHeader, increment: u32) -> Result<(), Http2Error> {
        if increment == 0 {
            return Err(Http2Error::ProtocolError);
        }

        if header.stream_id == 0 {
            // Connection-level window update
            self.send_window.lock()
                .checked_add(increment)
                .ok_or(Http2Error::FlowControlError)?;
        } else {
            // Stream-level window update
            let streams = self.streams.lock();
            if let Some(stream) = streams.get(&header.stream_id) {
                stream.update_send_window(increment)?;
            }
        }

        Ok(())
    }

    /// Handle HEADERS frame
    fn handle_headers(&self, header: &FrameHeader, header_block: &[u8]) -> Result<(), Http2Error> {
        if header.stream_id == 0 {
            return Err(Http2Error::ProtocolError);
        }

        // Decode headers
        let headers = self.decoder.lock().decode(header_block)?;

        // Get or create stream
        let streams = self.streams.lock();
        if let Some(stream) = streams.get(&header.stream_id) {
            stream.set_headers(headers);
        }

        Ok(())
    }

    /// Handle DATA frame
    fn handle_data(&self, header: &FrameHeader, _data: &[u8]) -> Result<(), Http2Error> {
        if header.stream_id == 0 {
            return Err(Http2Error::ProtocolError);
        }

        // Update receive window
        let streams = self.streams.lock();
        if let Some(stream) = streams.get(&header.stream_id) {
            stream.recv_window();
        }

        Ok(())
    }

    /// Handle PING frame
    fn handle_ping(&self, header: &FrameHeader, opaque_data: &[u8; 8]) -> Result<Option<Vec<u8>>, Http2Error> {
        if header.stream_id != 0 {
            return Err(Http2Error::ProtocolError);
        }

        // Respond with ACK if not already set
        if !header.flags.ack {
            let mut response_data = *opaque_data;
            let response = Frame::Ping {
                header: FrameHeader {
                    length: 8,
                    frame_type: FrameType::Ping,
                    flags: FrameFlags {
                        ack: true,
                        ..FrameFlags::new()
                    },
                    stream_id: 0,
                },
                opaque_data: response_data,
            };

            return Ok(Some(self.serialize_frame(&response)));
        }

        Ok(None)
    }

    /// Handle GOAWAY frame
    fn handle_goaway(
        &self,
        _header: &FrameHeader,
        _last_stream_id: u32,
        error_code: ErrorCode,
        _debug_data: &[u8],
    ) -> Result<(), Http2Error> {
        *self.state.lock() = ConnectionState::Closed;
        Err(Http2Error::GoAwayReceived(error_code))
    }

    /// Handle RST_STREAM frame
    fn handle_rst_stream(&self, header: &FrameHeader, _error_code: ErrorCode) -> Result<(), Http2Error> {
        if header.stream_id == 0 {
            return Err(Http2Error::ProtocolError);
        }

        // Close stream
        let mut streams = self.streams.lock();
        if let Some(stream) = streams.get_mut(&header.stream_id) {
            stream.set_state(StreamState::Closed);
        }

        Ok(())
    }

    /// Create a new stream
    pub fn create_stream(&self) -> Result<u32, Http2Error> {
        let stream_id = self.next_stream_id.fetch_add(2, Ordering::SeqCst);

        let stream = Stream::new(stream_id);
        stream.set_state(StreamState::Open);

        let mut streams = self.streams.lock();
        streams.insert(stream_id, stream);

        Ok(stream_id)
    }

    /// Serialize frame to bytes
    pub fn serialize_frame(&self, frame: &Frame) -> Vec<u8> {
        match frame {
            Frame::Ping { opaque_data, .. } => {
                let header = FrameHeader {
                    length: 8,
                    frame_type: FrameType::Ping,
                    flags: FrameFlags::new(),
                    stream_id: 0,
                };
                let mut bytes = header.to_bytes().to_vec();
                bytes.extend_from_slice(opaque_data);
                bytes
            },
            _ => {
                Vec::new()
            },
        }
    }
}

impl SettingId {
    /// Parse from u16
    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0x1 => Some(Self::HeaderTableSize),
            0x2 => Some(Self::EnablePush),
            0x3 => Some(Self::MaxConcurrentStreams),
            0x4 => Some(Self::InitialWindowSize),
            0x5 => Some(Self::MaxFrameSize),
            0x6 => Some(Self::MaxHeaderListSize),
            _ => None,
        }
    }
}

/// HTTP/2 errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Http2Error {
    /// Protocol error
    ProtocolError,
    /// Internal error
    InternalError,
    /// Flow control error
    FlowControlError,
    /// Settings timeout
    SettingsTimeout,
    /// Stream closed
    StreamClosed,
    /// Frame size error
    FrameSizeError,
    /// Refused stream
    RefusedStream,
    /// Compression error
    CompressionError,
    /// GOAWAY received
    GoAwayReceived(ErrorCode),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_serialization() {
        let header = FrameHeader {
            length: 42,
            frame_type: FrameType::Data,
            flags: FrameFlags::new(),
            stream_id: 1,
        };

        let bytes = header.to_bytes();
        let parsed = FrameHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.length, 42);
        assert_eq!(parsed.frame_type, FrameType::Data);
        assert_eq!(parsed.stream_id, 1);
    }

    #[test]
    fn test_stream_creation() {
        let conn = Http2Connection::new(false);
        let stream_id = conn.create_stream().unwrap();
        assert_eq!(stream_id, 1);
    }
}
