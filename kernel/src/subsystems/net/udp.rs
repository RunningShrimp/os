//! User Datagram Protocol (UDP) implementation
//!
//! This module provides UDP protocol support for connectionless datagram communication.

extern crate alloc;
use alloc::vec::Vec;
use super::ipv4::Ipv4Addr;

/// UDP header
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Length of header and data
    pub length: u16,
    /// Checksum
    pub checksum: u16,
}

impl UdpHeader {
    /// Size of UDP header in bytes
    pub const SIZE: usize = 8;

    /// Create a new UDP header
    pub fn new(src_port: u16, dst_port: u16, length: u16) -> Self {
        Self {
            src_port,
            dst_port,
            length,
            checksum: 0,
        }
    }

    /// Calculate UDP checksum (including pseudo-header)
    pub fn calculate_checksum(
        &self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        data: &[u8],
    ) -> u16 {
        let mut sum = 0u32;

        // Pseudo-header
        sum += source_addr.to_u32() >> 16;
        sum += source_addr.to_u32() & 0xFFFF;
        sum += dest_addr.to_u32() >> 16;
        sum += dest_addr.to_u32() & 0xFFFF;
        sum += 17; // UDP protocol
        sum += self.length as u32;

        // UDP header
        sum += self.src_port as u32;
        sum += self.dst_port as u32;
        sum += self.length as u32;
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

        // Add carry
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Set checksum
    pub fn set_checksum(
        &mut self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        data: &[u8],
    ) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum(source_addr, dest_addr, data);
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.src_port.to_be_bytes());
        bytes.extend_from_slice(&self.dst_port.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, UdpError> {
        if bytes.len() < Self::SIZE {
            return Err(UdpError::PacketTooSmall);
        }

        let src_port = u16::from_be_bytes([bytes[0], bytes[1]]);
        let dst_port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let length = u16::from_be_bytes([bytes[4], bytes[5]]);
        let checksum = u16::from_be_bytes([bytes[6], bytes[7]]);

        Ok(Self {
            src_port,
            dst_port,
            length,
            checksum,
        })
    }
}

/// UDP packet
#[derive(Debug, Clone)]
pub struct UdpPacket {
    /// Header
    pub header: UdpHeader,
    /// Payload data
    pub payload: Vec<u8>,
}

impl UdpPacket {
    /// Create a new UDP packet
    pub fn new(
        src_port: u16,
        dst_port: u16,
        payload: Vec<u8>,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
    ) -> Self {
        let total_length = (UdpHeader::SIZE + payload.len()) as u16;
        let mut header = UdpHeader::new(src_port, dst_port, total_length);

        // Calculate checksum
        header.set_checksum(source_addr, dest_addr, &payload);

        Self { header, payload }
    }

    /// Get source port
    pub fn src_port(&self) -> u16 {
        self.header.src_port
    }

    /// Get destination port
    pub fn dst_port(&self) -> u16 {
        self.header.dst_port
    }

    /// Get total packet length
    pub fn len(&self) -> usize {
        self.header.length as usize
    }

    /// Get payload length
    pub fn payload_len(&self) -> usize {
        self.len() - UdpHeader::SIZE
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, UdpError> {
        if bytes.len() < UdpHeader::SIZE {
            return Err(UdpError::PacketTooSmall);
        }

        let header = UdpHeader::from_bytes(&bytes[..UdpHeader::SIZE])?;

        if bytes.len() < header.length as usize {
            return Err(UdpError::PacketTooSmall);
        }

        let payload = bytes[UdpHeader::SIZE..header.length as usize].to_vec();

        Ok(Self { header, payload })
    }

    /// Verify checksum
    pub fn verify_checksum(
        &self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
    ) -> bool {
        // If checksum is 0, it's disabled
        if self.header.checksum == 0 {
            return true;
        }

        self.header.calculate_checksum(source_addr, dest_addr, &self.payload) == 0
    }
}

/// UDP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpError {
    /// Packet too small
    PacketTooSmall,
    /// Invalid checksum
    InvalidChecksum,
    /// Port unreachable
    PortUnreachable,
}

/// Well-known UDP ports
pub mod ports {
    /// DNS
    pub const DNS: u16 = 53;
    /// DHCP Server
    pub const DHCP_SERVER: u16 = 67;
    /// DHCP Client
    pub const DHCP_CLIENT: u16 = 68;
    /// NTP
    pub const NTP: u16 = 123;
    /// SNMP
    pub const SNMP: u16 = 161;
    /// Syslog
    pub const SYSLOG: u16 = 514;
}

/// UDP socket state (simplified for connectionless protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpSocketState {
    /// Unbound socket
    Unbound,
    /// Bound to local port
    Bound,
    /// Connected to remote endpoint
    Connected,
    /// Closed
    Closed,
}

/// UDP socket information
#[derive(Debug, Clone)]
pub struct UdpSocket {
    /// Local IP address
    pub local_ip: Ipv4Addr,
    /// Local port
    pub local_port: u16,
    /// Remote IP address (for connected sockets)
    pub remote_ip: Option<Ipv4Addr>,
    /// Remote port (for connected sockets)
    pub remote_port: Option<u16>,
    /// Socket state
    pub state: UdpSocketState,
}

impl UdpSocket {
    /// Create a new UDP socket
    pub fn new() -> Self {
        Self {
            local_ip: Ipv4Addr::UNSPECIFIED,
            local_port: 0,
            remote_ip: None,
            remote_port: None,
            state: UdpSocketState::Unbound,
        }
    }

    /// Bind to local address and port
    pub fn bind(&mut self, ip: Ipv4Addr, port: u16) {
        self.local_ip = ip;
        self.local_port = port;
        self.state = UdpSocketState::Bound;
    }

    /// Connect to remote address
    pub fn connect(&mut self, ip: Ipv4Addr, port: u16) {
        self.remote_ip = Some(ip);
        self.remote_port = Some(port);
        self.state = UdpSocketState::Connected;
    }

    /// Close socket
    pub fn close(&mut self) {
        self.state = UdpSocketState::Closed;
    }

    /// Check if socket is bound
    pub fn is_bound(&self) -> bool {
        matches!(self.state, UdpSocketState::Bound | UdpSocketState::Connected)
    }

    /// Check if socket is connected
    pub fn is_connected(&self) -> bool {
        self.state == UdpSocketState::Connected
    }
}

impl Default for UdpSocket {
    fn default() -> Self {
        Self::new()
    }
}