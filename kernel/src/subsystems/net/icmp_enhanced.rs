//! Enhanced ICMP Protocol Implementation
//!
//! This module provides a comprehensive ICMP protocol implementation with advanced features
//! like path MTU discovery, traceroute support, and enhanced error handling.

extern crate alloc;
use alloc::vec::Vec;
use crate::sync::Mutex;
use nos_nos_error_handling::unified::KernelError;

// Re-export existing ICMP functionality
pub use super::icmp::*;

// ============================================================================
// Enhanced ICMP Types
// ============================================================================

/// Extended ICMP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExtendedIcmpType {
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
    /// Router Advertisement
    RouterAdvertisement = 9,
    /// Router Solicitation
    RouterSolicitation = 10,
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
    /// Traceroute
    Traceroute = 30,
}

/// Extended ICMP codes for Destination Unreachable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DestinationUnreachableCode {
    /// Network unreachable
    NetworkUnreachable = 0,
    /// Host unreachable
    HostUnreachable = 1,
    /// Protocol unreachable
    ProtocolUnreachable = 2,
    /// Port unreachable
    PortUnreachable = 3,
    /// Fragmentation needed and DF set
    FragmentationNeeded = 4,
    /// Source route failed
    SourceRouteFailed = 5,
}

/// Extended ICMP codes for Time Exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeExceededCode {
    /// Time to live exceeded in transit
    TtlExceeded = 0,
    /// Fragment reassembly time exceeded
    FragmentReassemblyTimeExceeded = 1,
}

/// Extended ICMP codes for Parameter Problem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ParameterProblemCode {
    /// Pointer indicates the error
    PointerIndicatesError = 0,
    /// Missing a required option
    MissingRequiredOption = 1,
    /// Bad length
    BadLength = 2,
}

/// Extended ICMP codes for Redirect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RedirectCode {
    /// Redirect datagrams for the network
    RedirectNetwork = 0,
    /// Redirect datagrams for the host
    RedirectHost = 1,
    /// Redirect datagrams for the type of service and network
    RedirectTosNetwork = 2,
    /// Redirect datagrams for the type of service and host
    RedirectTosHost = 3,
}

// ============================================================================
// Enhanced ICMP Messages
// ============================================================================

/// ICMP Destination Unreachable message
#[derive(Debug, Clone)]
pub struct DestinationUnreachableMessage {
    /// Original IP header
    pub original_header: Vec<u8>,
    /// Original IP data (first 8 bytes)
    pub original_data: Vec<u8>,
}

/// ICMP Time Exceeded message
#[derive(Debug, Clone)]
pub struct TimeExceededMessage {
    /// Original IP header
    pub original_header: Vec<u8>,
    /// Original IP data (first 8 bytes)
    pub original_data: Vec<u8>,
}

/// ICMP Parameter Problem message
#[derive(Debug, Clone)]
pub struct ParameterProblemMessage {
    /// Pointer to the error
    pub pointer: u8,
    /// Original IP header
    pub original_header: Vec<u8>,
    /// Original IP data (first 8 bytes)
    pub original_data: Vec<u8>,
}

/// ICMP Redirect message
#[derive(Debug, Clone)]
pub struct RedirectMessage {
    /// Gateway address to use
    pub gateway_address: super::ipv4::Ipv4Addr,
    /// Original IP header
    pub original_header: Vec<u8>,
    /// Original IP data (first 8 bytes)
    pub original_data: Vec<u8>,
}

/// ICMP Timestamp message
#[derive(Debug, Clone)]
pub struct TimestampMessage {
    /// Original timestamp
    pub originate_timestamp: u32,
    /// Receive timestamp
    pub receive_timestamp: u32,
    /// Transmit timestamp
    pub transmit_timestamp: u32,
}

/// ICMP Address Mask message
#[derive(Debug, Clone)]
pub struct AddressMaskMessage {
    /// Address mask
    pub address_mask: u32,
}

/// ICMP Router Advertisement message
#[derive(Debug, Clone)]
pub struct RouterAdvertisementMessage {
    /// Number of addresses
    pub address_count: u8,
    /// Address entry size
    pub address_entry_size: u8,
    /// Lifetime in seconds
    pub lifetime: u16,
    /// Router addresses
    pub router_addresses: Vec<super::ipv4::Ipv4Addr>,
}

/// ICMP Traceroute message
#[derive(Debug, Clone)]
pub struct TracerouteMessage {
    /// Outbound packet identifier
    pub outbound_packet_id: u16,
    /// Outbound packet sequence number
    pub outbound_packet_sequence: u16,
    /// Addresses of intermediate gateways
    pub gateway_addresses: Vec<super::ipv4::Ipv4Addr>,
}

// ============================================================================
// Enhanced ICMP Packet
// ============================================================================

/// Enhanced ICMP packet
#[derive(Debug, Clone)]
pub struct EnhancedIcmpPacket {
    /// Header
    pub header: IcmpHeader,
    /// Message type (extended)
    pub message_type: ExtendedIcmpType,
    /// Message data
    pub message_data: IcmpMessageData,
}

/// ICMP message data
#[derive(Debug, Clone)]
pub enum IcmpMessageData {
    /// Echo data
    Echo(Vec<u8>),
    /// Destination Unreachable
    DestinationUnreachable(DestinationUnreachableMessage),
    /// Time Exceeded
    TimeExceeded(TimeExceededMessage),
    /// Parameter Problem
    ParameterProblem(ParameterProblemMessage),
    /// Redirect
    Redirect(RedirectMessage),
    /// Timestamp
    Timestamp(TimestampMessage),
    /// Address Mask
    AddressMask(AddressMaskMessage),
    /// Router Advertisement
    RouterAdvertisement(RouterAdvertisementMessage),
    /// Router Solicitation (no data)
    RouterSolicitation,
    /// Traceroute
    Traceroute(TracerouteMessage),
    /// Unknown data
    Unknown(Vec<u8>),
}

impl EnhancedIcmpPacket {
    /// Create a new enhanced ICMP packet
    pub fn new(message_type: ExtendedIcmpType, code: u8, message_data: IcmpMessageData) -> Self {
        let rest = match &message_data {
            IcmpMessageData::Echo(data) => {
                if data.len() >= 4 {
                    u32::from_be_bytes([data[0], data[1], data[2], data[3]])
                } else {
                    0
                }
            }
            IcmpMessageData::DestinationUnreachable(msg) => {
                // Use first byte of original header as rest
                if !msg.original_header.is_empty() {
                    msg.original_header[0] as u32
                } else {
                    0
                }
            }
            IcmpMessageData::TimeExceeded(msg) => {
                // Use first byte of original header as rest
                if !msg.original_header.is_empty() {
                    msg.original_header[0] as u32
                } else {
                    0
                }
            }
            IcmpMessageData::ParameterProblem(msg) => {
                (msg.pointer as u32) << 24
            }
            IcmpMessageData::Redirect(msg) => {
                // Use gateway address as rest
                let addr_bytes = msg.gateway_address.to_bytes();
                u32::from_be_bytes([addr_bytes[0], addr_bytes[1], addr_bytes[2], addr_bytes[3]])
            }
            IcmpMessageData::Timestamp(msg) => {
                msg.originate_timestamp
            }
            IcmpMessageData::AddressMask(msg) => {
                msg.address_mask
            }
            IcmpMessageData::RouterAdvertisement(msg) => {
                ((msg.lifetime as u32) << 16) | ((msg.address_count as u32) << 8) | (msg.address_entry_size as u32)
            }
            IcmpMessageData::RouterSolicitation => {
                0
            }
            IcmpMessageData::Traceroute(msg) => {
                ((msg.outbound_packet_id as u32) << 16) | (msg.outbound_packet_sequence as u32)
            }
            IcmpMessageData::Unknown(data) => {
                if data.len() >= 4 {
                    u32::from_be_bytes([data[0], data[1], data[2], data[3]])
                } else {
                    0
                }
            }
        };

        let header = IcmpHeader::new(
            match message_type {
                ExtendedIcmpType::EchoReply => IcmpType::EchoReply,
                ExtendedIcmpType::DestinationUnreachable => IcmpType::DestinationUnreachable,
                ExtendedIcmpType::SourceQuench => IcmpType::SourceQuench,
                ExtendedIcmpType::Redirect => IcmpType::Redirect,
                ExtendedIcmpType::EchoRequest => IcmpType::EchoRequest,
                ExtendedIcmpType::TimeExceeded => IcmpType::TimeExceeded,
                ExtendedIcmpType::ParameterProblem => IcmpType::ParameterProblem,
                ExtendedIcmpType::TimestampRequest => IcmpType::TimestampRequest,
                ExtendedIcmpType::TimestampReply => IcmpType::TimestampReply,
                ExtendedIcmpType::InformationRequest => IcmpType::InformationRequest,
                ExtendedIcmpType::InformationReply => IcmpType::InformationReply,
                ExtendedIcmpType::AddressMaskRequest => IcmpType::InformationRequest, // Reuse
                ExtendedIcmpType::AddressMaskReply => IcmpType::InformationReply, // Reuse
                ExtendedIcmpType::RouterAdvertisement => IcmpType::SourceQuench, // Reuse
                ExtendedIcmpType::RouterSolicitation => IcmpType::SourceQuench, // Reuse
                ExtendedIcmpType::Traceroute => IcmpType::ParameterProblem, // Reuse
            },
            match message_type {
                ExtendedIcmpType::DestinationUnreachable => {
                    match code {
                        0 => IcmpCode::NetUnreachable,
                        1 => IcmpCode::HostUnreachable,
                        2 => IcmpCode::ProtocolUnreachable,
                        3 => IcmpCode::PortUnreachable,
                        4 => IcmpCode::FragmentationNeeded,
                        5 => IcmpCode::SourceRouteFailed,
                        _ => IcmpCode::NetUnreachable,
                    }
                }
                ExtendedIcmpType::TimeExceeded => {
                    match code {
                        0 => IcmpCode::TtlExceeded,
                        1 => IcmpCode::FragmentReassemblyTimeExceeded,
                        _ => IcmpCode::TtlExceeded,
                    }
                }
                ExtendedIcmpType::ParameterProblem => {
                    match code {
                        0 => IcmpCode::PointerIndicatesError,
                        1 => IcmpCode::MissingRequiredOption,
                        2 => IcmpCode::BadLength,
                        _ => IcmpCode::PointerIndicatesError,
                    }
                }
                ExtendedIcmpType::Redirect => {
                    match code {
                        0 => IcmpCode::NetUnreachable,
                        1 => IcmpCode::HostUnreachable,
                        2 => IcmpCode::ProtocolUnreachable,
                        3 => IcmpCode::PortUnreachable,
                        _ => IcmpCode::NetUnreachable,
                    }
                }
                _ => IcmpCode::NetUnreachable, // Default
            },
            rest,
        );

        Self {
            header,
            message_type,
            message_data,
        }
    }

    /// Create echo request packet
    pub fn echo_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let mut echo_data = Vec::with_capacity(4 + data.len());
        echo_data.extend_from_slice(&identifier.to_be_bytes());
        echo_data.extend_from_slice(&sequence.to_be_bytes());
        echo_data.extend_from_slice(&data);

        Self::new(
            ExtendedIcmpType::EchoRequest,
            0,
            IcmpMessageData::Echo(echo_data),
        )
    }

    /// Create echo reply packet
    pub fn echo_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let mut echo_data = Vec::with_capacity(4 + data.len());
        echo_data.extend_from_slice(&identifier.to_be_bytes());
        echo_data.extend_from_slice(&sequence.to_be_bytes());
        echo_data.extend_from_slice(&data);

        Self::new(
            ExtendedIcmpType::EchoReply,
            0,
            IcmpMessageData::Echo(echo_data),
        )
    }

    /// Create destination unreachable packet
    pub fn destination_unreachable(
        code: DestinationUnreachableCode,
        original_header: Vec<u8>,
        original_data: Vec<u8>,
    ) -> Self {
        Self::new(
            ExtendedIcmpType::DestinationUnreachable,
            code as u8,
            IcmpMessageData::DestinationUnreachable(DestinationUnreachableMessage {
                original_header,
                original_data,
            }),
        )
    }

    /// Create time exceeded packet
    pub fn time_exceeded(
        code: TimeExceededCode,
        original_header: Vec<u8>,
        original_data: Vec<u8>,
    ) -> Self {
        Self::new(
            ExtendedIcmpType::TimeExceeded,
            code as u8,
            IcmpMessageData::TimeExceeded(TimeExceededMessage {
                original_header,
                original_data,
            }),
        )
    }

    /// Create timestamp request packet
    pub fn timestamp_request(originate_timestamp: u32) -> Self {
        Self::new(
            ExtendedIcmpType::TimestampRequest,
            0,
            IcmpMessageData::Timestamp(TimestampMessage {
                originate_timestamp: originate_timestamp,
                receive_timestamp: 0,
                transmit_timestamp: 0,
            }),
        )
    }

    /// Create timestamp reply packet
    pub fn timestamp_reply(
        originate_timestamp: u32,
        receive_timestamp: u32,
        transmit_timestamp: u32,
    ) -> Self {
        Self::new(
            ExtendedIcmpType::TimestampReply,
            0,
            IcmpMessageData::Timestamp(TimestampMessage {
                originate_timestamp,
                receive_timestamp,
                transmit_timestamp,
            }),
        )
    }

    /// Create address mask request packet
    pub fn address_mask_request() -> Self {
        Self::new(
            ExtendedIcmpType::AddressMaskRequest,
            0,
            IcmpMessageData::AddressMask(AddressMaskMessage {
                address_mask: 0,
            }),
        )
    }

    /// Create address mask reply packet
    pub fn address_mask_reply(address_mask: u32) -> Self {
        Self::new(
            ExtendedIcmpType::AddressMaskReply,
            0,
            IcmpMessageData::AddressMask(AddressMaskMessage { address_mask }),
        )
    }

    /// Get identifier (for echo messages)
    pub fn identifier(&self) -> u16 {
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                if data.len() >= 2 {
                    u16::from_be_bytes([data[0], data[1]])
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Get sequence number (for echo messages)
    pub fn sequence(&self) -> u16 {
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                if data.len() >= 4 {
                    u16::from_be_bytes([data[2], data[3]])
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.header.message_type as u8);
        bytes.push(self.header.code as u8);
        bytes.extend_from_slice(&self.header.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.header.rest.to_be_bytes());

        // Add message-specific data
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                bytes.extend_from_slice(data);
            }
            IcmpMessageData::DestinationUnreachable(msg) => {
                bytes.extend_from_slice(&msg.original_header);
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::TimeExceeded(msg) => {
                bytes.extend_from_slice(&msg.original_header);
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::ParameterProblem(msg) => {
                bytes.push(msg.pointer);
                bytes.extend_from_slice(&msg.original_header);
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::Redirect(msg) => {
                let addr_bytes = msg.gateway_address.to_bytes();
                bytes.extend_from_slice(&addr_bytes);
                bytes.extend_from_slice(&msg.original_header);
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::Timestamp(msg) => {
                bytes.extend_from_slice(&msg.originate_timestamp.to_be_bytes());
                bytes.extend_from_slice(&msg.receive_timestamp.to_be_bytes());
                bytes.extend_from_slice(&msg.transmit_timestamp.to_be_bytes());
            }
            IcmpMessageData::AddressMask(msg) => {
                bytes.extend_from_slice(&msg.address_mask.to_be_bytes());
            }
            IcmpMessageData::RouterAdvertisement(msg) => {
                bytes.extend_from_slice(&msg.lifetime.to_be_bytes());
                bytes.push(msg.address_count);
                bytes.push(msg.address_entry_size);
                for addr in &msg.router_addresses {
                    bytes.extend_from_slice(&addr.to_bytes());
                }
            }
            IcmpMessageData::RouterSolicitation => {
                // No additional data
            }
            IcmpMessageData::Traceroute(msg) => {
                bytes.extend_from_slice(&msg.outbound_packet_id.to_be_bytes());
                bytes.extend_from_slice(&msg.outbound_packet_sequence.to_be_bytes());
                for addr in &msg.gateway_addresses {
                    bytes.extend_from_slice(&addr.to_bytes());
                }
            }
            IcmpMessageData::Unknown(data) => {
                bytes.extend_from_slice(data);
            }
        }

        // Calculate and set checksum
        let mut header = self.header.clone();
        header.set_checksum(&bytes[8..]);
        bytes[2] = (header.checksum >> 8) as u8;
        bytes[3] = (header.checksum & 0xFF) as u8;

        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IcmpError> {
        if bytes.len() < 8 {
            return Err(IcmpError::PacketTooSmall);
        }

        let message_type = match bytes[0] {
            0 => ExtendedIcmpType::EchoReply,
            3 => ExtendedIcmpType::DestinationUnreachable,
            4 => ExtendedIcmpType::SourceQuench,
            5 => ExtendedIcmpType::Redirect,
            8 => ExtendedIcmpType::EchoRequest,
            9 => ExtendedIcmpType::RouterAdvertisement,
            10 => ExtendedIcmpType::RouterSolicitation,
            11 => ExtendedIcmpType::TimeExceeded,
            12 => ExtendedIcmpType::ParameterProblem,
            13 => ExtendedIcmpType::TimestampRequest,
            14 => ExtendedIcmpType::TimestampReply,
            15 => ExtendedIcmpType::InformationRequest,
            16 => ExtendedIcmpType::InformationReply,
            17 => ExtendedIcmpType::AddressMaskRequest,
            18 => ExtendedIcmpType::AddressMaskReply,
            30 => ExtendedIcmpType::Traceroute,
            _ => return Err(IcmpError::InvalidMessageType),
        };

        let code = bytes[1];
        let checksum = u16::from_be_bytes([bytes[2], bytes[3]]);
        let rest = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        // Parse message-specific data
        let message_data = match message_type {
            ExtendedIcmpType::EchoRequest | ExtendedIcmpType::EchoReply => {
                IcmpMessageData::Echo(bytes[8..].to_vec())
            }
            ExtendedIcmpType::DestinationUnreachable => {
                if bytes.len() < 8 + 8 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let original_header = bytes[8..8 + 20].to_vec(); // Assuming IPv4 header is 20 bytes
                let original_data = bytes[8 + 20..8 + 28].to_vec(); // First 8 bytes of original data
                IcmpMessageData::DestinationUnreachable(DestinationUnreachableMessage {
                    original_header,
                    original_data,
                })
            }
            ExtendedIcmpType::TimeExceeded => {
                if bytes.len() < 8 + 8 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let original_header = bytes[8..8 + 20].to_vec(); // Assuming IPv4 header is 20 bytes
                let original_data = bytes[8 + 20..8 + 28].to_vec(); // First 8 bytes of original data
                IcmpMessageData::TimeExceeded(TimeExceededMessage {
                    original_header,
                    original_data,
                })
            }
            ExtendedIcmpType::ParameterProblem => {
                if bytes.len() < 8 + 1 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let pointer = bytes[8];
                let original_header = bytes[9..9 + 20].to_vec(); // Assuming IPv4 header is 20 bytes
                let original_data = bytes[9 + 20..9 + 28].to_vec(); // First 8 bytes of original data
                IcmpMessageData::ParameterProblem(ParameterProblemMessage {
                    pointer,
                    original_header,
                    original_data,
                })
            }
            ExtendedIcmpType::Redirect => {
                if bytes.len() < 8 + 4 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let gateway_bytes = [bytes[8], bytes[9], bytes[10], bytes[11]];
                let gateway_address = super::ipv4::Ipv4Addr::from_bytes(gateway_bytes);
                let original_header = bytes[12..12 + 20].to_vec(); // Assuming IPv4 header is 20 bytes
                let original_data = bytes[12 + 20..12 + 28].to_vec(); // First 8 bytes of original data
                IcmpMessageData::Redirect(RedirectMessage {
                    gateway_address,
                    original_header,
                    original_data,
                })
            }
            ExtendedIcmpType::TimestampRequest | ExtendedIcmpType::TimestampReply => {
                if bytes.len() < 8 + 12 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let originate_timestamp = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
                let receive_timestamp = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
                let transmit_timestamp = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
                IcmpMessageData::Timestamp(TimestampMessage {
                    originate_timestamp,
                    receive_timestamp,
                    transmit_timestamp,
                })
            }
            ExtendedIcmpType::AddressMaskRequest | ExtendedIcmpType::AddressMaskReply => {
                if bytes.len() < 8 + 4 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let address_mask = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
                IcmpMessageData::AddressMask(AddressMaskMessage { address_mask })
            }
            ExtendedIcmpType::RouterAdvertisement => {
                if bytes.len() < 8 + 4 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let lifetime = u16::from_be_bytes([bytes[8], bytes[9]]);
                let address_count = bytes[10];
                let address_entry_size = bytes[11];
                
                let mut router_addresses = Vec::new();
                let mut offset = 12;
                for _ in 0..address_count {
                    if offset + 4 <= bytes.len() {
                        let addr_bytes = [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]];
                        router_addresses.push(super::ipv4::Ipv4Addr::from_bytes(addr_bytes));
                        offset += address_entry_size as usize;
                    }
                }
                
                IcmpMessageData::RouterAdvertisement(RouterAdvertisementMessage {
                    address_count,
                    address_entry_size,
                    lifetime,
                    router_addresses,
                })
            }
            ExtendedIcmpType::RouterSolicitation => {
                IcmpMessageData::RouterSolicitation
            }
            ExtendedIcmpType::Traceroute => {
                if bytes.len() < 8 + 4 {
                    return Err(IcmpError::PacketTooSmall);
                }
                let outbound_packet_id = u16::from_be_bytes([bytes[8], bytes[9]]);
                let outbound_packet_sequence = u16::from_be_bytes([bytes[10], bytes[11]]);
                
                let mut gateway_addresses = Vec::new();
                let mut offset = 12;
                while offset + 4 <= bytes.len() {
                    let addr_bytes = [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]];
                    gateway_addresses.push(super::ipv4::Ipv4Addr::from_bytes(addr_bytes));
                    offset += 4;
                }
                
                IcmpMessageData::Traceroute(TracerouteMessage {
                    outbound_packet_id,
                    outbound_packet_sequence,
                    gateway_addresses,
                })
            }
            _ => IcmpMessageData::Unknown(bytes[8..].to_vec()),
        };

        let packet = Self {
            header: IcmpHeader {
                message_type: match message_type {
                    ExtendedIcmpType::EchoReply => IcmpType::EchoReply,
                    ExtendedIcmpType::DestinationUnreachable => IcmpType::DestinationUnreachable,
                    ExtendedIcmpType::SourceQuench => IcmpType::SourceQuench,
                    ExtendedIcmpType::Redirect => IcmpType::Redirect,
                    ExtendedIcmpType::EchoRequest => IcmpType::EchoRequest,
                    ExtendedIcmpType::TimeExceeded => IcmpType::TimeExceeded,
                    ExtendedIcmpType::ParameterProblem => IcmpType::ParameterProblem,
                    ExtendedIcmpType::TimestampRequest => IcmpType::TimestampRequest,
                    ExtendedIcmpType::TimestampReply => IcmpType::TimestampReply,
                    ExtendedIcmpType::InformationRequest => IcmpType::InformationRequest,
                    ExtendedIcmpType::InformationReply => IcmpType::InformationReply,
                    ExtendedIcmpType::AddressMaskRequest => IcmpType::InformationRequest, // Reuse
                    ExtendedIcmpType::AddressMaskReply => IcmpType::InformationReply, // Reuse
                    ExtendedIcmpType::RouterAdvertisement => IcmpType::SourceQuench, // Reuse
                    ExtendedIcmpType::RouterSolicitation => IcmpType::SourceQuench, // Reuse
                    ExtendedIcmpType::Traceroute => IcmpType::ParameterProblem, // Reuse
                },
                code: match message_type {
                    ExtendedIcmpType::DestinationUnreachable => {
                        match code {
                            0 => IcmpCode::NetUnreachable,
                            1 => IcmpCode::HostUnreachable,
                            2 => IcmpCode::ProtocolUnreachable,
                            3 => IcmpCode::PortUnreachable,
                            4 => IcmpCode::FragmentationNeeded,
                            5 => IcmpCode::SourceRouteFailed,
                            _ => IcmpCode::NetUnreachable,
                        }
                    }
                    ExtendedIcmpType::TimeExceeded => {
                        match code {
                            0 => IcmpCode::TtlExceeded,
                            1 => IcmpCode::FragmentReassemblyTimeExceeded,
                            _ => IcmpCode::TtlExceeded,
                        }
                    }
                    ExtendedIcmpType::ParameterProblem => {
                        match code {
                            0 => IcmpCode::PointerIndicatesError,
                            1 => IcmpCode::MissingRequiredOption,
                            2 => IcmpCode::BadLength,
                            _ => IcmpCode::PointerIndicatesError,
                        }
                    }
                    ExtendedIcmpType::Redirect => {
                        match code {
                            0 => IcmpCode::NetUnreachable,
                            1 => IcmpCode::HostUnreachable,
                            2 => IcmpCode::ProtocolUnreachable,
                            3 => IcmpCode::PortUnreachable,
                            _ => IcmpCode::NetUnreachable,
                        }
                    }
                    _ => IcmpCode::NetUnreachable, // Default
                },
                checksum,
                rest,
            },
            message_type,
            message_data,
        };

        // Verify checksum
        let payload = &bytes[8..];
        if packet.header.calculate_checksum(payload) != checksum {
            return Err(IcmpError::InvalidChecksum);
        }

        Ok(packet)
    }

    /// Get packet type
    pub fn get_type(&self) -> IcmpType {
        match self.message_type {
            ExtendedIcmpType::EchoReply => IcmpType::EchoReply,
            ExtendedIcmpType::DestinationUnreachable => IcmpType::DestinationUnreachable,
            ExtendedIcmpType::SourceQuench => IcmpType::SourceQuench,
            ExtendedIcmpType::Redirect => IcmpType::Redirect,
            ExtendedIcmpType::EchoRequest => IcmpType::EchoRequest,
            ExtendedIcmpType::TimeExceeded => IcmpType::TimeExceeded,
            ExtendedIcmpType::ParameterProblem => IcmpType::ParameterProblem,
            ExtendedIcmpType::TimestampRequest => IcmpType::TimestampRequest,
            ExtendedIcmpType::TimestampReply => IcmpType::TimestampReply,
            ExtendedIcmpType::InformationRequest => IcmpType::InformationRequest,
            ExtendedIcmpType::InformationReply => IcmpType::InformationReply,
            ExtendedIcmpType::AddressMaskRequest => IcmpType::AddressMaskRequest,
            ExtendedIcmpType::AddressMaskReply => IcmpType::AddressMaskReply,
            ExtendedIcmpType::RouterAdvertisement => IcmpType::RouterAdvertisement,
            ExtendedIcmpType::RouterSolicitation => IcmpType::RouterSolicitation,
        }
    }

    /// Get packet code
    pub fn get_code(&self) -> u8 {
        self.header.code
    }

    /// Get packet identifier (for echo messages)
    pub fn get_identifier(&self) -> u16 {
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                if data.len() >= 2 {
                    u16::from_be_bytes([data[0], data[1]])
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Get packet sequence number (for echo messages)
    pub fn get_sequence_number(&self) -> u16 {
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                if data.len() >= 4 {
                    u16::from_be_bytes([data[2], data[3]])
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Get packet data
    pub fn get_data(&self) -> &[u8] {
        match &self.message_data {
            IcmpMessageData::Echo(data) => data,
            _ => &[],
        }
    }

    /// Get source IP address (placeholder - would be set by network layer)
    pub fn get_source(&self) -> super::ipv4::Ipv4Addr {
        // This would be set by the network layer
        super::ipv4::Ipv4Addr::UNSPECIFIED
    }

    /// Set destination IP address (placeholder - would be used by network layer)
    pub fn set_destination(&mut self, _destination: super::ipv4::Ipv4Addr) {
        // This would be used by the network layer
    }

    /// Set TTL (placeholder - would be used by network layer)
    pub fn set_ttl(&mut self, _ttl: u8) {
        // This would be used by the network layer
    }

    /// Set DSCP (placeholder - would be used by network layer)
    pub fn set_dscp(&mut self, _dscp: u8) {
        // This would be used by the network layer
    }

    /// Set MTU (placeholder - would be used by network layer)
    pub fn set_mtu(&mut self, _mtu: u16) {
        // This would be used by the network layer
    }

    /// Convert packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Add header
        bytes.push(self.header.type_field);
        bytes.push(self.header.code);
        bytes.extend_from_slice(&self.header.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.header.rest.to_be_bytes());
        
        // Add message data
        match &self.message_data {
            IcmpMessageData::Echo(data) => {
                bytes.extend_from_slice(data);
            }
            IcmpMessageData::DestinationUnreachable(msg) => {
                bytes.push(msg.unused);
                bytes.extend_from_slice(&msg.original_header.to_be_bytes());
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::TimeExceeded(msg) => {
                bytes.push(msg.unused);
                bytes.extend_from_slice(&msg.original_header.to_be_bytes());
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::ParameterProblem(msg) => {
                bytes.push(msg.pointer);
                bytes.extend_from_slice(&msg.original_header.to_be_bytes());
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::Redirect(msg) => {
                bytes.extend_from_slice(&msg.gateway_address.to_bytes());
                bytes.extend_from_slice(&msg.original_header.to_be_bytes());
                bytes.extend_from_slice(&msg.original_data);
            }
            IcmpMessageData::Timestamp(msg) => {
                bytes.extend_from_slice(&msg.identifier.to_be_bytes());
                bytes.extend_from_slice(&msg.sequence_number.to_be_bytes());
                bytes.extend_from_slice(&msg.originate_timestamp.to_be_bytes());
                bytes.extend_from_slice(&msg.receive_timestamp.to_be_bytes());
                bytes.extend_from_slice(&msg.transmit_timestamp.to_be_bytes());
            }
            IcmpMessageData::AddressMask(msg) => {
                bytes.extend_from_slice(&msg.identifier.to_be_bytes());
                bytes.extend_from_slice(&msg.sequence_number.to_be_bytes());
                bytes.extend_from_slice(&msg.address_mask.to_be_bytes());
            }
            IcmpMessageData::RouterAdvertisement(msg) => {
                bytes.push(msg.num_addresses);
                bytes.push(msg.address_entry_size);
                bytes.extend_from_slice(&msg.lifetime.to_be_bytes());
                for addr in &msg.addresses {
                    bytes.extend_from_slice(&addr.address.to_bytes());
                    bytes.extend_from_slice(&addr.preference_level.to_be_bytes());
                }
            }
            IcmpMessageData::RouterSolicitation(msg) => {
                bytes.extend_from_slice(&msg.reserved.to_be_bytes());
            }
        }
        
        bytes
    }
}

// ============================================================================
// Enhanced ICMP Statistics
// ============================================================================

/// Enhanced ICMP statistics
#[derive(Debug, Default, Clone)]
pub struct EnhancedIcmpStats {
    /// Total packets transmitted
    pub packets_transmitted: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Total bytes transmitted
    pub bytes_transmitted: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Echo requests transmitted
    pub echo_requests_tx: u64,
    /// Echo replies transmitted
    pub echo_replies_tx: u64,
    /// Echo requests received
    pub echo_requests_rx: u64,
    /// Echo replies received
    pub echo_replies_rx: u64,
    /// Destination unreachable messages transmitted
    pub dest_unreachable_tx: u64,
    /// Destination unreachable messages received
    pub dest_unreachable_rx: u64,
    /// Time exceeded messages transmitted
    pub time_exceeded_tx: u64,
    /// Time exceeded messages received
    pub time_exceeded_rx: u64,
    /// Parameter problem messages transmitted
    pub param_problem_tx: u64,
    /// Parameter problem messages received
    pub param_problem_rx: u64,
    /// Redirect messages transmitted
    pub redirect_tx: u64,
    /// Redirect messages received
    pub redirect_rx: u64,
    /// Timestamp requests transmitted
    pub timestamp_requests_tx: u64,
    /// Timestamp replies transmitted
    pub timestamp_replies_tx: u64,
    /// Timestamp requests received
    pub timestamp_requests_rx: u64,
    /// Timestamp replies received
    pub timestamp_replies_rx: u64,
    /// Address mask requests transmitted
    pub address_mask_requests_tx: u64,
    /// Address mask replies transmitted
    pub address_mask_replies_tx: u64,
    /// Address mask requests received
    pub address_mask_requests_rx: u64,
    /// Address mask replies received
    pub address_mask_replies_rx: u64,
    /// Router advertisements transmitted
    pub router_advertisements_tx: u64,
    /// Router advertisements received
    pub router_advertisements_rx: u64,
    /// Router solicitations transmitted
    pub router_solicitations_tx: u64,
    /// Router solicitations received
    pub router_solicitations_rx: u64,
    /// Traceroute messages transmitted
    pub traceroute_tx: u64,
    /// Traceroute messages received
    pub traceroute_rx: u64,
    /// Invalid packets received
    pub invalid_packets_rx: u64,
    /// Packets with invalid checksum
    pub bad_checksum_rx: u64,
    /// Last activity timestamp
    pub last_activity_timestamp: u64,
}

// ============================================================================
// Enhanced ICMP Processor
// ============================================================================

/// Enhanced ICMP processor
pub struct EnhancedIcmpProcessor {
    /// Base ICMP processor
    base_processor: Mutex<IcmpProcessor>,
    /// Statistics
    stats: Mutex<EnhancedIcmpStats>,
    /// Configuration
    config: Mutex<IcmpConfig>,
    /// Echo request handlers
    echo_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Destination unreachable handlers
    dest_unreachable_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Time exceeded handlers
    time_exceeded_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Parameter problem handlers
    param_problem_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Redirect handlers
    redirect_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Timestamp handlers
    timestamp_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Address mask handlers
    address_mask_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Router advertisement handlers
    router_advertisement_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Router solicitation handlers
    router_solicitation_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
    /// Traceroute handlers
    traceroute_handlers: Mutex<Vec<Box<dyn Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket>>>>,
}

impl EnhancedIcmpProcessor {
    /// Create a new enhanced ICMP processor
    pub fn new() -> Self {
        Self {
            base_processor: Mutex::new(IcmpProcessor::new()),
            stats: Mutex::new(EnhancedIcmpStats::default()),
            config: Mutex::new(IcmpConfig::default()),
            echo_handlers: Mutex::new(Vec::new()),
            dest_unreachable_handlers: Mutex::new(Vec::new()),
            time_exceeded_handlers: Mutex::new(Vec::new()),
            param_problem_handlers: Mutex::new(Vec::new()),
            redirect_handlers: Mutex::new(Vec::new()),
            timestamp_handlers: Mutex::new(Vec::new()),
            address_mask_handlers: Mutex::new(Vec::new()),
            router_advertisement_handlers: Mutex::new(Vec::new()),
            router_solicitation_handlers: Mutex::new(Vec::new()),
            traceroute_handlers: Mutex::new(Vec::new()),
        }
    }

    /// Add echo request handler
    pub fn add_echo_handler<F>(&self, handler: F)
    where
        F: Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.echo_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add destination unreachable handler
    pub fn add_dest_unreachable_handler<F>(&self, handler: F)
    where
        F: Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.dest_unreachable_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add time exceeded handler
    pub fn add_time_exceeded_handler<F>(&self, handler: F)
    where
        F: Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.time_exceeded_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add parameter problem handler
    pub fn add_param_problem_handler<F>(&self, handler: F)
    where
        F: Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.param_problem_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add redirect handler
    pub fn add_redirect_handler<F>(&self, handler: F)
    where
        F: Fn(super::ipv4::Ipv4Addr, super::ipv4::Ipv4Addr, EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.redirect_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add timestamp handler
    pub fn add_timestamp_handler<F>(&self, handler: F)
    where
        F: Fn(EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.timestamp_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add address mask handler
    pub fn add_address_mask_handler<F>(&self, handler: F)
    where
        F: Fn(EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.address_mask_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add information request handler
    pub fn add_information_request_handler<F>(&self, handler: F)
    where
        F: Fn(EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.information_request_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Add information reply handler
    pub fn add_information_reply_handler<F>(&self, handler: F)
    where
        F: Fn(EnhancedIcmpPacket) -> Option<EnhancedIcmpPacket> + 'static,
    {
        let mut handlers = self.information_reply_handlers.lock();
        handlers.push(Box::new(handler));
    }

    /// Get comprehensive statistics
    pub fn get_comprehensive_stats(&self) -> IcmpComprehensiveStats {
        let stats = self.stats.lock();
        IcmpComprehensiveStats {
            echo_requests_sent: stats.echo_requests_sent,
            echo_replies_sent: stats.echo_replies_sent,
            echo_requests_received: stats.echo_requests_received,
            echo_replies_received: stats.echo_replies_received,
            destination_unreachable_sent: stats.destination_unreachable_sent,
            destination_unreachable_received: stats.destination_unreachable_received,
            time_exceeded_sent: stats.time_exceeded_sent,
            time_exceeded_received: stats.time_exceeded_received,
            parameter_problem_sent: stats.parameter_problem_sent,
            parameter_problem_received: stats.parameter_problem_received,
            source_quench_sent: stats.source_quench_sent,
            source_quench_received: stats.source_quench_received,
            redirect_sent: stats.redirect_sent,
            redirect_received: stats.redirect_received,
            timestamp_requests_sent: stats.timestamp_requests_sent,
            timestamp_replies_sent: stats.timestamp_replies_sent,
            timestamp_requests_received: stats.timestamp_requests_received,
            timestamp_replies_received: stats.timestamp_replies_received,
            address_mask_requests_sent: stats.address_mask_requests_sent,
            address_mask_replies_sent: stats.address_mask_replies_sent,
            address_mask_requests_received: stats.address_mask_requests_received,
            address_mask_replies_received: stats.address_mask_replies_received,
            information_requests_sent: stats.information_requests_sent,
            information_replies_sent: stats.information_replies_sent,
            information_requests_received: stats.information_requests_received,
            information_replies_received: stats.information_replies_received,
            total_packets_sent: stats.total_packets_sent,
            total_packets_received: stats.total_packets_received,
            packets_dropped: stats.packets_dropped,
            malformed_packets: stats.malformed_packets,
            rate_limited_packets: stats.rate_limited_packets,
            average_rtt: stats.average_rtt,
            min_rtt: stats.min_rtt,
            max_rtt: stats.max_rtt,
            active_handlers: {
                let mut total = 0;
                total += self.echo_handlers.lock().len();
                total += self.echo_reply_handlers.lock().len();
                total += self.destination_unreachable_handlers.lock().len();
                total += self.time_exceeded_handlers.lock().len();
                total += self.parameter_problem_handlers.lock().len();
                total += self.source_quench_handlers.lock().len();
                total += self.redirect_handlers.lock().len();
                total += self.timestamp_handlers.lock().len();
                total += self.address_mask_handlers.lock().len();
                total += self.information_request_handlers.lock().len();
                total += self.information_reply_handlers.lock().len();
                total
            },
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = EnhancedIcmpStats::default();
    }

    /// Configure rate limiting
    pub fn configure_rate_limiting(&self, max_packets_per_second: u32) {
        let mut config = self.config.lock();
        config.max_packets_per_second = max_packets_per_second;
    }

    /// Enable/disable rate limiting
    pub fn set_rate_limiting_enabled(&self, enabled: bool) {
        let mut config = self.config.lock();
        config.rate_limiting_enabled = enabled;
    }

    /// Get current configuration
    pub fn get_config(&self) -> IcmpConfig {
        self.config.lock().clone()
    }

    /// Send ICMP packet with custom options
    pub fn send_packet_with_options(&self, packet: EnhancedIcmpPacket, options: &IcmpSendOptions) -> Result<(), IcmpError> {
        // Check rate limiting
        if self.is_rate_limited() {
            self.update_stats(|stats| {
                stats.rate_limited_packets += 1;
            });
            return Err(IcmpError::RateLimited);
        }

        // Apply TTL if specified
        let mut packet = packet;
        if let Some(ttl) = options.ttl {
            packet.set_ttl(ttl);
        }

        // Apply DSCP if specified
        if let Some(dscp) = options.dscp {
            packet.set_dscp(dscp);
        }

        // Send the packet
        match self.send_packet(packet) {
            Ok(()) => {
                self.update_stats(|stats| {
                    stats.total_packets_sent += 1;
                });
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Send ICMP packet to specific destination
    pub fn send_to_destination(&self, packet: EnhancedIcmpPacket, destination: super::ipv4::Ipv4Addr) -> Result<(), IcmpError> {
        // Set destination in packet
        let mut packet = packet;
        packet.set_destination(destination);

        // Send the packet
        self.send_packet(packet)
    }

    /// Broadcast ICMP packet
    pub fn broadcast_packet(&self, packet: EnhancedIcmpPacket) -> Result<(), IcmpError> {
        // Set broadcast destination
        let mut packet = packet;
        packet.set_destination(super::ipv4::Ipv4Addr::BROADCAST);

        // Send the packet
        self.send_packet(packet)
    }

    /// Multicast ICMP packet
    pub fn multicast_packet(&self, packet: EnhancedIcmpPacket, group: super::ipv4::Ipv4Addr) -> Result<(), IcmpError> {
        // Set multicast destination
        let mut packet = packet;
        packet.set_destination(group);

        // Send the packet
        self.send_packet(packet)
    }

    /// Send ICMP packet with path MTU discovery
    pub fn send_with_path_mtu_discovery(&self, packet: EnhancedIcmpPacket, initial_mtu: u16) -> Result<(), IcmpError> {
        let mut current_mtu = initial_mtu;
        let mut packet = packet;

        loop {
            // Set packet size based on current MTU
            packet.set_mtu(current_mtu);

            match self.send_packet(packet.clone()) {
                Ok(()) => return Ok(()),
                Err(IcmpError::PacketTooLarge) => {
                    // Reduce MTU and try again
                    current_mtu = current_mtu.saturating_sub(100);
                    if current_mtu < 576 { // Minimum MTU for IPv4
                        return Err(IcmpError::PacketTooLarge);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform ICMP traceroute
    pub fn traceroute(&self, destination: super::ipv4::Ipv4Addr, max_hops: u8, timeout_ms: u64) -> Result<Vec<TracerouteHop>, IcmpError> {
        let mut hops = Vec::new();
        let mut ttl = 1;

        while ttl <= max_hops {
            let start_time = crate::time::get_monotonic_time();

            // Create echo request with specific TTL
            let packet = EnhancedIcmpPacket::echo_request(
                0, // identifier
                ttl as u16, // sequence number
                &[],
            );
            packet.set_ttl(ttl);

            // Send packet
            self.send_to_destination(packet, destination)?;

            // Wait for reply or timeout
            let mut reply_received = false;
            let mut timeout_remaining = timeout_ms;

            while timeout_remaining > 0 && !reply_received {
                // Check for incoming packets
                if let Some(reply) = self.receive_packet_with_timeout(100) {
                    if reply.get_type() == IcmpType::TimeExceeded {
                        let hop = TracerouteHop {
                            ttl,
                            address: reply.get_source(),
                            rtt_ms: (crate::time::get_monotonic_time() - start_time) * 1000,
                            hostname: None,
                        };
                        hops.push(hop);
                        reply_received = true;
                    } else if reply.get_type() == IcmpType::EchoReply {
                        let hop = TracerouteHop {
                            ttl,
                            address: reply.get_source(),
                            rtt_ms: (crate::time::get_monotonic_time() - start_time) * 1000,
                            hostname: None,
                        };
                        hops.push(hop);
                        return Ok(hops);
                    }
                }
                timeout_remaining -= 100;
            }

            if !reply_received {
                let hop = TracerouteHop {
                    ttl,
                    address: super::ipv4::Ipv4Addr::UNSPECIFIED,
                    rtt_ms: timeout_ms,
                    hostname: None,
                };
                hops.push(hop);
            }

            ttl += 1;
        }

        Ok(hops)
    }

    /// Perform ICMP ping
    pub fn ping(&self, destination: super::ipv4::Ipv4Addr, count: u32, interval_ms: u64, timeout_ms: u64) -> Result<PingResult, IcmpError> {
        let mut results = Vec::new();
        let mut sent = 0;
        let mut received = 0;
        let mut total_rtt = 0u64;
        let mut min_rtt = u64::MAX;
        let mut max_rtt = 0u64;

        for i in 0..count {
            let start_time = crate::time::get_monotonic_time();

            // Create echo request
            let packet = EnhancedIcmpPacket::echo_request(
                0, // identifier
                i as u16, // sequence number
                &[],
            );

            // Send packet
            match self.send_to_destination(packet, destination) {
                Ok(()) => sent += 1,
                Err(e) => return Err(e),
            }

            // Wait for reply
            let mut reply_received = false;
            let mut timeout_remaining = timeout_ms;

            while timeout_remaining > 0 && !reply_received {
                if let Some(reply) = self.receive_packet_with_timeout(100) {
                    if reply.get_type() == IcmpType::EchoReply && 
                       reply.get_identifier() == 0 && 
                       reply.get_sequence_number() == i as u16 {
                        let rtt = (crate::time::get_monotonic_time() - start_time) * 1000;
                        results.push(PingReply {
                            sequence: i,
                            rtt_ms: rtt,
                            bytes: reply.get_data().len(),
                        });
                        received += 1;
                        total_rtt += rtt;
                        min_rtt = min_rtt.min(rtt);
                        max_rtt = max_rtt.max(rtt);
                        reply_received = true;
                    }
                }
                timeout_remaining -= 100;
            }

            if !reply_received {
                results.push(PingReply {
                    sequence: i,
                    rtt_ms: timeout_ms,
                    bytes: 0,
                });
            }

            // Wait for interval before next ping
            if i < count - 1 {
                crate::time::sleep(interval_ms);
            }
        }

        let avg_rtt = if received > 0 { total_rtt / received } else { 0 };
        let packet_loss = if sent > 0 { ((sent - received) * 100) / sent } else { 0 };

        Ok(PingResult {
            destination,
            packets_sent: sent,
            packets_received: received,
            packet_loss_percent: packet_loss,
            min_rtt_ms: if min_rtt == u64::MAX { 0 } else { min_rtt },
            max_rtt_ms: max_rtt,
            avg_rtt_ms: avg_rtt,
            replies: results,
        })
    }

    /// Receive ICMP packet with timeout
    fn receive_packet_with_timeout(&self, timeout_ms: u64) -> Option<EnhancedIcmpPacket> {
        // This would interface with the network stack to receive packets
        // For now, return None as a placeholder
        crate::time::sleep(timeout_ms);
        None
    }

    /// Send ICMP packet
    fn send_packet(&self, packet: EnhancedIcmpPacket) -> Result<(), IcmpError> {
        // This would interface with the network stack to send packets
        // For now, return Ok as a placeholder
        log::debug!("Sending ICMP packet: type={:?}, code={:?}", packet.get_type(), packet.get_code());
        Ok(())
    }

    /// Update statistics with a closure
    fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut EnhancedIcmpStats),
    {
        let mut stats = self.stats.lock();
        updater(&mut stats);
    }

    /// Check if rate limiting should be applied
    fn is_rate_limited(&self) -> bool {
        let config = self.config.lock();
        if !config.rate_limiting_enabled {
            return false;
        }

        let mut stats = self.stats.lock();
        let current_time = crate::time::get_monotonic_time();
        
        // Simple rate limiting implementation
        // In a real implementation, this would use a token bucket or similar algorithm
        if current_time - stats.last_activity_timestamp < 1.0 / config.max_packets_per_second as f64 {
            return true;
        }
        
        // Update last activity timestamp
        stats.last_activity_timestamp = current_time;
        
        false
    }
}

/// ICMP send options
#[derive(Debug, Clone)]
pub struct IcmpSendOptions {
    pub ttl: Option<u8>,
    pub dscp: Option<u8>,
    pub dont_fragment: Option<bool>,
}

impl Default for IcmpSendOptions {
    fn default() -> Self {
        Self {
            ttl: None,
            dscp: None,
            dont_fragment: None,
        }
    }
}

/// Traceroute hop information
#[derive(Debug, Clone)]
pub struct TracerouteHop {
    pub ttl: u8,
    pub address: super::ipv4::Ipv4Addr,
    pub rtt_ms: u64,
    pub hostname: Option<String>,
}

/// Ping reply information
#[derive(Debug, Clone)]
pub struct PingReply {
    pub sequence: u32,
    pub rtt_ms: u64,
    pub bytes: usize,
}

/// Ping result
#[derive(Debug, Clone)]
pub struct PingResult {
    pub destination: super::ipv4::Ipv4Addr,
    pub packets_sent: u32,
    pub packets_received: u32,
    pub packet_loss_percent: u32,
    pub min_rtt_ms: u64,
    pub max_rtt_ms: u64,
    pub avg_rtt_ms: u64,
    pub replies: Vec<PingReply>,
}

/// ICMP configuration
#[derive(Debug, Clone)]
pub struct IcmpConfig {
    /// Maximum packets per second (rate limiting)
    pub max_packets_per_second: u32,
    /// Whether rate limiting is enabled
    pub rate_limiting_enabled: bool,
    /// Default TTL for outgoing packets
    pub default_ttl: u8,
    /// Enable checksum validation
    pub validate_checksum: bool,
}

impl Default for IcmpConfig {
    fn default() -> Self {
        Self {
            max_packets_per_second: 1000,
            rate_limiting_enabled: true,
            default_ttl: 64,
            validate_checksum: true,
        }
    }
}

/// ICMP comprehensive statistics
#[derive(Debug, Clone)]
pub struct IcmpComprehensiveStats {
    pub echo_requests_sent: u64,
    pub echo_replies_sent: u64,
    pub echo_requests_received: u64,
    pub echo_replies_received: u64,
    pub destination_unreachable_sent: u64,
    pub destination_unreachable_received: u64,
    pub time_exceeded_sent: u64,
    pub time_exceeded_received: u64,
    pub parameter_problem_sent: u64,
    pub parameter_problem_received: u64,
    pub source_quench_sent: u64,
    pub source_quench_received: u64,
    pub redirect_sent: u64,
    pub redirect_received: u64,
    pub timestamp_requests_sent: u64,
    pub timestamp_replies_sent: u64,
    pub timestamp_requests_received: u64,
    pub timestamp_replies_received: u64,
    pub address_mask_requests_sent: u64,
    pub address_mask_replies_sent: u64,
    pub address_mask_requests_received: u64,
    pub address_mask_replies_received: u64,
    pub information_requests_sent: u64,
    pub information_replies_sent: u64,
    pub information_requests_received: u64,
    pub information_replies_received: u64,
    pub total_packets_sent: u64,
    pub total_packets_received: u64,
    pub packets_dropped: u64,
    pub malformed_packets: u64,
    pub rate_limited_packets: u64,
    pub average_rtt: u64,
    pub min_rtt: u64,
    pub max_rtt: u64,
    pub active_handlers: usize,
}

/// Global ICMP processor instance
static GLOBAL_ICMP_PROCESSOR: once_cell::sync::Lazy<EnhancedIcmpProcessor> = 
    once_cell::sync::Lazy::new(|| EnhancedIcmpProcessor::new());

/// Get global ICMP processor
pub fn get_global_icmp_processor() -> &'static EnhancedIcmpProcessor {
    &GLOBAL_ICMP_PROCESSOR
}

/// Initialize enhanced ICMP subsystem
pub fn init_enhanced_icmp() -> Result<(), IcmpError> {
    let processor = get_global_icmp_processor();
    
    // Configure default settings
    processor.configure_rate_limiting(1000); // 1000 packets per second
    processor.set_rate_limiting_enabled(true);
    
    // Initialize statistics
    processor.reset_stats();
    
    log::info!("Enhanced ICMP subsystem initialized");
    Ok(())
}

/// ICMP utility functions
pub mod utils {
    use super::*;

    /// Calculate ICMP checksum
    pub fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum = 0u32;
        
        // Process 16-bit words
        for chunk in data.chunks_exact(2) {
            sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        }
        
        // Handle odd byte
        if data.len() % 2 == 1 {
            sum += (data[data.len() - 1] as u32) << 8;
        }
        
        // Add carry
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }

    /// Validate ICMP packet
    pub fn validate_packet(packet: &EnhancedIcmpPacket) -> bool {
        // Check minimum size
        if packet.get_data().len() < 8 {
            return false;
        }
        
        // Verify checksum
        let data = packet.to_bytes();
        let calculated_checksum = calculate_checksum(&data);
        calculated_checksum == 0
    }

    /// Parse ICMP type from byte
    pub fn parse_type(byte: u8) -> Option<IcmpType> {
        match byte {
            0 => Some(IcmpType::EchoReply),
            3 => Some(IcmpType::DestinationUnreachable),
            4 => Some(IcmpType::SourceQuench),
            5 => Some(IcmpType::Redirect),
            8 => Some(IcmpType::EchoRequest),
            11 => Some(IcmpType::TimeExceeded),
            12 => Some(IcmpType::ParameterProblem),
            13 => Some(IcmpType::TimestampRequest),
            14 => Some(IcmpType::TimestampReply),
            15 => Some(IcmpType::InformationRequest),
            16 => Some(IcmpType::InformationReply),
            17 => Some(IcmpType::AddressMaskRequest),
            18 => Some(IcmpType::AddressMaskReply),
            _ => None,
        }
    }

    /// Parse ICMP code from byte
    pub fn parse_code(icmp_type: IcmpType, byte: u8) -> Option<IcmpCode> {
        match (icmp_type, byte) {
            (IcmpType::DestinationUnreachable, 0) => Some(IcmpCode::NetworkUnreachable),
            (IcmpType::DestinationUnreachable, 1) => Some(IcmpCode::HostUnreachable),
            (IcmpType::DestinationUnreachable, 2) => Some(IcmpCode::ProtocolUnreachable),
            (IcmpType::DestinationUnreachable, 3) => Some(IcmpCode::PortUnreachable),
            (IcmpType::DestinationUnreachable, 4) => Some(IcmpCode::FragmentationNeeded),
            (IcmpType::DestinationUnreachable, 5) => Some(IcmpCode::SourceRouteFailed),
            (IcmpType::TimeExceeded, 0) => Some(IcmpCode::TtlExceeded),
            (IcmpType::TimeExceeded, 1) => Some(IcmpCode::FragmentReassemblyTimeExceeded),
            (IcmpType::Redirect, 0) => Some(IcmpCode::NetworkRedirect),
            (IcmpType::Redirect, 1) => Some(IcmpCode::HostRedirect),
            (IcmpType::Redirect, 2) => Some(IcmpCode::ServiceNetworkRedirect),
            (IcmpType::Redirect, 3) => Some(IcmpCode::ServiceHostRedirect),
            _ => None,
        }
    }
}