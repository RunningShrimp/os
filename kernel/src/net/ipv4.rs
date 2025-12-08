//! IPv4 protocol implementation
//!
//! This module provides IPv4 packet handling, address management, and routing functionality.

extern crate alloc;
use alloc::vec::Vec;
use core::fmt;

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Addr {
    /// Address in network byte order
    addr: u32,
}

impl Ipv4Addr {
    /// Unspecified address (0.0.0.0)
    pub const UNSPECIFIED: Self = Self { addr: 0 };

    /// Loopback address (127.0.0.1)
    pub const LOCALHOST: Self = Self { addr: 0x7F000001 };

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Self = Self { addr: 0xFFFFFFFF };

    /// All multicast nodes (224.0.0.1)
    pub const ALL_MULTICAST_NODES: Self = Self { addr: 0xE0000001 };

    /// Create a new IPv4 address from bytes
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            addr: ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
        }
    }

    /// Create IPv4 address from u32 (host byte order)
    pub const fn from_u32(addr: u32) -> Self {
        Self { addr }
    }

    /// Create IPv4 address from u32 (network byte order)
    pub fn from_be_bytes(bytes: [u8; 4]) -> Self {
        Self {
            addr: u32::from_be_bytes(bytes)
        }
    }

    /// Get address as u32 (host byte order)
    pub const fn to_u32(self) -> u32 {
        self.addr
    }

    /// Get address as bytes (network byte order)
    pub const fn to_be_bytes(self) -> [u8; 4] {
        self.addr.to_be_bytes()
    }

    /// Check if address is unspecified (0.0.0.0)
    pub const fn is_unspecified(self) -> bool {
        self.addr == 0
    }

    /// Check if address is loopback (127.0.0.0/8)
    pub const fn is_loopback(self) -> bool {
        (self.addr & 0xFF000000) == 0x7F000000
    }

    /// Check if address is private (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
    pub const fn is_private(self) -> bool {
        self.is_in_range(10, 0, 0, 0, 10, 255, 255, 255) ||
        self.is_in_range(172, 16, 0, 0, 172, 31, 255, 255) ||
        self.is_in_range(192, 168, 0, 0, 192, 168, 255, 255)
    }

    /// Check if address is multicast (224.0.0.0/4)
    pub const fn is_multicast(self) -> bool {
        (self.addr & 0xF0000000) == 0xE0000000
    }

    /// Check if address is broadcast (255.255.255.255)
    pub const fn is_broadcast(self) -> bool {
        self.addr == 0xFFFFFFFF
    }

    /// Check if address is in the given range
    const fn is_in_range(
        self,
        start_a: u8, start_b: u8, start_c: u8, start_d: u8,
        end_a: u8, end_b: u8, end_c: u8, end_d: u8
    ) -> bool {
        let start = Self::new(start_a, start_b, start_c, start_d).addr;
        let end = Self::new(end_a, end_b, end_c, end_d).addr;
        self.addr >= start && self.addr <= end
    }

    /// Get octets of the address
    pub const fn octets(self) -> [u8; 4] {
        [
            (self.addr >> 24) as u8,
            (self.addr >> 16) as u8,
            (self.addr >> 8) as u8,
            self.addr as u8,
        ]
    }

    /// Parse IPv4 address from string
    pub fn from_str(s: &str) -> Result<Self, Ipv4ParseError> {
        let mut parts = s.split('.');
        let mut octets = [0u8; 4];

        for (i, part) in (&mut parts).take(4).enumerate() {
            match part.parse::<u8>() {
                Ok(octet) => octets[i] = octet,
                Err(_) => return Err(Ipv4ParseError::InvalidFormat),
            }
        }

        if parts.next().is_some() {
            return Err(Ipv4ParseError::InvalidFormat);
        }

        Ok(Self::new(octets[0], octets[1], octets[2], octets[3]))
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let octets = self.octets();
        write!(f, "{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
    }
}

impl Default for Ipv4Addr {
    fn default() -> Self {
        Self::UNSPECIFIED
    }
}

/// IPv4 address parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ipv4ParseError {
    /// Invalid format
    InvalidFormat,
    /// Invalid octet value
    InvalidOctet,
}

/// IPv4 header
#[derive(Debug, Clone)]
pub struct Ipv4Header {
    /// Version (4) + IHL (Header Length)
    pub version_ihl: u8,
    /// Type of Service
    pub tos: u8,
    /// Total Length
    pub total_length: u16,
    /// Identification
    pub identification: u16,
    /// Flags + Fragment Offset
    pub flags_fragment: u16,
    /// Time to Live
    pub ttl: u8,
    /// Protocol
    pub protocol: u8,
    /// Header Checksum
    pub checksum: u16,
    /// Source IP Address
    pub source_addr: Ipv4Addr,
    /// Destination IP Address
    pub dest_addr: Ipv4Addr,
}

impl Ipv4Header {
    /// IPv4 version
    pub const VERSION: u8 = 4;

    /// Minimum header length (no options)
    pub const MIN_HEADER_LEN: u8 = 5;

    /// Maximum header length
    pub const MAX_HEADER_LEN: u8 = 15;

    /// Header size in bytes (without options)
    pub const HEADER_SIZE: usize = 20;

    /// Minimum header size in bytes (alias for HEADER_SIZE)
    pub const MIN_SIZE: usize = 20;

    /// Create a new IPv4 header
    pub fn new(
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        protocol: u8,
        payload_len: u16,
        ttl: u8,
    ) -> Self {
        let ihl = Self::MIN_HEADER_LEN;
        let total_length = Self::HEADER_SIZE + payload_len as usize;

        Self {
            version_ihl: (Self::VERSION << 4) | ihl,
            tos: 0,
            total_length: total_length as u16,
            identification: 0, // Will be set by the sender
            flags_fragment: 0, // No fragmentation by default
            ttl,
            protocol,
            checksum: 0, // Will be calculated
            source_addr,
            dest_addr,
        }
    }

    /// Get the IHL (Internet Header Length) in 32-bit words
    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }

    /// Get the version
    pub fn version(&self) -> u8 {
        (self.version_ihl >> 4) & 0x0F
    }

    /// Get the header size in bytes
    pub fn header_size(&self) -> usize {
        (self.ihl() as usize) * 4
    }

    /// Get the payload size in bytes
    pub fn payload_size(&self) -> usize {
        self.total_length as usize - self.header_size()
    }

    /// Get the DF (Don't Fragment) flag
    pub fn dont_fragment(&self) -> bool {
        (self.flags_fragment & 0x4000) != 0
    }

    /// Get the MF (More Fragments) flag
    pub fn more_fragments(&self) -> bool {
        (self.flags_fragment & 0x2000) != 0
    }

    /// Get the fragment offset
    pub fn fragment_offset(&self) -> u16 {
        self.flags_fragment & 0x1FFF
    }

    /// Set the identification field
    pub fn set_identification(&mut self, identification: u16) {
        self.identification = identification;
    }

    /// Set fragmentation flags
    pub fn set_fragmentation(&mut self, dont_fragment: bool, more_fragments: bool, offset: u16) {
        self.flags_fragment = 0;
        if dont_fragment {
            self.flags_fragment |= 0x4000;
        }
        if more_fragments {
            self.flags_fragment |= 0x2000;
        }
        self.flags_fragment |= offset & 0x1FFF;
    }

    /// Calculate header checksum
    pub fn calculate_checksum(&self) -> u16 {
        let mut sum = 0u32;

        // Convert header to words for checksum calculation
        let header_words = [
            ((self.version_ihl as u16) << 8) | (self.tos as u16),
            self.total_length,
            self.identification,
            self.flags_fragment,
            ((self.ttl as u16) << 8) | (self.protocol as u16),
            self.checksum,
            (self.source_addr.to_u32() >> 16) as u16,
            self.source_addr.to_u32() as u16,
            (self.dest_addr.to_u32() >> 16) as u16,
            self.dest_addr.to_u32() as u16,
        ];

        // Sum all 16-bit words
        for &word in &header_words {
            sum += word as u32;
        }

        // Add carry
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        // One's complement
        !sum as u16
    }

    /// Verify header checksum
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == 0
    }

    /// Set checksum (calculates it)
    pub fn set_checksum(&mut self) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum();
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header_size());

        // Version and IHL
        bytes.push(self.version_ihl);

        // Type of Service
        bytes.push(self.tos);

        // Total Length
        bytes.extend_from_slice(&self.total_length.to_be_bytes());

        // Identification
        bytes.extend_from_slice(&self.identification.to_be_bytes());

        // Flags and Fragment Offset
        bytes.extend_from_slice(&self.flags_fragment.to_be_bytes());

        // TTL
        bytes.push(self.ttl);

        // Protocol
        bytes.push(self.protocol);

        // Checksum
        bytes.extend_from_slice(&self.checksum.to_be_bytes());

        // Source Address
        bytes.extend_from_slice(&self.source_addr.to_be_bytes());

        // Destination Address
        bytes.extend_from_slice(&self.dest_addr.to_be_bytes());

        // Options would go here if present

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Ipv4Error> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(Ipv4Error::PacketTooSmall);
        }

        let version_ihl = bytes[0];
        let version = version_ihl >> 4;
        let ihl = version_ihl & 0x0F;

        if version != Self::VERSION {
            return Err(Ipv4Error::InvalidVersion);
        }

        if ihl < Self::MIN_HEADER_LEN || ihl > Self::MAX_HEADER_LEN {
            return Err(Ipv4Error::InvalidHeaderLength);
        }

        let header_size = (ihl as usize) * 4;
        if bytes.len() < header_size {
            return Err(Ipv4Error::PacketTooSmall);
        }

        let tos = bytes[1];
        let total_length = u16::from_be_bytes([bytes[2], bytes[3]]);
        let identification = u16::from_be_bytes([bytes[4], bytes[5]]);
        let flags_fragment = u16::from_be_bytes([bytes[6], bytes[7]]);
        let ttl = bytes[8];
        let protocol = bytes[9];
        let checksum = u16::from_be_bytes([bytes[10], bytes[11]]);

        let source_addr = Ipv4Addr::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let dest_addr = Ipv4Addr::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        let header = Self {
            version_ihl,
            tos,
            total_length,
            identification,
            flags_fragment,
            ttl,
            protocol,
            checksum,
            source_addr,
            dest_addr,
        };

        // Verify checksum
        if !header.verify_checksum() {
            return Err(Ipv4Error::InvalidChecksum);
        }

        Ok(header)
    }
}

/// IPv4 packet
#[derive(Debug, Clone)]
pub struct Ipv4Packet {
    /// Header
    pub header: Ipv4Header,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Ipv4Packet {
    /// Create a new IPv4 packet
    pub fn new(
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        protocol: u8,
        payload: Vec<u8>,
        ttl: u8,
    ) -> Self {
        let mut header = Ipv4Header::new(
            source_addr,
            dest_addr,
            protocol,
            payload.len() as u16,
            ttl,
        );

        // Calculate checksum
        header.set_checksum();

        Self { header, payload }
    }

    /// Create a packet from header and payload
    pub fn from_header_and_payload(header: Ipv4Header, payload: Vec<u8>) -> Self {
        Self { header, payload }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Ipv4Error> {
        let header = Ipv4Header::from_bytes(bytes)?;
        let header_size = header.header_size();

        if bytes.len() < header_size + header.payload_size() {
            return Err(Ipv4Error::PacketTooSmall);
        }

        let payload = bytes[header_size..header_size + header.payload_size()].to_vec();

        Ok(Self { header, payload })
    }

    /// Get total packet length
    pub fn len(&self) -> usize {
        self.header_size() + self.payload.len()
    }

    /// Get header size
    pub fn header_size(&self) -> usize {
        self.header.header_size()
    }

    /// Check if packet is fragmented
    pub fn is_fragmented(&self) -> bool {
        self.header.more_fragments() || self.header.fragment_offset() > 0
    }

    /// Check if this is the first fragment
    pub fn is_first_fragment(&self) -> bool {
        self.header.fragment_offset() == 0
    }

    /// Check if this is the last fragment
    pub fn is_last_fragment(&self) -> bool {
        !self.header.more_fragments()
    }
}

/// IPv4 protocol numbers
pub mod protocols {
    /// ICMP
    pub const ICMP: u8 = 1;
    /// TCP
    pub const TCP: u8 = 6;
    /// UDP
    pub const UDP: u8 = 17;
    /// IPv6 encapsulation
    pub const IPV6: u8 = 41;
    /// OSPF
    pub const OSPF: u8 = 89;
    /// SCTP
    pub const SCTP: u8 = 132;
}

/// IPv4 errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ipv4Error {
    /// Packet too small
    PacketTooSmall,
    /// Invalid version
    InvalidVersion,
    /// Invalid header length
    InvalidHeaderLength,
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid total length
    InvalidTotalLength,
    /// Protocol not supported
    ProtocolNotSupported,
}

/// Default TTL for IPv4 packets
pub const DEFAULT_TTL: u8 = 64;

/// Maximum transmission unit for IPv4
pub const DEFAULT_MTU: usize = 1500;