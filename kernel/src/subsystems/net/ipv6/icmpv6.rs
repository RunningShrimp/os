//! ICMPv6 protocol implementation
//!
//! This module provides ICMPv6 protocol support for IPv6 networks.
//! Implements RFC 4443 (ICMPv6) and RFC 4861 (Neighbor Discovery).

extern crate alloc;
use alloc::vec::Vec;

use crate::subsystems::net::ipv6::Ipv6Addr;

/// ICMPv6 message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6Type {
    /// Destination Unreachable
    DestinationUnreachable = 1,
    /// Packet Too Big
    PacketTooBig = 2,
    /// Time Exceeded
    TimeExceeded = 3,
    /// Parameter Problem
    ParameterProblem = 4,
    /// Echo Request
    EchoRequest = 128,
    /// Echo Reply
    EchoReply = 129,
    /// Multicast Listener Query
    MulticastListenerQuery = 130,
    /// Multicast Listener Report
    MulticastListenerReport = 131,
    /// Multicast Listener Done
    MulticastListenerDone = 132,
    /// Router Solicitation
    RouterSolicitation = 133,
    /// Router Advertisement
    RouterAdvertisement = 134,
    /// Neighbor Solicitation
    NeighborSolicitation = 135,
    /// Neighbor Advertisement
    NeighborAdvertisement = 136,
    /// Redirect Message
    Redirect = 137,
}

impl Icmpv6Type {
    /// Parse from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::DestinationUnreachable),
            2 => Some(Self::PacketTooBig),
            3 => Some(Self::TimeExceeded),
            4 => Some(Self::ParameterProblem),
            128 => Some(Self::EchoRequest),
            129 => Some(Self::EchoReply),
            130 => Some(Self::MulticastListenerQuery),
            131 => Some(Self::MulticastListenerReport),
            132 => Some(Self::MulticastListenerDone),
            133 => Some(Self::RouterSolicitation),
            134 => Some(Self::RouterAdvertisement),
            135 => Some(Self::NeighborSolicitation),
            136 => Some(Self::NeighborAdvertisement),
            137 => Some(Self::Redirect),
            _ => None,
        }
    }

    /// Check if this is an error message
    pub fn is_error_message(&self) -> bool {
        matches!(
            self,
            Self::DestinationUnreachable
                | Self::PacketTooBig
                | Self::TimeExceeded
                | Self::ParameterProblem
        )
    }

    /// Check if this is an informational message
    pub fn is_informational(&self) -> bool {
        !self.is_error_message()
    }
}

/// ICMPv6 codes for Destination Unreachable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6DstUnreachableCode {
    /// No route to destination
    NoRoute = 0,
    /// Communication with destination administratively prohibited
    AdminProhibited = 1,
    /// Beyond scope of source address
    BeyondScope = 2,
    /// Address unreachable
    AddressUnreachable = 3,
    /// Port unreachable
    PortUnreachable = 4,
    /// Source address failed ingress/egress policy
    PolicyFailed = 5,
    /// Reject route to destination
    RejectRoute = 6,
}

/// ICMPv6 codes for Time Exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6TimeExceededCode {
    /// Hop limit exceeded in transit
    HopLimitExceeded = 0,
    /// Fragment reassembly time exceeded
    ReassemblyTimeExceeded = 1,
}

/// ICMPv6 codes for Parameter Problem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6ParameterProblemCode {
    /// Erroneous header field encountered
    ErroneousHeaderField = 0,
    /// Unrecognized Next Header type encountered
    UnrecognizedNextHeader = 1,
    /// Unrecognized IPv6 option encountered
    UnrecognizedOption = 2,
}

/// ICMPv6 header
#[derive(Debug, Clone)]
pub struct Icmpv6Header {
    /// Message type
    pub message_type: Icmpv6Type,
    /// Message code
    pub code: u8,
    /// Checksum
    pub checksum: u16,
}

impl Icmpv6Header {
    /// Create a new ICMPv6 header
    pub fn new(message_type: Icmpv6Type, code: u8) -> Self {
        Self {
            message_type,
            code,
            checksum: 0,
        }
    }

    /// Calculate checksum (includes pseudo-header)
    pub fn calculate_checksum(&self, source: Ipv6Addr, dest: Ipv6Addr, data: &[u8]) -> u16 {
        let mut sum = 0u32;

        // Pseudo-header
        for chunk in source.0.chunks(2) {
            sum += (((chunk[0] as u16) << 8) | (chunk[1] as u16)) as u32;
        }
        for chunk in dest.0.chunks(2) {
            sum += (((chunk[0] as u16) << 8) | (chunk[1] as u16)) as u32;
        }

        // Upper layer packet length
        sum += data.len() as u32;

        // Next header (ICMPv6 = 58)
        sum += 58u32;

        // ICMPv6 header
        sum += (((self.message_type as u16) << 8) | (self.code as u16)) as u32;
        sum += self.checksum as u32;

        // Data
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

        // Fold 32-bit sum to 16 bits
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Set checksum
    pub fn set_checksum(&mut self, source: Ipv6Addr, dest: Ipv6Addr, data: &[u8]) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum(source, dest, data);
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);
        bytes.push(self.message_type as u8);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }
}

/// ICMPv6 packet
#[derive(Debug, Clone)]
pub struct Icmpv6Packet {
    /// Header
    pub header: Icmpv6Header,
    /// Message-specific data
    pub payload: Vec<u8>,
}

impl Icmpv6Packet {
    /// Create a new ICMPv6 packet
    pub fn new(message_type: Icmpv6Type, code: u8, payload: Vec<u8>) -> Self {
        let header = Icmpv6Header::new(message_type, code);
        Self { header, payload }
    }

    /// Create echo request packet
    pub fn echo_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&identifier.to_be_bytes());
        payload.extend_from_slice(&sequence.to_be_bytes());
        payload.extend_from_slice(&data);

        Self::new(Icmpv6Type::EchoRequest, 0, payload)
    }

    /// Create echo reply packet
    pub fn echo_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&identifier.to_be_bytes());
        payload.extend_from_slice(&sequence.to_be_bytes());
        payload.extend_from_slice(&data);

        Self::new(Icmpv6Type::EchoReply, 0, payload)
    }

    /// Get identifier (for echo messages)
    pub fn identifier(&self) -> u16 {
        if self.payload.len() >= 2 {
            u16::from_be_bytes([self.payload[0], self.payload[1]])
        } else {
            0
        }
    }

    /// Get sequence number (for echo messages)
    pub fn sequence(&self) -> u16 {
        if self.payload.len() >= 4 {
            u16::from_be_bytes([self.payload[2], self.payload[3]])
        } else {
            0
        }
    }

    /// Get echo data (for echo messages)
    pub fn echo_data(&self) -> &[u8] {
        if self.payload.len() > 4 {
            &self.payload[4..]
        } else {
            &[]
        }
    }

    /// Create neighbor solicitation packet
    pub fn neighbor_solicitation(target: Ipv6Addr) -> Self {
        let mut payload = Vec::with_capacity(20);
        payload.extend_from_slice(&target.0);

        // Add source link-layer option (placeholder)
        // In practice, this would include the sender's MAC address

        Self::new(Icmpv6Type::NeighborSolicitation, 0, payload)
    }

    /// Create neighbor advertisement packet
    pub fn neighbor_advertisement(
        target: Ipv6Addr,
        router: bool,
        solicited: bool,
        override_flag: bool,
    ) -> Self {
        let mut flags = 0u8;
        if router {
            flags |= 0x80;
        }
        if solicited {
            flags |= 0x40;
        }
        if override_flag {
            flags |= 0x20;
        }

        let mut payload = Vec::with_capacity(21);
        payload.push(flags);
        payload.extend_from_slice(&[0, 0, 0]); // Reserved
        payload.extend_from_slice(&target.0);

        Self::new(Icmpv6Type::NeighborAdvertisement, 0, payload)
    }

    /// Create router solicitation packet
    pub fn router_solicitation() -> Self {
        Self::new(Icmpv6Type::RouterSolicitation, 0, Vec::new())
    }

    /// Create packet too big message
    pub fn packet_to_big(mtu: u32, invoking_packet: &[u8]) -> Self {
        let mut payload = Vec::with_capacity(4 + invoking_packet.len());
        payload.extend_from_slice(&mtu.to_be_bytes());
        payload.extend_from_slice(invoking_packet);

        Self::new(Icmpv6Type::PacketTooBig, 0, payload)
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Icmpv6Error> {
        if bytes.len() < 4 {
            return Err(Icmpv6Error::PacketTooSmall);
        }

        let message_type = Icmpv6Type::from_u8(bytes[0])
            .ok_or(Icmpv6Error::InvalidMessageType)?;

        let code = bytes[1];
        let checksum = u16::from_be_bytes([bytes[2], bytes[3]]);

        let payload = if bytes.len() > 4 {
            bytes[4..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            header: Icmpv6Header {
                message_type,
                code,
                checksum,
            },
            payload,
        })
    }

    /// Verify checksum
    pub fn verify_checksum(&self, source: Ipv6Addr, dest: Ipv6Addr) -> bool {
        self.header.checksum
            == self.header.calculate_checksum(source, dest, &self.payload)
    }

    /// Set and calculate checksum
    pub fn set_checksum(&mut self, source: Ipv6Addr, dest: Ipv6Addr) {
        self.header.set_checksum(source, dest, &self.payload);
    }
}

/// ICMPv6 errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icmpv6Error {
    /// Packet too small
    PacketTooSmall,
    /// Invalid message type
    InvalidMessageType,
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid packet format
    InvalidPacket,
    /// Rate limited
    RateLimited,
}

/// ICMPv6 processor for handling incoming packets
pub struct Icmpv6Processor {
    /// Echo request handler
    echo_handler: Option<alloc::boxed::Box<dyn Fn(Ipv6Addr, Ipv6Addr, Icmpv6Packet) -> Option<Icmpv6Packet>>>,
}

impl Icmpv6Processor {
    /// Create a new ICMPv6 processor
    pub fn new() -> Self {
        Self { echo_handler: None }
    }

    /// Set echo request handler
    pub fn set_echo_handler<F>(&mut self, handler: F)
    where
        F: Fn(Ipv6Addr, Ipv6Addr, Icmpv6Packet) -> Option<Icmpv6Packet> + 'static,
    {
        self.echo_handler = Some(alloc::boxed::Box::new(handler));
    }

    /// Process an incoming ICMPv6 packet
    pub fn process_packet(
        &self,
        source: Ipv6Addr,
        dest: Ipv6Addr,
        packet: Icmpv6Packet,
    ) -> Option<Icmpv6Packet> {
        // Verify checksum first
        if !packet.verify_checksum(source, dest) {
            return None;
        }

        match packet.header.message_type {
            Icmpv6Type::EchoRequest => {
                if let Some(ref handler) = self.echo_handler {
                    handler(source, dest, packet)
                } else {
                    // Default: send echo reply
                    Some(Icmpv6Packet::echo_reply(
                        packet.identifier(),
                        packet.sequence(),
                        packet.echo_data().to_vec(),
                    ))
                }
            }
            Icmpv6Type::EchoReply => {
                // Handle echo reply
                None
            }
            Icmpv6Type::NeighborSolicitation => {
                // Handle neighbor solicitation
                None
            }
            Icmpv6Type::NeighborAdvertisement => {
                // Handle neighbor advertisement
                None
            }
            Icmpv6Type::RouterSolicitation => {
                // Handle router solicitation
                None
            }
            Icmpv6Type::RouterAdvertisement => {
                // Handle router advertisement
                None
            }
            _ => {
                // Handle other ICMPv6 message types
                None
            }
        }
    }
}

impl Default for Icmpv6Processor {
    fn default() -> Self {
        Self::new()
    }
}

/// Neighbor Discovery Protocol (NDP) cache entry
#[derive(Debug, Clone)]
pub struct NdpEntry {
    /// IPv6 address
    pub address: Ipv6Addr,
    /// Link-layer address (MAC)
    pub lladdr: [u8; 6],
    /// Entry state
    pub state: NdpState,
    /// Time when entry was created
    pub created_at: u64,
    /// Time until entry expires
    pub expires_at: u64,
}

/// NDP entry states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NdpState {
    /// Address resolution in progress
    Incomplete,
    /// Reachable
    Reachable,
    /// Stale (may not be reachable)
    Stale,
    /// Delay (confirmation pending)
    Delay,
    /// Probe (actively probing reachability)
    Probe,
    /// Permanent entry (will not expire)
    Permanent,
}

/// NDP cache for neighbor discovery
pub struct NdpCache {
    /// Cache entries
    entries: alloc::vec::Vec<NdpEntry>,
    /// Maximum number of entries
    max_entries: usize,
}

impl NdpCache {
    /// Create a new NDP cache
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: alloc::vec::Vec::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Look up an entry by IPv6 address
    pub fn lookup(&self, address: Ipv6Addr) -> Option<&NdpEntry> {
        self.entries.iter().find(|e| e.address == address)
    }

    /// Look up an entry mutably
    pub fn lookup_mut(&mut self, address: Ipv6Addr) -> Option<&mut NdpEntry> {
        self.entries.iter_mut().find(|e| e.address == address)
    }

    /// Add or update an entry
    pub fn insert(&mut self, entry: NdpEntry) {
        // Remove existing entry if present
        self.entries.retain(|e| e.address != entry.address);

        // Add new entry
        self.entries.push(entry);

        // Enforce capacity limit
        if self.entries.len() > self.max_entries {
            // Remove oldest entries
            self.entries.sort_by_key(|e| e.created_at);
            self.entries.drain(0..(self.entries.len() - self.max_entries));
        }
    }

    /// Remove an entry
    pub fn remove(&mut self, address: Ipv6Addr) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|e| e.address != address);
        self.entries.len() < original_len
    }

    /// Remove expired entries
    pub fn prune_expired(&mut self, current_time: u64) {
        self.entries
            .retain(|e| e.state == NdpState::Permanent || e.expires_at > current_time);
    }

    /// Get all entries
    pub fn entries(&self) -> &[NdpEntry] {
        &self.entries
    }
}

impl Default for NdpCache {
    fn default() -> Self {
        Self::new(1024)
    }
}
