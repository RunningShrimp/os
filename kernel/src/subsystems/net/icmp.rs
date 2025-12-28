//! Internet Control Message Protocol (ICMP) implementation
//!
//! This module provides ICMP protocol support for network diagnostics and error reporting.

extern crate alloc;
use alloc::vec::Vec;

use alloc::boxed::Box;
use super::ipv4::Ipv4Addr;

/// ICMP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcmpType {
    /// Echo Reply
    EchoReply = 0,
    /// Destination Unreachable
    DestinationUnreachable = 3,
    /// Source Quench
    SourceQuench = 4,
    /// Redirect Message
    Redirect = 5,
    /// Echo Request
    EchoRequest = 8,
    /// Time Exceeded
    TimeExceeded = 11,
    /// Parameter Problem
    ParameterProblem = 12,
    /// Timestamp Request
    TimestampRequest = 13,
    /// Timestamp Reply
    TimestampReply = 14,
    /// Information Request
    InformationRequest = 15,
    /// Information Reply
    InformationReply = 16,
    /// Address Mask Request
    AddressMaskRequest = 17,
    /// Address Mask Reply
    AddressMaskReply = 18,
}

/// ICMP codes for different message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcmpCode {
    // Destination Unreachable codes (0-5)
    NetUnreachable = 0,
    HostUnreachable = 1,
    ProtocolUnreachable = 2,
    PortUnreachable = 3,
    FragmentationNeeded = 4,
    SourceRouteFailed = 5,

    // Time Exceeded codes (10-11)
    TtlExceeded = 10,
    FragmentReassemblyTimeExceeded = 11,

    // Parameter Problem codes (20-22)
    PointerIndicatesError = 20,
    MissingRequiredOption = 21,
    BadLength = 22,
}

/// ICMP packet header
#[derive(Debug, Clone)]
pub struct IcmpHeader {
    /// Message type
    pub message_type: IcmpType,
    /// Message code
    pub code: IcmpCode,
    /// Checksum
    pub checksum: u16,
    /// Rest of header (message-specific)
    pub rest: u32,
}

impl IcmpHeader {
    /// Create a new ICMP header
    pub fn new(message_type: IcmpType, code: IcmpCode, rest: u32) -> Self {
        Self {
            message_type,
            code,
            checksum: 0,
            rest,
        }
    }

    /// Calculate checksum
    pub fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let mut sum = 0u32;

        // Add header words
        sum += (((self.message_type as u16) << 8) | (self.code as u16)) as u32;
        sum += (self.rest & 0xFFFF) as u32;
        sum += ((self.rest >> 16) & 0xFFFF) as u32;

        // Add data
        let mut i = 0;
        while i < data.len() {
            if i + 1 < data.len() {
                sum += (((data[i] as u16) << 8) | (data[i + 1] as u16)) as u32;
                i += 2;
            } else {
                sum += ((data[i] as u16) << 8) as u32;
                i += 1;
            }
        }

        // Add carry
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Set checksum
    pub fn set_checksum(&mut self, data: &[u8]) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum(data);
    }
}

/// ICMP packet
#[derive(Debug, Clone)]
pub struct IcmpPacket {
    /// Header
    pub header: IcmpHeader,
    /// Payload data
    pub payload: Vec<u8>,
}

impl IcmpPacket {
    /// Create a new ICMP packet
    pub fn new(message_type: IcmpType, code: IcmpCode, rest: u32, payload: Vec<u8>) -> Self {
        let mut header = IcmpHeader::new(message_type, code, rest);
        header.set_checksum(&payload);
        Self { header, payload }
    }

    /// Create echo request packet
    pub fn echo_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let rest = ((identifier as u32) << 16) | (sequence as u32);
        Self::new(IcmpType::EchoRequest, IcmpCode::NetUnreachable, rest, data)
    }

    /// Create echo reply packet
    pub fn echo_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let rest = ((identifier as u32) << 16) | (sequence as u32);
        Self::new(IcmpType::EchoReply, IcmpCode::NetUnreachable, rest, data)
    }

    /// Get identifier (for echo messages)
    pub fn identifier(&self) -> u16 {
        ((self.header.rest >> 16) & 0xFFFF) as u16
    }

    /// Get sequence number (for echo messages)
    pub fn sequence(&self) -> u16 {
        (self.header.rest & 0xFFFF) as u16
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.header.message_type as u8);
        bytes.push(self.header.code as u8);
        bytes.extend_from_slice(&self.header.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.header.rest.to_be_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IcmpError> {
        if bytes.len() < 8 {
            return Err(IcmpError::PacketTooSmall);
        }

        let message_type = match bytes[0] {
            0 => IcmpType::EchoReply,
            3 => IcmpType::DestinationUnreachable,
            4 => IcmpType::SourceQuench,
            5 => IcmpType::Redirect,
            8 => IcmpType::EchoRequest,
            11 => IcmpType::TimeExceeded,
            12 => IcmpType::ParameterProblem,
            13 => IcmpType::TimestampRequest,
            14 => IcmpType::TimestampReply,
            15 => IcmpType::InformationRequest,
            16 => IcmpType::InformationReply,
            _ => return Err(IcmpError::InvalidMessageType),
        };

        let code = match bytes[1] {
            0 => IcmpCode::NetUnreachable,
            1 => IcmpCode::HostUnreachable,
            2 => IcmpCode::ProtocolUnreachable,
            3 => IcmpCode::PortUnreachable,
            4 => IcmpCode::FragmentationNeeded,
            5 => IcmpCode::SourceRouteFailed,
            _ => IcmpCode::NetUnreachable, // Default
        };

        let checksum = u16::from_be_bytes([bytes[2], bytes[3]]);
        let rest = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let payload = bytes[8..].to_vec();

        let packet = Self {
            header: IcmpHeader {
                message_type,
                code,
                checksum,
                rest,
            },
            payload,
        };

        // Verify checksum
        if packet.header.calculate_checksum(&packet.payload) != checksum {
            return Err(IcmpError::InvalidChecksum);
        }

        Ok(packet)
    }
}

/// ICMP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcmpError {
    /// Packet too small
    PacketTooSmall,
    /// Invalid message type
    InvalidMessageType,
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid packet format
    InvalidPacket,
    /// Packet too large
    PacketTooLarge,
    /// Rate limited
    RateLimited,
    /// Network error
    NetworkError,
    /// Buffer too small
    BufferTooSmall,
}

/// ICMP processor for handling incoming ICMP packets
pub struct IcmpProcessor {
    /// Echo request handler
    echo_handler: Option<Box<dyn Fn(Ipv4Addr, Ipv4Addr, IcmpPacket) -> Option<IcmpPacket>>>,
}

impl IcmpProcessor {
    /// Create a new ICMP processor
    pub fn new() -> Self {
        Self {
            echo_handler: None,
        }
    }

    /// Set echo request handler
    pub fn set_echo_handler<F>(&mut self, handler: F)
    where
        F: Fn(Ipv4Addr, Ipv4Addr, IcmpPacket) -> Option<IcmpPacket> + 'static,
    {
        self.echo_handler = Some(Box::new(handler));
    }

    /// Process an incoming ICMP packet
    pub fn process_packet(
        &self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        packet: IcmpPacket,
    ) -> Option<IcmpPacket> {
        match packet.header.message_type {
            IcmpType::EchoRequest => {
                // Handle echo request
                if let Some(ref handler) = self.echo_handler {
                    handler(source_addr, dest_addr, packet)
                } else {
                    // Default: send echo reply
                    Some(IcmpPacket::echo_reply(
                        packet.identifier(),
                        packet.sequence(),
                        packet.payload,
                    ))
                }
            }
            IcmpType::EchoReply => {
                // Handle echo reply (could notify waiting processes)
                None
            }
            _ => {
                // Handle other ICMP message types
                None
            }
        }
    }
}

impl Default for IcmpProcessor {
    fn default() -> Self {
        Self::new()
    }
}