//! WebSocket protocol implementation
//!
//! This module implements the WebSocket protocol (RFC 6455), providing
//! full-duplex communication channels over TCP connections.
//!
//! # Features
//! - WebSocket handshake (HTTP Upgrade)
//! - Frame encoding and decoding
//! - Ping/Pong support
//! - Message fragmentation
//! - Extension support (compression, etc.)
//!
//! # References
//! - RFC 6455: The WebSocket Protocol
//! - RFC 7692: Compression Extensions for WebSocket

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

use crate::subsystems::sync::Mutex;

/// WebSocket protocol version
pub const WEBSOCKET_VERSION: u8 = 13;

/// WebSocket GUID for handshake validation
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Default frame size limit (16 MB)
const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Default message size limit (1 GB)
const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024 * 1024 * 1024;

/// WebSocket frame opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    /// Continuation frame
    Continuation = 0x0,
    /// Text frame
    Text = 0x1,
    /// Binary frame
    Binary = 0x2,
    /// Close frame
    Close = 0x8,
    /// Ping frame
    Ping = 0x9,
    /// Pong frame
    PingAck = 0xA,
}

impl Opcode {
    /// Create opcode from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x0 => Some(Self::Continuation),
            0x1 => Some(Self::Text),
            0x2 => Some(Self::Binary),
            0x8 => Some(Self::Close),
            0x9 => Some(Self::Ping),
            0xA => Some(Self::PingAck),
            _ => None,
        }
    }

    /// Check if opcode is for a control frame
    pub fn is_control(self) -> bool {
        (self as u8) >= 0x8
    }
}

/// WebSocket frame header
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// FIN bit: indicates final fragment
    pub fin: bool,
    /// RSV1 bit: extension negotiated
    pub rsv1: bool,
    /// RSV2 bit: extension negotiated
    pub rsv2: bool,
    /// RSV3 bit: extension negotiated
    pub rsv3: bool,
    /// Frame opcode
    pub opcode: Opcode,
    /// Mask flag: frames from client must be masked
    pub masked: bool,
    /// Payload length
    pub payload_length: u64,
    /// Masking key (if present)
    pub mask_key: Option<[u8; 4]>,
}

impl FrameHeader {
    /// Size of header when serialized
    pub fn header_size(&self) -> usize {
        let mut size = 2; // Basic header

        // Extended payload length
        if self.payload_length <= 125 {
            // No extension
        } else if self.payload_length <= 65535 {
            size += 2;
        } else {
            size += 8;
        }

        // Masking key
        if self.masked {
            size += 4;
        }

        size
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header_size());

        // First byte: FIN, RSVs, and opcode
        let first = (self.fin as u8) << 7
            | (self.rsv1 as u8) << 6
            | (self.rsv2 as u8) << 5
            | (self.rsv3 as u8) << 4
            | (self.opcode as u8);
        bytes.push(first);

        // Second byte: MASK and payload length
        let mut second = (self.masked as u8) << 7;

        if self.payload_length <= 125 {
            second |= self.payload_length as u8;
            bytes.push(second);
        } else if self.payload_length <= 65535 {
            second |= 126;
            bytes.push(second);
            bytes.extend_from_slice(&(self.payload_length as u16).to_be_bytes());
        } else {
            second |= 127;
            bytes.push(second);
            bytes.extend_from_slice(&self.payload_length.to_be_bytes());
        }

        // Masking key
        if self.masked {
            if let Some(key) = self.mask_key {
                bytes.extend_from_slice(&key);
            }
        }

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WebSocketError> {
        if bytes.len() < 2 {
            return Err(WebSocketError::IncompleteFrame);
        }

        let first = bytes[0];
        let second = bytes[1];

        let fin = (first & 0x80) != 0;
        let rsv1 = (first & 0x40) != 0;
        let rsv2 = (first & 0x20) != 0;
        let rsv3 = (first & 0x10) != 0;
        let opcode = Opcode::from_byte(first & 0x0F)
            .ok_or(WebSocketError::InvalidOpcode)?;

        let masked = (second & 0x80) != 0;
        let mut payload_length = (second & 0x7F) as u64;

        let mut offset = 2;

        // Extended payload length
        if payload_length == 126 {
            if bytes.len() < offset + 2 {
                return Err(WebSocketError::IncompleteFrame);
            }
            payload_length = u16::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
            ]) as u64;
            offset += 2;
        } else if payload_length == 127 {
            if bytes.len() < offset + 8 {
                return Err(WebSocketError::IncompleteFrame);
            }
            payload_length = u64::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            offset += 8;
        }

        // Masking key
        let mut mask_key = None;
        if masked {
            if bytes.len() < offset + 4 {
                return Err(WebSocketError::IncompleteFrame);
            }
            mask_key = Some([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;
        }

        Ok(Self {
            fin,
            rsv1,
            rsv2,
            rsv3,
            opcode,
            masked,
            payload_length,
            mask_key,
        })
    }
}

/// WebSocket frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame header
    pub header: FrameHeader,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame
    pub fn new(opcode: Opcode, payload: Vec<u8>, fin: bool) -> Self {
        let payload_length = payload.len() as u64;
        Self {
            header: FrameHeader {
                fin,
                rsv1: false,
                rsv2: false,
                rsv3: false,
                opcode,
                masked: false,
                payload_length,
                mask_key: None,
            },
            payload,
        }
    }

    /// Create a text frame
    pub fn text(text: String) -> Self {
        Self::new(Opcode::Text, text.into_bytes(), true)
    }

    /// Create a binary frame
    pub fn binary(data: Vec<u8>) -> Self {
        Self::new(Opcode::Binary, data, true)
    }

    /// Create a ping frame
    pub fn ping(data: Vec<u8>) -> Self {
        Self::new(Opcode::Ping, data, true)
    }

    /// Create a pong frame
    pub fn pong(data: Vec<u8>) -> Self {
        Self::new(Opcode::PingAck, data, true)
    }

    /// Create a close frame
    pub fn close(code: u16, reason: String) -> Self {
        let mut payload = Vec::with_capacity(2 + reason.len());
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        Self::new(Opcode::Close, payload, true)
    }

    /// Serialize frame to bytes (with masking)
    pub fn to_bytes(&self, mask: bool) -> Vec<u8> {
        let mut header = self.header.clone();
        header.masked = mask;

        if mask {
            // Generate random mask key
            header.mask_key = Some([
                0x12, 0x34, 0x56, 0x78, // In real implementation, use proper random
            ]);
        }

        let mut bytes = header.to_bytes();
        bytes.extend_from_slice(&self.payload);

        // Apply masking if needed
        if mask {
            if let Some(key) = header.mask_key {
                let header_len = header.header_size();
                for (i, byte) in bytes.iter_mut().enumerate().skip(header_len) {
                    *byte ^= key[i % 4];
                }
            }
        }

        bytes
    }

    /// Parse frame from bytes (and unmask if needed)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WebSocketError> {
        let header = FrameHeader::from_bytes(bytes)?;

        let header_len = header.header_size();
        let total_len = header_len + header.payload_length as usize;

        if bytes.len() < total_len {
            return Err(WebSocketError::IncompleteFrame);
        }

        let mut payload = bytes[header_len..total_len].to_vec();

        // Unmask if needed
        if header.masked {
            if let Some(key) = header.mask_key {
                for (i, byte) in payload.iter_mut().enumerate() {
                    *byte ^= key[i % 4];
                }
            }
        }

        Ok(Self { header, payload })
    }

    /// Check if this is the final frame
    pub fn is_final(&self) -> bool {
        self.header.fin
    }

    /// Get frame opcode
    pub fn opcode(&self) -> Opcode {
        self.header.opcode
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketState {
    /// Connecting - handshake in progress
    Connecting,
    /// Open - connection established
    Open,
    /// Closing - close frame sent
    Closing,
    /// Closed - connection closed
    Closed,
}

/// WebSocket close status codes
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum CloseCode {
    /// Normal closure
    Normal = 1000,
    /// Endpoint is going away
    GoingAway = 1001,
    /// Protocol error
    ProtocolError = 1002,
    /// Unsupported data
    UnsupportedData = 1003,
    /// No status code received
    NoStatus = 1005,
    /// Abnormal closure
    Abnormal = 1006,
    /// Invalid frame payload data
    InvalidPayload = 1007,
    /// Policy violation
    PolicyViolation = 1008,
    /// Message too big
    MessageTooBig = 1009,
    /// Mandatory extension
    MandatoryExtension = 1010,
    /// Internal server error
    InternalError = 1011,
    /// TLS handshake failure
    TlsHandshake = 1015,
}

impl CloseCode {
    /// Create close code from u16
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            1000 => Some(Self::Normal),
            1001 => Some(Self::GoingAway),
            1002 => Some(Self::ProtocolError),
            1003 => Some(Self::UnsupportedData),
            1005 => Some(Self::NoStatus),
            1006 => Some(Self::Abnormal),
            1007 => Some(Self::InvalidPayload),
            1008 => Some(Self::PolicyViolation),
            1009 => Some(Self::MessageTooBig),
            1010 => Some(Self::MandatoryExtension),
            1011 => Some(Self::InternalError),
            1015 => Some(Self::TlsHandshake),
            _ => None,
        }
    }
}

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Maximum frame size
    pub max_frame_size: usize,
    /// Maximum message size
    pub max_message_size: usize,
    /// Enable compression (per-message-deflate)
    pub compression_enabled: bool,
    /// Compression window bits
    pub compression_window_bits: u8,
    /// Enable message fragmentation
    pub allow_fragments: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            compression_enabled: false,
            compression_window_bits: 15,
            allow_fragments: true,
        }
    }
}

/// WebSocket connection
pub struct WebSocketConnection {
    /// Connection state
    state: Mutex<WebSocketState>,
    /// Current message being assembled (for fragmented messages)
    current_message: Mutex<Option<Vec<u8>>>,
    /// Current message type
    current_opcode: Mutex<Option<Opcode>>,
    /// Configuration
    config: WebSocketConfig,
    /// Close code
    close_code: Mutex<Option<CloseCode>>,
    /// Close reason
    close_reason: Mutex<Option<String>>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            state: Mutex::new(WebSocketState::Connecting),
            current_message: Mutex::new(None),
            current_opcode: Mutex::new(None),
            config,
            close_code: Mutex::new(None),
            close_reason: Mutex::new(None),
        }
    }

    /// Get connection state
    pub fn state(&self) -> WebSocketState {
        *self.state.lock()
    }

    /// Handle incoming frame
    pub fn handle_frame(&self, frame: &Frame) -> Result<Option<Vec<u8>>, WebSocketError> {
        let state = *self.state.lock();

        // Validate state
        match state {
            WebSocketState::Closed => {
                return Err(WebSocketError::ConnectionClosed);
            },
            WebSocketState::Closing => {
                // Only accept close frames
                if frame.opcode() != Opcode::Close {
                    return Err(WebSocketError::InvalidState);
                }
            },
            _ => {},
        }

        // Handle control frames
        if frame.opcode().is_control() {
            return self.handle_control_frame(frame);
        }

        // Handle data frames
        self.handle_data_frame(frame)
    }

    /// Handle control frame (ping/pong/close)
    fn handle_control_frame(&self, frame: &Frame) -> Result<Option<Vec<u8>>, WebSocketError> {
        match frame.opcode() {
            Opcode::Ping => {
                // Respond with pong
                let pong = Frame::pong(frame.payload.clone());
                return Ok(Some(pong.to_bytes(true)));
            },
            Opcode::PingAck => {
                // Pong received, no action needed
                return Ok(None);
            },
            Opcode::Close => {
                // Parse close code and reason
                let code = if frame.payload.len() >= 2 {
                    let code_bytes = [frame.payload[0], frame.payload[1]];
                    let code_u16 = u16::from_be_bytes(code_bytes);
                    CloseCode::from_u16(code_u16).unwrap_or(CloseCode::Abnormal)
                } else {
                    CloseCode::Normal
                };

                let reason = if frame.payload.len() > 2 {
                    String::from_utf8_lossy(&frame.payload[2..]).into_owned()
                } else {
                    String::new()
                };

                *self.close_code.lock() = Some(code);
                *self.close_reason.lock() = Some(reason.clone());

                // Update state
                let mut state = self.state.lock();
                if *state == WebSocketState::Open {
                    *state = WebSocketState::Closing;
                    // Send close response
                    let close_response = Frame::close(code as u16, reason);
                    return Ok(Some(close_response.to_bytes(true)));
                } else {
                    *state = WebSocketState::Closed;
                    return Ok(None);
                }
            },
            _ => {
                return Err(WebSocketError::InvalidOpcode);
            },
        }
    }

    /// Handle data frame (text/binary/continuation)
    fn handle_data_frame(&self, frame: &Frame) -> Result<Option<Vec<u8>>, WebSocketError> {
        // Validate payload length
        if frame.payload.len() > self.config.max_frame_size {
            return Err(WebSocketError::FrameTooLarge);
        }

        let opcode = frame.opcode();
        let is_final = frame.is_final();

        match opcode {
            Opcode::Text | Opcode::Binary => {
                // Start of new message
                let mut current_msg = self.current_message.lock();
                let mut current_op = self.current_opcode.lock();

                *current_msg = Some(frame.payload.clone());
                *current_op = Some(opcode);

                if is_final {
                    // Complete message received
                    let message = current_msg.take().unwrap();
                    *current_op = None;
                    return Ok(Some(message));
                } else if !self.config.allow_fragments {
                    return Err(WebSocketError::FragmentationDisabled);
                }

                Ok(None)
            },
            Opcode::Continuation => {
                // Continuation of fragmented message
                let mut current_msg = self.current_message.lock();

                if current_msg.is_none() {
                    return Err(WebSocketError::UnexpectedContinuation);
                }

                // Check total message size
                let current_size = current_msg.as_ref().unwrap().len();
                if current_size + frame.payload.len() > self.config.max_message_size {
                    return Err(WebSocketError::MessageTooLarge);
                }

                // Append fragment
                current_msg.as_mut().unwrap().extend_from_slice(&frame.payload);

                if is_final {
                    // Complete message received
                    let message = current_msg.take().unwrap();
                    *self.current_opcode.lock() = None;
                    return Ok(Some(message));
                }

                Ok(None)
            },
            _ => {
                Err(WebSocketError::InvalidOpcode)
            },
        }
    }

    /// Send text message
    pub fn send_text(&self, text: String) -> Result<Vec<u8>, WebSocketError> {
        let state = *self.state.lock();
        if state != WebSocketState::Open {
            return Err(WebSocketError::InvalidState);
        }

        let frame = Frame::text(text);
        Ok(frame.to_bytes(true))
    }

    /// Send binary message
    pub fn send_binary(&self, data: Vec<u8>) -> Result<Vec<u8>, WebSocketError> {
        let state = *self.state.lock();
        if state != WebSocketState::Open {
            return Err(WebSocketError::InvalidState);
        }

        let frame = Frame::binary(data);
        Ok(frame.to_bytes(true))
    }

    /// Send ping
    pub fn send_ping(&self, data: Vec<u8>) -> Result<Vec<u8>, WebSocketError> {
        let state = *self.state.lock();
        if state != WebSocketState::Open {
            return Err(WebSocketError::InvalidState);
        }

        let frame = Frame::ping(data);
        Ok(frame.to_bytes(true))
    }

    /// Close connection
    pub fn close(&self, code: CloseCode, reason: String) -> Result<Vec<u8>, WebSocketError> {
        let mut state = self.state.lock();

        match *state {
            WebSocketState::Open => {
                *state = WebSocketState::Closing;
            },
            WebSocketState::Closing => {
                // Already closing, just update state
                *state = WebSocketState::Closed;
                return Ok(Vec::new());
            },
            WebSocketState::Closed => {
                return Err(WebSocketError::ConnectionClosed);
            },
            _ => {
                return Err(WebSocketError::InvalidState);
            },
        }

        *self.close_code.lock() = Some(code);
        *self.close_reason.lock() = Some(reason.clone());

        let frame = Frame::close(code as u16, reason);
        Ok(frame.to_bytes(true))
    }

    /// Mark connection as open (after successful handshake)
    pub fn set_open(&self) {
        *self.state.lock() = WebSocketState::Open;
    }
}

/// Perform WebSocket handshake (server side)
pub fn server_handshake(
    request: &str,
    _config: &WebSocketConfig,
) -> Result<String, WebSocketError> {
    // Parse request headers
    let mut upgrade = None;
    let mut connection = None;
    let mut sec_key = None;
    let mut version = None;

    for line in request.lines() {
        if line.starts_with("Upgrade:") {
            upgrade = Some(line["Upgrade:".len()..].trim());
        } else if line.starts_with("Connection:") {
            connection = Some(line["Connection:".len()..].trim());
        } else if line.starts_with("Sec-WebSocket-Key:") {
            sec_key = Some(line["Sec-WebSocket-Key:".len()..].trim());
        } else if line.starts_with("Sec-WebSocket-Version:") {
            version = Some(line["Sec-WebSocket-Version:".len()..].trim());
        }
    }

    // Validate headers
    if upgrade != Some("websocket") {
        return Err(WebSocketError::InvalidHandshake);
    }

    let connection_has_upgrade = connection
        .map(|c| c.to_lowercase().contains("upgrade"))
        .unwrap_or(false);

    if !connection_has_upgrade {
        return Err(WebSocketError::InvalidHandshake);
    }

    let key = sec_key.ok_or(WebSocketError::InvalidHandshake)?;

    if version != Some("13") {
        return Err(WebSocketError::UnsupportedVersion);
    }

    // Compute accept key
    let accept_key = compute_accept_key(key);

    // Build response
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        accept_key
    );

    Ok(response)
}

/// Perform WebSocket handshake (client side)
pub fn client_handshake(
    host: &str,
    path: &str,
    _config: &WebSocketConfig,
) -> String {
    let key = generate_key();

    format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {}\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n",
        path, host, key
    )
}

/// Compute accept key from client key
pub fn compute_accept_key(key: &str) -> String {
    // In a real implementation, this would use SHA-1
    // For now, return a placeholder
    alloc::format!("{}{}", key, WEBSOCKET_GUID)
}

/// Generate a random WebSocket key
pub fn generate_key() -> String {
    // In a real implementation, this would generate a random 16-byte key
    // and base64-encode it
    String::from("dGhlIHNhbXBsZSBub25jZQ==")
}

/// WebSocket errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebSocketError {
    /// Incomplete frame
    IncompleteFrame,
    /// Invalid opcode
    InvalidOpcode,
    /// Frame too large
    FrameTooLarge,
    /// Message too large
    MessageTooLarge,
    /// Connection closed
    ConnectionClosed,
    /// Invalid state
    InvalidState,
    /// Invalid handshake
    InvalidHandshake,
    /// Unsupported version
    UnsupportedVersion,
    /// Fragmentation disabled
    FragmentationDisabled,
    /// Unexpected continuation frame
    UnexpectedContinuation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_serialization() {
        let frame = Frame::text("Hello".to_string());
        let bytes = frame.to_bytes(false);

        let parsed = Frame::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.opcode(), Opcode::Text);
        assert_eq!(parsed.payload, b"Hello");
    }

    #[test]
    fn test_frame_masking() {
        let frame = Frame::binary(vec![1, 2, 3, 4]);
        let bytes = frame.to_bytes(true);

        let parsed = Frame::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_ping_pong() {
        let ping = Frame::ping(vec![1, 2, 3]);
        assert_eq!(ping.opcode(), Opcode::Ping);

        let pong = Frame::pong(vec![1, 2, 3]);
        assert_eq!(pong.opcode(), Opcode::PingAck);
    }
}
