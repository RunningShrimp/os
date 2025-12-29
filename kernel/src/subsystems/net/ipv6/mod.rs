//! IPv6 protocol implementation
//!
//! This module provides IPv6 packet handling, address management, and routing functionality.
//! Implements RFC 2460 (IPv6 Specification) and related standards.

extern crate alloc;
use alloc::vec::Vec;
use core::fmt;

// Public submodules
pub mod icmpv6;
pub mod route;

/// IPv6 address (128 bits / 16 octets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv6Addr(pub [u8; 16]);

impl Ipv6Addr {
    /// Unspecified address (::)
    pub const UNSPECIFIED: Self = Self([0; 16]);

    /// Loopback address (::1)
    pub const LOCALHOST: Self = Self({
        let mut addr = [0; 16];
        addr[15] = 1;
        addr
    });

    /// All nodes multicast address (ff00::1)
    pub const ALL_NODES_MULTICAST: Self = Self({
        let mut addr = [0; 16];
        addr[0] = 0xff;
        addr[15] = 1;
        addr
    });

    /// All routers multicast address (ff00::2)
    pub const ALL_ROUTERS_MULTICAST: Self = Self({
        let mut addr = [0; 16];
        addr[0] = 0xff;
        addr[15] = 2;
        addr
    });

    /// IPv4-mapped IPv6 address prefix (::ffff:0:0/96)
    /// Used to represent IPv4 addresses in IPv6 format
    pub fn new_v4_mapped(v4: super::ipv4::Ipv4Addr) -> Self {
        let mut addr = [0; 16];
        addr[10] = 0xff;
        addr[11] = 0xff;
        let octets = v4.octets();
        addr[12] = octets[0];
        addr[13] = octets[1];
        addr[14] = octets[2];
        addr[15] = octets[3];
        Self(addr)
    }

    /// Create IPv6 address from 16 octets
    pub const fn new(octets: [u8; 16]) -> Self {
        Self(octets)
    }

    /// Create IPv6 address from 8 16-bit segments
    pub const fn from_segments(segments: [u16; 8]) -> Self {
        let mut octets = [0; 16];
        let mut i = 0;
        while i < 8 {
            octets[i * 2] = (segments[i] >> 8) as u8;
            octets[i * 2 + 1] = segments[i] as u8;
            i += 1;
        }
        Self(octets)
    }

    /// Get address as 16 octets
    pub const fn octets(self) -> [u8; 16] {
        self.0
    }

    /// Get address as 8 16-bit segments
    pub const fn segments(self) -> [u16; 8] {
        [
            ((self.0[0] as u16) << 8) | (self.0[1] as u16),
            ((self.0[2] as u16) << 8) | (self.0[3] as u16),
            ((self.0[4] as u16) << 8) | (self.0[5] as u16),
            ((self.0[6] as u16) << 8) | (self.0[7] as u16),
            ((self.0[8] as u16) << 8) | (self.0[9] as u16),
            ((self.0[10] as u16) << 8) | (self.0[11] as u16),
            ((self.0[12] as u16) << 8) | (self.0[13] as u16),
            ((self.0[14] as u16) << 8) | (self.0[15] as u16),
        ]
    }

    /// Check if address is unspecified (::)
    pub const fn is_unspecified(self) -> bool {
        self.0[0] == 0
            && self.0[1] == 0
            && self.0[2] == 0
            && self.0[3] == 0
            && self.0[4] == 0
            && self.0[5] == 0
            && self.0[6] == 0
            && self.0[7] == 0
            && self.0[8] == 0
            && self.0[9] == 0
            && self.0[10] == 0
            && self.0[11] == 0
            && self.0[12] == 0
            && self.0[13] == 0
            && self.0[14] == 0
            && self.0[15] == 0
    }

    /// Check if address is loopback (::1)
    pub const fn is_loopback(self) -> bool {
        self.0[0] == 0
            && self.0[1] == 0
            && self.0[2] == 0
            && self.0[3] == 0
            && self.0[4] == 0
            && self.0[5] == 0
            && self.0[6] == 0
            && self.0[7] == 0
            && self.0[8] == 0
            && self.0[9] == 0
            && self.0[10] == 0
            && self.0[11] == 0
            && self.0[12] == 0
            && self.0[13] == 0
            && self.0[14] == 0
            && self.0[15] == 1
    }

    /// Check if address is unicast link-local (fe80::/10)
    pub const fn is_unicast_link_local(&self) -> bool {
        (self.0[0] == 0xfe) && ((self.0[1] & 0xc0) == 0x80)
    }

    /// Check if address is unicast site-local (fec0::/10) - deprecated
    pub const fn is_unicast_site_local(&self) -> bool {
        (self.0[0] == 0xfe) && ((self.0[1] & 0xc0) == 0xc0)
    }

    /// Check if address is unique local (fc00::/7)
    pub const fn is_unique_local(&self) -> bool {
        (self.0[0] & 0xfe) == 0xfc
    }

    /// Check if address is multicast (ff00::/8)
    pub const fn is_multicast(&self) -> bool {
        self.0[0] == 0xff
    }

    /// Check if address is IPv4-mapped (::ffff:0:0/96)
    pub const fn is_v4_mapped(&self) -> bool {
        self.0[0] == 0
            && self.0[1] == 0
            && self.0[2] == 0
            && self.0[3] == 0
            && self.0[4] == 0
            && self.0[5] == 0
            && self.0[6] == 0
            && self.0[7] == 0
            && self.0[8] == 0
            && self.0[9] == 0
            && self.0[10] == 0xff
            && self.0[11] == 0xff
    }

    /// Extract IPv4 address if this is IPv4-mapped
    pub fn to_v4_mapped(self) -> Option<super::ipv4::Ipv4Addr> {
        if self.is_v4_mapped() {
            Some(super::ipv4::Ipv4Addr::new(
                self.0[12],
                self.0[13],
                self.0[14],
                self.0[15],
            ))
        } else {
            None
        }
    }

    /// Get multicast scope if address is multicast
    pub fn multicast_scope(&self) -> Option<MulticastScope> {
        if !self.is_multicast() {
            return None;
        }
        match self.0[1] & 0x0f {
            1 => Some(MulticastScope::InterfaceLocal),
            2 => Some(MulticastScope::LinkLocal),
            3 => Some(MulticastScope::RealmLocal),
            4 => Some(MulticastScope::AdminLocal),
            5 => Some(MulticastScope::SiteLocal),
            8 => Some(MulticastScope::OrganizationLocal),
            14 => Some(MulticastScope::Global),
            _ => None,
        }
    }

    /// Parse IPv6 address from string
    pub fn from_str(s: &str) -> Result<Self, Ipv6ParseError> {
        // Handle :: shorthand for zero compression
        let parts: Vec<&str> = s.split("::").collect();

        if parts.len() > 2 {
            return Err(Ipv6ParseError::InvalidFormat);
        }

        let mut segments = [0u16; 8];
        let mut seg_idx = 0;

        if parts.len() == 1 {
            // No compression
            for part in parts[0].split(':') {
                if seg_idx >= 8 {
                    return Err(Ipv6ParseError::InvalidFormat);
                }
                segments[seg_idx] = u16::from_str_radix(part, 16)
                    .map_err(|_| Ipv6ParseError::InvalidSegment)?;
                seg_idx += 1;
            }
        } else {
            // Compression present
            let left_parts: Vec<&str> = if parts[0].is_empty() {
                Vec::new()
            } else {
                parts[0].split(':').collect()
            };
            let right_parts: Vec<&str> = if parts[1].is_empty() {
                Vec::new()
            } else {
                parts[1].split(':').collect()
            };

            if left_parts.len() + right_parts.len() > 8 {
                return Err(Ipv6ParseError::InvalidFormat);
            }

            // Parse left side
            for part in &left_parts {
                segments[seg_idx] = u16::from_str_radix(part, 16)
                    .map_err(|_| Ipv6ParseError::InvalidSegment)?;
                seg_idx += 1;
            }

            // Calculate zeros to insert
            let zero_count = 8 - left_parts.len() - right_parts.len();
            seg_idx += zero_count;

            // Parse right side
            for part in &right_parts {
                segments[seg_idx] = u16::from_str_radix(part, 16)
                    .map_err(|_| Ipv6ParseError::InvalidSegment)?;
                seg_idx += 1;
            }
        }

        Ok(Self::from_segments(segments))
    }
}

impl Default for Ipv6Addr {
    fn default() -> Self {
        Self::UNSPECIFIED
    }
}

impl fmt::Display for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments = self.segments();

        // Find longest run of zeros for compression
        let mut zero_start = 0;
        let mut zero_len = 0;
        let mut current_start = 0;
        let mut current_len = 0;

        for (i, &seg) in segments.iter().enumerate() {
            if seg == 0 {
                if current_len == 0 {
                    current_start = i;
                }
                current_len += 1;
            } else {
                if current_len > zero_len {
                    zero_start = current_start;
                    zero_len = current_len;
                }
                current_len = 0;
            }
        }

        if current_len > zero_len {
            zero_start = current_start;
            zero_len = current_len;
        }

        // Format with compression if beneficial
        if zero_len >= 2 {
            for i in 0..zero_start {
                write!(f, "{:x}", segments[i])?;
                if i < 7 {
                    write!(f, ":")?;
                }
            }
            write!(f, ":")?;
            for i in (zero_start + zero_len)..8 {
                write!(f, "{:x}", segments[i])?;
                if i < 7 {
                    write!(f, ":")?;
                }
            }
        } else {
            for i in 0..8 {
                write!(f, "{:x}", segments[i])?;
                if i < 7 {
                    write!(f, ":")?;
                }
            }
        }

        Ok(())
    }
}

/// Multicast scope values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MulticastScope {
    /// Interface-local (1)
    InterfaceLocal,
    /// Link-local (2)
    LinkLocal,
    /// Realm-local (3)
    RealmLocal,
    /// Admin-local (4)
    AdminLocal,
    /// Site-local (5)
    SiteLocal,
    /// Organization-local (8)
    OrganizationLocal,
    /// Global (14)
    Global,
}

/// IPv6 address parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ipv6ParseError {
    /// Invalid format
    InvalidFormat,
    /// Invalid segment
    InvalidSegment,
    /// Too many segments
    TooManySegments,
    /// Invalid scope ID
    InvalidScopeId,
}

/// IPv6 header (40 bytes fixed)
///
/// RFC 2460 Section 3: Base Header Format
#[derive(Debug, Clone)]
pub struct Ipv6Header {
    /// Version (4 bits) - must be 6
    /// Traffic Class (8 bits) - DS field and ECN
    pub version_tc: [u8; 2],
    /// Flow Label (20 bits)
    pub flow_label: u32,
    /// Payload Length (16 bits) - length of payload after header
    pub payload_length: u16,
    /// Next Header (8 bits) - protocol of next header
    pub next_header: u8,
    /// Hop Limit (8 bits) - decremented by each router
    pub hop_limit: u8,
    /// Source address (128 bits)
    pub source_addr: Ipv6Addr,
    /// Destination address (128 bits)
    pub dest_addr: Ipv6Addr,
}

impl Ipv6Header {
    /// IPv6 version
    pub const VERSION: u8 = 6;

    /// Header size in bytes (fixed for IPv6)
    pub const HEADER_SIZE: usize = 40;

    /// Create a new IPv6 header
    pub fn new(
        source_addr: Ipv6Addr,
        dest_addr: Ipv6Addr,
        next_header: u8,
        payload_len: u16,
        hop_limit: u8,
    ) -> Self {
        let mut version_tc = [0; 2];
        version_tc[0] = (Self::VERSION << 4) & 0xF0;

        Self {
            version_tc,
            flow_label: 0,
            payload_length: payload_len,
            next_header,
            hop_limit,
            source_addr,
            dest_addr,
        }
    }

    /// Get version (should be 6)
    pub fn version(&self) -> u8 {
        (self.version_tc[0] >> 4) & 0x0F
    }

    /// Get traffic class (DS field + ECN)
    pub fn traffic_class(&self) -> u8 {
        ((self.version_tc[0] & 0x0F) << 4) | ((self.version_tc[1] >> 4) & 0x0F)
    }

    /// Set traffic class
    pub fn set_traffic_class(&mut self, tc: u8) {
        self.version_tc[0] = (self.version_tc[0] & 0xF0) | ((tc >> 4) & 0x0F);
        self.version_tc[1] = (self.version_tc[1] & 0x0F) | ((tc & 0x0F) << 4);
    }

    /// Get flow label (20 bits)
    pub fn flow_label(&self) -> u32 {
        self.flow_label & 0x000FFFFF
    }

    /// Set flow label (20 bits)
    pub fn set_flow_label(&mut self, label: u32) {
        self.flow_label = label & 0x000FFFFF;
    }

    /// Get DSCP (Differentiated Services Code Point) - upper 6 bits of traffic class
    pub fn dscp(&self) -> u8 {
        self.traffic_class() >> 2
    }

    /// Set DSCP
    pub fn set_dscp(&mut self, dscp: u8) {
        let tc = (self.traffic_class() & 0x03) | ((dscp & 0x3F) << 2);
        self.set_traffic_class(tc);
    }

    /// Get ECN (Explicit Congestion Notification) - lower 2 bits of traffic class
    pub fn ecn(&self) -> u8 {
        self.traffic_class() & 0x03
    }

    /// Set ECN
    pub fn set_ecn(&mut self, ecn: u8) {
        let tc = (self.traffic_class() & 0xFC) | (ecn & 0x03);
        self.set_traffic_class(tc);
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut bytes = [0; Self::HEADER_SIZE];

        bytes[0] = self.version_tc[0];
        bytes[1] = self.version_tc[1];

        // Flow label (4 bytes: version_tc[2], version_tc[3], and part of flow_label)
        let flow = self.flow_label().to_be_bytes();
        bytes[2] = flow[0];
        bytes[3] = flow[1];
        bytes[4] = flow[2];
        bytes[5] = flow[3];

        // Payload length
        let payload = self.payload_length.to_be_bytes();
        bytes[6] = payload[0];
        bytes[7] = payload[1];

        // Next header
        bytes[8] = self.next_header;

        // Hop limit
        bytes[9] = self.hop_limit;

        // Source address
        bytes[10..26].copy_from_slice(&self.source_addr.0);

        // Destination address
        bytes[26..42].copy_from_slice(&self.dest_addr.0);

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Ipv6Error> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(Ipv6Error::PacketTooSmall);
        }

        let mut version_tc = [0; 2];
        version_tc[0] = bytes[0];
        version_tc[1] = bytes[1];

        let version = bytes[0] >> 4;
        if version != Self::VERSION {
            return Err(Ipv6Error::InvalidVersion);
        }

        let flow_label = ((bytes[2] as u32) << 24)
            | ((bytes[3] as u32) << 16)
            | ((bytes[4] as u32) << 8)
            | (bytes[5] as u32);

        let payload_length = u16::from_be_bytes([bytes[6], bytes[7]]);
        let next_header = bytes[8];
        let hop_limit = bytes[9];

        let mut source_addr = [0; 16];
        source_addr.copy_from_slice(&bytes[10..26]);

        let mut dest_addr = [0; 16];
        dest_addr.copy_from_slice(&bytes[26..42]);

        Ok(Self {
            version_tc,
            flow_label,
            payload_length,
            next_header,
            hop_limit,
            source_addr: Ipv6Addr(source_addr),
            dest_addr: Ipv6Addr(dest_addr),
        })
    }

    /// Get header size in bytes
    pub fn header_size(&self) -> usize {
        Self::HEADER_SIZE
    }

    /// Get payload size in bytes
    pub fn payload_size(&self) -> usize {
        self.payload_length as usize
    }
}

/// IPv6 packet
#[derive(Debug, Clone)]
pub struct Ipv6Packet {
    /// Header
    pub header: Ipv6Header,
    /// Extension headers
    pub extension_headers: Vec<ExtensionHeader>,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Ipv6Packet {
    /// Create a new IPv6 packet
    pub fn new(
        source_addr: Ipv6Addr,
        dest_addr: Ipv6Addr,
        next_header: u8,
        payload: Vec<u8>,
        hop_limit: u8,
    ) -> Self {
        let header = Ipv6Header::new(
            source_addr,
            dest_addr,
            next_header,
            payload.len() as u16,
            hop_limit,
        );

        Self {
            header,
            extension_headers: Vec::new(),
            payload,
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Main header
        bytes.extend_from_slice(&self.header.to_bytes());

        // Extension headers
        for ext_header in &self.extension_headers {
            bytes.extend_from_slice(&ext_header.to_bytes());
        }

        // Payload
        bytes.extend_from_slice(&self.payload);

        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Ipv6Error> {
        let header = Ipv6Header::from_bytes(bytes)?;

        let mut offset = header.header_size();
        let mut next_header = header.next_header;
        let mut extension_headers = Vec::new();

        // Parse extension headers if present
        while is_extension_header(next_header) {
            let ext_header = ExtensionHeader::from_bytes(&bytes[offset..], next_header)?;
            next_header = ext_header.next_header();
            offset += ext_header.len();
            extension_headers.push(ext_header);
        }

        // Parse payload
        let payload_start = offset;
        let payload_end = payload_start + header.payload_size();
        if payload_end > bytes.len() {
            return Err(Ipv6Error::PacketTooSmall);
        }

        let payload = bytes[payload_start..payload_end].to_vec();

        Ok(Self {
            header,
            extension_headers,
            payload,
        })
    }

    /// Get total packet length
    pub fn len(&self) -> usize {
        self.header.header_size()
            + self.extension_headers.iter().map(|h| h.len()).sum::<usize>()
            + self.payload.len()
    }

    /// Get the upper-layer protocol (after extension headers)
    pub fn upper_layer_protocol(&self) -> u8 {
        if let Some(last_ext) = self.extension_headers.last() {
            last_ext.next_header()
        } else {
            self.header.next_header
        }
    }
}

/// Check if protocol number is an extension header
fn is_extension_header(protocol: u8) -> bool {
    matches!(
        protocol,
        0 | // Hop-by-Hop Options
        43 | // Routing
        44 | // Fragment
        50 | // Encapsulating Security Payload
        51 | // Authentication Header
        60    // Destination Options
    )
}

/// IPv6 extension header
#[derive(Debug, Clone)]
pub enum ExtensionHeader {
    /// Hop-by-Hop Options (protocol 0)
    HopByHop {
        next_header: u8,
        options: Vec<u8>,
    },
    /// Routing Header (protocol 43)
    Routing {
        next_header: u8,
        routing_type: u8,
        segments_left: u8,
        data: Vec<u8>,
    },
    /// Fragment Header (protocol 44)
    Fragment {
        next_header: u8,
        fragment_offset: u16,
        more_fragments: bool,
        identification: u32,
    },
    /// Destination Options (protocol 60)
    DestinationOptions {
        next_header: u8,
        options: Vec<u8>,
    },
}

impl ExtensionHeader {
    /// Get the next header field
    pub fn next_header(&self) -> u8 {
        match self {
            Self::HopByHop { next_header, .. } => *next_header,
            Self::Routing { next_header, .. } => *next_header,
            Self::Fragment { next_header, .. } => *next_header,
            Self::DestinationOptions { next_header, .. } => *next_header,
        }
    }

    /// Get the length of this extension header in bytes
    pub fn len(&self) -> usize {
        match self {
            Self::HopByHop { options, .. } => 2 + options.len(),
            Self::Routing { data, .. } => 4 + data.len(),
            Self::Fragment { .. } => 8,
            Self::DestinationOptions { options, .. } => 2 + options.len(),
        }
    }

    /// Serialize extension header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        match self {
            Self::HopByHop { next_header, options } => {
                bytes.push(*next_header);
                let hdr_ext_len = ((options.len() + 2) / 8) - 1;
                bytes.push(hdr_ext_len as u8);
                bytes.extend_from_slice(options);
            }
            Self::Routing { next_header, routing_type, segments_left, data } => {
                bytes.push(*next_header);
                let hdr_ext_len = ((data.len() + 4) / 8) - 1;
                bytes.push(hdr_ext_len as u8);
                bytes.push(*routing_type);
                bytes.push(*segments_left);
                bytes.extend_from_slice(data);
            }
            Self::Fragment { next_header, fragment_offset, more_fragments, identification } => {
                bytes.push(*next_header);
                bytes.push(0); // Reserved
                let offset = fragment_offset >> 3;
                let flags = if *more_fragments { 0x01 } else { 0x00 } << 5;
                bytes.extend_from_slice(&(((offset & 0x1FFF) as u16 | flags).to_be_bytes()));
                bytes.extend_from_slice(&identification.to_be_bytes());
            }
            Self::DestinationOptions { next_header, options } => {
                bytes.push(*next_header);
                let hdr_ext_len = ((options.len() + 2) / 8) - 1;
                bytes.push(hdr_ext_len as u8);
                bytes.extend_from_slice(options);
            }
        }

        // Pad to 8-octet boundary
        while bytes.len() % 8 != 0 {
            bytes.push(0);
        }

        bytes
    }

    /// Parse extension header from bytes
    pub fn from_bytes(bytes: &[u8], header_type: u8) -> Result<Self, Ipv6Error> {
        if bytes.len() < 2 {
            return Err(Ipv6Error::PacketTooSmall);
        }

        let next_header = bytes[0];
        let hdr_ext_len = (bytes[1] as usize) * 8 + 8;

        match header_type {
            0 => {
                // Hop-by-Hop Options
                if bytes.len() < hdr_ext_len {
                    return Err(Ipv6Error::PacketTooSmall);
                }
                let options = bytes[2..hdr_ext_len].to_vec();
                Ok(Self::HopByHop { next_header, options })
            }
            43 => {
                // Routing Header
                if bytes.len() < 4 {
                    return Err(Ipv6Error::PacketTooSmall);
                }
                let routing_type = bytes[2];
                let segments_left = bytes[3];
                let data = if bytes.len() > hdr_ext_len {
                    bytes[4..hdr_ext_len].to_vec()
                } else {
                    Vec::new()
                };
                Ok(Self::Routing { next_header, routing_type, segments_left, data })
            }
            44 => {
                // Fragment Header
                if bytes.len() < 8 {
                    return Err(Ipv6Error::PacketTooSmall);
                }
                let fragment_offset = u16::from_be_bytes([bytes[2], bytes[3]]);
                let more_fragments = (fragment_offset & 0x01) != 0;
                let offset = fragment_offset >> 3;
                let identification = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                Ok(Self::Fragment { next_header, fragment_offset: offset, more_fragments, identification })
            }
            60 => {
                // Destination Options
                if bytes.len() < hdr_ext_len {
                    return Err(Ipv6Error::PacketTooSmall);
                }
                let options = bytes[2..hdr_ext_len].to_vec();
                Ok(Self::DestinationOptions { next_header, options })
            }
            _ => Err(Ipv6Error::InvalidExtensionHeader),
        }
    }
}

/// IPv6 protocol numbers (Next Header field)
pub mod protocols {
    /// IPv6 Hop-by-Hop Option
    pub const HOPOPT: u8 = 0;
    /// ICMPv6
    pub const ICMPV6: u8 = 58;
    /// TCP
    pub const TCP: u8 = 6;
    /// UDP
    pub const UDP: u8 = 17;
    /// IPv6-Encapsulated IPv4
    pub const IPV6: u8 = 41;
    /// IPv6 Route Header
    pub const ROUTING: u8 = 43;
    /// IPv6 Fragment Header
    pub const FRAGMENT: u8 = 44;
    /// Encapsulating Security Payload
    pub const ESP: u8 = 50;
    /// Authentication Header
    pub const AH: u8 = 51;
    /// IPv6 Destination Options
    pub const DSTOPTS: u8 = 60;
    /// OSPF for IPv6
    pub const OSPF: u8 = 89;
}

/// IPv6 errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ipv6Error {
    /// Packet too small
    PacketTooSmall,
    /// Invalid version
    InvalidVersion,
    /// Invalid header length
    InvalidHeaderLength,
    /// Invalid extension header
    InvalidExtensionHeader,
    /// Invalid total length
    InvalidTotalLength,
    /// Protocol not supported
    ProtocolNotSupported,
    /// Invalid checksum
    InvalidChecksum,
}

/// Default hop limit for IPv6 packets
pub const DEFAULT_HOP_LIMIT: u8 = 64;

/// Maximum transmission unit for IPv6
pub const DEFAULT_MTU: usize = 1280; // Minimum MTU for IPv6

/// Minimum MTU for IPv6 (RFC 8200)
pub const MINIMUM_MTU: usize = 1280;
